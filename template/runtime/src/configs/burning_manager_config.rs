//! Burning Manager pallet configuration for the parachain runtime.
//!
//! Configures the token-driven fee processing system for TMCTOL framework.
//!
//! All ecosystem account IDs and economic parameters are imported from
//! `primitives::ecosystem`, serving as the single source of truth.

use super::*;
use crate::configs::axial_router_config::AssetConversionAdapter;
use alloc::vec::Vec;
use pallet_axial_router::AssetConversionApi as AxialRouterConversionApi;
use polkadot_sdk::sp_runtime::{DispatchError, Permill};
use polkadot_sdk::*;
use primitives::{ecosystem, AssetKind};

parameter_types! {
  /// Dust threshold in reference asset units (ecosystem constant: 0.1)
  pub const BurningManagerDustThreshold: Balance = ecosystem::params::BURNING_MANAGER_DUST_THRESHOLD;

  /// Minimum amount of native tokens required to trigger burning (ecosystem constant: 10)
  pub const BurningManagerMinBurnNative: Balance = ecosystem::params::BURNING_MANAGER_MIN_BURN_NATIVE;

  /// Pallet ID for the burning manager (from ecosystem constants)
  pub const BurningManagerPalletId: PalletId = PalletId(*ecosystem::pallet_ids::BURNING_MANAGER_PALLET_ID);

  /// Precision for price calculations (ecosystem constant: 10^12)
  pub const BurningManagerPrecision: Balance = ecosystem::params::PRECISION;

  /// Reference asset for threshold checks (ecosystem constant: Native)
  pub const BurningManagerReferenceAsset: AssetKind = AssetKind::Native;

  /// Slippage tolerance for swaps (ecosystem constant: 2%)
  pub const BurningManagerSlippageTolerance: Permill = ecosystem::params::BURNING_MANAGER_SLIPPAGE_TOLERANCE;
}

impl pallet_burning_manager::AssetConversionApi<AccountId, u128> for AssetConversionAdapter {
  fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<[u8; 32]> {
    <AssetConversionAdapter as AxialRouterConversionApi<AccountId, Balance>>::get_pool_id(
      asset1, asset2,
    )
    .map(AssetConversionAdapter::encode_pool_id)
  }

  fn get_pool_reserves(pool_id: [u8; 32]) -> Option<(u128, u128)> {
    let pool_pair = AssetConversionAdapter::decode_pool_id(pool_id)?;
    <AssetConversionAdapter as AxialRouterConversionApi<AccountId, Balance>>::get_pool_reserves(
      pool_pair,
    )
  }

  fn swap_exact_tokens_for_tokens(
    who: &AccountId,
    path: Vec<AssetKind>,
    amount_in: u128,
    min_amount_out: u128,
  ) -> Result<u128, DispatchError> {
    <AssetConversionAdapter as AxialRouterConversionApi<AccountId, Balance>>::swap_exact_tokens_for_tokens(
      who.clone(),
      path,
      amount_in,
      min_amount_out,
      who.clone(),
      true,
    )
  }
}

/// Price Tools Adapter for burning manager
pub struct BurningManagerPriceToolsAdapter;

impl pallet_burning_manager::PriceTools<AssetKind, u128> for BurningManagerPriceToolsAdapter {
  fn quote_spot_price(
    asset_from: AssetKind,
    asset_to: AssetKind,
    amount: u128,
  ) -> Result<u128, DispatchError> {
    pallet_axial_router::Pallet::<Runtime>::quote_price(asset_from, asset_to, amount)
  }

  fn get_oracle_price(asset_from: AssetKind, asset_to: AssetKind) -> Option<u128> {
    pallet_axial_router::Pallet::<Runtime>::get_oracle_price(asset_from, asset_to)
  }
}

impl pallet_burning_manager::Config for Runtime {
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type AssetConversion = AssetConversionAdapter;
  type Assets = pallet_assets::Pallet<Runtime>;
  type Currency = Balances;
  type DefaultDustThreshold = BurningManagerDustThreshold;
  type DefaultMinBurnNative = BurningManagerMinBurnNative;
  type DefaultSlippageTolerance = BurningManagerSlippageTolerance;
  type PalletId = BurningManagerPalletId;
  type Precision = BurningManagerPrecision;
  type PriceTools = BurningManagerPriceToolsAdapter;
  type ReferenceAsset = BurningManagerReferenceAsset;
  type WeightInfo = crate::weights::pallet_burning_manager::SubstrateWeight<Runtime>;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = RuntimeBurningManagerBenchmarkHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RuntimeBurningManagerBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_burning_manager::BenchmarkHelper<AssetKind, AccountId, Balance>
  for RuntimeBurningManagerBenchmarkHelper
{
  fn ensure_funded(
    who: &AccountId,
    asset: AssetKind,
    amount: Balance,
  ) -> polkadot_sdk::sp_runtime::DispatchResult {
    use crate::configs::axial_router_config::BurningManagerAccount;
    use polkadot_sdk::frame_support::traits::{
      fungibles::{Inspect, Mutate},
      Currency,
    };

    match asset {
      AssetKind::Native => {
        let _ = <Balances as Currency<AccountId>>::deposit_creating(who, amount);
      }
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        if !<pallet_assets::Pallet<Runtime> as Inspect<AccountId>>::asset_exists(id) {
          let _ = pallet_assets::Pallet::<Runtime>::force_create(
            RuntimeOrigin::root(),
            id,
            polkadot_sdk::sp_runtime::MultiAddress::Id(BurningManagerAccount::get()),
            true,
            1,
          );
        }
        <pallet_assets::Pallet<Runtime> as Mutate<AccountId>>::mint_into(id, who, amount)?;
      }
    }
    Ok(())
  }

  fn create_asset(asset: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult {
    use crate::configs::axial_router_config::BurningManagerAccount;
    use polkadot_sdk::frame_support::traits::fungibles::Inspect;

    if let AssetKind::Local(id) | AssetKind::Foreign(id) = asset {
      if !<pallet_assets::Pallet<Runtime> as Inspect<AccountId>>::asset_exists(id) {
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

  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> polkadot_sdk::sp_runtime::DispatchResult {
    use crate::configs::axial_router_config::BurningManagerAccount;
    use alloc::boxed::Box;
    use polkadot_sdk::frame_support::traits::Currency;

    let creator = BurningManagerAccount::get();
    let _ =
      <Balances as Currency<AccountId>>::deposit_creating(&creator, 1_000_000_000_000_000_000);

    crate::AssetConversion::create_pool(
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
    use alloc::boxed::Box;

    crate::AssetConversion::add_liquidity(
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
