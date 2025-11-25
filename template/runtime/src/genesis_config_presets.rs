use crate::{AccountId, RuntimeGenesisConfig, SessionKeys, EXISTENTIAL_DEPOSIT};

use alloc::{vec, vec::Vec};

use polkadot_sdk::{staging_parachain_info as parachain_info, staging_xcm as xcm, *};

use pallet_balances::GenesisConfig as BalancesGenesisConfig;
use pallet_collator_selection::GenesisConfig as CollatorSelectionGenesisConfig;
use pallet_session::GenesisConfig as SessionGenesisConfig;
use pallet_sudo::GenesisConfig as SudoGenesisConfig;
use pallet_xcm::GenesisConfig as PolkadotXcmGenesisConfig;
use parachain_info::GenesisConfig as ParachainInfoGenesisConfig;

use cumulus_primitives_core::ParaId;
use frame_support::build_struct_json_patch;
use parachains_common::AuraId;
use serde_json::Value;
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
/// Parachain id used for genesis config presets of parachain template.
#[docify::export_content]
pub const PARACHAIN_ID: u32 = 1000;

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> SessionKeys {
  SessionKeys { aura: keys }
}

fn testnet_genesis(
  invulnerables: Vec<(AccountId, AuraId)>,
  endowed_accounts: Vec<AccountId>,
  root: AccountId,
  id: ParaId,
) -> Value {
  build_struct_json_patch!(RuntimeGenesisConfig {
    balances: BalancesGenesisConfig {
      balances: endowed_accounts
        .iter()
        .cloned()
        .map(|k| (k, 1u128 << 60))
        .collect::<Vec<_>>(),
    },
    parachain_info: ParachainInfoGenesisConfig { parachain_id: id },
    collator_selection: CollatorSelectionGenesisConfig {
      invulnerables: invulnerables
        .iter()
        .cloned()
        .map(|(acc, _)| acc)
        .collect::<Vec<_>>(),
      candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
    },
    session: SessionGenesisConfig {
      keys: invulnerables
        .into_iter()
        .map(|(acc, aura)| {
          (
            acc.clone(),                 // account id
            acc,                         // validator id
            template_session_keys(aura), // session keys
          )
        })
        .collect::<Vec<_>>(),
    },
    polkadot_xcm: PolkadotXcmGenesisConfig {
      safe_xcm_version: Some(SAFE_XCM_VERSION)
    },
    sudo: SudoGenesisConfig { key: Some(root) },
  })
}

fn local_testnet_genesis() -> Value {
  testnet_genesis(
    // initial collators.
    vec![
      (
        Sr25519Keyring::Alice.to_account_id(),
        Sr25519Keyring::Alice.public().into(),
      ),
      (
        Sr25519Keyring::Bob.to_account_id(),
        Sr25519Keyring::Bob.public().into(),
      ),
      (
        Sr25519Keyring::Charlie.to_account_id(),
        Sr25519Keyring::Charlie.public().into(),
      ),
    ],
    Sr25519Keyring::well_known()
      .map(|k| k.to_account_id())
      .collect(),
    Sr25519Keyring::Alice.to_account_id(),
    PARACHAIN_ID.into(),
  )
}

fn development_config_genesis() -> Value {
  testnet_genesis(
    // initial collators.
    vec![
      (
        Sr25519Keyring::Alice.to_account_id(),
        Sr25519Keyring::Alice.public().into(),
      ),
      (
        Sr25519Keyring::Bob.to_account_id(),
        Sr25519Keyring::Bob.public().into(),
      ),
      (
        Sr25519Keyring::Charlie.to_account_id(),
        Sr25519Keyring::Charlie.public().into(),
      ),
    ],
    Sr25519Keyring::well_known()
      .map(|k| k.to_account_id())
      .collect(),
    Sr25519Keyring::Alice.to_account_id(),
    PARACHAIN_ID.into(),
  )
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<vec::Vec<u8>> {
  let patch = match id.as_ref() {
    sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_testnet_genesis(),
    sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
    _ => return None,
  };
  Some(
    serde_json::to_string(&patch)
      .expect("serialization to json is expected to work. qed.")
      .into_bytes(),
  )
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
  vec![
    PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
    PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
  ]
}
