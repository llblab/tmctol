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

/// Helper for benchmarking
#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<AssetKind, AccountId, Balance> {
  fn ensure_funded(
    who: &AccountId,
    asset: AssetKind,
    amount: Balance,
  ) -> polkadot_sdk::sp_runtime::DispatchResult;
  fn create_asset(asset: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult;
  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult;
  fn add_liquidity(
    who: &AccountId,
    asset1: AssetKind,
    asset2: AssetKind,
    amount1: Balance,
    amount2: Balance,
  ) -> polkadot_sdk::sp_runtime::DispatchResult;
}

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
  use super::WeightInfo;
  use alloc::{vec, vec::Vec};
  use primitives::AssetKind;

  use polkadot_sdk::frame_support::{
    ensure,
    pallet_prelude::*,
    traits::{
      fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
      fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
      tokens::{Fortitude, Precision, Preservation},
    },
    PalletId,
  };
  use polkadot_sdk::frame_system::pallet_prelude::*;
  use polkadot_sdk::sp_runtime::{
    traits::{AccountIdConversion, Zero},
    DispatchError, Permill,
  };

  /// Configuration trait for the burning manager pallet
  #[pallet::config]
  pub trait Config: polkadot_sdk::frame_system::Config<RuntimeEvent: From<Event<Self>>> {
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

  const MAX_PROCESS_PER_IDLE: usize = 3;

  /// The pallet struct
  #[pallet::pallet]
  pub struct Pallet<T>(polkadot_sdk::frame_support::pallet_prelude::PhantomData<T>);

  /// Storage for tracking total burned native tokens
  #[pallet::storage]
  #[pallet::getter(fn total_burned)]
  pub type TotalBurned<T: Config> = StorageValue<_, u128, ValueQuery>;

  /// Storage for tracking total foreign tokens swapped
  #[pallet::storage]
  #[pallet::getter(fn total_swapped)]
  pub type TotalSwapped<T: Config> = StorageMap<
    _,
    polkadot_sdk::frame_support::Blake2_128Concat,
    AssetKind, // Changed from u32 to AssetKind
    u128,
    ValueQuery,
  >;

  /// Storage for list of burnable assets
  #[pallet::storage]
  #[pallet::getter(fn burnable_assets)]
  pub type BurnableAssets<T: Config> = StorageValue<
    _,
    polkadot_sdk::frame_support::BoundedVec<
      AssetKind, // Changed from u32 to AssetKind
      polkadot_sdk::frame_support::traits::ConstU32<100>,
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
      burn_amount: u128,
    },
    /// Fee processing completed
    FeesProcessed {
      native_burned: u128,
      foreign_swapped: Vec<(AssetKind, u128, u128)>, // (asset, foreign_amount, native_received)
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
    /// Process fees manually (for testing)
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::process_fees())]
    pub fn process_fees(origin: OriginFor<T>) -> DispatchResult {
      let _who = ensure_signed(origin)?;
      Self::process_pending_fees()
    }

    /// Burn native tokens manually (for testing)
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::burn_native_tokens())]
    pub fn burn_native_tokens(origin: OriginFor<T>, amount: u128) -> DispatchResult {
      let _who = ensure_signed(origin)?;
      ensure!(!amount.is_zero(), Error::<T>::AmountTooSmall);
      Self::process_native_burn(amount)
    }

    /// Add a burnable asset (governance only)
    #[pallet::call_index(2)]
    #[pallet::weight(10_000)]
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
    #[pallet::call_index(3)]
    #[pallet::weight(10_000)]
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
    #[pallet::call_index(4)]
    #[pallet::weight(10_000)]
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
    #[pallet::call_index(5)]
    #[pallet::weight(10_000)]
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
    fn on_initialize(_n: BlockNumberFor<T>) -> polkadot_sdk::frame_support::weights::Weight {
      let weight = T::WeightInfo::process_fees();

      // Process native fees on each block initialization
      let burning_manager_account = Self::account_id();
      let native_balance = T::Currency::balance(&burning_manager_account);

      if native_balance >= MinBurnNative::<T>::get() {
        let _ = Self::process_native_burn(native_balance);
      }

      weight
    }

    fn on_idle(
      _n: BlockNumberFor<T>,
      _remaining_weight: polkadot_sdk::frame_support::weights::Weight,
    ) -> polkadot_sdk::frame_support::weights::Weight {
      let weight = T::WeightInfo::process_fees();

      // Process foreign batch on idle
      if Self::process_foreign_batch(Self::last_processed_index() as usize).is_err() {
        // If processing fails, continue
      }

      weight
    }
  }

  /// Internal implementation for the burning manager
  impl<T: Config> Pallet<T> {
    /// Get the account ID of the burning manager
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    /// Process foreign batch starting from index
    fn process_foreign_batch(start_index: usize) -> DispatchResult {
      let burnable_assets = Self::burnable_assets();
      let len = burnable_assets.len();
      if len == 0 {
        return Ok(());
      }

      let mut index = start_index % len;
      let mut processed = 0;
      let burning_manager_account = Self::account_id();
      let reference_asset = T::ReferenceAsset::get();

      while processed < MAX_PROCESS_PER_IDLE {
        let asset = burnable_assets[index];

        // Get balance based on asset kind
        let balance = match asset {
          AssetKind::Native => T::Currency::balance(&burning_manager_account),
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            T::Assets::balance(id, &burning_manager_account)
          }
        };

        if balance.is_zero() {
          processed += 1;
          index = (index + 1) % len;
          continue;
        }

        let quote = T::PriceTools::quote_spot_price(asset, reference_asset, balance)?;

        if quote >= DustThreshold::<T>::get() {
          if asset == AssetKind::Native {
            Self::process_native_burn(balance)?;
          } else {
            let native_received = Self::process_foreign_swap(asset, balance)?;
            Self::process_native_burn(native_received)?;
          }
        }
        processed += 1;
        index = (index + 1) % len;
      }

      LastProcessedIndex::<T>::put(index as u32);
      Ok(())
    }

    /// Process pending fees by burning native tokens and swapping foreign tokens
    fn process_pending_fees() -> DispatchResult {
      let burning_manager_account = Self::account_id();
      let mut total_native_burned = Zero::zero();
      let mut foreign_swapped = Vec::new();

      // Process native token burning
      let native_balance = T::Currency::balance(&burning_manager_account);
      if native_balance >= MinBurnNative::<T>::get() {
        Self::process_native_burn(native_balance)?;
        total_native_burned = native_balance;
      }

      // Process foreign token swapping for burnable assets
      let burnable_assets = Self::burnable_assets();
      let reference_asset = T::ReferenceAsset::get();

      for &asset in &burnable_assets {
        // Skip native if it's in the list (handled separately above)
        if matches!(asset, AssetKind::Native) {
          continue;
        }

        let balance = match asset {
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            T::Assets::balance(id, &burning_manager_account)
          }
          _ => Zero::zero(),
        };

        if balance.is_zero() {
          continue;
        }

        let quote = T::PriceTools::quote_spot_price(asset, reference_asset, balance)?;

        if quote >= DustThreshold::<T>::get() {
          let native_received = Self::process_foreign_swap(asset, balance)?;
          Self::process_native_burn(native_received)?;
          foreign_swapped.push((asset, balance, native_received));
          total_native_burned = total_native_burned.saturating_add(native_received);
        }
      }

      // Emit comprehensive fee processing event
      if !total_native_burned.is_zero() || !foreign_swapped.is_empty() {
        Self::deposit_event(Event::FeesProcessed {
          native_burned: total_native_burned,
          foreign_swapped,
        });
      }

      Ok(())
    }

    /// Process native token burning
    fn process_native_burn(amount: u128) -> DispatchResult {
      let burning_manager_account = Self::account_id();

      // Burn native tokens
      T::Currency::burn_from(
        &burning_manager_account,
        amount,
        Preservation::Expendable,
        Precision::BestEffort,
        Fortitude::Polite,
      )?;

      // Update total burned
      let new_total = TotalBurned::<T>::get().saturating_add(amount);
      TotalBurned::<T>::put(new_total);

      Self::deposit_event(Event::NativeTokensBurned { amount, new_total });

      Ok(())
    }

    /// Process foreign token swapping
    fn process_foreign_swap(asset: AssetKind, amount: u128) -> Result<u128, DispatchError> {
      let burning_manager_account = Self::account_id();
      let native_asset = AssetKind::Native;

      // Get oracle price for slippage protection
      let min_amount_out =
        if let Some(oracle_price) = T::PriceTools::get_oracle_price(asset, native_asset) {
          let expected_out = amount.saturating_mul(oracle_price) / T::Precision::get();
          SlippageTolerance::<T>::get().mul_floor(expected_out)
        } else {
          return Err(Error::<T>::SwapFailed.into());
        };

      // Swap foreign tokens for native tokens
      let path = vec![asset, native_asset];

      let native_received = T::AssetConversion::swap_exact_tokens_for_tokens(
        &burning_manager_account,
        path,
        amount,
        min_amount_out,
      )?;

      // Update total swapped tracking
      let new_total = TotalSwapped::<T>::get(asset).saturating_add(amount);
      TotalSwapped::<T>::insert(asset, new_total);

      Self::deposit_event(Event::ForeignTokensSwapped {
        foreign_asset: asset,
        foreign_amount: amount,
        native_received,
        burn_amount: native_received, // Burn 100%
      });

      Ok(native_received)
    }
  }
}
