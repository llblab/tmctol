//! The Substrate Node Template runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

extern crate alloc;

use alloc::vec::Vec;
use frame_support::weights::{
  Weight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};
use polkadot_sdk::{sp_runtime::traits::BlakeTwo256, staging_parachain_info as parachain_info, *};
use smallvec::smallvec;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{Perbill, generic, impl_opaque_keys};

mod apis;
mod benchmarks;
mod chain_specs;
mod configs;
mod genesis_config_presets;
#[cfg(any(test, feature = "std"))]
mod governance;
#[cfg(any(test, feature = "std"))]
mod monitoring;

#[cfg(any(test, feature = "std"))]
mod tests;

mod weights;
use weights::ExtrinsicBaseWeight;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Runtime version
#[sp_version::runtime_version]
pub const VERSION: sp_version::RuntimeVersion = sp_version::RuntimeVersion {
  spec_name: alloc::borrow::Cow::Borrowed("tmctol-runtime"),
  impl_name: alloc::borrow::Cow::Borrowed("tmctol-runtime"),
  apis: apis::RUNTIME_API_VERSIONS,
  authoring_version: 1,
  impl_version: 1,
  system_version: 1,
  spec_version: 100,
  transaction_version: 100,
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
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedBlock = generic::SignedBlock<Block>;
pub type BlockId = generic::BlockId<Block>;
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
pub type Signature = sp_runtime::MultiSignature;

/// The extension to the basic transaction logic.
pub type TxExtension = cumulus_pallet_weight_reclaim::StorageWeightReclaim<
  Runtime,
  (
    frame_system::AuthorizeCall<Runtime>,
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
  ),
>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
  generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// Opaque types for CLI machinery that doesn't need runtime specifics.
pub mod opaque {
  use super::*;
  pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
  use sp_runtime::{
    generic,
    traits::{BlakeTwo256, Hash as HashT},
  };

  pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
  pub type Block = generic::Block<Header, UncheckedExtrinsic>;
  pub type BlockId = generic::BlockId<Block>;
  pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
  pub struct SessionKeys {
    pub aura: Aura,
  }
}

// Constants (12 decimals, matching DOT/KSM standard)
pub const UNIT: Balance = 1_000_000_000_000;
pub const MILLI_UNIT: Balance = UNIT / 1_000;
pub const MICRO_UNIT: Balance = UNIT / 1_000_000;
pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_UNIT;
pub const SLOT_DURATION: u64 = 6_000;

// Async backing parameters
mod async_backing_params {
  pub(crate) const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
  pub(crate) const BLOCK_PROCESSING_VELOCITY: u32 = 1;
  pub(crate) const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;
}
pub(crate) use async_backing_params::*;

pub const HOURS: BlockNumber = 600;
pub const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(2_000_000_000_000, 5_000_000);
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

/// Handles converting a weight scalar to a fee value.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
  type Balance = Balance;
  fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
    let p = MILLI_UNIT / 10;
    let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
    smallvec![WeightToFeeCoefficient {
      degree: 1,
      negative: false,
      coeff_frac: Perbill::from_rational(p % q, q),
      coeff_integer: p / q,
    }]
  }
}

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
  pub type WeightReclaim = cumulus_pallet_weight_reclaim;

  // Monetary stuff.
  #[runtime::pallet_index(10)]
  pub type Balances = pallet_balances;
  #[runtime::pallet_index(11)]
  pub type TransactionPayment = pallet_transaction_payment;

  // Governance.
  #[runtime::pallet_index(15)]
  pub type Sudo = pallet_sudo;

  // Collator support. The order of these 4 are important and shall not change.
  #[runtime::pallet_index(20)]
  pub type Authorship = pallet_authorship;
  #[runtime::pallet_index(21)]
  pub type CollatorSelection = pallet_collator_selection;
  #[runtime::pallet_index(22)]
  pub type Session = pallet_session;
  #[runtime::pallet_index(23)]
  pub type Aura = pallet_aura;
  #[runtime::pallet_index(24)]
  pub type AuraExt = cumulus_pallet_aura_ext;

  // XCM helpers.
  #[runtime::pallet_index(30)]
  pub type XcmpQueue = cumulus_pallet_xcmp_queue;
  #[runtime::pallet_index(31)]
  pub type PolkadotXcm = pallet_xcm;
  #[runtime::pallet_index(32)]
  pub type CumulusXcm = cumulus_pallet_xcm;
  #[runtime::pallet_index(33)]
  pub type MessageQueue = pallet_message_queue;

  // Assets.
  #[runtime::pallet_index(40)]
  pub type Assets = pallet_assets;
  #[runtime::pallet_index(41)]
  pub type AssetConversion = pallet_asset_conversion;
  #[runtime::pallet_index(42)]
  pub type AssetRegistry = pallet_asset_registry;

  // TMCTOL pallets.
  #[runtime::pallet_index(50)]
  pub type AxialRouter = pallet_axial_router;
  #[runtime::pallet_index(51)]
  pub type TokenMintingCurve = pallet_token_minting_curve;
  #[runtime::pallet_index(52)]
  pub type TreasuryOwnedLiquidity = pallet_treasury_owned_liquidity;
  #[runtime::pallet_index(53)]
  pub type ZapManager = pallet_zap_manager;
  #[runtime::pallet_index(54)]
  pub type BurningManager = pallet_burning_manager;
  #[runtime::pallet_index(55)]
  pub type AAA = pallet_aaa;
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

pub use genesis_config_presets::{PARACHAIN_ID, template_session_keys};

pub type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
  Runtime,
  RELAY_CHAIN_SLOT_DURATION_MILLIS,
  BLOCK_PROCESSING_VELOCITY,
  UNINCLUDED_SEGMENT_CAPACITY,
>;

cumulus_pallet_parachain_system::register_validate_block! {
  Runtime = Runtime,
  BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
