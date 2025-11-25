//! Treasury-Owned Liquidity pallet configuration for the parachain runtime.
//!
//! Configures the 4-bucket TOL distribution system for TMCTOL framework
//! with mathematical floor price guarantees and capital efficiency optimization
//! through Zap algorithm integration with AssetConversion pallet.
//!
//! All account IDs and economic parameters are imported from `primitives::ecosystem`
//! to maintain a single source of truth across the runtime.

use super::*;

use polkadot_sdk::frame_support::traits::Get;
use polkadot_sdk::*;
use primitives::{ecosystem, AssetKind};
use sp_runtime::{traits::AccountIdConversion, DispatchError, Permill};

use crate::configs::axial_router_config::AssetConversionAdapter;
use pallet_axial_router::AssetConversionApi as AxialRouterApi;

/// Treasury account for TOL management (derived from PalletId)
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

/// Bucket A account for LP token storage (derived from ID)
pub struct BucketAAccount;
impl Get<AccountId> for BucketAAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_A_ID).into_account_truncating()
  }
}

/// Bucket B account for LP token storage (derived from ID)
pub struct BucketBAccount;
impl Get<AccountId> for BucketBAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_B_ID).into_account_truncating()
  }
}

/// Bucket C account for LP token storage (derived from ID)
pub struct BucketCAccount;
impl Get<AccountId> for BucketCAccount {
  fn get() -> AccountId {
    PalletId(*ecosystem::pallet_ids::BUCKET_C_ID).into_account_truncating()
  }
}

/// Bucket D account for LP token storage (derived from ID)
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

  /// Target allocation for TOL bucket A (ecosystem constant: 50%)
  pub const TolBucketAAllocation: u32 = ecosystem::params::TOL_BUCKET_A_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket B (ecosystem constant: 16.67%)
  pub const TolBucketBAllocation: u32 = ecosystem::params::TOL_BUCKET_B_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket C (ecosystem constant: 16.67%)
  pub const TolBucketCAllocation: u32 = ecosystem::params::TOL_BUCKET_C_ALLOCATION.deconstruct();

  /// Target allocation for TOL bucket D (ecosystem constant: 16.66%)
  pub const TolBucketDAllocation: u32 = ecosystem::params::TOL_BUCKET_D_ALLOCATION.deconstruct();

  /// Bucket A distribution ratio (50%)
  pub const TolBucketARatio: Permill = ecosystem::params::TOL_BUCKET_A_ALLOCATION;

  /// Bucket B distribution ratio (16.67%)
  pub const TolBucketBRatio: Permill = ecosystem::params::TOL_BUCKET_B_ALLOCATION;

  /// Bucket C distribution ratio (16.67%)
  pub const TolBucketCRatio: Permill = ecosystem::params::TOL_BUCKET_C_ALLOCATION;

  /// Maximum TOL requests to process per block
  pub const TolMaxRequestsPerBlock: u32 = 10;

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
}

impl pallet_treasury_owned_liquidity::pallet::Config for Runtime {
  type AdminOrigin = frame_system::EnsureRoot<AccountId>;
  type AssetConversion = AssetConversionAdapter;
  type Assets = pallet_assets::Pallet<Runtime>;
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
  type Currency = Balances;
  type MaxPriceDeviation = TolMaxPriceDeviation;
  type MaxTolRequestsPerBlock = TolMaxRequestsPerBlock;
  type MinSwapForeign = TolMinSwapForeign;
  type PalletId = TolPalletId;
  type Precision = TolPrecision;
  type TreasuryAccount = TolTreasuryAccount;
  type WeightInfo = crate::weights::pallet_treasury_owned_liquidity::SubstrateWeight<Runtime>;
  type ZapManagerAccount = ZapManagerAccount;
}
