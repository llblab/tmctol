//! Asset-related pallet configurations for the parachain runtime.
//!
//! Configures:
//! - `pallet-assets`: Fungible asset management
//! - `pallet-asset-conversion`: Uniswap V2-like DEX functionality

use polkadot_sdk::{
  frame_support::{parameter_types, traits::*},
  pallet_asset_conversion, pallet_assets,
  sp_runtime::traits::AccountIdConversion,
};

use crate::{AccountId, Balance, Balances, Runtime, RuntimeEvent, EXISTENTIAL_DEPOSIT};
pub use primitives::AssetKind;

/// Asset ID type used throughout the runtime
pub type AssetId = u32;

/// Ensure that the asset operations can only be performed by root or the asset owner
pub type AssetsForceOrigin = polkadot_sdk::frame_system::EnsureRoot<AccountId>;

/// Converter to distinguish between native and asset tokens
pub struct NativeOrAssetIdConverter;

impl
  polkadot_sdk::sp_runtime::traits::Convert<
    AssetKind,
    polkadot_sdk::sp_runtime::Either<(), AssetId>,
  > for NativeOrAssetIdConverter
{
  fn convert(asset_kind: AssetKind) -> polkadot_sdk::sp_runtime::Either<(), AssetId> {
    match asset_kind {
      AssetKind::Native => polkadot_sdk::sp_runtime::Either::Left(()),
      AssetKind::Local(asset_id) | AssetKind::Foreign(asset_id) => {
        polkadot_sdk::sp_runtime::Either::Right(asset_id)
      }
    }
  }
}

polkadot_sdk::frame_support::parameter_types! {
  /// Native asset ID
  pub const NativeAssetId: AssetKind = AssetKind::Native;
}

parameter_types! {
  // -- Assets Pallet Constants --
  /// Minimum balance required to approve an asset transfer
  pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
  /// Minimum balance required to keep an asset account alive
  pub const AssetAccountDeposit: Balance = EXISTENTIAL_DEPOSIT;
  /// Minimum balance required to create an asset
  pub const AssetDeposit: Balance = EXISTENTIAL_DEPOSIT;
  /// Minimum balance required to create metadata for an asset
  pub const MetadataDepositBase: Balance = EXISTENTIAL_DEPOSIT;
  /// Additional deposit required per byte of metadata
  pub const MetadataDepositPerByte: Balance = EXISTENTIAL_DEPOSIT;
  /// Maximum length of asset name
  pub const StringLimit: u32 = 50;

  // -- Asset Conversion Constants --
  pub const AssetConversionPalletId: polkadot_sdk::frame_support::PalletId = polkadot_sdk::frame_support::PalletId(*b"py/ascon");
  /// Liquidity withdrawal fee (0%)
  pub const LiquidityWithdrawalFee: polkadot_sdk::sp_runtime::Permill = polkadot_sdk::sp_runtime::Permill::from_percent(0);
  /// Minimum liquidity that must be minted when creating a pool
  pub const MintMinLiquidity: Balance = 100;
  /// Pool setup fee to prevent spam pool creation (temporarily disabled for testing)
  pub const PoolSetupFee: Balance = 0;
}

impl pallet_assets::Config for Runtime {
  type ApprovalDeposit = ApprovalDeposit;
  type AssetAccountDeposit = AssetAccountDeposit;
  type AssetDeposit = AssetDeposit;
  type AssetId = AssetId;
  type AssetIdParameter = AssetId;
  type Balance = Balance;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = ();
  type CallbackHandle = ();
  type CreateOrigin = polkadot_sdk::frame_system::EnsureSigned<AccountId>;
  type Currency = Balances;
  type Extra = ();
  type ReserveData = ();
  type ForceOrigin = AssetsForceOrigin;
  type Freezer = ();
  type Holder = ();
  type MetadataDepositBase = MetadataDepositBase;
  type MetadataDepositPerByte = MetadataDepositPerByte;
  type RemoveItemsLimit = ConstU32<1000>;
  type RuntimeEvent = RuntimeEvent;
  type StringLimit = StringLimit;
  type WeightInfo = ();
}

parameter_types! {
  pub const AssetRegistryPalletId: polkadot_sdk::frame_support::PalletId = polkadot_sdk::frame_support::PalletId(*primitives::ecosystem::pallet_ids::ASSET_REGISTRY_PALLET_ID);
}

pub struct AssetRegistryAccount;
impl polkadot_sdk::frame_support::traits::Get<AccountId> for AssetRegistryAccount {
  fn get() -> AccountId {
    AssetRegistryPalletId::get().into_account_truncating()
  }
}

impl pallet_asset_registry::Config for Runtime {
  type RegistryOrigin = AssetsForceOrigin;
  type AssetIdGenerator = crate::configs::xcm_config::LocationToAssetId;
  type AssetOwner = AssetRegistryAccount;
}

impl pallet_asset_conversion::Config for Runtime {
  type AssetKind = AssetKind;
  type Assets = polkadot_sdk::frame_support::traits::fungible::UnionOf<
    Balances,
    pallet_assets::Pallet<Runtime>,
    NativeOrAssetIdConverter,
    AssetKind,
    AccountId,
  >;
  type Balance = Balance;
  #[cfg(feature = "runtime-benchmarks")]
  type BenchmarkHelper = ();
  type HigherPrecisionBalance = polkadot_sdk::sp_core::U256;
  type LiquidityWithdrawalFee = LiquidityWithdrawalFee;
  type LPFee = ConstU32<3>;
  type MaxSwapPathLength = ConstU32<4>;
  type MintMinLiquidity = MintMinLiquidity;
  type PalletId = AssetConversionPalletId;
  type PoolAssetId = u32;
  type PoolAssets = pallet_assets::Pallet<Runtime>;
  type PoolId = (AssetKind, AssetKind);
  type PoolLocator = pallet_asset_conversion::WithFirstAsset<
    NativeAssetId,
    AccountId,
    AssetKind,
    pallet_asset_conversion::AccountIdConverter<AssetConversionPalletId, (AssetKind, AssetKind)>,
  >;
  type PoolSetupFee = PoolSetupFee;
  type PoolSetupFeeAsset = NativeAssetId;
  type PoolSetupFeeTarget = ();
  type RuntimeEvent = RuntimeEvent;
  type WeightInfo = ();
}
