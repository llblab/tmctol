//! Axial Router pallet configuration for the parachain runtime.
//!
//! Configures the minimalist multi-token routing system optimized for TMC ecosystems
//! with Native-anchored routing and advanced fee processing.

use super::*;

use alloc::{boxed::Box, vec::Vec};
use codec::{Decode, Encode};
use polkadot_sdk::frame_support::pallet_prelude::Zero;
use polkadot_sdk::frame_support::traits::fungible::Inspect as NativeInspect;
use polkadot_sdk::frame_support::traits::{
  fungibles::{Inspect as FungiblesInspect, Mutate},
  Currency, Get,
};

use polkadot_sdk::sp_runtime::{
  traits::AccountIdConversion, ArithmeticError, DispatchError, Permill, TokenError,
};
use polkadot_sdk::*;

use crate::{AssetConversion, RuntimeOrigin};
use primitives::{ecosystem, AssetKind};

parameter_types! {
  /// Router fee as Permill (derived from ecosystem constant 50bps = 0.5%)
  pub const AxialRouterFee: Permill = ecosystem::params::AXIAL_ROUTER_FEE;

  /// Native asset (AssetKind::Native)
  pub const NativeAsset: AssetKind = AssetKind::Native;

  /// Pallet ID for the Axial router
  pub const AxialRouterPalletId: PalletId = PalletId(*ecosystem::pallet_ids::AXIAL_ROUTER_PALLET_ID);

  /// Minimum foreign amount for swapping (threshold for buffer processing)
  pub const MinSwapForeign: Balance = ecosystem::params::MIN_SWAP_FOREIGN;

  /// Precision constant for all calculations
  pub const AxialRouterPrecision: Balance = ecosystem::params::PRECISION;

  /// EMA oracle half-life in blocks
  pub const AxialRouterEmaHalfLife: u32 = ecosystem::params::EMA_HALF_LIFE_BLOCKS;

  /// Maximum price deviation allowed
  pub const AxialRouterMaxPriceDeviation: Permill = ecosystem::params::MAX_PRICE_DEVIATION;

  /// Maximum number of hops in multi-hop routing
  pub const AxialRouterMaxHops: u32 = ecosystem::params::MAX_HOPS;
}

pub struct BurningManagerAccount;

impl polkadot_sdk::frame_support::traits::Get<AccountId> for BurningManagerAccount {
  fn get() -> AccountId {
    // Use derived account ID for token-driven coordination (modl prefix)
    PalletId(*ecosystem::pallet_ids::BURNING_MANAGER_PALLET_ID).into_account_truncating()
  }
}

/// TMC pallet adapter for Axial Router integration
pub struct TmcPalletAdapter<T: pallet_axial_router::pallet::Config>(core::marker::PhantomData<T>);

/// Price oracle implementation for manipulation-resistant pricing
pub struct PriceOracleImpl<T: pallet_axial_router::pallet::Config>(core::marker::PhantomData<T>);

/// Token-driven fee manager implementation with account-based coordination
pub struct FeeManagerImpl<T: pallet_axial_router::pallet::Config>(core::marker::PhantomData<T>);

pub struct AssetConversionAdapter;

impl AssetConversionAdapter {
  pub fn encode_pool_id(pool: (AssetKind, AssetKind)) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    let encoded = pool.encode();
    if encoded.len() <= 32 {
      bytes[..encoded.len()].copy_from_slice(&encoded);
    }
    bytes
  }

  pub fn decode_pool_id(pool_id: [u8; 32]) -> Option<(AssetKind, AssetKind)> {
    let mut slice = &pool_id[..];
    <(AssetKind, AssetKind)>::decode(&mut slice).ok()
  }
}

impl pallet_axial_router::AssetConversionApi<AccountId, Balance> for AssetConversionAdapter {
  fn get_pool_id(asset_a: AssetKind, asset_b: AssetKind) -> Option<(AssetKind, AssetKind)> {
    if asset_a == asset_b {
      return None;
    }

    if asset_a < asset_b {
      Some((asset_a, asset_b))
    } else {
      Some((asset_b, asset_a))
    }
  }

  fn get_pool_reserves(pool_id: (AssetKind, AssetKind)) -> Option<(Balance, Balance)> {
    let (asset_a, asset_b) = pool_id;
    AssetConversion::get_reserves(asset_a, asset_b).ok()
  }

  fn quote_price_exact_tokens_for_tokens(
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_in: Balance,
    include_fee: bool,
  ) -> Option<Balance> {
    AssetConversion::quote_price_exact_tokens_for_tokens(
      asset_in,
      asset_out,
      amount_in,
      include_fee,
    )
  }

  fn swap_exact_tokens_for_tokens(
    who: AccountId,
    path: Vec<AssetKind>,
    amount_in: Balance,
    min_amount_out: Balance,
    recipient: AccountId,
    keep_alive: bool,
  ) -> Result<Balance, sp_runtime::DispatchError> {
    if path.len() < 2usize {
      return Err(DispatchError::Other("Invalid asset path"));
    }

    // Get target asset and snapshot balance before swap
    let target_asset = *path.last().unwrap();

    // Snapshot recipient balance before swap
    let balance_before = match target_asset {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(&recipient),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, &recipient)
      }
    };

    // Convert path from RouterAssetKind to AssetKind and box it
    let boxed_path: Vec<Box<AssetKind>> = path.iter().cloned().map(Box::new).collect();

    let origin = RuntimeOrigin::signed(who.clone());
    AssetConversion::swap_exact_tokens_for_tokens(
      origin,
      boxed_path,
      amount_in,
      min_amount_out,
      recipient.clone(),
      keep_alive,
    )?;

    // Snapshot recipient balance after swap and calculate actual amount received
    let balance_after = match target_asset {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(&recipient),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, &recipient)
      }
    };

    let actual_amount_out = balance_after.saturating_sub(balance_before);

    // Return actual amount received instead of calculated quote
    Ok(actual_amount_out)
  }
}

impl<T> pallet_axial_router::TmcInterface<T::AccountId, Balance> for TmcPalletAdapter<T>
where
  T: pallet_axial_router::pallet::Config
    + pallet_token_minting_curve::pallet::Config<Balance = Balance>,
{
  fn has_curve(asset: AssetKind) -> bool {
    pallet_token_minting_curve::Pallet::<T>::has_curve(asset)
  }

  fn calculate_user_receives(
    foreign_asset: AssetKind,
    foreign_amount: Balance,
  ) -> Result<Balance, sp_runtime::DispatchError> {
    pallet_token_minting_curve::Pallet::<T>::calculate_user_receives(foreign_asset, foreign_amount)
  }

  fn mint_with_distribution(
    who: &T::AccountId,
    foreign_asset: AssetKind,
    foreign_amount: Balance,
  ) -> Result<Balance, sp_runtime::DispatchError> {
    pallet_token_minting_curve::Pallet::<T>::mint_with_distribution(
      who,
      foreign_asset,
      foreign_amount,
    )
  }

  fn do_burn_internal(
    token_asset: AssetKind,
    amount: Balance,
  ) -> Result<(), sp_runtime::DispatchError> {
    pallet_token_minting_curve::Pallet::<T>::do_burn_internal(token_asset, amount)
  }

  fn burn_tokens(token_asset: AssetKind, amount: Balance) -> Result<(), sp_runtime::DispatchError> {
    if amount.is_zero() {
      return Err(TokenError::BelowMinimum.into());
    }

    // Use secure direct internal call instead of Origin spoofing
    pallet_token_minting_curve::Pallet::<T>::do_burn_internal(token_asset, amount)
  }
}

impl<T: pallet_axial_router::pallet::Config> pallet_axial_router::PriceOracle<Balance>
  for PriceOracleImpl<T>
{
  fn update_ema_price(
    asset_in: AssetKind,
    asset_out: AssetKind,
    price: Balance,
    _tvl: Balance,
  ) -> Result<(), sp_runtime::DispatchError> {
    let ema_half_life = T::EmaHalfLife::get();

    // Get current block number
    let _current_block = polkadot_sdk::frame_system::Pallet::<T>::block_number();

    // Get previous EMA price
    let previous_ema_price = pallet_axial_router::EmaPrices::<T>::get(asset_in, asset_out);

    // Calculate new EMA price
    let new_ema_price = if previous_ema_price.is_zero() {
      price // First update, use current price
    } else {
      let alpha = polkadot_sdk::sp_runtime::Permill::from_rational(1u32, ema_half_life + 1);
      let ema_part1 = alpha.mul_floor(price);
      let ema_part2 = (polkadot_sdk::sp_runtime::Permill::from_percent(100) - alpha)
        .mul_floor(previous_ema_price);
      ema_part1 + ema_part2
    };

    // Store updated EMA price
    pallet_axial_router::EmaPrices::<T>::insert(asset_in, asset_out, new_ema_price);

    // Event emission is handled by the Axial Router pallet itself during operations
    // The EMA price update is stored and will be available for tracking

    Ok(())
  }

  fn get_ema_price(asset_in: AssetKind, asset_out: AssetKind) -> Option<Balance> {
    Some(pallet_axial_router::EmaPrices::<T>::get(
      asset_in, asset_out,
    ))
  }

  fn validate_price_deviation(
    asset_in: AssetKind,
    asset_out: AssetKind,
    current_price: Balance,
  ) -> Result<(), sp_runtime::DispatchError> {
    let max_price_deviation = T::MaxPriceDeviation::get();

    if let Some(ema_price) = Self::get_ema_price(asset_in, asset_out) {
      if ema_price.is_zero() {
        return Ok(()); // No EMA data yet, skip validation
      }

      // Calculate price deviation
      let deviation = if current_price > ema_price {
        polkadot_sdk::sp_runtime::Permill::from_rational(current_price - ema_price, ema_price)
      } else {
        polkadot_sdk::sp_runtime::Permill::from_rational(ema_price - current_price, ema_price)
      };

      if deviation > max_price_deviation {
        // Price deviation events are handled by the Axial Router pallet's monitoring system
        // The deviation is logged through standard error mechanisms

        return Err(DispatchError::Other("Price deviation exceeded"));
      }
    }

    Ok(())
  }
}

impl<T: pallet_axial_router::pallet::Config>
  pallet_axial_router::FeeRoutingAdapter<T::AccountId, Balance> for FeeManagerImpl<T>
{
  fn route_fee(
    who: &T::AccountId,
    asset: AssetKind,
    amount: Balance,
  ) -> sp_runtime::DispatchResult {
    let burning_manager_account = T::BurningManagerAccount::get();

    match asset {
      AssetKind::Native => {
        let native_fee: <<T as pallet_axial_router::pallet::Config>::Currency as polkadot_sdk::frame_support::traits::Currency<T::AccountId>>::Balance = amount
          .try_into()
          .map_err(|_| DispatchError::Arithmetic(ArithmeticError::Overflow))?;
        T::Currency::transfer(
          who,
          &burning_manager_account,
          native_fee,
          polkadot_sdk::frame_support::traits::tokens::ExistenceRequirement::KeepAlive,
        )
        .map_err(|_| DispatchError::Token(TokenError::FundsUnavailable))?;
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        T::Assets::transfer(
          id,
          who,
          &burning_manager_account,
          amount,
          polkadot_sdk::frame_support::traits::tokens::Preservation::Protect,
        )
        .map_err(|_| DispatchError::Token(TokenError::FundsUnavailable))?;
      }
    }

    Ok(())
  }
}

impl pallet_axial_router::pallet::Config for Runtime {
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type AssetConversion = AssetConversionAdapter;
  type Assets = pallet_assets::Pallet<Runtime>;
  type BurningManagerAccount = BurningManagerAccount;
  type Currency = Balances;
  type DefaultRouterFee = AxialRouterFee;
  type EmaHalfLife = AxialRouterEmaHalfLife;
  type FeeAdapter = FeeManagerImpl<Runtime>;
  type MaxHops = AxialRouterMaxHops;
  type MaxPriceDeviation = AxialRouterMaxPriceDeviation;
  type MinSwapForeign = MinSwapForeign;
  type NativeAsset = NativeAsset;
  type Precision = AxialRouterPrecision;
  type PriceOracle = PriceOracleImpl<Runtime>;
  type TmcPallet = TmcPalletAdapter<Runtime>;
  type WeightInfo = crate::weights::pallet_axial_router::SubstrateWeight<Runtime>;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = RuntimeBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RuntimeBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_axial_router::types::BenchmarkHelper<AssetKind, AccountId, Balance>
  for RuntimeBenchmarkHelper
{
  fn create_asset(asset: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult {
    if let AssetKind::Local(id) | AssetKind::Foreign(id) = asset {
      if !<pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::asset_exists(id) {
        let _ = pallet_assets::Pallet::<Runtime>::force_create(
          RuntimeOrigin::root(),
          id,
          polkadot_sdk::sp_runtime::MultiAddress::Id(BurningManagerAccount::get()),
          true,
          1,
        );
      }
    }
    Ok(())
  }

  fn mint_asset(
    asset: AssetKind,
    to: &AccountId,
    amount: Balance,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    match asset {
      AssetKind::Native => {
        let _ = <Balances as Currency<AccountId>>::deposit_creating(to, amount);
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as Mutate<AccountId>>::mint_into(id, to, amount)?;
      }
    }
    Ok(())
  }

  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult {
    let creator = BurningManagerAccount::get();
    let _ =
      <Balances as Currency<AccountId>>::deposit_creating(&creator, 1_000_000_000_000_000_000);

    AssetConversion::create_pool(
      RuntimeOrigin::signed(creator),
      Box::new(asset1),
      Box::new(asset2),
    )?;
    Ok(())
  }

  fn add_liquidity(
    who: &AccountId,
    asset1: AssetKind,
    asset2: AssetKind,
    amount1: Balance,
    amount2: Balance,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    AssetConversion::add_liquidity(
      RuntimeOrigin::signed(who.clone()),
      Box::new(asset1),
      Box::new(asset2),
      amount1,
      amount2,
      0,
      0,
      who.clone(),
    )?;
    Ok(())
  }
}
