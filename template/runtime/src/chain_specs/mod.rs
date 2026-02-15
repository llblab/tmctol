//! Elegant Chain Specification Hierarchy
//!
//! Provides a unified, type-safe approach to chain configuration with
//! clear separation between development, testnet, and production environments.

#![allow(dead_code)]

use crate::{
  AccountId, AuraId, Balance, BlockNumber, EXISTENTIAL_DEPOSIT, PARACHAIN_ID, ParachainInfoConfig,
  RuntimeGenesisConfig, UNIT,
};
use alloc::{vec, vec::Vec};
use cumulus_primitives_core::ParaId;
use frame_support::{build_struct_json_patch, traits::Get};
use polkadot_sdk::{staging_xcm as xcm, *};
use serde_json::Value;
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;

/// Unified chain specification builder
pub struct ChainSpecBuilder {
  /// Chain identifier
  pub chain_id: ParaId,
  /// Initial validators/collators
  pub validators: Vec<(AccountId, AuraId)>,
  /// Endowed accounts with initial balances
  pub endowed_accounts: Vec<AccountId>,
  /// Root account for sudo/emergency access
  pub root_account: AccountId,
  /// Economic parameters
  pub economic_params: EconomicParams,
  /// Network parameters
  pub network_params: NetworkParams,
}

/// Economic parameters for chain configuration
#[derive(Debug, Clone)]
pub struct EconomicParams {
  /// Collator candidacy bond
  pub candidacy_bond: Balance,
  /// Initial endowment for accounts
  pub initial_endowment: Balance,
  /// XCM version for cross-chain compatibility
  pub safe_xcm_version: u32,
}

/// Network parameters for chain configuration
#[derive(Debug, Clone)]
pub struct NetworkParams {
  /// Session length in blocks
  pub session_length: BlockNumber,
}

impl Default for EconomicParams {
  fn default() -> Self {
    Self {
      candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
      initial_endowment: 1u128 << 60, // 1 << 60 units
      safe_xcm_version: xcm::prelude::XCM_VERSION,
    }
  }
}

impl Default for NetworkParams {
  fn default() -> Self {
    Self {
      session_length: 6 * 60 * 24, // 1 day in 6s blocks
    }
  }
}

impl ChainSpecBuilder {
  /// Create a new chain specification builder
  pub fn new(chain_id: ParaId) -> Self {
    Self {
      chain_id,
      validators: Vec::new(),
      endowed_accounts: Vec::new(),
      root_account: Sr25519Keyring::Alice.to_account_id(),
      economic_params: EconomicParams::default(),
      network_params: NetworkParams::default(),
    }
  }

  /// Set the root account
  pub fn with_root(mut self, account: AccountId) -> Self {
    self.root_account = account;
    self
  }

  /// Configure economic parameters
  pub fn with_economic_params(mut self, params: EconomicParams) -> Self {
    self.economic_params = params;
    self
  }

  /// Configure network parameters
  pub fn with_network_params(mut self, params: NetworkParams) -> Self {
    self.network_params = params;
    self
  }

  /// Build the genesis configuration as JSON patch
  pub fn build_genesis_patch(&self) -> Value {
    let mut endowed_accounts = self.endowed_accounts.clone();
    let aaa_fee_sink = <crate::Runtime as pallet_aaa::Config>::FeeSink::get();
    if !endowed_accounts.contains(&aaa_fee_sink) {
      endowed_accounts.push(aaa_fee_sink);
    }

    build_struct_json_patch!(RuntimeGenesisConfig {
      balances: pallet_balances::GenesisConfig {
        balances: endowed_accounts
          .iter()
          .cloned()
          .map(|account| (account, self.economic_params.initial_endowment))
          .collect::<Vec<_>>(),
      },
      parachain_info: ParachainInfoConfig {
        parachain_id: self.chain_id
      },
      collator_selection: pallet_collator_selection::GenesisConfig {
        invulnerables: self
          .validators
          .iter()
          .cloned()
          .map(|(account, _)| account)
          .collect::<Vec<_>>(),
        candidacy_bond: self.economic_params.candidacy_bond,
        ..Default::default()
      },
      session: pallet_session::GenesisConfig {
        keys: self
          .validators
          .iter()
          .cloned()
          .map(|(account, aura_key)| {
            (
              account.clone(),
              account,
              crate::template_session_keys(aura_key),
            )
          })
          .collect::<Vec<_>>(),
      },
      polkadot_xcm: pallet_xcm::GenesisConfig {
        safe_xcm_version: Some(self.economic_params.safe_xcm_version)
      },
      sudo: pallet_sudo::GenesisConfig {
        key: Some(self.root_account.clone())
      },
    })
  }

  /// Build and serialize the genesis configuration
  pub fn build(&self) -> Vec<u8> {
    let patch = self.build_genesis_patch();
    serde_json::to_string(&patch)
      .expect("JSON serialization should never fail")
      .into_bytes()
  }
}

/// Development configuration with well-known accounts
pub fn development_config() -> ChainSpecBuilder {
  let validators = vec![
    (
      Sr25519Keyring::Alice.to_account_id(),
      Sr25519Keyring::Alice.public().into(),
    ),
    (
      Sr25519Keyring::Bob.to_account_id(),
      Sr25519Keyring::Bob.public().into(),
    ),
  ];

  let endowed_accounts = Sr25519Keyring::well_known()
    .map(|k| k.to_account_id())
    .collect();

  ChainSpecBuilder::new(PARACHAIN_ID.into())
    .with_root(Sr25519Keyring::Alice.to_account_id())
    .with_validators(validators)
    .with_endowed_accounts(endowed_accounts)
}

/// Testnet configuration with enhanced security
pub fn testnet_config() -> ChainSpecBuilder {
  let validators = vec![
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
  ];

  let endowed_accounts = vec![
    Sr25519Keyring::Alice.to_account_id(),
    Sr25519Keyring::Bob.to_account_id(),
    Sr25519Keyring::Charlie.to_account_id(),
    Sr25519Keyring::Dave.to_account_id(),
    Sr25519Keyring::Eve.to_account_id(),
    Sr25519Keyring::Ferdie.to_account_id(),
  ];

  let economic_params = EconomicParams {
    candidacy_bond: EXISTENTIAL_DEPOSIT * 32, // Higher bond for testnet
    initial_endowment: 1u128 << 50,           // Smaller endowments
    ..EconomicParams::default()
  };

  ChainSpecBuilder::new(PARACHAIN_ID.into())
    .with_root(Sr25519Keyring::Alice.to_account_id())
    .with_validators(validators)
    .with_endowed_accounts(endowed_accounts)
    .with_economic_params(economic_params)
}

/// Production configuration with minimal privileges
pub fn production_config(
  validators: Vec<(AccountId, AuraId)>,
  root_account: AccountId,
) -> ChainSpecBuilder {
  let economic_params = EconomicParams {
    candidacy_bond: 10_000 * UNIT,  // Significant bond for production
    initial_endowment: 1000 * UNIT, // Modest initial endowments
    ..EconomicParams::default()
  };

  let network_params = NetworkParams {
    session_length: 6 * 60 * 24 * 7, // 1 week sessions
  };

  ChainSpecBuilder::new(PARACHAIN_ID.into())
    .with_root(root_account)
    .with_validators(validators)
    .with_economic_params(economic_params)
    .with_network_params(network_params)
}

// Extension methods for builder pattern elegance
pub trait ChainSpecBuilderExt {
  /// Add multiple validators at once
  fn with_validators(self, validators: Vec<(AccountId, AuraId)>) -> Self;

  /// Add multiple endowed accounts at once
  fn with_endowed_accounts(self, accounts: Vec<AccountId>) -> Self;
}

impl ChainSpecBuilderExt for ChainSpecBuilder {
  fn with_validators(mut self, validators: Vec<(AccountId, AuraId)>) -> Self {
    self.validators = validators;
    self
  }

  fn with_endowed_accounts(mut self, accounts: Vec<AccountId>) -> Self {
    self.endowed_accounts = accounts;
    self
  }
}

/// Provides the JSON representation of predefined genesis config for given preset ID
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
  let builder = match id.as_ref() {
    sp_genesis_builder::DEV_RUNTIME_PRESET => development_config(),
    sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => testnet_config(),
    _ => return None,
  };

  Some(builder.build())
}

/// List of supported preset names
pub fn preset_names() -> Vec<PresetId> {
  vec![
    PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
    PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_development_config_builds() {
    let config = development_config();
    let genesis_bytes = config.build();

    assert!(!genesis_bytes.is_empty());
    let genesis_str = String::from_utf8(genesis_bytes).unwrap();
    assert!(genesis_str.contains("balances"));
    assert!(genesis_str.contains("sudo"));
  }

  #[test]
  fn test_production_config_customization() {
    let validators = vec![(
      Sr25519Keyring::Alice.to_account_id(),
      Sr25519Keyring::Alice.public().into(),
    )];

    let config = production_config(validators, Sr25519Keyring::Alice.to_account_id());
    assert_eq!(config.economic_params.candidacy_bond, 10_000 * UNIT);
  }
}
