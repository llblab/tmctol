#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
pub mod tests;

#[cfg(test)]
pub mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[polkadot_sdk::frame_support::pallet(dev_mode)]
pub mod pallet {
  use super::WeightInfo;
  use alloc::{vec, vec::Vec};
  use polkadot_sdk::frame_support::{
    ensure,
    pallet_prelude::*,
    traits::{
      fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
      fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
      tokens::Preservation,
      Currency, EnsureOrigin,
    },
    PalletId,
  };
  use polkadot_sdk::frame_system::{pallet_prelude::*, Config as SystemConfig};
  use polkadot_sdk::sp_runtime::{traits::AccountIdConversion, DispatchError};
  use primitives::AssetKind;

  #[pallet::config]
  pub trait Config: polkadot_sdk::frame_system::Config
  where
    <<Self as Config>::Currency as Currency<<Self as SystemConfig>::AccountId>>::Balance:
      From<u128>,
  {
    /// Asset management interface for fungible tokens
    type Assets: FungiblesInspect<Self::AccountId, AssetId = u32, Balance = u128>
      + FungiblesMutate<Self::AccountId, AssetId = u32, Balance = u128>;
    /// Native currency interface
    type Currency: Currency<Self::AccountId>
      + NativeInspect<Self::AccountId, Balance = u128>
      + NativeMutate<Self::AccountId, Balance = u128>;
    /// Asset conversion API for interacting with AMM pools
    type AssetConversion: AssetConversionApi<Self::AccountId, u128>;
    /// Price oracle for manipulation-resistant verification
    type PriceOracle: PriceOracle<u128>;
    /// Pallet ID for account derivation
    #[pallet::constant]
    type PalletId: Get<PalletId>;
    /// Destination account for minted LP tokens (Protocol Owned Liquidity)
    #[pallet::constant]
    type TolTreasuryAccount: Get<Self::AccountId>;
    /// Minimum foreign balance required to trigger a zap
    #[pallet::constant]
    type MinSwapForeign: Get<u128>;
    /// Threshold for considering a balance as "dust" (too small to process)
    #[pallet::constant]
    type DustThreshold: Get<u128>;
    /// Origin required to enable/disable assets and trigger sweeps
    type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    /// Retry cooldown in blocks after a failed zap attempt
    #[pallet::constant]
    type RetryCooldown: Get<BlockNumberFor<Self>>;
    /// Weight information for extrinsics
    type WeightInfo: WeightInfo;
  }

  /// Trait defining required Asset Conversion capabilities
  pub trait AssetConversionApi<AccountId, Balance> {
    fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<AssetKind>;

    fn get_pool_reserves(pool_id: AssetKind) -> Option<(Balance, Balance)>;

    fn create_pool(asset1: AssetKind, asset2: AssetKind) -> Result<AssetKind, DispatchError>;

    fn add_liquidity(
      who: &AccountId,
      asset1: AssetKind,
      asset2: AssetKind,
      amount1_desired: Balance,
      amount2_desired: Balance,
      amount1_min: Balance,
      amount2_min: Balance,
    ) -> Result<(Balance, Balance, Balance), DispatchError>;

    fn swap_exact_tokens_for_tokens(
      who: &AccountId,
      asset_in: AssetKind,
      asset_out: AssetKind,
      amount_in: Balance,
      amount_out_min: Balance,
    ) -> Result<Balance, DispatchError>;
  }

  /// Trait defining Price Oracle capabilities for safety checks
  pub trait PriceOracle<Balance> {
    fn get_ema_price(asset_in: AssetKind, asset_out: AssetKind) -> Option<Balance>;

    fn validate_price_deviation(
      asset_in: AssetKind,
      asset_out: AssetKind,
      current_price: Balance,
    ) -> Result<(), DispatchError>;
  }

  /// The pallet module, the place to define dispatchable calls, storage items, events, errors, etc.
  ///
  /// ## Opportunistic Liquidity Provisioning
  /// The Zap Manager implements an "Opportunistic" strategy that transforms raw capital into
  /// Protocol-Owned Liquidity (POL) without aggressive pre-swap balancing.
  ///
  /// 1. **Add As-Is**: Adds maximum possible liquidity with current balance ratios.
  /// 2. **Foreign Surplus**: Swaps remaining foreign tokens to Native if above dust threshold.
  /// 3. **Native Accumulation**: Holds surplus Native tokens ("Patriotic Accumulation") to catch
  ///    future foreign inflows.
  ///
  /// ## Omnivorous Architecture
  /// The pallet is "Omnivorous", meaning it processes any whitelisted asset found in its account
  /// during `on_initialize`, regardless of the source (TMC emission, XCM, user transfer).
  ///
  /// ## Safety
  /// - **Oracle Guard**: Validates spot prices against an EMA oracle before executing zaps to
  ///   prevent sandwich attacks.
  /// - **Whitelist**: Only processes assets explicitly enabled via `EnabledAssets` to prevent
  ///   DoS vectors.
  #[pallet::pallet]
  pub struct Pallet<T>(polkadot_sdk::frame_support::pallet_prelude::PhantomData<T>);

  /// Whitelist of assets enabled for automatic zapping
  #[pallet::storage]
  #[pallet::getter(fn enabled_assets)]
  pub type EnabledAssets<T: Config> =
    StorageMap<_, polkadot_sdk::frame_support::Blake2_128Concat, AssetKind, (), OptionQuery>;

  /// Next allowed block for attempting a zap on a specific asset
  #[pallet::storage]
  pub type NextZapAttempt<T: Config> = StorageMap<
    _,
    polkadot_sdk::frame_support::Blake2_128Concat,
    AssetKind,
    BlockNumberFor<T>,
    OptionQuery,
  >;

  #[pallet::event]
  #[pallet::generate_deposit(pub(super) fn deposit_event)]
  pub enum Event<T: Config> {
    /// Asset enabled for zapping
    AssetEnabled { asset: AssetKind },
    /// Asset disabled for zapping
    AssetDisabled { asset: AssetKind },
    /// Assets manually swept to treasury
    AssetsSwept {
      assets: Vec<AssetKind>,
      destination: T::AccountId,
    },
    /// LP tokens distributed to TOL Treasury
    LPTokensDistributed {
      token_asset: AssetKind,
      lp_amount: u128,
      destination: T::AccountId,
    },
    /// Zap operation completed successfully
    ZapCompleted {
      token_asset: AssetKind,
      native_used: u128,
      foreign_used: u128,
      lp_tokens_minted: u128,
    },
    /// Surplus foreign tokens swapped to native
    SurplusSwapped {
      asset_in: AssetKind,
      asset_out: AssetKind,
      amount_in: u128,
      amount_out: u128,
    },
    /// Surplus native tokens held for future liquidity
    NativeHeld { amount: u128 },
  }

  #[pallet::error]
  pub enum Error<T> {
    /// Account balance insufficient for operation
    InsufficientBalance,
    /// Failed to create liquidity pool
    PoolCreationFailed,
    /// Failed to add liquidity to pool
    LiquidityAdditionFailed,
    /// Arithmetic overflow in calculation
    ArithmeticOverflow,
    /// Amount below minimum threshold
    AmountTooSmall,
    /// Asset not valid or not enabled
    InvalidAsset,
    /// Price deviation exceeds allowed limits (oracle guard)
    PriceDeviationExceeded,
    /// Token swap failed
    SwapFailed,
    /// Liquidity pool not found
    PoolNotFound,
  }

  #[pallet::call]
  impl<T: Config> Pallet<T> {
    /// Enable an asset for automatic zapping
    ///
    /// Adds the asset to `EnabledAssets`. The Native asset cannot be enabled as it is the base pair.
    #[pallet::call_index(0)]
    #[pallet::weight(T::WeightInfo::enable_asset())]
    pub fn enable_asset(origin: OriginFor<T>, asset: AssetKind) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      ensure!(asset != AssetKind::Native, Error::<T>::InvalidAsset);

      EnabledAssets::<T>::insert(asset, ());

      Self::deposit_event(Event::AssetEnabled { asset });

      Ok(())
    }

    /// Disable an asset for automatic zapping
    ///
    /// Removes the asset from `EnabledAssets`. Existing balances will remain untouched until swept.
    #[pallet::call_index(1)]
    #[pallet::weight(T::WeightInfo::disable_asset())]
    pub fn disable_asset(origin: OriginFor<T>, asset: AssetKind) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      EnabledAssets::<T>::remove(asset);

      Self::deposit_event(Event::AssetDisabled { asset });

      Ok(())
    }

    /// Manually trigger a sweep of assets to the TOL Treasury
    ///
    /// Useful for recovering assets that were disabled or sent by mistake.
    /// Can only be called for assets that are NOT currently enabled.
    #[pallet::call_index(2)]
    #[pallet::weight(T::WeightInfo::sweep_trigger())]
    pub fn sweep_trigger(origin: OriginFor<T>, asset: AssetKind) -> DispatchResult {
      T::AdminOrigin::ensure_origin(origin)?;

      ensure!(
        !EnabledAssets::<T>::contains_key(asset),
        Error::<T>::InvalidAsset
      );

      let zap_account = Self::account_id();
      let treasury = T::TolTreasuryAccount::get();

      let balance = match asset {
        AssetKind::Native => <T::Currency as NativeInspect<T::AccountId>>::balance(&zap_account),
        AssetKind::Local(id) | AssetKind::Foreign(id) => T::Assets::balance(id, &zap_account),
      };

      let min_balance = match asset {
        AssetKind::Native => <T::Currency as NativeInspect<T::AccountId>>::minimum_balance(),
        AssetKind::Local(id) | AssetKind::Foreign(id) => T::Assets::minimum_balance(id),
      };

      let sweep_amount = balance.saturating_sub(min_balance);

      if sweep_amount > 0 {
        match asset {
          AssetKind::Native => {
            <T::Currency as NativeMutate<T::AccountId>>::transfer(
              &zap_account,
              &treasury,
              sweep_amount,
              Preservation::Expendable,
            )?;
          }
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            T::Assets::transfer(
              id,
              &zap_account,
              &treasury,
              sweep_amount,
              Preservation::Expendable,
            )?;
          }
        }
        Self::deposit_event(Event::AssetsSwept {
          assets: vec![asset],
          destination: treasury,
        });
      }

      Ok(())
    }
  }

  #[pallet::hooks]
  impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    /// Main execution hook for the Zap Manager
    ///
    /// Scans `EnabledAssets` for balances exceeding `MinSwapForeign` and triggers
    /// `execute_opportunistic_zap`. This implements the "Omnivorous" intake model.
    fn on_initialize(n: BlockNumberFor<T>) -> polkadot_sdk::frame_support::weights::Weight {
      let weight = T::WeightInfo::enable_asset();

      for (token_asset, _) in EnabledAssets::<T>::iter() {
        // Check cooldown
        if let Some(next_attempt) = NextZapAttempt::<T>::get(token_asset) {
          if n < next_attempt {
            continue;
          }
        }

        let zap_account = Self::account_id();

        let foreign_balance = match token_asset {
          AssetKind::Native => continue,
          AssetKind::Local(id) | AssetKind::Foreign(id) => {
            let balance = T::Assets::balance(id, &zap_account);
            let min_balance = T::Assets::minimum_balance(id);
            balance.saturating_sub(min_balance)
          }
        };

        if foreign_balance >= T::MinSwapForeign::get() {
          match Self::execute_opportunistic_zap(token_asset, foreign_balance) {
            Ok(_) => {
              NextZapAttempt::<T>::remove(token_asset);
            }
            Err(_) => {
              NextZapAttempt::<T>::insert(token_asset, n + T::RetryCooldown::get());
            }
          }
        }
      }

      weight
    }
  }

  impl<T: Config> Pallet<T> {
    /// Get the pallet's account ID (derived from PalletId)
    pub fn account_id() -> T::AccountId {
      T::PalletId::get().into_account_truncating()
    }

    /// Execute the opportunistic zap strategy
    ///
    /// 1. **Check Oracle**: Verify spot price safety.
    /// 2. **Add Liquidity**: Add maximum liquidity with current balances.
    /// 3. **Swap Surplus**: Convert remaining Foreign to Native.
    /// 4. **Hold Native**: Keep remaining Native for future matching.
    fn execute_opportunistic_zap(
      token_asset: AssetKind,
      foreign_available: u128,
    ) -> DispatchResult {
      let native_asset = AssetKind::Native;
      let zap_account = Self::account_id();

      // Ensure pool exists
      let mut pool_id = T::AssetConversion::get_pool_id(native_asset, token_asset);
      if pool_id.is_none() {
        pool_id = Some(T::AssetConversion::create_pool(native_asset, token_asset)?);
      }
      let pool_id = pool_id.ok_or(Error::<T>::PoolCreationFailed)?;

      let native_ed = <T::Currency as NativeInspect<T::AccountId>>::minimum_balance();
      let current_native = <T::Currency as NativeInspect<T::AccountId>>::balance(&zap_account);
      let native_available = current_native.saturating_sub(native_ed);

      // Price Protection: Validate Spot vs Oracle
      let (reserve_native, reserve_foreign) =
        T::AssetConversion::get_pool_reserves(pool_id).unwrap_or((0, 0));

      if !reserve_native.is_zero() && !reserve_foreign.is_zero() {
        let precision = primitives::params::PRECISION;
        let spot_price = reserve_native
          .saturating_mul(precision)
          .checked_div(reserve_foreign)
          .ok_or(Error::<T>::ArithmeticOverflow)?;

        T::PriceOracle::validate_price_deviation(token_asset, native_asset, spot_price)
          .map_err(|_| Error::<T>::PriceDeviationExceeded)?;
      }

      // Step 1: Calculate opportunistic amounts (add as-is)
      let (native_to_add, foreign_to_add) = if reserve_native.is_zero() || reserve_foreign.is_zero()
      {
        (native_available, foreign_available)
      } else {
        let foreign_optimal =
          Self::quote_amount(native_available, reserve_native, reserve_foreign)?;

        if foreign_optimal <= foreign_available {
          // Limited by Native
          (native_available, foreign_optimal)
        } else {
          // Limited by Foreign
          let native_optimal =
            Self::quote_amount(foreign_available, reserve_foreign, reserve_native)?;
          (native_optimal.min(native_available), foreign_available)
        }
      };

      let mut lp_tokens_minted = 0u128;
      let mut native_used = 0u128;
      let mut foreign_used = 0u128;

      // Execute Add Liquidity
      if native_to_add > 0 && foreign_to_add > 0 {
        let (used_native, used_foreign, lp_minted) = T::AssetConversion::add_liquidity(
          &zap_account,
          native_asset,
          token_asset,
          native_to_add,
          foreign_to_add,
          0,
          0,
        )?;

        lp_tokens_minted = lp_minted;
        native_used = used_native;
        foreign_used = used_foreign;

        // Immediately transfer LP tokens to Treasury (Protocol Owned Liquidity)
        Self::transfer_lp_tokens_to_tol(pool_id, lp_tokens_minted)?;

        Self::deposit_event(Event::LPTokensDistributed {
          token_asset: pool_id,
          lp_amount: lp_tokens_minted,
          destination: T::TolTreasuryAccount::get(),
        });
      }

      // Step 2: Manage Foreign Surplus (Swap to Native)
      let foreign_surplus = foreign_available.saturating_sub(foreign_used);
      let dust_threshold = T::DustThreshold::get();

      if foreign_surplus > dust_threshold {
        if let Ok(native_received) = T::AssetConversion::swap_exact_tokens_for_tokens(
          &zap_account,
          token_asset,
          native_asset,
          foreign_surplus,
          0,
        ) {
          Self::deposit_event(Event::SurplusSwapped {
            asset_in: token_asset,
            asset_out: native_asset,
            amount_in: foreign_surplus,
            amount_out: native_received,
          });
        }
      }

      // Step 3: Manage Native Surplus (Patriotic Accumulation)
      let current_native_after =
        <T::Currency as NativeInspect<T::AccountId>>::balance(&zap_account);
      let native_surplus = current_native_after.saturating_sub(native_ed);

      if native_surplus > dust_threshold {
        // We do nothing but emit an event to track accumulation
        Self::deposit_event(Event::NativeHeld {
          amount: native_surplus,
        });
      }

      if lp_tokens_minted > 0 {
        Self::deposit_event(Event::ZapCompleted {
          token_asset,
          native_used,
          foreign_used,
          lp_tokens_minted,
        });
      }

      Ok(())
    }

    /// Calculate optimal amount given reserves (XYK formula)
    fn quote_amount(
      amount_a: u128,
      reserve_a: u128,
      reserve_b: u128,
    ) -> Result<u128, DispatchError> {
      if reserve_a.is_zero() {
        return Err(Error::<T>::ArithmeticOverflow.into());
      }

      use polkadot_sdk::sp_core::U256;

      let amount_a_u256 = U256::from(amount_a);
      let reserve_b_u256 = U256::from(reserve_b);
      let reserve_a_u256 = U256::from(reserve_a);

      let result = amount_a_u256
        .checked_mul(reserve_b_u256)
        .ok_or(Error::<T>::ArithmeticOverflow)?
        .checked_div(reserve_a_u256)
        .ok_or(Error::<T>::ArithmeticOverflow)?;

      if result > U256::from(u128::MAX) {
        return Err(Error::<T>::ArithmeticOverflow.into());
      }

      Ok(result.as_u128())
    }

    /// Transfer minted LP tokens to the TOL Treasury
    fn transfer_lp_tokens_to_tol(lp_token: AssetKind, lp_amount: u128) -> DispatchResult {
      let zap_account = Self::account_id();
      let tol_account = T::TolTreasuryAccount::get();

      match lp_token {
        AssetKind::Native => {
          <T::Currency as NativeMutate<T::AccountId>>::transfer(
            &zap_account,
            &tol_account,
            lp_amount,
            Preservation::Expendable,
          )?;
        }
        AssetKind::Local(id) | AssetKind::Foreign(id) => {
          T::Assets::transfer(
            id,
            &zap_account,
            &tol_account,
            lp_amount,
            Preservation::Expendable,
          )?;
        }
      }

      Ok(())
    }
  }
}
