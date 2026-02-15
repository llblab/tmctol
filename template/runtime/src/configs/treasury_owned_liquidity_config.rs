//! Treasury-Owned Liquidity pallet configuration for the parachain runtime.
//!
//! Configures the `tol_id`-scoped 4-bucket TOL distribution system for TMCTOL framework
//! with mathematical floor price guarantees and capital efficiency optimization
//! through Zap algorithm integration with AssetConversion pallet.
//!
//! All account IDs and economic parameters are imported from `primitives::ecosystem`
//! to maintain a single source of truth across the runtime.

use super::*;

use polkadot_sdk::frame_support::traits::Get;
use polkadot_sdk::frame_support::traits::fungible::Inspect as NativeInspect;
use polkadot_sdk::frame_support::traits::fungibles::Inspect as FungiblesInspect;
use polkadot_sdk::*;
use primitives::assets::TYPE_LP;
use primitives::{AssetKind, ecosystem};
use sp_runtime::{DispatchError, Permill, traits::AccountIdConversion};

use crate::configs::axial_router_config::AssetConversionAdapter;
use crate::{AssetConversion, RuntimeOrigin};
use pallet_axial_router::AssetConversionApi as AxialRouterApi;

/// Compatibility treasury destination for TOL config surface
/// Mapped to the TOL pallet account by default
pub struct TolTreasuryAccount;
impl Get<AccountId> for TolTreasuryAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::TOL_PALLET_ID).into_account_truncating()
  }
}

/// Zap manager account for receiving LP tokens (derived from PalletId)
pub struct ZapManagerAccount;
impl Get<AccountId> for ZapManagerAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::ZAP_MANAGER_PALLET_ID).into_account_truncating()
  }
}

/// Bucket A (Anchor) account for LP token storage (derived from ID)
pub struct BucketAAccount;
impl Get<AccountId> for BucketAAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_A_ID).into_account_truncating()
  }
}

/// Bucket B (Building) account for LP token storage (derived from ID)
pub struct BucketBAccount;
impl Get<AccountId> for BucketBAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_B_ID).into_account_truncating()
  }
}

/// Bucket C (Capital) account for LP token storage (derived from ID)
pub struct BucketCAccount;
impl Get<AccountId> for BucketCAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_C_ID).into_account_truncating()
  }
}

/// Bucket D (Dormant) account for LP token storage (derived from ID)
pub struct BucketDAccount;
impl Get<AccountId> for BucketDAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_D_ID).into_account_truncating()
  }
}

parameter_types! {
  /// Precision for mathematical calculations (ecosystem constant: 10^12)
  pub const TolPrecision: Balance = ecosystem::params::PRECISION;

  /// Minimum foreign amount for swap operations (ecosystem constant: 1e18)
  pub const TolMinSwapForeign: Balance = ecosystem::params::TOL_MIN_SWAP_FOREIGN;

  /// Maximum price deviation for swaps (ecosystem constant: 20%)
  pub const TolMaxPriceDeviation: Permill = ecosystem::params::TOL_MAX_PRICE_DEVIATION;

  /// Target allocation for TOL bucket A Anchor (ecosystem constant: 50%)
  pub const TolBucketAAllocation: u32 = ecosystem::params::TOL_BUCKET_A_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket B Building (ecosystem constant: 16.67%)
  pub const TolBucketBAllocation: u32 = ecosystem::params::TOL_BUCKET_B_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket C Capital (ecosystem constant: 16.67%)
  pub const TolBucketCAllocation: u32 = ecosystem::params::TOL_BUCKET_C_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket D Dormant (ecosystem constant: 16.66%)
  pub const TolBucketDAllocation: u32 = ecosystem::params::TOL_BUCKET_D_ALLOCATION.deconstruct();

  /// Bucket A (Anchor) distribution ratio (50%)
  pub const TolBucketARatio: Permill = ecosystem::params::TOL_BUCKET_A_ALLOCATION;

  /// Bucket B (Building) distribution ratio (16.67%)
  pub const TolBucketBRatio: Permill = ecosystem::params::TOL_BUCKET_B_ALLOCATION;

  /// Bucket C (Capital) distribution ratio (16.67%)
  pub const TolBucketCRatio: Permill = ecosystem::params::TOL_BUCKET_C_ALLOCATION;

  /// Bucket D (Dormant) distribution ratio (16.67%)
  pub const TolBucketDRatio: Permill = ecosystem::params::TOL_BUCKET_D_ALLOCATION;

  /// Maximum TOL requests to process per block
  pub const TolMaxRequestsPerBlock: u32 = 10;

  /// Maximum bucket non-LP sweeps processed in one idle block
  pub const TolMaxNonLpSweepsPerBlock: u32 = 16;

  /// Maximum active TOL domains
  pub const TolMaxDomains: u32 = 1024;

  /// Pallet ID for the TOL pallet
  pub const TolPalletId: PalletId = PalletId(*b"tolpalle");
}

impl pallet_treasury_owned_liquidity::AssetConversionApi<AccountId, Balance>
  for AssetConversionAdapter
{
  fn get_pool_id(asset_a: AssetKind, asset_b: AssetKind) -> Option<[u8; 32]> {
    <AssetConversionAdapter as AxialRouterApi<AccountId, Balance>>::get_pool_id(asset_a, asset_b)
      .map(AssetConversionAdapter::encode_pool_id)
  }

  fn get_pool_reserves(pool_id: [u8; 32]) -> Option<(Balance, Balance)> {
    let (asset_a, asset_b) = AssetConversionAdapter::decode_pool_id(pool_id)?;
    <AssetConversionAdapter as AxialRouterApi<AccountId, Balance>>::get_pool_reserves((
      asset_a, asset_b,
    ))
  }

  fn quote_price_exact_tokens_for_tokens(
    asset_in: AssetKind,
    asset_out: AssetKind,
    amount_in: Balance,
    include_fee: bool,
  ) -> Option<Balance> {
    <AssetConversionAdapter as AxialRouterApi<AccountId, Balance>>::quote_price_exact_tokens_for_tokens(
      asset_in,
      asset_out,
      amount_in,
      include_fee,
    )
  }

  fn swap_exact_tokens_for_tokens(
    who: &AccountId,
    path: alloc::vec::Vec<AssetKind>,
    amount_in: Balance,
    min_amount_out: Balance,
    recipient: AccountId,
    keep_alive: bool,
  ) -> Result<Balance, DispatchError> {
    <AssetConversionAdapter as AxialRouterApi<AccountId, Balance>>::swap_exact_tokens_for_tokens(
      who.clone(),
      path,
      amount_in,
      min_amount_out,
      recipient,
      keep_alive,
    )
  }

  fn remove_liquidity(
    who: &AccountId,
    asset1: AssetKind,
    asset2: AssetKind,
    lp_amount: Balance,
  ) -> Result<(Balance, Balance), DispatchError> {
    use alloc::boxed::Box;

    let before1 = match asset1 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };
    let before2 = match asset2 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };

    AssetConversion::remove_liquidity(
      RuntimeOrigin::signed(who.clone()),
      Box::new(asset1),
      Box::new(asset2),
      lp_amount,
      0,
      0,
      who.clone(),
    )?;

    let after1 = match asset1 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };
    let after2 = match asset2 {
      AssetKind::Native => <Balances as NativeInspect<AccountId>>::balance(who),
      AssetKind::Local(id) | AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as FungiblesInspect<AccountId>>::balance(id, who)
      }
    };

    Ok((
      after1.saturating_sub(before1),
      after2.saturating_sub(before2),
    ))
  }

  fn get_pool_pair_for_lp(lp_token_id: u32) -> Option<(AssetKind, AssetKind)> {
    for (pool_key, pool_info) in pallet_asset_conversion::Pools::<Runtime>::iter() {
      if pool_info.lp_token == lp_token_id {
        return Some(pool_key);
      }
    }
    None
  }

  fn initialize_lp_asset_namespace() {
    let lp_start = TYPE_LP | 1;
    pallet_asset_conversion::NextPoolAssetId::<Runtime>::mutate(|next_id| {
      if next_id.is_none_or(|current| current < lp_start) {
        *next_id = Some(lp_start);
      }
    });
  }
}

impl pallet_treasury_owned_liquidity::pallet::Config for Runtime {
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type AssetConversion = AssetConversionAdapter;
  type Assets = pallet_assets::Pallet<Runtime>;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = RuntimeTolBenchmarkHelper;
  type BucketAAccount = BucketAAccount;
  type BucketAAllocation = TolBucketAAllocation;
  type BucketARatio = TolBucketARatio;
  type BucketBAccount = BucketBAccount;
  type BucketBAllocation = TolBucketBAllocation;
  type BucketBRatio = TolBucketBRatio;
  type BucketCAccount = BucketCAccount;
  type BucketCAllocation = TolBucketCAllocation;
  type BucketCRatio = TolBucketCRatio;
  type BucketDAccount = BucketDAccount;
  type BucketDAllocation = TolBucketDAllocation;
  type BurningManagerAccount = BurningManagerAccount;
  type Currency = Balances;
  type MaxNonLpSweepsPerBlock = TolMaxNonLpSweepsPerBlock;
  type MaxPriceDeviation = TolMaxPriceDeviation;
  type MaxTolDomains = TolMaxDomains;
  type MaxTolRequestsPerBlock = TolMaxRequestsPerBlock;
  type MinSwapForeign = TolMinSwapForeign;
  type PalletId = TolPalletId;
  type Precision = TolPrecision;
  type TreasuryAccount = TolTreasuryAccount;
  type WeightInfo = crate::weights::pallet_treasury_owned_liquidity::SubstrateWeight<Runtime>;
  type ZapManagerAccount = ZapManagerAccount;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct RuntimeTolBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_treasury_owned_liquidity::BenchmarkHelper<AccountId> for RuntimeTolBenchmarkHelper {
  fn create_asset(asset_id: u32) -> sp_runtime::DispatchResult {
    use polkadot_sdk::frame_support::traits::fungibles::Inspect;

    if !<pallet_assets::Pallet<Runtime> as Inspect<AccountId>>::asset_exists(asset_id) {
      let admin = crate::configs::axial_router_config::BurningManagerAccount::get();
      pallet_assets::Pallet::<Runtime>::force_create(
        RuntimeOrigin::root(),
        asset_id,
        sp_runtime::MultiAddress::Id(admin),
        true,
        1,
      )?;
    }
    Ok(())
  }

  fn fund_account(
    who: &AccountId,
    asset: primitives::AssetKind,
    amount: Balance,
  ) -> sp_runtime::DispatchResult {
    use polkadot_sdk::frame_support::traits::{Currency, fungibles::Mutate};

    match asset {
      primitives::AssetKind::Native => {
        let _ = <Balances as Currency<AccountId>>::deposit_creating(who, amount);
      }
      primitives::AssetKind::Local(id) | primitives::AssetKind::Foreign(id) => {
        <pallet_assets::Pallet<Runtime> as Mutate<AccountId>>::mint_into(id, who, amount)?;
      }
    }
    Ok(())
  }
}
