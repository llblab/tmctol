//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

extern crate alloc;

use alloc::{borrow::Cow, vec::Vec};
use frame_support::weights::{IdentityFee, Weight};
use polkadot_sdk::{sp_runtime::traits::BlakeTwo256, staging_parachain_info as parachain_info, *};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::Perbill;

mod apis;
mod benchmarks;
mod chain_specs;
mod genesis_config_presets;
#[cfg(any(test, feature = "std"))]
mod governance;
#[cfg(any(test, feature = "std"))]
mod monitoring;

#[cfg(any(test, feature = "std"))]
mod tests;
mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version (place holder)
#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
  spec_name: Cow::Borrowed("tmctol-runtime"),
  impl_name: Cow::Borrowed("tmctol-runtime"),
  authoring_version: 1,
  spec_version: 100,
  impl_version: 1,
  apis: apis::RUNTIME_API_VERSIONS,
  transaction_version: 100,
  system_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
  sp_version::NativeVersion {
    runtime_version: VERSION,
    can_author_with: Default::default(),
  }
}

// Type aliases
pub type AccountId = sp_runtime::AccountId32;
pub type Balance = u128;
pub type BlockNumber = u32;
pub type Hash = sp_core::H256;
pub type Nonce = u32;
pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<
  Address,
  <Runtime as frame_system::Config>::RuntimeCall,
  Signature,
  SignedExtra,
>;
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Signature = sp_runtime::MultiSignature;
pub type SignedExtra = (
  frame_system::CheckNonZeroSender<Runtime>,
  frame_system::CheckSpecVersion<Runtime>,
  frame_system::CheckTxVersion<Runtime>,
  frame_system::CheckGenesis<Runtime>,
  frame_system::CheckEra<Runtime>,
  frame_system::CheckNonce<Runtime>,
  frame_system::CheckWeight<Runtime>,
  pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
  cumulus_pallet_weight_reclaim::StorageWeightReclaim<Runtime, ()>,
);

sp_runtime::impl_opaque_keys! {
  pub struct SessionKeys {
    pub aura: AuraId,
  }
}
pub type WeightToFee = IdentityFee<Balance>;

// Constants
pub const MICRO_UNIT: Balance = 1_000_000_000_000; // 10^-6 UNIT
pub const UNIT: Balance = 1_000_000_000_000_000_000;
pub const EXISTENTIAL_DEPOSIT: Balance = 1_000_000_000_000_000; // 10^-3 UNIT
pub const SLOT_DURATION: u64 = 6_000;

// Async backing parameters
mod async_backing_params {
  /// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
  /// into the relay chain.
  pub(crate) const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
  /// How many parachain blocks are processed by the relay chain per parent. Limits the
  /// number of blocks authored per slot.
  pub(crate) const BLOCK_PROCESSING_VELOCITY: u32 = 1;
  /// Relay chain slot duration, in milliseconds.
  pub(crate) const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
}
pub(crate) use async_backing_params::*;

pub const HOURS: BlockNumber = 600;
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(2_000_000_000_000, 5_000_000);
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

#[frame_support::runtime]
mod runtime {
  #[runtime::runtime]
  #[runtime::derive(
    RuntimeCall,
    RuntimeEvent,
    RuntimeError,
    RuntimeOrigin,
    RuntimeFreezeReason,
    RuntimeHoldReason,
    RuntimeSlashReason,
    RuntimeLockId,
    RuntimeTask,
    RuntimeViewFunction
  )]
  pub struct Runtime;

  #[runtime::pallet_index(0)]
  pub type System = frame_system;
  #[runtime::pallet_index(1)]
  pub type ParachainSystem = cumulus_pallet_parachain_system;
  #[runtime::pallet_index(2)]
  pub type Timestamp = pallet_timestamp;
  #[runtime::pallet_index(3)]
  pub type ParachainInfo = parachain_info;
  #[runtime::pallet_index(4)]
  pub type Balances = pallet_balances;
  #[runtime::pallet_index(5)]
  pub type TransactionPayment = pallet_transaction_payment;
  #[runtime::pallet_index(6)]
  pub type Sudo = pallet_sudo;
  #[runtime::pallet_index(7)]
  pub type Aura = pallet_aura;
  #[runtime::pallet_index(8)]
  pub type AuraExt = cumulus_pallet_aura_ext;
  #[runtime::pallet_index(9)]
  pub type Session = pallet_session;
  #[runtime::pallet_index(10)]
  pub type CollatorSelection = pallet_collator_selection;
  #[runtime::pallet_index(11)]
  pub type Authorship = pallet_authorship;
  #[runtime::pallet_index(12)]
  pub type Assets = pallet_assets;
  #[runtime::pallet_index(13)]
  pub type AssetConversion = pallet_asset_conversion;
  #[runtime::pallet_index(14)]
  pub type AxialRouter = pallet_axial_router;
  #[runtime::pallet_index(15)]
  pub type TokenMintingCurve = pallet_token_minting_curve;
  #[runtime::pallet_index(16)]
  pub type TreasuryOwnedLiquidity = pallet_treasury_owned_liquidity;
  #[runtime::pallet_index(17)]
  pub type ZapManager = pallet_zap_manager;
  #[runtime::pallet_index(18)]
  pub type XcmpQueue = cumulus_pallet_xcmp_queue;
  #[runtime::pallet_index(19)]
  pub type BurningManager = pallet_burning_manager;
  #[runtime::pallet_index(20)]
  pub type MessageQueue = pallet_message_queue;
  #[runtime::pallet_index(21)]
  pub type PolkadotXcm = pallet_xcm;
  #[runtime::pallet_index(22)]
  pub type CumulusXcm = cumulus_pallet_xcm;
  #[runtime::pallet_index(23)]
  pub type WeightReclaim = cumulus_pallet_weight_reclaim;
  #[runtime::pallet_index(24)]
  pub type AssetRegistry = pallet_asset_registry;
}

pub type Migrations = (
  cumulus_pallet_xcmp_queue::migration::v5::MigrateV4ToV5<Runtime>,
  pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
);

pub type Executive = frame_executive::Executive<
  Runtime,
  Block,
  frame_system::ChainContext<Runtime>,
  Runtime,
  AllPalletsWithSystem,
>;

mod configs;

pub use genesis_config_presets::{template_session_keys, PARACHAIN_ID};

pub type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
  Runtime,
  RELAY_CHAIN_SLOT_DURATION_MILLIS,
  BLOCK_PROCESSING_VELOCITY,
  UNINCLUDED_SEGMENT_CAPACITY,
>;

// Register validate block for parachain
cumulus_pallet_parachain_system::register_validate_block! {
  Runtime = Runtime,
  BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
