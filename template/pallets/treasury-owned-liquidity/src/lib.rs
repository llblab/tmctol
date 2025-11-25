#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// Re-export pallet items for runtime construction
pub use pallet::*;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

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
    path: polkadot_sdk::sp_std::vec::Vec<primitives::AssetKind>,
    amount_in: Balance,
    min_amount_out: Balance,
    recipient: AccountId,
    keep_alive: bool,
  ) -> Result<Balance, polkadot_sdk::sp_runtime::DispatchError>;
}

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
  use super::{AssetConversionApi, WeightInfo};
  use polkadot_sdk::frame_support::{
    ensure,
    pallet_prelude::*,
    traits::{
      fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
      fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
      tokens::Preservation,
      Currency,
    },
    PalletId,
  };
  use polkadot_sdk::frame_system;
  use polkadot_sdk::frame_system::pallet_prelude::*;
  use polkadot_sdk::sp_runtime::{
    traits::{AccountIdConversion, Zero},
    DispatchError,
  };
  use primitives::AssetKind;

  /// Configuration trait for the treasury owned liquidity pallet
  #[pallet::config]
  pub trait Config: polkadot_sdk::frame_system::Config {
    /// The assets pallet for managing fungible tokens (Local)
    type Assets: FungiblesInspect<Self::AccountId, AssetId = u32, Balance = u128>
      + FungiblesMutate<Self::AccountId, AssetId = u32, Balance = u128>;

    /// The currency trait for managing native tokens (AssetKind::Native)
    type Currency: Currency<Self::AccountId>
      + NativeInspect<Self::AccountId, Balance = u128>
      + NativeMutate<Self::AccountId, Balance = u128>;

    /// The treasury account for receiving LP tokens
    #[pallet::constant]
    type TreasuryAccount: Get<Self::AccountId>;

    /// The pallet ID for the TOL
    #[pallet::constant]
    type PalletId: Get<PalletId>;

    /// Precision for calculations
    #[pallet::constant]
    type Precision: Get<u128>;

    /// Allocation percentage for Bucket A (Floor Price) in PPM
    #[pallet::constant]
    type BucketAAllocation: Get<u32>;

    /// Allocation percentage for Bucket B (Operations) in PPM
    #[pallet::constant]
    type BucketBAllocation: Get<u32>;

    /// Allocation percentage for Bucket C (Overflow) in PPM
    #[pallet::constant]
    type BucketCAllocation: Get<u32>;

    /// Allocation percentage for Bucket D (Reserve) in PPM
    #[pallet::constant]
    type BucketDAllocation: Get<u32>;

    /// Account for Bucket A
    #[pallet::constant]
    type BucketAAccount: Get<Self::AccountId>;

    /// Account for Bucket B
    #[pallet::constant]
    type BucketBAccount: Get<Self::AccountId>;

    /// Account for Bucket C
    #[pallet::constant]
    type BucketCAccount: Get<Self::AccountId>;

    /// Account for Bucket D
    #[pallet::constant]
    type BucketDAccount: Get<Self::AccountId>;

    /// Zap Manager Account
    #[pallet::constant]
    type ZapManagerAccount: Get<Self::AccountId>;

    /// Bucket A ceiling ratio (e.g., 110%)
    #[pallet::constant]
    type BucketARatio: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Bucket B ceiling ratio (e.g., 125%)
    #[pallet::constant]
    type BucketBRatio: Get<polkadot_sdk::sp_runtime::Permill>;

    /// Bucket C ceiling ratio (e.g., 150%)
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

    /// Origin that can create and manage TOL configurations
    type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Weight information for extrinsics
    type WeightInfo: WeightInfo;

    /// Maximum number of TOL requests processed per block
    #[pallet::constant]
    type MaxTolRequestsPerBlock: Get<u32>;
  }

  #[pallet::pallet]
  pub struct Pallet<T>(_);

  pub type AssetId = u32;
  pub type Balance = u128;

  /// Configuration for a specific TOL instance
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
    /// Asset to allocate
    pub token_asset: AssetKind,
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

  /// Configurations for TOL instances
  #[pallet::storage]
  #[pallet::getter(fn tol_configurations)]
  pub type TolConfigurations<T: Config> = StorageMap<_, Blake2_128Concat, AssetKind, TolConfig>;

  /// Bucket A Allocations (Floor Price)
  #[pallet::storage]
  #[pallet::getter(fn bucket_a)]
  pub type BucketA<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, BucketAllocation, ValueQuery>;

  /// Bucket B Allocations (Operations)
  #[pallet::storage]
  #[pallet::getter(fn bucket_b)]
  pub type BucketB<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, BucketAllocation, ValueQuery>;

  /// Bucket C Allocations (Overflow)
  #[pallet::storage]
  #[pallet::getter(fn bucket_c)]
  pub type BucketC<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, BucketAllocation, ValueQuery>;

  /// Bucket D Allocations (Reserve)
  #[pallet::storage]
  #[pallet::getter(fn bucket_d)]
  pub type BucketD<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, BucketAllocation, ValueQuery>;

  /// Pending zap buffers
  #[pallet::storage]
  #[pallet::getter(fn zap_buffers)]
  pub type ZapBuffers<T: Config> =
    StorageMap<_, Blake2_128Concat, AssetKind, ZapBuffer, ValueQuery>;

  /// Pending allocation requests
  #[pallet::storage]
  #[pallet::getter(fn pending_allocation_requests)]
  pub type PendingRequests<T: Config> =
    StorageValue<_, BoundedVec<TolAllocationRequest, T::MaxTolRequestsPerBlock>, ValueQuery>;

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// TOL instance created
    TolCreated {
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    },
    /// Liquidity added to bucket
    LiquidityAdded {
      token_asset: AssetKind,
      bucket_id: u8,
      native_amount: Balance,
      foreign_amount: Balance,
      lp_tokens_received: Balance,
    },
    /// Bucket allocation updated
    BucketAllocationUpdated {
      token_asset: AssetKind,
      bucket_id: u8,
      new_allocation_ppm: u32,
    },
    /// Liquidity zapped
    LiquidityZapped {
      token_asset: AssetKind,
      native_used: Balance,
      foreign_used: Balance,
      lp_tokens_minted: Balance,
    },
    /// Zap buffer updated
    ZapBufferUpdated {
      token_asset: AssetKind,
      pending_native: Balance,
      pending_foreign: Balance,
    },
    /// Treasury withdrawal
    TreasuryWithdraw {
      asset: AssetKind,
      amount: Balance,
      destination: T::AccountId,
    },
    /// TOL requests processed
    TolRequestsProcessed { count: u32 },
    /// LP tokens received from Zap Manager
    LPTokensReceived {
      lp_asset_id: AssetKind,
      lp_amount: Balance,
      distributed_block: BlockNumberFor<T>,
    },
    /// Economic metrics recorded
    EconomicMetricsRecorded {
      total_tokens: u32,
      total_buffers: u32,
      total_buffer_native: Balance,
      total_buffer_foreign: Balance,
      block_number: BlockNumberFor<T>,
    },
    /// LP tokens distributed to buckets
    LPTokensDistributed {
      lp_asset_id: AssetKind,
      bucket_a_amount: Balance,
      bucket_b_amount: Balance,
      bucket_c_amount: Balance,
      bucket_d_amount: Balance,
      total_amount: Balance,
    },
    /// Buffer analytics recorded
    BufferAnalyticsRecorded {
      token_asset: AssetKind,
      pending_native: Balance,
      pending_foreign: Balance,
      buffer_age: BlockNumberFor<T>,
    },
  }

  #[pallet::error]
  pub enum Error<T> {
    /// TOL instance already exists
    TolAlreadyExists,
    /// No TOL instance exists
    NoTolExists,
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
  }

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
      // Process pending TOL allocation requests
      Self::process_tol_allocation_requests()
    }
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Create a new TOL instance
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::create_tol())]
    pub fn create_tol(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      foreign_asset: AssetKind,
      total_allocation: Balance,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;

      ensure!(
        !TolConfigurations::<T>::contains_key(token_asset),
        Error::<T>::TolAlreadyExists
      );

      let config = TolConfig {
        token_asset,
        foreign_asset,
        total_tol_allocation: total_allocation,
        current_tol_supply: Zero::zero(),
      };

      TolConfigurations::<T>::insert(token_asset, config);

      // Initialize buckets with default allocations
      BucketA::<T>::insert(
        token_asset,
        BucketAllocation {
          target_allocation_ppm: T::BucketAAllocation::get(),
          ..Default::default()
        },
      );
      BucketB::<T>::insert(
        token_asset,
        BucketAllocation {
          target_allocation_ppm: T::BucketBAllocation::get(),
          ..Default::default()
        },
      );
      BucketC::<T>::insert(
        token_asset,
        BucketAllocation {
          target_allocation_ppm: T::BucketCAllocation::get(),
          ..Default::default()
        },
      );
      BucketD::<T>::insert(
        token_asset,
        BucketAllocation {
          target_allocation_ppm: T::BucketDAllocation::get(),
          ..Default::default()
        },
      );

      Self::deposit_event(Event::TolCreated {
        token_asset,
        foreign_asset,
        total_allocation,
      });

      Ok(())
    }

    /// Update bucket allocation
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::update_bucket_allocation())]
    pub fn update_bucket_allocation(
      origin: OriginFor<T>,
      token_asset: AssetKind,
      bucket_id: u8,
      new_allocation_ppm: u32,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;

      ensure!(
        TolConfigurations::<T>::contains_key(token_asset),
        Error::<T>::NoTolExists
      );

      match bucket_id {
        0 => BucketA::<T>::mutate(token_asset, |b| {
          b.target_allocation_ppm = new_allocation_ppm
        }),
        1 => BucketB::<T>::mutate(token_asset, |b| {
          b.target_allocation_ppm = new_allocation_ppm
        }),
        2 => BucketC::<T>::mutate(token_asset, |b| {
          b.target_allocation_ppm = new_allocation_ppm
        }),
        3 => BucketD::<T>::mutate(token_asset, |b| {
          b.target_allocation_ppm = new_allocation_ppm
        }),
        _ => return Err(Error::<T>::InvalidBucketType.into()),
      }

      Self::deposit_event(Event::BucketAllocationUpdated {
        token_asset,
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

      Self::add_allocation_request(token_asset, total_native, total_foreign)?;

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

      // Distribute LP tokens to buckets based on configuration
      Self::distribute_lp_tokens_to_buckets(lp_asset, lp_amount)?;

      Self::deposit_event(Event::LPTokensReceived {
        lp_asset_id: lp_asset,
        lp_amount,
        distributed_block: <frame_system::Pallet<T>>::block_number(),
      });

      Ok(())
    }

    /// Withdraw buffer
    #[pallet::call_index(4)]
    #[pallet::weight(T::WeightInfo::withdraw_buffer())]
    pub fn withdraw_buffer(
      origin: OriginFor<T>,
      asset: AssetKind,
      amount: Balance,
      destination: T::AccountId,
    ) -> DispatchResult {
      let _who = T::AdminOrigin::ensure_origin(origin)?;

      let treasury_account = Self::account_id();

      match asset {
        AssetKind::Native => {
          <T::Currency as NativeMutate<T::AccountId>>::transfer(
            &treasury_account,
            &destination,
            amount,
            Preservation::Preserve,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::transfer(
            id,
            &treasury_account,
            &destination,
            amount,
            Preservation::Preserve,
          )?;
        }
      }

      Self::deposit_event(Event::TreasuryWithdraw {
        asset,
        amount,
        destination,
      });

      Ok(())
    }
  }

  impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    pub fn add_allocation_request(
      token_asset: AssetKind,
      total_native: Balance,
      total_foreign: Balance,
    ) -> DispatchResult {
      PendingRequests::<T>::try_mutate(|requests| {
        if requests.len() >= T::MaxTolRequestsPerBlock::get() as usize {
          return Err(Error::<T>::InvalidAllocation.into());
        }
        requests
          .try_push(TolAllocationRequest {
            token_asset,
            total_native,
            total_foreign,
          })
          .map_err(|_| Error::<T>::InvalidAllocation.into())
      })
    }

    pub fn process_tol_allocation_requests() -> Weight {
      let requests = PendingRequests::<T>::take();
      let mut weight = Weight::zero();

      for request in requests.iter() {
        let _ = Self::add_to_zap_buffer(
          request.token_asset,
          request.total_native,
          request.total_foreign,
        );
        weight = weight.saturating_add(T::WeightInfo::receive_mint_allocation());
      }

      if !requests.is_empty() {
        Self::deposit_event(Event::TolRequestsProcessed {
          count: requests.len() as u32,
        });
      }

      weight
    }

    pub fn add_to_zap_buffer(
      token_asset: AssetKind,
      native_amount: Balance,
      foreign_amount: Balance,
    ) -> DispatchResult {
      ZapBuffers::<T>::mutate(token_asset, |buffer| {
        buffer.pending_native = buffer.pending_native.saturating_add(native_amount);
        buffer.pending_foreign = buffer.pending_foreign.saturating_add(foreign_amount);
      });

      Self::deposit_event(Event::ZapBufferUpdated {
        token_asset,
        pending_native: ZapBuffers::<T>::get(token_asset).pending_native,
        pending_foreign: ZapBuffers::<T>::get(token_asset).pending_foreign,
      });

      Ok(())
    }

    pub fn distribute_lp_tokens_to_buckets(lp_asset: AssetKind, amount: Balance) -> DispatchResult {
      // Fetch allocations
      let alloc_a = BucketA::<T>::get(lp_asset).target_allocation_ppm;
      let alloc_b = BucketB::<T>::get(lp_asset).target_allocation_ppm;
      let alloc_c = BucketC::<T>::get(lp_asset).target_allocation_ppm;

      // Calculate distribution (Bucket D gets remainder)
      let total_ppm = 1_000_000u128;
      let amount_a = amount.saturating_mul(alloc_a as u128) / total_ppm;
      let amount_b = amount.saturating_mul(alloc_b as u128) / total_ppm;
      let amount_c = amount.saturating_mul(alloc_c as u128) / total_ppm;
      let amount_d = amount
        .saturating_sub(amount_a)
        .saturating_sub(amount_b)
        .saturating_sub(amount_c);

      // Update bucket state
      BucketA::<T>::mutate(lp_asset, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_a)
      });
      BucketB::<T>::mutate(lp_asset, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_b)
      });
      BucketC::<T>::mutate(lp_asset, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_c)
      });
      BucketD::<T>::mutate(lp_asset, |b| {
        b.lp_tokens = b.lp_tokens.saturating_add(amount_d)
      });

      let treasury_account = Self::account_id();

      let transfer_lp = |dest: &T::AccountId, amt: Balance| -> DispatchResult {
        if amt.is_zero() {
          return Ok(());
        }
        match lp_asset {
          AssetKind::Native => <T::Currency as NativeMutate<T::AccountId>>::transfer(
            &treasury_account,
            dest,
            amt,
            Preservation::Preserve,
          )
          .map(|_| ()),
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            T::Assets::transfer(id, &treasury_account, dest, amt, Preservation::Preserve)
              .map(|_| ())
          }
        }
      };

      transfer_lp(&T::BucketAAccount::get(), amount_a)?;
      transfer_lp(&T::BucketBAccount::get(), amount_b)?;
      transfer_lp(&T::BucketCAccount::get(), amount_c)?;
      transfer_lp(&T::BucketDAccount::get(), amount_d)?;

      Self::deposit_event(Event::LPTokensDistributed {
        lp_asset_id: lp_asset,
        bucket_a_amount: amount_a,
        bucket_b_amount: amount_b,
        bucket_c_amount: amount_c,
        bucket_d_amount: amount_d,
        total_amount: amount,
      });

      Ok(())
    }

    /// Get total TOL reserves for a token
    pub fn get_total_tol_reserves(token_asset: AssetKind) -> Option<(Balance, Balance)> {
      let mut total_native: Balance = 0;
      let mut total_foreign: Balance = 0;

      // We collect totals from all buckets
      let bucket_a = BucketA::<T>::get(token_asset);
      total_native = total_native.saturating_add(bucket_a.native_reserves);
      total_foreign = total_foreign.saturating_add(bucket_a.foreign_reserves);

      let bucket_b = BucketB::<T>::get(token_asset);
      total_native = total_native.saturating_add(bucket_b.native_reserves);
      total_foreign = total_foreign.saturating_add(bucket_b.foreign_reserves);

      let bucket_c = BucketC::<T>::get(token_asset);
      total_native = total_native.saturating_add(bucket_c.native_reserves);
      total_foreign = total_foreign.saturating_add(bucket_c.foreign_reserves);

      let bucket_d = BucketD::<T>::get(token_asset);
      total_native = total_native.saturating_add(bucket_d.native_reserves);
      total_foreign = total_foreign.saturating_add(bucket_d.foreign_reserves);

      Some((total_native, total_foreign))
    }

    /// Check if buffer should trigger zap based on threshold
    pub fn should_trigger_zap(token_asset: AssetKind) -> bool {
      let buffer = ZapBuffers::<T>::get(token_asset);
      buffer.pending_foreign >= T::MinSwapForeign::get()
    }
  }
}
