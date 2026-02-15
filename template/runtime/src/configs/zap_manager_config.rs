//! Zap Manager pallet configuration for the parachain runtime.
//!
//! Configures the token-driven liquidity provisioning system for TMCTOL framework
//! with automated LP token creation and token-resolved transfer to TOL ingress accounts.
//!
//! All account IDs and economic parameters are imported from `primitives::ecosystem`
//! to maintain a single source of truth across the runtime.

use super::*;
use alloc::boxed::Box;
use polkadot_sdk::sp_runtime::DispatchError;
use polkadot_sdk::*;
use primitives::{AssetInspector, AssetKind, ecosystem};

use crate::configs::axial_router_config::{AssetConversionAdapter, PriceOracleImpl};
use crate::{AssetConversion, RuntimeOrigin};
use pallet_axial_router::AssetConversionApi as AxialRouterApi;

parameter_types! {
  /// Precision for calculations (ecosystem constant: 10^12)
  pub const ZapManagerPrecision: Balance = ecosystem::params::PRECISION;

  /// Minimum swap amount for foreign assets (ecosystem constant: 1e18)
  pub const ZapManagerMinSwapForeign: Balance = ecosystem::params::ZAP_MANAGER_MIN_SWAP_FOREIGN;

  /// Dust threshold for surplus handling (ecosystem constant: 0.01e18)
  pub const ZapManagerDustThreshold: Balance = ecosystem::params::ZAP_MANAGER_DUST_THRESHOLD;

  /// Retry cooldown for failed zaps (ecosystem constant: 10 blocks)
  pub const ZapManagerRetryCooldown: u32 = ecosystem::params::ZAP_MANAGER_RETRY_COOLDOWN;

  /// Zap Manager pallet ID
  pub const ZapManagerPalletId: PalletId = PalletId(*ecosystem::pallet_ids::ZAP_MANAGER_PALLET_ID);
}

/// Token-scoped TOL account resolver for Zap LP routing
pub struct TolAccountResolver;
impl pallet_zap_manager::TolAccountResolver<AccountId> for TolAccountResolver {
  fn resolve_tol_account(token_asset: AssetKind) -> AccountId {
    if token_asset.is_lp() {
      if let Some(ingress) =
        crate::TreasuryOwnedLiquidity::ingress_account_for_lp_asset(token_asset)
      {
        return ingress;
      }
    }
    crate::TreasuryOwnedLiquidity::ingress_account_for_token(token_asset)
  }
}

impl pallet_zap_manager::AssetConversionApi<AccountId, u128> for AssetConversionAdapter {
  fn get_pool_id(asset1: AssetKind, asset2: AssetKind) -> Option<AssetKind> {
    let pool_id =
      <AssetConversionAdapter as AxialRouterApi<AccountId, Balance>>::get_pool_id(asset1, asset2)?;

    let pool_info = pallet_asset_conversion::Pools::<Runtime>::get(pool_id)?;
    Some(AssetKind::Local(pool_info.lp_token))
  }

  fn get_pool_reserves(pool_id: AssetKind) -> Option<(u128, u128)> {
    let lp_token_id = match pool_id {
      AssetKind::Local(id) | AssetKind::Foreign(id) => id,
      _ => return None,
    };

    for (pool_key, pool_info) in pallet_asset_conversion::Pools::<Runtime>::iter() {
      if pool_info.lp_token == lp_token_id {
        let (asset1, asset2) = pool_key;
        return AssetConversion::get_reserves(asset1, asset2).ok();
      }
    }

    None
  }

  fn create_pool(asset1: AssetKind, asset2: AssetKind) -> Result<AssetKind, DispatchError> {
    let zap_account = pallet_zap_manager::Pallet::<Runtime>::account_id();
    AssetConversion::create_pool(
      RuntimeOrigin::signed(zap_account),
      Box::new(asset1),
      Box::new(asset2),
    )?;

    <Self as pallet_zap_manager::AssetConversionApi<AccountId, u128>>::get_pool_id(asset1, asset2)
      .ok_or(DispatchError::Other("Failed to calculate pool ID"))
  }

  fn add_liquidity(
    who: &AccountId,
    asset1: AssetKind,
    asset2: AssetKind,
    amount1_desired: u128,
    amount2_desired: u128,
    amount1_min: u128,
    amount2_min: u128,
  ) -> Result<(u128, u128, u128), DispatchError> {
    use polkadot_sdk::frame_support::traits::fungible::Inspect as NativeInspect;
    use polkadot_sdk::frame_support::traits::fungibles::Inspect as FungiblesInspect;

    let lp_token = <Self as pallet_zap_manager::AssetConversionApi<AccountId, u128>>::get_pool_id(
      asset1, asset2,
    )
    .ok_or(DispatchError::Other("Pool does not exist"))?;

    let lp_balance_before = match lp_token {
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
      _ => 0,
    };

    let asset1_before = match asset1 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };
    let asset2_before = match asset2 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };

    AssetConversion::add_liquidity(
      RuntimeOrigin::signed(who.clone()),
      Box::new(asset1),
      Box::new(asset2),
      amount1_desired,
      amount2_desired,
      amount1_min,
      amount2_min,
      who.clone(),
    )?;

    let lp_balance_after = match lp_token {
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
      _ => 0,
    };

    let asset1_after = match asset1 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };
    let asset2_after = match asset2 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };

    let used_asset1 = asset1_before.saturating_sub(asset1_after);
    let used_asset2 = asset2_before.saturating_sub(asset2_after);
    let minted = lp_balance_after.saturating_sub(lp_balance_before);

    Ok((used_asset1, used_asset2, minted))
  }

  fn swap_exact_tokens_for_tokens(
    who: &AccountId,
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_in: u128,
    amount_out_min: u128,
  ) -> Result<u128, DispatchError> {
    // Route through axial router (fee=0 for system accounts)
    pallet_axial_router::Pallet::<Runtime>::execute_swap_for(
      who,
      asset_in,
      asset_out,
      amount_in,
      amount_out_min,
      who,
    )
  }
}

pub struct ZapPriceOracleAdapter;

impl pallet_zap_manager::PriceOracle<u128> for ZapPriceOracleAdapter {
  fn get_ema_price(asset_in: AssetKind, asset_out: AssetKind) -> Option<u128> {
    <PriceOracleImpl<Runtime> as pallet_axial_router::PriceOracle<u128>>::get_ema_price(
      asset_in, asset_out,
    )
  }

  fn validate_price_deviation(
    asset_in: AssetKind,
    asset_out: AssetKind,
    current_price: u128,
  ) -> Result<(), DispatchError> {
    <PriceOracleImpl<Runtime> as pallet_axial_router::PriceOracle<u128>>::validate_price_deviation(
      asset_in,
      asset_out,
      current_price,
    )
  }
}

impl pallet_zap_manager::Config for Runtime {
  type AssetConversion = AssetConversionAdapter;
  type Assets = pallet_assets::Pallet<Runtime>;
  type Currency = Balances;
  type MinSwapForeign = ZapManagerMinSwapForeign;
  type DustThreshold = ZapManagerDustThreshold;
  type RetryCooldown = ZapManagerRetryCooldown;
  type PalletId = ZapManagerPalletId;
  type PriceOracle = ZapPriceOracleAdapter;
  type TolAccountResolver = TolAccountResolver;
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type WeightInfo = crate::weights::pallet_zap_manager::SubstrateWeight<Runtime>;
}
