//! Burning Manager Pallet
//!
//! Passive deflationary engine that converts non-native tokens to native and burns them.

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

/// Helper for benchmarking
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<AssetKind, AccountId, Balance> {
  fn ensure_funded(
    who: &AccountId,
    asset: AssetKind,
    amount: Balance,
  ) -> frame::deps::sp_runtime::DispatchResult;
  fn create_asset(asset: AssetKind) -> frame::deps::sp_runtime::DispatchResult;
  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> frame::deps::sp_runtime::DispatchResult;
  fn add_liquidity(
    who: &AccountId,
    asset1: AssetKind,
    asset2: AssetKind,
    amount1: Balance,
    amount2: Balance,
  ) -> frame::deps::sp_runtime::DispatchResult;
}

#[frame::pallet]
pub mod pallet {
  use super::WeightInfo;
  use alloc::{vec, vec::Vec};
  use frame::deps::{
    frame_support::traits::{
      fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
      fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
      tokens::{Fortitude, Precision, Preservation},
    },
    sp_runtime::{
      DispatchError, Permill,
      traits::{AccountIdConversion, Zero},
    },
  };
  use frame::prelude::*;
  use primitives::AssetKind;

  /// Configuration trait for the burning manager pallet
  #[pallet::config]
  pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
    /// The assets pallet for managing local fungible tokens (AssetKind::Local)
    type Assets: FungiblesInspect<Self::AccountId, AssetId = u32, Balance = u128>
      + FungiblesMutate<Self::AccountId, AssetId = u32, Balance = u128>;

    /// The currency trait for managing native tokens (AssetKind::Native)
    type Currency: NativeInspect<Self::AccountId, Balance = u128>
      + NativeMutate<Self::AccountId, Balance = u128>;

    /// The asset conversion pallet for swapping foreign tokens
    type AssetConversion: AssetConversionApi<Self::AccountId, u128>;

    /// The pallet ID for the burning manager
    #[pallet::constant]
    type PalletId: Get<PalletId>;

    /// Reference asset for threshold checks and pricing
    #[pallet::constant]
    type ReferenceAsset: Get<AssetKind>;

    /// Default minimum amount of native tokens required to trigger burning
    #[pallet::constant]
    type DefaultMinBurnNative: Get<u128>;

    /// Default dust threshold in reference asset units
    #[pallet::constant]
    type DefaultDustThreshold: Get<u128>;

    /// Precision for price calculations
    #[pallet::constant]
    type Precision: Get<u128>;

    /// Default slippage tolerance for swaps
    #[pallet::constant]
    type DefaultSlippageTolerance: Get<Permill>;

    /// Origin that can perform governance operations
    type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

    /// Weight information for extrinsics
    type WeightInfo: WeightInfo;

    /// Price tools for decoupling price queries
    type PriceTools: PriceTools<AssetKind, u128>;

    /// Helper for benchmarking
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper: crate::BenchmarkHelper<AssetKind, Self::AccountId, u128>;
  }

  /// Asset conversion API trait for swapping foreign tokens
  pub trait AssetConversionApi<AccountId, Balance> {
    /// Get the pool ID for a given asset pair
    fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<[u8; 32]>;

    /// Get the reserves for a given pool
    fn get_pool_reserves(pool_id: [u8; 32]) -> Option<(Balance, Balance)>;

    /// Execute swap
    fn swap_exact_tokens_for_tokens(
      who: &AccountId,
      path: Vec<AssetKind>,
      amount_in: Balance,
      min_amount_out: Balance,
    ) -> Result<Balance, DispatchError>;

    /// Remove liquidity from LP token
    fn remove_liquidity(
      who: &AccountId,
      asset1: AssetKind,
      asset2: AssetKind,
      lp_amount: Balance,
    ) -> Result<(Balance, Balance), DispatchError>;

    /// Resolve LP token ID to its constituent asset pair by iterating pools.
    /// Returns None if no pool matches the given LP token ID.
    fn get_pool_pair_for_lp(lp_token_id: u32) -> Option<(AssetKind, AssetKind)>;
  }

  /// Price tools trait for decoupling from specific router implementation
  pub trait PriceTools<AssetId, Balance> {
    /// Get spot price quote for swapping amount from asset_from to asset_to
    fn quote_spot_price(
      asset_from: AssetId,
      asset_to: AssetId,
      amount: Balance,
    ) -> Result<Balance, DispatchError>;

    /// Get oracle price for asset pair
    fn get_oracle_price(asset_from: AssetId, asset_to: AssetId) -> Option<Balance>;
  }

  /// The pallet struct
  #[pallet::pallet]
  pub struct Pallet<T>(PhantomData<T>);

  /// Storage for tracking total burned native tokens
  #[pallet::storage]
  #[pallet::getter(fn total_burned)]
  pub type TotalBurned<T: Config> = StorageValue<_, u128, ValueQuery>;

  /// Storage for tracking total foreign tokens swapped
  #[pallet::storage]
  #[pallet::getter(fn total_swapped)]
  pub type TotalSwapped<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    AssetKind, // Changed from u32 to AssetKind
    u128,
    ValueQuery,
  >;

  /// Storage for list of burnable assets
  #[pallet::storage]
  #[pallet::getter(fn burnable_assets)]
  pub type BurnableAssets<T: Config> = StorageValue<
    _,
    BoundedVec<
      AssetKind, // Changed from u32 to AssetKind
      ConstU32<100>,
    >,
    ValueQuery,
  >;

  /// Last processed index for smart batching
  #[pallet::storage]
  #[pallet::getter(fn last_processed_index)]
  pub type LastProcessedIndex<T: Config> = StorageValue<_, u32, ValueQuery>;

  /// Current minimum burn amount for native tokens (can be updated by governance)
  #[pallet::storage]
  #[pallet::getter(fn min_burn_native)]
  pub type MinBurnNative<T: Config> = StorageValue<_, u128, ValueQuery, T::DefaultMinBurnNative>;

  /// Current dust threshold (can be updated by governance)
  #[pallet::storage]
  #[pallet::getter(fn dust_threshold)]
  pub type DustThreshold<T: Config> = StorageValue<_, u128, ValueQuery, T::DefaultDustThreshold>;

  /// Current slippage tolerance (can be updated by governance)
  #[pallet::storage]
  #[pallet::getter(fn slippage_tolerance)]
  pub type SlippageTolerance<T: Config> =
    StorageValue<_, Permill, ValueQuery, T::DefaultSlippageTolerance>;

  /// Events for the burning manager pallet
  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Native tokens burned
    NativeTokensBurned { amount: u128, new_total: u128 },
    /// Foreign tokens swapped to native
    ForeignTokensSwapped {
      foreign_asset: AssetKind,
      foreign_amount: u128,
      native_received: u128,
    },
    /// LP tokens unwound into constituent assets
    LpUnwound {
      lp_asset: AssetKind,
      lp_amount: u128,
      asset1: AssetKind,
      amount1: u128,
      asset2: AssetKind,
      amount2: u128,
    },
    /// Minimum burn amount updated
    MinBurnUpdated { old_amount: u128, new_amount: u128 },
    /// Dust threshold updated
    DustThresholdUpdated {
      old_threshold: u128,
      new_threshold: u128,
    },
    /// Slippage tolerance updated
    SlippageToleranceUpdated {
      old_tolerance: Permill,
      new_tolerance: Permill,
    },
  }

  /// Errors for the burning manager pallet
  #[pallet::error]
  pub enum Error<T> {
    /// Insufficient balance for operation
    InsufficientBalance,
    /// Failed to execute swap
    SwapFailed,
    /// Amount too small for operation
    AmountTooSmall,
    /// Arithmetic overflow occurred
    ArithmeticOverflow,
    /// Too many burnable assets
    TooManyAssets,
  }

  /// Implementation of the burning manager pallet
  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Burn native tokens manually (for testing)
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::burn_native_tokens())]
    pub fn burn_native_tokens(origin: OriginFor<T>, amount: u128) -> DispatchResult {
      let _who = ensure_signed(origin)?;
      ensure!(!amount.is_zero(), Error::<T>::AmountTooSmall);
      Self::process_native_burn(amount)
    }

    /// Add a burnable asset (governance only)
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::add_burnable_asset())]
    pub fn add_burnable_asset(origin: OriginFor<T>, asset: AssetKind) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;
      BurnableAssets::<T>::try_mutate(|assets| {
        if !assets.contains(&asset) {
          assets
            .try_push(asset)
            .map_err(|_| Error::<T>::TooManyAssets)?;
        }
        Ok(())
      })
    }

    /// Update minimum burn amount for native tokens (governance only)
    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::update_min_burn_native())]
    pub fn update_min_burn_native(origin: OriginFor<T>, new_amount: u128) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;
      let old_amount = MinBurnNative::<T>::get();
      MinBurnNative::<T>::put(new_amount);
      Self::deposit_event(Event::MinBurnUpdated {
        old_amount,
        new_amount,
      });
      Ok(())
    }

    /// Update dust threshold (governance only)
    #[pallet::call_index(3)]
    #[pallet::weight(T::WeightInfo::update_dust_threshold())]
    pub fn update_dust_threshold(origin: OriginFor<T>, new_threshold: u128) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;
      let old_threshold = DustThreshold::<T>::get();
      DustThreshold::<T>::put(new_threshold);
      Self::deposit_event(Event::DustThresholdUpdated {
        old_threshold,
        new_threshold,
      });
      Ok(())
    }

    /// Update slippage tolerance (governance only)
    #[pallet::call_index(4)]
    #[pallet::weight(T::WeightInfo::update_slippage_tolerance())]
    pub fn update_slippage_tolerance(
      origin: OriginFor<T>,
      new_tolerance: Permill,
    ) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;
      let old_tolerance = SlippageTolerance::<T>::get();
      SlippageTolerance::<T>::put(new_tolerance);
      Self::deposit_event(Event::SlippageToleranceUpdated {
        old_tolerance,
        new_tolerance,
      });
      Ok(())
    }
  }

  /// Hooks for the burning manager pallet
  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    fn on_idle(_n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
      let mut consumed = Weight::zero();
      // Phase 1: Process ONE non-native asset (LP priority, then foreign swap)
      let foreign_weight = T::WeightInfo::process_foreign_fees();
      if remaining_weight.all_gte(consumed.saturating_add(foreign_weight)) {
        let _ = Self::process_one_non_native();
        consumed = consumed.saturating_add(foreign_weight);
      }
      // Phase 2: Burn ALL native on account (includes swap proceeds from phase 1)
      let burn_weight = T::WeightInfo::burn_native_tokens();
      if remaining_weight.all_gte(consumed.saturating_add(burn_weight)) {
        let bm = Self::account_id();
        let native_balance = T::Currency::balance(&bm);
        if native_balance >= MinBurnNative::<T>::get() {
          let _ = Self::process_native_burn(native_balance);
        }
        consumed = consumed.saturating_add(burn_weight);
      }
      consumed
    }
  }

  impl<T: Config> Pallet<T> {
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    /// Phase 1: Find and process one non-native asset.
    /// LP tokens have priority (top of stack). Then foreign tokens above dust threshold.
    fn process_one_non_native() -> DispatchResult {
      let burnable_assets = Self::burnable_assets();
      let len = burnable_assets.len();
      if len == 0 {
        return Ok(());
      }
      let bm = Self::account_id();
      let start = (Self::last_processed_index() as usize) % len;
      let reference_asset = T::ReferenceAsset::get();
      use primitives::AssetInspector;
      // Pass 1: LP tokens (priority — unwrap immediately)
      for i in 0..len {
        let idx = (start + i) % len;
        let asset = burnable_assets[idx];
        if !asset.is_lp() {
          continue;
        }
        let balance = match asset {
          AssetKind::Local(id) | AssetKind::Foreign(id) => T::Assets::balance(id, &bm),
          _ => continue,
        };
        if balance.is_zero() {
          continue;
        }
        LastProcessedIndex::<T>::put((idx + 1) as u32);
        return Self::process_lp_unwinding(asset, balance);
      }
      // Pass 2: Non-native, non-LP tokens (swap to native)
      for i in 0..len {
        let idx = (start + i) % len;
        let asset = burnable_assets[idx];
        if matches!(asset, AssetKind::Native) || asset.is_lp() {
          continue;
        }
        let balance = match asset {
          AssetKind::Local(id) | AssetKind::Foreign(id) => T::Assets::balance(id, &bm),
          _ => continue,
        };
        if balance.is_zero() {
          continue;
        }
        let quote = T::PriceTools::quote_spot_price(asset, reference_asset, balance)?;
        if quote < DustThreshold::<T>::get() {
          continue;
        }
        LastProcessedIndex::<T>::put((idx + 1) as u32);
        let _ = Self::process_foreign_swap(asset, balance);
        return Ok(());
      }
      Ok(())
    }

    /// Burn native tokens from BM account
    fn process_native_burn(amount: u128) -> DispatchResult {
      let bm = Self::account_id();
      let actually_burned = T::Currency::burn_from(
        &bm,
        amount,
        Preservation::Expendable,
        Precision::BestEffort,
        Fortitude::Polite,
      )?;
      if actually_burned.is_zero() {
        return Ok(());
      }
      let new_total = TotalBurned::<T>::get().saturating_add(actually_burned);
      TotalBurned::<T>::put(new_total);
      Self::deposit_event(Event::NativeTokensBurned {
        amount: actually_burned,
        new_total,
      });
      Ok(())
    }

    /// Swap non-native asset to native
    fn process_foreign_swap(asset: AssetKind, amount: u128) -> Result<u128, DispatchError> {
      let bm = Self::account_id();
      let native_asset = AssetKind::Native;
      let min_amount_out =
        if let Some(oracle_price) = T::PriceTools::get_oracle_price(asset, native_asset) {
          let expected_out = amount.saturating_mul(oracle_price) / T::Precision::get();
          expected_out.saturating_sub(SlippageTolerance::<T>::get().mul_floor(expected_out))
        } else {
          return Err(Error::<T>::SwapFailed.into());
        };
      let native_received = T::AssetConversion::swap_exact_tokens_for_tokens(
        &bm,
        vec![asset, native_asset],
        amount,
        min_amount_out,
      )?;
      let new_total = TotalSwapped::<T>::get(asset).saturating_add(amount);
      TotalSwapped::<T>::insert(asset, new_total);
      Self::deposit_event(Event::ForeignTokensSwapped {
        foreign_asset: asset,
        foreign_amount: amount,
        native_received,
      });
      Ok(native_received)
    }

    /// Unwrap LP token into constituent assets. Native stays on BM for phase 2.
    /// Foreign stays on BM for next on_idle cycle.
    pub fn process_lp_unwinding(lp_asset: AssetKind, lp_amount: u128) -> DispatchResult {
      let lp_token_id = match lp_asset {
        AssetKind::Local(id) => id,
        _ => return Ok(()),
      };
      let (asset1, asset2) =
        T::AssetConversion::get_pool_pair_for_lp(lp_token_id).ok_or(Error::<T>::SwapFailed)?;
      let bm = Self::account_id();
      let (amount1, amount2) =
        T::AssetConversion::remove_liquidity(&bm, asset1, asset2, lp_amount)?;
      Self::deposit_event(Event::LpUnwound {
        lp_asset,
        lp_amount,
        asset1,
        amount1,
        asset2,
        amount2,
      });
      Ok(())
    }
  }

  /// Genesis configuration — ensures pallet account is ED-free
  #[pallet::genesis_config]
  #[derive(frame::prelude::DefaultNoBound)]
  pub struct GenesisConfig<T: Config> {
    #[serde(skip)]
    pub _marker: core::marker::PhantomData<T>,
  }

  #[pallet::genesis_build]
  impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
    fn build(&self) {
      // Pallet account survives zero native balance via provider reference
      frame_system::Pallet::<T>::inc_providers(&Pallet::<T>::account_id());
    }
  }
}
