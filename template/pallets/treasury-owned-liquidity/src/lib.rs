//! Treasury-Owned Liquidity Pallet
//!
//! Implements multi-bucket XYK pool management for TMCTOL framework.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

use frame::deps::sp_runtime;
use polkadot_sdk::sp_std;

/// Helper for benchmarking — creates assets that don't exist in benchmark context
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<AccountId> {
  fn create_asset(asset_id: u32) -> frame::deps::sp_runtime::DispatchResult;
  fn fund_account(
    who: &AccountId,
    asset: primitives::AssetKind,
    amount: u128,
  ) -> frame::deps::sp_runtime::DispatchResult;
}

#[cfg(feature = "runtime-benchmarks")]
impl<AccountId> BenchmarkHelper<AccountId> for () {
  fn create_asset(_asset_id: u32) -> frame::deps::sp_runtime::DispatchResult {
    Ok(())
  }
  fn fund_account(
    _who: &AccountId,
    _asset: primitives::AssetKind,
    _amount: u128,
  ) -> frame::deps::sp_runtime::DispatchResult {
    Ok(())
  }
}

/// Asset conversion API trait for creating and managing liquidity pools
pub trait AssetConversionApi<AccountId, Balance> {
  /// Get the pool ID for a given asset pair
  fn get_pool_id(asset1: primitives::AssetKind, asset2: primitives::AssetKind) -> Option<[u8; 32]>;

  /// Get the reserves for a given pool
  fn get_pool_reserves(pool_id: [u8; 32]) -> Option<(Balance, Balance)>;

  /// Quote price for exact tokens for tokens
  fn quote_price_exact_tokens_for_tokens(
    asset1: primitives::AssetKind,
    asset2: primitives::AssetKind,
    amount: Balance,
    include_fee: bool,
  ) -> Option<Balance>;

  /// Swap exact tokens for tokens
  fn swap_exact_tokens_for_tokens(
    who: &AccountId,
    path: sp_std::vec::Vec<primitives::AssetKind>,
    amount_in: Balance,
    min_amount_out: Balance,
    recipient: AccountId,
    keep_alive: bool,
  ) -> Result<Balance, sp_runtime::DispatchError>;

  /// Remove liquidity from a pool and return constituent assets
  fn remove_liquidity(
    who: &AccountId,
    asset1: primitives::AssetKind,
    asset2: primitives::AssetKind,
    lp_amount: Balance,
  ) -> Result<(Balance, Balance), sp_runtime::DispatchError>;

  /// Resolve LP token ID to the underlying pool pair
  fn get_pool_pair_for_lp(
    lp_token_id: u32,
  ) -> Option<(primitives::AssetKind, primitives::AssetKind)>;

  /// Initialize LP token namespace for clean-slate deployments
  fn initialize_lp_asset_namespace();
}

#[frame::pallet]
pub mod pallet {
  use super::{AssetConversionApi, WeightInfo};
  use alloc::vec::Vec;
  use frame::deps::{
    frame_support::traits::{
      Currency,
      fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
      fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
      tokens::Preservation,
    },
    sp_runtime::{
      DispatchError,
      traits::{AccountIdConversion, Zero},
    },
  };
  use frame::prelude::*;
  use primitives::{AssetInspector, AssetKind};

  /// Configuration trait for the treasury owned liquidity pallet
  #[pallet::config]
  pub trait Config: frame_system::Config {
    /// The assets pallet for managing fungible tokens (Local)
    type Assets: FungiblesInspect<Self::AccountId, AssetId = u32, Balance = u128>
      + FungiblesMutate<Self::AccountId, AssetId = u32, Balance = u128>;

    /// The currency trait for managing native tokens (AssetKind::Native)
    type Currency: Currency<Self::AccountId>
      + NativeInspect<Self::AccountId, Balance = u128>
      + NativeMutate<Self::AccountId, Balance = u128>;

    /// Compatibility hook for a treasury destination account
    /// Primary LP ingress for TOL remains the pallet account (`PalletId`)
    #[pallet::constant]
    type TreasuryAccount: Get<Self::AccountId>;

    /// The pallet ID for the TOL
    #[pallet::constant]
    type PalletId: Get<PalletId>;

    /// Precision for calculations
    #[pallet::constant]
    type Precision: Get<u128>;

    /// Allocation percentage for Bucket A (Anchor) in PPM
    #[pallet::constant]
    type BucketAAllocation: Get<u32>;

    /// Allocation percentage for Bucket B (Building) in PPM
    #[pallet::constant]
    type BucketBAllocation: Get<u32>;

    /// Allocation percentage for Bucket C (Capital) in PPM
    #[pallet::constant]
    type BucketCAllocation: Get<u32>;

    /// Allocation percentage for Bucket D (Dormant) in PPM
    #[pallet::constant]
    type BucketDAllocation: Get<u32>;

    /// Account for Bucket A (Anchor)
    #[pallet::constant]
    type BucketAAccount: Get<Self::AccountId>;

    /// Account for Bucket B (Building)
    #[pallet::constant]
    type BucketBAccount: Get<Self::AccountId>;

    /// Account for Bucket C (Capital)
    #[pallet::constant]
    type BucketCAccount: Get<Self::AccountId>;

    /// Account for Bucket D (Dormant)
    #[pallet::constant]
    type BucketDAccount: Get<Self::AccountId>;

    /// Zap Manager account
    #[pallet::constant]
    type ZapManagerAccount: Get<Self::AccountId>;

    /// Burning Manager account
    #[pallet::constant]
    type BurningManagerAccount: Get<Self::AccountId>;

    /// Bucket A (Anchor) ceiling ratio (e.g., 110%)
    #[pallet::constant]
    type BucketARatio: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Bucket B (Building) ceiling ratio (e.g., 125%)
    #[pallet::constant]
    type BucketBRatio: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Bucket C (Capital) ceiling ratio (e.g., 150%)
    #[pallet::constant]
    type BucketCRatio: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Asset Conversion Adapter
    type AssetConversion: AssetConversionApi<Self::AccountId, u128>;

    /// Minimum swap amount for foreign tokens
    #[pallet::constant]
    type MinSwapForeign: Get<u128>;

    /// Maximum allowed price deviation for oracle validation
    #[pallet::constant]
    type MaxPriceDeviation: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Origin that can initialize and manage TOL domain configurations
    type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Weight information for extrinsics
    type WeightInfo: WeightInfo;

    /// Maximum number of TOL requests processed per block
    #[pallet::constant]
    type MaxTolRequestsPerBlock: Get<u32>;

    /// Maximum number of non-LP bucket sweep transfers processed per idle block
    #[pallet::constant]
    type MaxNonLpSweepsPerBlock: Get<u32>;

    /// Maximum number of active TOL domains
    #[pallet::constant]
    type MaxTolDomains: Get<u32>;

    /// Benchmark helper for creating assets in benchmark context
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper: crate::BenchmarkHelper<Self::AccountId>;
  }

  #[pallet::pallet]
  pub struct Pallet<T>(_);

  pub type Balance = u128;
  pub type TolId = u32;
  const DEFAULT_TOL_ID: TolId = 0;

  /// TOL domain configuration
  #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
  pub struct TolConfig {
    /// The asset this TOL manages
    pub token_asset: AssetKind,
    /// The foreign asset used for liquidity (usually Native or Stable)
    pub foreign_asset: AssetKind,
    /// Total allocation amount
    pub total_tol_allocation: Balance,
    /// Current circulating supply tracked by TOL
    pub current_tol_supply: Balance,
  }

  /// Lifecycle action emitted when ensuring token-domain state
  #[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    Eq,
    PartialEq,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
  )]
  pub enum DomainEnsureAction {
    /// Domain was created for the token
    Created,
    /// Existing domain/binding was updated
    Rebound,
    /// Domain already matched the requested state
    Noop,
  }

  /// Allocation state for a bucket
  #[derive(
    Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default,
  )]
  pub struct BucketAllocation {
    /// Target allocation in PPM
    pub target_allocation_ppm: u32,
    /// Current native reserves
    pub native_reserves: Balance,
    /// Current foreign reserves
    pub foreign_reserves: Balance,
    /// LP tokens held
    pub lp_tokens: Balance,
  }

  /// Buffer for pending zaps
  #[derive(
    Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Default,
  )]
  pub struct ZapBuffer {
    /// Pending native tokens
    pub pending_native: Balance,
    /// Pending foreign tokens
    pub pending_foreign: Balance,
  }

  /// Request for TOL allocation
  #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
  pub struct TolAllocationRequest {
    /// Total native amount
    pub total_native: Balance,
    /// Total foreign amount
    pub total_foreign: Balance,
  }

  /// Result of a batch zap operation
  #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
  pub struct BatchZapResult {
    pub token_asset: AssetKind,
    pub events_processed: u32,
    pub native_used: Balance,
    pub foreign_used: Balance,
    pub lp_minted: Balance,
    pub leftover_native: Balance,
    pub leftover_foreign: Balance,
  }

  /// Result of a single zap operation
  #[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
  pub struct ZapResult {
    pub native_used: Balance,
    pub foreign_used: Balance,
    pub lp_minted: Balance,
    pub leftover_native: Balance,
    pub leftover_foreign: Balance,
  }

  /// TOL configuration by domain ID
  #[pallet::storage]
  #[pallet::getter(fn tol_configuration)]
  pub type TolConfigurations<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, TolConfig, OptionQuery>;

  /// Active TOL domains
  #[pallet::storage]
  #[pallet::getter(fn active_tol_domains)]
  pub type ActiveTolDomains<T: Config> =
    StorageValue<_, BoundedVec<TolId, T::MaxTolDomains>, ValueQuery>;

  /// Token to TOL domain binding
  #[pallet::storage]
  #[pallet::getter(fn token_tol_binding)]
  pub type TokenTolBindings<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, TolId, OptionQuery>;

  /// Bucket A allocation (Anchor) by domain
  #[pallet::storage]
  #[pallet::getter(fn bucket_a)]
  pub type BucketA<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, BucketAllocation, ValueQuery>;

  /// Bucket B allocation (Building) by domain
  #[pallet::storage]
  #[pallet::getter(fn bucket_b)]
  pub type BucketB<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, BucketAllocation, ValueQuery>;

  /// Bucket C allocation (Capital) by domain
  #[pallet::storage]
  #[pallet::getter(fn bucket_c)]
  pub type BucketC<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, BucketAllocation, ValueQuery>;

  /// Bucket D allocation (Dormant) by domain
  #[pallet::storage]
  #[pallet::getter(fn bucket_d)]
  pub type BucketD<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, BucketAllocation, ValueQuery>;

  /// Zap buffer by domain
  #[pallet::storage]
  #[pallet::getter(fn zap_buffer)]
  pub type ZapBufferState<T: Config> =
    StorageMap<_, Blake2_128Concat, TolId, ZapBuffer, ValueQuery>;

  /// Pending allocation requests by domain
  #[pallet::storage]
  #[pallet::getter(fn pending_allocation_requests)]
  pub type PendingRequests<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    TolId,
    BoundedVec<TolAllocationRequest, T::MaxTolRequestsPerBlock>,
    ValueQuery,
  >;

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// TOL domain configuration created
    TolCreated {
      tol_id: TolId,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    },
    /// Bucket allocation updated
    BucketAllocationUpdated {
      tol_id: TolId,
      token_asset: AssetKind,
      bucket_id: u8,
      new_allocation_ppm: u32,
    },
    /// Token bound to a TOL domain
    TokenTolBound {
      token_asset: AssetKind,
      tol_id: TolId,
    },
    /// Token-domain lifecycle ensure was executed
    TokenDomainEnsured {
      token_asset: AssetKind,
      tol_id: TolId,
      action: DomainEnsureAction,
      previous_foreign_asset: Option<AssetKind>,
      foreign_asset: AssetKind,
    },
    /// Zap buffer updated
    ZapBufferUpdated {
      tol_id: TolId,
      token_asset: AssetKind,
      pending_native: Balance,
      pending_foreign: Balance,
    },
    /// Treasury withdrawal
    TreasuryWithdraw {
      tol_id: TolId,
      asset: AssetKind,
      amount: Balance,
      destination: T::AccountId,
    },
    /// TOL requests processed
    TolRequestsProcessed { tol_id: TolId, count: u32 },
    /// LP tokens received from Zap Manager
    LPTokensReceived {
      tol_id: TolId,
      lp_asset_id: AssetKind,
      lp_amount: Balance,
      distributed_block: BlockNumberFor<T>,
    },
    /// LP tokens distributed to buckets
    LPTokensDistributed {
      tol_id: TolId,
      lp_asset_id: AssetKind,
      bucket_a_amount: Balance,
      bucket_b_amount: Balance,
      bucket_c_amount: Balance,
      bucket_d_amount: Balance,
      total_amount: Balance,
    },
    /// LP liquidity was manually unwound from a specific bucket
    BucketLiquidityUnwound {
      tol_id: TolId,
      token_asset: AssetKind,
      bucket_id: u8,
      lp_asset: AssetKind,
      lp_amount: Balance,
      native_out: Balance,
      foreign_out: Balance,
      destination: T::AccountId,
    },
    /// Non-LP asset swept from bucket account to Burning Manager
    NonLpAssetSwept {
      tol_id: TolId,
      bucket_id: u8,
      asset: AssetKind,
      amount: Balance,
      destination: T::AccountId,
    },
  }

  #[pallet::error]
  pub enum Error<T> {
    /// TOL configuration for domain already exists
    TolAlreadyExists,
    /// TOL configuration does not exist
    NoTolExists,
    /// TOL domain is not configured
    TolDomainNotFound,
    /// Too many active TOL domains
    TooManyTolDomains,
    /// Invalid allocation parameters
    InvalidAllocation,
    /// Treasury balance too low
    InsufficientTreasuryBalance,
    /// Arithmetic overflow
    ArithmeticOverflow,
    /// Invalid bucket type
    InvalidBucketType,
    /// Insufficient liquidity
    InsufficientLiquidity,
    /// Zap slippage exceeded
    ZapSlippageExceeded,
    /// Asset conversion error
    AssetConversionError,
    /// Invalid asset
    InvalidAsset,
    /// Bucket A Anchor is immutable and cannot be unwound
    BucketAUnwindDisabled,
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Initialize TOL configuration for a domain
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::create_tol())]
    pub fn create_tol(
      origin: OriginFor<T>,
      tol_id: TolId,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;
      Self::create_tol_for_domain(tol_id, token_asset, foreign_asset, total_allocation)
    }

    /// Update bucket allocation for a domain
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::update_bucket_allocation())]
    pub fn update_bucket_allocation(
      origin: OriginFor<T>,
      tol_id: TolId,
      bucket_id: u8,
      new_allocation_ppm: u32,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;
      let config = TolConfigurations::<T>::get(tol_id).ok_or(Error::<T>::TolDomainNotFound)?;
      match bucket_id {
        0 => BucketA::<T>::mutate(tol_id, |b| b.target_allocation_ppm = new_allocation_ppm),
        1 => BucketB::<T>::mutate(tol_id, |b| b.target_allocation_ppm = new_allocation_ppm),
        2 => BucketC::<T>::mutate(tol_id, |b| b.target_allocation_ppm = new_allocation_ppm),
        3 => BucketD::<T>::mutate(tol_id, |b| b.target_allocation_ppm = new_allocation_ppm),
        _ => return Err(Error::<T>::InvalidBucketType.into()),
      }
      Self::deposit_event(Event::BucketAllocationUpdated {
        tol_id,
        token_asset: config.token_asset,
        bucket_id,
        new_allocation_ppm,
      });
      Ok(())
    }

    /// Receive mint allocation from TMC (called by ZapManager)
    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::receive_mint_allocation())]
    pub fn receive_mint_allocation(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      total_native: Balance,
      total_foreign: Balance,
    ) -> DispatchResult {
      let who = ensure_signed(origin)?;
      ensure!(who == T::ZapManagerAccount::get(), DispatchError::BadOrigin);
      let tol_id = Self::tol_id_for_token(token_asset).unwrap_or(DEFAULT_TOL_ID);
      Self::add_allocation_request_for_tol(tol_id, total_native, total_foreign)?;
      Ok(())
    }

    /// Receive LP tokens from Zap Manager
    #[pallet::call_index(3)]
    #[pallet::weight(T::WeightInfo::receive_lp_tokens())]
    pub fn receive_lp_tokens(
      origin: OriginFor<T>,
      lp_asset: AssetKind,
      lp_amount: Balance,
    ) -> DispatchResult {
      let who = ensure_signed(origin)?;
      ensure!(who == T::ZapManagerAccount::get(), DispatchError::BadOrigin);
      ensure!(Self::is_lp_asset(lp_asset), Error::<T>::InvalidAsset);
      // Distribute LP tokens to buckets based on domain configuration
      let tol_id = Self::distribute_lp_tokens_to_buckets(lp_asset, lp_amount)?;
      Self::deposit_event(Event::LPTokensReceived {
        tol_id,
        lp_asset_id: lp_asset,
        lp_amount,
        distributed_block: <frame_system::Pallet<T>>::block_number(),
      });
      Ok(())
    }

    /// Withdraw buffer from a domain ingress account
    #[pallet::call_index(4)]
    #[pallet::weight(T::WeightInfo::withdraw_buffer())]
    pub fn withdraw_buffer(
      origin: OriginFor<T>,
      tol_id: TolId,
      asset: AssetKind,
      amount: Balance,
      destination: T::AccountId,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;
      ensure!(
        TolConfigurations::<T>::contains_key(tol_id),
        Error::<T>::TolDomainNotFound
      );
      let tol_account = Self::ingress_account_for_tol_id(tol_id);
      Self::transfer_asset(
        &tol_account,
        &destination,
        asset,
        amount,
        Preservation::Preserve,
      )?;
      Self::deposit_event(Event::TreasuryWithdraw {
        tol_id,
        asset,
        amount,
        destination,
      });
      Ok(())
    }

    /// Manually unwind LP liquidity from a non-anchor bucket.
    ///
    /// This is a governance-only emergency/operations path until automated DripVault
    /// streaming for buckets B/C is introduced.
    #[pallet::call_index(5)]
    #[pallet::weight(T::WeightInfo::withdraw_buffer())]
    pub fn unwind_bucket_liquidity(
      origin: OriginFor<T>,
      bucket_id: u8,
      lp_asset: AssetKind,
      lp_amount: Balance,
      destination: T::AccountId,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;
      ensure!(bucket_id != 0, Error::<T>::BucketAUnwindDisabled);
      ensure!(lp_amount > 0, Error::<T>::InvalidAllocation);
      ensure!(Self::is_lp_asset(lp_asset), Error::<T>::InvalidAsset);
      let tol_id = Self::resolve_tol_id_for_lp_asset(lp_asset)?;
      let config = TolConfigurations::<T>::get(tol_id).ok_or(Error::<T>::TolDomainNotFound)?;
      let mut bucket = Self::get_bucket_allocation(tol_id, bucket_id)?;
      ensure!(
        bucket.lp_tokens >= lp_amount,
        Error::<T>::InsufficientLiquidity
      );
      let lp_token_id = match lp_asset {
        AssetKind::Local(id) | AssetKind::Foreign(id) => id,
        AssetKind::Native => return Err(Error::<T>::InvalidAsset.into()),
      };
      let (asset1, asset2) = T::AssetConversion::get_pool_pair_for_lp(lp_token_id)
        .ok_or(Error::<T>::AssetConversionError)?;
      let bucket_account = Self::bucket_account_for_tol_id(tol_id, bucket_id)?;
      let (amount1, amount2) =
        T::AssetConversion::remove_liquidity(&bucket_account, asset1, asset2, lp_amount)
          .map_err(|_| Error::<T>::AssetConversionError)?;
      Self::transfer_asset(
        &bucket_account,
        &destination,
        asset1,
        amount1,
        Preservation::Expendable,
      )?;
      Self::transfer_asset(
        &bucket_account,
        &destination,
        asset2,
        amount2,
        Preservation::Expendable,
      )?;
      let (native_out, foreign_out) =
        Self::classify_unwound_assets(asset1, amount1, asset2, amount2);
      bucket.lp_tokens = bucket.lp_tokens.saturating_sub(lp_amount);
      bucket.native_reserves = bucket.native_reserves.saturating_sub(native_out);
      bucket.foreign_reserves = bucket.foreign_reserves.saturating_sub(foreign_out);
      Self::set_bucket_allocation(tol_id, bucket_id, bucket)?;
      Self::deposit_event(Event::BucketLiquidityUnwound {
        tol_id,
        token_asset: config.token_asset,
        bucket_id,
        lp_asset,
        lp_amount,
        native_out,
        foreign_out,
        destination,
      });
      Ok(())
    }

    /// Bind token asset to a TOL domain ID
    #[pallet::call_index(6)]
    #[pallet::weight(T::WeightInfo::update_bucket_allocation())]
    pub fn bind_token_to_tol(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      tol_id: TolId,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;
      ensure!(
        TolConfigurations::<T>::contains_key(tol_id),
        Error::<T>::TolDomainNotFound
      );
      TokenTolBindings::<T>::insert(token_asset, tol_id);
      Self::ensure_domain_accounts_initialized(tol_id);
      Self::deposit_event(Event::TokenTolBound {
        token_asset,
        tol_id,
      });
      Ok(())
    }
  }

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
      // Process pending TOL allocation requests
      Self::process_tol_allocation_requests()
    }

    fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
      Self::sweep_non_lp_bucket_assets(remaining_weight)
    }
  }

  impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    pub fn ingress_account_for_tol_id(tol_id: TolId) -> T::AccountId {
      if tol_id == DEFAULT_TOL_ID {
        Self::account_id()
      } else {
        T::PalletId::get().into_sub_account_truncating((*b"tolins", tol_id))
      }
    }

    fn bucket_account_for_tol_id(
      tol_id: TolId,
      bucket_id: u8,
    ) -> Result<T::AccountId, DispatchError> {
      if tol_id == DEFAULT_TOL_ID {
        return match bucket_id {
          0 => Ok(T::BucketAAccount::get()),
          1 => Ok(T::BucketBAccount::get()),
          2 => Ok(T::BucketCAccount::get()),
          3 => Ok(T::BucketDAccount::get()),
          _ => Err(Error::<T>::InvalidBucketType.into()),
        };
      }
      let account = match bucket_id {
        0 => T::PalletId::get().into_sub_account_truncating((*b"tolba", tol_id)),
        1 => T::PalletId::get().into_sub_account_truncating((*b"tolbb", tol_id)),
        2 => T::PalletId::get().into_sub_account_truncating((*b"tolbc", tol_id)),
        3 => T::PalletId::get().into_sub_account_truncating((*b"tolbd", tol_id)),
        _ => return Err(Error::<T>::InvalidBucketType.into()),
      };
      Ok(account)
    }

    fn ensure_domain_accounts_initialized(tol_id: TolId) {
      if tol_id == DEFAULT_TOL_ID {
        return;
      }
      frame_system::Pallet::<T>::inc_providers(&Self::ingress_account_for_tol_id(tol_id));
      for bucket_id in 0..4 {
        if let Ok(account) = Self::bucket_account_for_tol_id(tol_id, bucket_id) {
          frame_system::Pallet::<T>::inc_providers(&account);
        }
      }
    }

    fn register_active_tol_domain(tol_id: TolId) -> DispatchResult {
      ActiveTolDomains::<T>::try_mutate(|domains| {
        if domains.contains(&tol_id) {
          return Ok(());
        }
        domains
          .try_push(tol_id)
          .map_err(|_| Error::<T>::TooManyTolDomains.into())
      })
    }

    fn create_tol_for_domain(
      tol_id: TolId,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    ) -> DispatchResult {
      ensure!(
        TolConfigurations::<T>::get(tol_id).is_none(),
        Error::<T>::TolAlreadyExists
      );
      Self::register_active_tol_domain(tol_id)?;
      let config = TolConfig {
        token_asset,
        foreign_asset,
        total_tol_allocation: total_allocation,
        current_tol_supply: Zero::zero(),
      };
      TolConfigurations::<T>::insert(tol_id, config);
      BucketA::<T>::insert(
        tol_id,
        BucketAllocation {
          target_allocation_ppm: T::BucketAAllocation::get(),
          ..Default::default()
        },
      );
      BucketB::<T>::insert(
        tol_id,
        BucketAllocation {
          target_allocation_ppm: T::BucketBAllocation::get(),
          ..Default::default()
        },
      );
      BucketC::<T>::insert(
        tol_id,
        BucketAllocation {
          target_allocation_ppm: T::BucketCAllocation::get(),
          ..Default::default()
        },
      );
      BucketD::<T>::insert(
        tol_id,
        BucketAllocation {
          target_allocation_ppm: T::BucketDAllocation::get(),
          ..Default::default()
        },
      );
      if TokenTolBindings::<T>::get(token_asset).is_none() {
        TokenTolBindings::<T>::insert(token_asset, tol_id);
      }
      Self::ensure_domain_accounts_initialized(tol_id);
      Self::deposit_event(Event::TolCreated {
        tol_id,
        token_asset,
        foreign_asset,
        total_allocation,
      });
      Ok(())
    }

    pub fn default_tol_id_for_token(token_asset: AssetKind) -> Option<TolId> {
      match token_asset {
        AssetKind::Native => Some(DEFAULT_TOL_ID),
        AssetKind::Local(_) if token_asset.is_lp() => None,
        AssetKind::Local(id) | AssetKind::Foreign(id) => Some(id),
      }
    }

    pub fn ensure_domain_for_token(
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    ) -> DispatchResult {
      let default_tol_id =
        Self::default_tol_id_for_token(token_asset).ok_or(Error::<T>::InvalidAsset)?;
      ensure!(!Self::is_lp_asset(token_asset), Error::<T>::InvalidAsset);
      let existing_binding = TokenTolBindings::<T>::get(token_asset);
      let target_tol_id = existing_binding.unwrap_or(default_tol_id);
      if let Some(mut config) = TolConfigurations::<T>::get(target_tol_id) {
        ensure!(
          config.token_asset == token_asset,
          Error::<T>::TolAlreadyExists
        );
        let mut action = DomainEnsureAction::Noop;
        let previous_foreign_asset = Some(config.foreign_asset);
        if config.foreign_asset != foreign_asset {
          config.foreign_asset = foreign_asset;
          TolConfigurations::<T>::insert(target_tol_id, config);
          action = DomainEnsureAction::Rebound;
        }
        if existing_binding.is_none() {
          TokenTolBindings::<T>::insert(token_asset, target_tol_id);
          if action == DomainEnsureAction::Noop {
            action = DomainEnsureAction::Rebound;
          }
        }
        Self::register_active_tol_domain(target_tol_id)?;
        Self::ensure_domain_accounts_initialized(target_tol_id);
        Self::deposit_event(Event::TokenDomainEnsured {
          token_asset,
          tol_id: target_tol_id,
          action,
          previous_foreign_asset,
          foreign_asset,
        });
        return Ok(());
      }
      if existing_binding.is_some() {
        return Err(Error::<T>::TolDomainNotFound.into());
      }
      Self::create_tol_for_domain(default_tol_id, token_asset, foreign_asset, total_allocation)?;
      Self::deposit_event(Event::TokenDomainEnsured {
        token_asset,
        tol_id: default_tol_id,
        action: DomainEnsureAction::Created,
        previous_foreign_asset: None,
        foreign_asset,
      });
      Ok(())
    }

    pub fn tol_id_for_token(token_asset: AssetKind) -> Option<TolId> {
      TokenTolBindings::<T>::get(token_asset)
    }

    pub fn ingress_account_for_token(token_asset: AssetKind) -> T::AccountId {
      let tol_id = Self::tol_id_for_token(token_asset).unwrap_or(DEFAULT_TOL_ID);
      Self::ingress_account_for_tol_id(tol_id)
    }

    pub fn ingress_account_for_lp_asset(lp_asset: AssetKind) -> Option<T::AccountId> {
      Self::resolve_lp_ingress_account(lp_asset).ok()
    }

    pub fn resolve_tol_id_for_lp_asset(lp_asset: AssetKind) -> Result<TolId, DispatchError> {
      let lp_token_id = match lp_asset {
        AssetKind::Local(id) | AssetKind::Foreign(id) => id,
        AssetKind::Native => return Err(Error::<T>::InvalidAsset.into()),
      };
      let (asset_a, asset_b) = T::AssetConversion::get_pool_pair_for_lp(lp_token_id)
        .ok_or(Error::<T>::AssetConversionError)?;
      for asset in [asset_a, asset_b] {
        if let Some(tol_id) = TokenTolBindings::<T>::get(asset) {
          return Ok(tol_id);
        }
      }
      if TolConfigurations::<T>::contains_key(DEFAULT_TOL_ID) {
        return Ok(DEFAULT_TOL_ID);
      }
      Err(Error::<T>::NoTolExists.into())
    }

    fn resolve_lp_ingress_account(lp_asset: AssetKind) -> Result<T::AccountId, DispatchError> {
      let tol_id = Self::resolve_tol_id_for_lp_asset(lp_asset)?;
      Ok(Self::ingress_account_for_tol_id(tol_id))
    }

    fn get_bucket_allocation(
      tol_id: TolId,
      bucket_id: u8,
    ) -> Result<BucketAllocation, DispatchError> {
      match bucket_id {
        0 => Ok(BucketA::<T>::get(tol_id)),
        1 => Ok(BucketB::<T>::get(tol_id)),
        2 => Ok(BucketC::<T>::get(tol_id)),
        3 => Ok(BucketD::<T>::get(tol_id)),
        _ => Err(Error::<T>::InvalidBucketType.into()),
      }
    }

    fn set_bucket_allocation(
      tol_id: TolId,
      bucket_id: u8,
      allocation: BucketAllocation,
    ) -> DispatchResult {
      match bucket_id {
        0 => BucketA::<T>::insert(tol_id, allocation),
        1 => BucketB::<T>::insert(tol_id, allocation),
        2 => BucketC::<T>::insert(tol_id, allocation),
        3 => BucketD::<T>::insert(tol_id, allocation),
        _ => return Err(Error::<T>::InvalidBucketType.into()),
      }
      Ok(())
    }

    fn classify_unwound_assets(
      asset1: AssetKind,
      amount1: Balance,
      asset2: AssetKind,
      amount2: Balance,
    ) -> (Balance, Balance) {
      let mut native_out: Balance = 0;
      let mut foreign_out: Balance = 0;
      let mut classify = |asset: AssetKind, amount: Balance| {
        if amount == 0 {
          return;
        }
        match asset {
          AssetKind::Native => native_out = native_out.saturating_add(amount),
          AssetKind::Local(_) | AssetKind::Foreign(_) => {
            foreign_out = foreign_out.saturating_add(amount)
          }
        }
      };
      classify(asset1, amount1);
      classify(asset2, amount2);
      (native_out, foreign_out)
    }

    fn transfer_asset(
      from: &T::AccountId,
      to: &T::AccountId,
      asset: AssetKind,
      amount: Balance,
      preservation: Preservation,
    ) -> DispatchResult {
      if amount.is_zero() {
        return Ok(());
      }
      match asset {
        AssetKind::Native => {
          <T::Currency as NativeMutate<T::AccountId>>::transfer(from, to, amount, preservation)
            .map(|_| ())
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::transfer(id, from, to, amount, preservation).map(|_| ())
        }
      }
    }

    fn asset_balance(who: &T::AccountId, asset: AssetKind) -> Balance {
      match asset {
        AssetKind::Native => <T::Currency as NativeInspect<T::AccountId>>::balance(who),
        AssetKind::Local(id) | AssetKind::Foreign(id) => T::Assets::balance(id, who),
      }
    }

    fn is_lp_asset(asset: AssetKind) -> bool {
      match asset {
        AssetKind::Native => false,
        AssetKind::Local(_) | AssetKind::Foreign(_) => asset.is_lp(),
      }
    }

    fn tracked_non_lp_assets(config: &TolConfig) -> Vec<AssetKind> {
      let mut assets = Vec::from([config.token_asset]);
      if config.foreign_asset != config.token_asset {
        assets.push(config.foreign_asset);
      }
      if !assets
        .iter()
        .any(|asset| matches!(asset, AssetKind::Native))
      {
        assets.push(AssetKind::Native);
      }
      assets
        .into_iter()
        .filter(|asset| !Self::is_lp_asset(*asset))
        .collect()
    }

    fn bucket_accounts_with_ids_for_tol(
      tol_id: TolId,
    ) -> Result<[(u8, T::AccountId); 4], DispatchError> {
      Ok([
        (0, Self::bucket_account_for_tol_id(tol_id, 0)?),
        (1, Self::bucket_account_for_tol_id(tol_id, 1)?),
        (2, Self::bucket_account_for_tol_id(tol_id, 2)?),
        (3, Self::bucket_account_for_tol_id(tol_id, 3)?),
      ])
    }

    pub fn sweep_non_lp_bucket_assets(remaining_weight: Weight) -> Weight {
      let max_sweeps = T::MaxNonLpSweepsPerBlock::get();
      if max_sweeps == 0 || remaining_weight.is_zero() {
        return Weight::zero();
      }
      let inspect_weight = T::DbWeight::get().reads(1);
      let transfer_weight = T::DbWeight::get().reads_writes(2, 2);
      let mut weight_used = Weight::zero();
      let burning_manager_account = T::BurningManagerAccount::get();
      let mut sweeps_done: u32 = 0;
      let domains = ActiveTolDomains::<T>::get();
      'domains: for tol_id in domains.into_iter() {
        let config = match TolConfigurations::<T>::get(tol_id) {
          Some(config) => config,
          None => continue,
        };
        let assets = Self::tracked_non_lp_assets(&config);
        let bucket_accounts = match Self::bucket_accounts_with_ids_for_tol(tol_id) {
          Ok(accounts) => accounts,
          Err(_) => continue,
        };
        for asset in assets {
          for (bucket_id, bucket_account) in bucket_accounts.iter() {
            if sweeps_done >= max_sweeps {
              break 'domains;
            }
            if weight_used
              .saturating_add(inspect_weight)
              .any_gt(remaining_weight)
            {
              break 'domains;
            }
            let amount = Self::asset_balance(bucket_account, asset);
            weight_used = weight_used.saturating_add(inspect_weight);
            if amount.is_zero() {
              continue;
            }
            if weight_used
              .saturating_add(transfer_weight)
              .any_gt(remaining_weight)
            {
              break 'domains;
            }
            if Self::transfer_asset(
              bucket_account,
              &burning_manager_account,
              asset,
              amount,
              Preservation::Expendable,
            )
            .is_ok()
            {
              Self::deposit_event(Event::NonLpAssetSwept {
                tol_id,
                bucket_id: *bucket_id,
                asset,
                amount,
                destination: burning_manager_account.clone(),
              });
              sweeps_done = sweeps_done.saturating_add(1);
            }
            weight_used = weight_used.saturating_add(transfer_weight);
          }
        }
      }
      weight_used
    }

    pub fn add_allocation_request_for_tol(
      tol_id: TolId,
      total_native: Balance,
      total_foreign: Balance,
    ) -> DispatchResult {
      TolConfigurations::<T>::get(tol_id).ok_or(Error::<T>::TolDomainNotFound)?;
      PendingRequests::<T>::try_mutate(tol_id, |requests| {
        if requests.len() >= T::MaxTolRequestsPerBlock::get() as usize {
          return Err(Error::<T>::InvalidAllocation.into());
        }
        requests
          .try_push(TolAllocationRequest {
            total_native,
            total_foreign,
          })
          .map_err(|_| Error::<T>::InvalidAllocation.into())
      })
    }

    pub fn process_tol_allocation_requests() -> Weight {
      let mut weight = Weight::zero();
      for tol_id in ActiveTolDomains::<T>::get().into_iter() {
        let requests = PendingRequests::<T>::take(tol_id);
        for request in requests.iter() {
          let _ =
            Self::add_to_zap_buffer_for_tol(tol_id, request.total_native, request.total_foreign);
          weight = weight.saturating_add(T::WeightInfo::receive_mint_allocation());
        }
        if !requests.is_empty() {
          Self::deposit_event(Event::TolRequestsProcessed {
            tol_id,
            count: requests.len() as u32,
          });
        }
      }
      weight
    }

    pub fn add_to_zap_buffer_for_tol(
      tol_id: TolId,
      native_amount: Balance,
      foreign_amount: Balance,
    ) -> DispatchResult {
      let config = TolConfigurations::<T>::get(tol_id).ok_or(Error::<T>::TolDomainNotFound)?;
      ZapBufferState::<T>::mutate(tol_id, |buffer| {
        buffer.pending_native = buffer.pending_native.saturating_add(native_amount);
        buffer.pending_foreign = buffer.pending_foreign.saturating_add(foreign_amount);
      });
      let buffer = ZapBufferState::<T>::get(tol_id);
      Self::deposit_event(Event::ZapBufferUpdated {
        tol_id,
        token_asset: config.token_asset,
        pending_native: buffer.pending_native,
        pending_foreign: buffer.pending_foreign,
      });
      Ok(())
    }

    pub fn add_allocation_request(total_native: Balance, total_foreign: Balance) -> DispatchResult {
      Self::add_allocation_request_for_tol(DEFAULT_TOL_ID, total_native, total_foreign)
    }

    pub fn add_to_zap_buffer(native_amount: Balance, foreign_amount: Balance) -> DispatchResult {
      Self::add_to_zap_buffer_for_tol(DEFAULT_TOL_ID, native_amount, foreign_amount)
    }

    fn split_amount_by_ppm(
      total_amount: Balance,
      alloc_a: u32,
      alloc_b: u32,
      alloc_c: u32,
    ) -> (Balance, Balance, Balance, Balance) {
      let total_ppm = 1_000_000u128;
      let amount_a = total_amount.saturating_mul(alloc_a as u128) / total_ppm;
      let amount_b = total_amount.saturating_mul(alloc_b as u128) / total_ppm;
      let amount_c = total_amount.saturating_mul(alloc_c as u128) / total_ppm;
      let amount_d = total_amount
        .saturating_sub(amount_a)
        .saturating_sub(amount_b)
        .saturating_sub(amount_c);

      (amount_a, amount_b, amount_c, amount_d)
    }

    fn estimate_reserves_for_lp_distribution(
      lp_asset: AssetKind,
      lp_amount: Balance,
    ) -> Option<(Balance, Balance)> {
      let lp_token_id = match lp_asset {
        AssetKind::Local(id) | AssetKind::Foreign(id) => id,
        AssetKind::Native => return None,
      };
      let (asset_a, asset_b) = T::AssetConversion::get_pool_pair_for_lp(lp_token_id)?;
      let pool_id = T::AssetConversion::get_pool_id(asset_a, asset_b)?;
      let (reserve_a, reserve_b) = T::AssetConversion::get_pool_reserves(pool_id)?;
      let total_lp_supply = T::Assets::total_issuance(lp_token_id);
      if total_lp_supply.is_zero() {
        return None;
      }
      let (pool_native, pool_foreign) = if asset_a == AssetKind::Native {
        (reserve_a, reserve_b)
      } else if asset_b == AssetKind::Native {
        (reserve_b, reserve_a)
      } else {
        return None;
      };
      let native_for_distribution = lp_amount.saturating_mul(pool_native) / total_lp_supply;
      let foreign_for_distribution = lp_amount.saturating_mul(pool_foreign) / total_lp_supply;
      Some((native_for_distribution, foreign_for_distribution))
    }

    pub fn distribute_lp_tokens_to_buckets(
      lp_asset: AssetKind,
      amount: Balance,
    ) -> Result<TolId, DispatchError> {
      ensure!(Self::is_lp_asset(lp_asset), Error::<T>::InvalidAsset);
      let tol_id = Self::resolve_tol_id_for_lp_asset(lp_asset)?;
      ensure!(
        TolConfigurations::<T>::contains_key(tol_id),
        Error::<T>::TolDomainNotFound
      );
      // Use storage-backed allocations so governance updates affect real distribution.
      let alloc_a = BucketA::<T>::get(tol_id).target_allocation_ppm;
      let alloc_b = BucketB::<T>::get(tol_id).target_allocation_ppm;
      let alloc_c = BucketC::<T>::get(tol_id).target_allocation_ppm;
      ensure!(
        (alloc_a as u128)
          .saturating_add(alloc_b as u128)
          .saturating_add(alloc_c as u128)
          <= 1_000_000u128,
        Error::<T>::InvalidAllocation
      );
      // Bucket D gets the remainder for strict conservation.
      let (amount_a, amount_b, amount_c, amount_d) =
        Self::split_amount_by_ppm(amount, alloc_a, alloc_b, alloc_c);
      let (native_total, foreign_total) =
        Self::estimate_reserves_for_lp_distribution(lp_asset, amount).unwrap_or((0, 0));
      let (native_a, native_b, native_c, native_d) =
        Self::split_amount_by_ppm(native_total, alloc_a, alloc_b, alloc_c);
      let (foreign_a, foreign_b, foreign_c, foreign_d) =
        Self::split_amount_by_ppm(foreign_total, alloc_a, alloc_b, alloc_c);
      // Update bucket state
      BucketA::<T>::mutate(tol_id, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_a);
        b.native_reserves = b.native_reserves.saturating_add(native_a);
        b.foreign_reserves = b.foreign_reserves.saturating_add(foreign_a);
      });
      BucketB::<T>::mutate(tol_id, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_b);
        b.native_reserves = b.native_reserves.saturating_add(native_b);
        b.foreign_reserves = b.foreign_reserves.saturating_add(foreign_b);
      });
      BucketC::<T>::mutate(tol_id, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_c);
        b.native_reserves = b.native_reserves.saturating_add(native_c);
        b.foreign_reserves = b.foreign_reserves.saturating_add(foreign_c);
      });
      BucketD::<T>::mutate(tol_id, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_d);
        b.native_reserves = b.native_reserves.saturating_add(native_d);
        b.foreign_reserves = b.foreign_reserves.saturating_add(foreign_d);
      });
      let ingress_account = Self::ingress_account_for_tol_id(tol_id);
      let bucket_a_account = Self::bucket_account_for_tol_id(tol_id, 0)?;
      let bucket_b_account = Self::bucket_account_for_tol_id(tol_id, 1)?;
      let bucket_c_account = Self::bucket_account_for_tol_id(tol_id, 2)?;
      let bucket_d_account = Self::bucket_account_for_tol_id(tol_id, 3)?;
      let transfer_lp = |dest: &T::AccountId, amt: Balance| -> DispatchResult {
        if amt.is_zero() {
          return Ok(());
        }
        match lp_asset {
          AssetKind::Native => <T::Currency as NativeMutate<T::AccountId>>::transfer(
            &ingress_account,
            dest,
            amt,
            Preservation::Expendable,
          )
          .map(|_| ()),
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            T::Assets::transfer(id, &ingress_account, dest, amt, Preservation::Expendable)
              .map(|_| ())
          }
        }
      };
      transfer_lp(&bucket_a_account, amount_a)?;
      transfer_lp(&bucket_b_account, amount_b)?;
      transfer_lp(&bucket_c_account, amount_c)?;
      transfer_lp(&bucket_d_account, amount_d)?;
      Self::deposit_event(Event::LPTokensDistributed {
        tol_id,
        lp_asset_id: lp_asset,
        bucket_a_amount: amount_a,
        bucket_b_amount: amount_b,
        bucket_c_amount: amount_c,
        bucket_d_amount: amount_d,
        total_amount: amount,
      });
      Ok(tol_id)
    }

    /// Get total TOL reserves across all active domains and buckets
    pub fn get_total_tol_reserves() -> (Balance, Balance) {
      let mut total_native: Balance = 0;
      let mut total_foreign: Balance = 0;
      for tol_id in ActiveTolDomains::<T>::get().into_iter() {
        let bucket_a = BucketA::<T>::get(tol_id);
        total_native = total_native.saturating_add(bucket_a.native_reserves);
        total_foreign = total_foreign.saturating_add(bucket_a.foreign_reserves);
        let bucket_b = BucketB::<T>::get(tol_id);
        total_native = total_native.saturating_add(bucket_b.native_reserves);
        total_foreign = total_foreign.saturating_add(bucket_b.foreign_reserves);
        let bucket_c = BucketC::<T>::get(tol_id);
        total_native = total_native.saturating_add(bucket_c.native_reserves);
        total_foreign = total_foreign.saturating_add(bucket_c.foreign_reserves);
        let bucket_d = BucketD::<T>::get(tol_id);
        total_native = total_native.saturating_add(bucket_d.native_reserves);
        total_foreign = total_foreign.saturating_add(bucket_d.foreign_reserves);
      }
      (total_native, total_foreign)
    }
  }

  #[pallet::genesis_config]
  #[derive(frame::prelude::DefaultNoBound)]
  pub struct GenesisConfig<T: Config> {
    #[serde(skip)]
    pub _marker: core::marker::PhantomData<T>,
  }

  #[pallet::genesis_build]
  impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
    fn build(&self) {
      T::AssetConversion::initialize_lp_asset_namespace();
      frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::account_id());
      frame_system::Pallet::<T>::inc_providers(&T::BucketAAccount::get());
      frame_system::Pallet::<T>::inc_providers(&T::BucketBAccount::get());
      frame_system::Pallet::<T>::inc_providers(&T::BucketCAccount::get());
      frame_system::Pallet::<T>::inc_providers(&T::BucketDAccount::get());
    }
  }
}
