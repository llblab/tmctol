//! Common Test Utilities for Runtime Integration Tests
//!
//! This module provides shared utilities and setup functions for all runtime integration tests,
//! ensuring consistent test environment initialization and reducing code duplication.

use crate::{
  AccountId, AssetConversion, Assets, Balance, Balances, EXISTENTIAL_DEPOSIT, Runtime,
  RuntimeOrigin, System, configs::AssetKind,
};
use alloc::vec;
use polkadot_sdk::frame_support::{
  assert_ok,
  dispatch::DispatchResult,
  traits::{Currency, Get},
};
use polkadot_sdk::sp_std::boxed::Box;
use polkadot_sdk::{
  pallet_asset_conversion::{self, PoolLocator},
  polkadot_runtime_common::BuildStorage,
  sp_io::TestExternalities,
  sp_runtime::{DispatchError, ModuleError},
};
use primitives::assets::{TYPE_FOREIGN, TYPE_STD};

// Standard test accounts
pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId::new([3u8; 32]);
pub const DAVE: AccountId = AccountId::new([4u8; 32]);
pub const EVE: AccountId = AccountId::new([5u8; 32]);

// Axial Router account from pallet configuration
pub fn axial_router_account() -> AccountId {
  crate::AxialRouter::account_id()
}

// TOL treasury account from pallet configuration
pub fn tol_treasury_account() -> AccountId {
  crate::configs::TolTreasuryAccount::get()
}

// TOL ingress account for explicit TolId domain
pub fn tol_ingress_account_for_tol_id(tol_id: u32) -> AccountId {
  crate::TreasuryOwnedLiquidity::ingress_account_for_tol_id(tol_id)
}

// Fill active TOL domains to configured capacity for negative-path tests
pub fn saturate_active_tol_domains(start_id: u32) {
  let max_domains =
    <crate::Runtime as pallet_treasury_owned_liquidity::Config>::MaxTolDomains::get();
  pallet_treasury_owned_liquidity::ActiveTolDomains::<crate::Runtime>::mutate(|domains| {
    domains.clear();
    for i in 0..max_domains {
      assert!(domains.try_push(start_id.saturating_add(i)).is_ok());
    }
  });
}

// Standard test constants
pub const INITIAL_BALANCE: Balance = 10000000 * EXISTENTIAL_DEPOSIT;

// Test asset IDs using Bitmask Architecture
pub const ASSET_NATIVE: AssetKind = AssetKind::Native;

// Standard Tokens (0x1...)
pub const ASSET_A: u32 = TYPE_STD | 1;
pub const ASSET_B: u32 = TYPE_STD | 2;
pub const ASSET_D: u32 = TYPE_STD | 3;
pub const ASSET_E: u32 = TYPE_STD | 4;

// Foreign Assets (0xF...)
pub const ASSET_FOREIGN: u32 = TYPE_FOREIGN | 1;

// Token-driven actor accounts from pallet configurations
pub fn burning_manager_account() -> AccountId {
  crate::BurningManager::account_id()
}

pub fn zap_manager_account() -> AccountId {
  crate::ZapManager::account_id()
}

pub fn aaa_fee_sink_account() -> AccountId {
  <Runtime as pallet_aaa::Config>::FeeSink::get()
}

pub fn tmc_pallet_account() -> AccountId {
  crate::TokenMintingCurve::account_id()
}

// Swap test constants
pub const SWAP_AMOUNT: Balance = 20000 * EXISTENTIAL_DEPOSIT;
pub const MIN_AMOUNT_OUT: Balance = 1;
// TMCTOL test constants
pub const MINT_AMOUNT: Balance = 10 * EXISTENTIAL_DEPOSIT;
pub const TOL_TOTAL_ALLOCATION: Balance = 1_000_000_000_000_000_000;

// Pool constants
pub const LIQUIDITY_AMOUNT: Balance = INITIAL_BALANCE / 2;
pub const MIN_LIQUIDITY: Balance = 0;

/// Initialize test externalities with a clean state
pub fn new_test_ext() -> TestExternalities {
  let mut t = polkadot_sdk::frame_system::GenesisConfig::<Runtime>::default()
    .build_storage()
    .unwrap();

  // Initialize balances for test accounts (sufficient for asset deposits)
  let initial_balances = vec![
    (ALICE, INITIAL_BALANCE),
    (BOB, INITIAL_BALANCE),
    (CHARLIE, INITIAL_BALANCE),
    (DAVE, INITIAL_BALANCE),
    (EVE, INITIAL_BALANCE),
  ];

  polkadot_sdk::pallet_balances::GenesisConfig::<Runtime> {
    balances: initial_balances,
    ..Default::default()
  }
  .assimilate_storage(&mut t)
  .unwrap();

  // Pallet genesis configs: ED-free accounts + tracked assets
  pallet_axial_router::GenesisConfig::<Runtime>::default()
    .assimilate_storage(&mut t)
    .unwrap();
  pallet_burning_manager::GenesisConfig::<Runtime>::default()
    .assimilate_storage(&mut t)
    .unwrap();
  pallet_zap_manager::GenesisConfig::<Runtime>::default()
    .assimilate_storage(&mut t)
    .unwrap();
  pallet_treasury_owned_liquidity::GenesisConfig::<Runtime>::default()
    .assimilate_storage(&mut t)
    .unwrap();
  pallet_token_minting_curve::GenesisConfig::<Runtime>::default()
    .assimilate_storage(&mut t)
    .unwrap();

  let mut ext = TestExternalities::new(t);
  ext.execute_with(|| System::set_block_number(1));
  ext
}

/// Primary helper for tests that need seeded assets/accounts.
pub fn seeded_test_ext() -> TestExternalities {
  setup_basic_test_environment()
}

/// Mint test assets to multiple accounts
pub fn create_test_asset(asset_id: u32, owner: &AccountId) -> DispatchResult {
  Assets::force_create(
    RuntimeOrigin::root(),
    asset_id,
    owner.clone().into(),
    true,
    1,
  )
}

/// Mint helper for tests
pub fn mint_tokens(
  asset_id: u32,
  minter: &AccountId,
  beneficiary: &AccountId,
  amount: Balance,
) -> DispatchResult {
  Assets::mint(
    RuntimeOrigin::signed(minter.clone()),
    asset_id,
    beneficiary.clone().into(),
    amount,
  )
}

/// Setup a basic test environment with common assets and accounts
pub fn setup_basic_test_environment() -> TestExternalities {
  let mut ext = new_test_ext();
  ext.execute_with(|| {
    System::set_block_number(1);

    // Create test assets using standard asset IDs for consistency
    let basic_assets = vec![ASSET_A, ASSET_B, ASSET_D, ASSET_E, ASSET_FOREIGN];
    for &asset_id in &basic_assets {
      create_test_asset(asset_id, &ALICE).unwrap();
      // Set ALICE as admin for minting to other accounts
      let _ = Assets::set_team(
        RuntimeOrigin::signed(ALICE),
        asset_id,
        ALICE.into(), // issuer
        ALICE.into(), // admin
        ALICE.into(), // freezer
      );
    }

    // Add native token deposits for system accounts to enable asset operations
    let system_accounts = vec![
      tol_treasury_account(),
      axial_router_account(),
      burning_manager_account(),
      zap_manager_account(),
      aaa_fee_sink_account(),
      tmc_pallet_account(),
    ];
    for account in &system_accounts {
      let _ = <Balances as Currency<AccountId>>::deposit_creating(account, INITIAL_BALANCE);
    }

    // Mint assets to test accounts
    let test_accounts = vec![
      ALICE,
      BOB,
      CHARLIE,
      DAVE,
      EVE,
      tol_treasury_account(),
      axial_router_account(),
      burning_manager_account(),
      zap_manager_account(),
      tmc_pallet_account(),
    ];
    for &asset_id in &basic_assets {
      for account in &test_accounts {
        let amount = if asset_id == ASSET_FOREIGN && *account == ALICE {
          INITIAL_BALANCE.saturating_mul(1000)
        } else {
          INITIAL_BALANCE
        };
        let _ = mint_tokens(asset_id, &ALICE, account, amount);
      }
    }
  });
  ext
}

/// Assert that an operation returns Ok and return the result
#[macro_export]
macro_rules! assert_ok_result {
  ($result:expr) => {
    match $result {
      Ok(result) => result,
      Err(e) => panic!("Expected Ok, got Err: {:?}", e),
    }
  };
}

/// Assert that an operation returns Err with a specific error
#[macro_export]
macro_rules! assert_err {
  ($result:expr, $expected_error:pat) => {
    match $result {
      Err(e) => {
        if let $expected_error = e.error {
          // Expected error pattern matched
        } else {
          panic!(
            "Expected error pattern {:?}, got {:?}",
            stringify!($expected_error),
            e.error
          );
        }
      }
      Ok(_) => panic!("Expected Err, got Ok"),
    }
  };
}

/// Helper to create a new liquidity pool for a given pair of assets
pub fn create_pool(
  origin: RuntimeOrigin,
  asset1: crate::configs::AssetKind,
  asset2: crate::configs::AssetKind,
) -> DispatchResult {
  crate::AssetConversion::create_pool(origin, Box::new(asset1), Box::new(asset2))
}

/// Helper to add liquidity to an existing pool
fn canonical_asset_pair(
  asset1: &crate::configs::AssetKind,
  asset2: &crate::configs::AssetKind,
) -> (crate::configs::AssetKind, crate::configs::AssetKind) {
  if let Ok(pair) =
    <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(asset1, asset2)
  {
    pair
  } else if let Ok(pair) =
    <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(asset2, asset1)
  {
    pair
  } else if asset1 <= asset2 {
    (*asset1, *asset2)
  } else {
    (*asset2, *asset1)
  }
}

#[allow(clippy::too_many_arguments)]
pub fn add_liquidity(
  origin: RuntimeOrigin,
  asset1: crate::configs::AssetKind,
  asset2: crate::configs::AssetKind,
  amount1_desired: Balance,
  amount2_desired: Balance,
  amount1_min: Balance,
  amount2_min: Balance,
  mint_to: &AccountId,
) -> DispatchResult {
  let (canonical_asset1, canonical_asset2) = canonical_asset_pair(&asset1, &asset2);
  let (desired_first, desired_second, min_first, min_second) = if canonical_asset1 == asset1 {
    (amount1_desired, amount2_desired, amount1_min, amount2_min)
  } else {
    (amount2_desired, amount1_desired, amount2_min, amount1_min)
  };

  crate::AssetConversion::add_liquidity(
    origin,
    Box::new(canonical_asset1),
    Box::new(canonical_asset2),
    desired_first,
    desired_second,
    min_first,
    min_second,
    mint_to.clone(),
  )
}

/// Ensure an AssetConversion pool exists, ignoring `PoolExists` and Assets `InUse` errors.
pub fn ensure_asset_conversion_pool(asset1: AssetKind, asset2: AssetKind) {
  let (canonical_asset1, canonical_asset2) = canonical_asset_pair(&asset1, &asset2);
  let result = AssetConversion::create_pool(
    RuntimeOrigin::signed(ALICE),
    Box::new(canonical_asset1),
    Box::new(canonical_asset2),
  );
  if let Err(error) = result {
    // Handle Assets pallet "InUse" error (index 12) - asset already in use
    if let DispatchError::Module(ModuleError {
      index: 12,
      error: [3, 0, 0, 0],
      ..
    }) = &error
    {
      return;
    }
    if let DispatchError::Module(ModuleError {
      index: 12,
      message: Some("InUse"),
      ..
    }) = &error
    {
      return;
    }
    // Handle AssetConversion pallet "PoolExists" error (index 13)
    if let DispatchError::Module(ModuleError {
      index: 13,
      message: Some("PoolExists"),
      ..
    }) = &error
    {
      return;
    }
    panic!("Unexpected AssetConversion pool creation error: {error:?}");
  }
}

/// Sets up the asset conversion infrastructure used by Axial Router tests.
pub fn setup_axial_router_infrastructure() -> Result<(), &'static str> {
  use crate::configs::AssetKind;

  // Create single pool for native ↔ asset pair used by tests
  // Using single pool to avoid "InUse" errors from Assets pallet in test environment
  ensure_asset_conversion_pool(ASSET_NATIVE, AssetKind::Local(ASSET_A));
  assert_ok!(add_liquidity(
    RuntimeOrigin::signed(ALICE),
    ASSET_NATIVE,
    AssetKind::Local(ASSET_A),
    LIQUIDITY_AMOUNT,
    LIQUIDITY_AMOUNT,
    MIN_LIQUIDITY,
    MIN_LIQUIDITY,
    &ALICE,
  ));
  Ok(())
}

/// Setup test environment for TMCTOL tests
pub fn setup_tmctol_test_environment() -> Result<(), &'static str> {
  use crate::configs::AssetKind;

  // Create assets
  let _ = create_test_asset(ASSET_A, &ALICE);
  let _ = create_test_asset(ASSET_FOREIGN, &ALICE);

  // Mint assets to test accounts
  let _ = mint_tokens(ASSET_A, &ALICE, &ALICE, 100_000_000_000_000_000_000);
  let _ = mint_tokens(ASSET_FOREIGN, &ALICE, &ALICE, 100_000_000_000_000_000_000);
  let _ = mint_tokens(ASSET_A, &ALICE, &tol_treasury_account(), 1_000_000_000);
  let _ = mint_tokens(
    ASSET_FOREIGN,
    &ALICE,
    &tol_treasury_account(),
    1_000_000_000,
  );

  // Create TOL configuration
  assert_ok!(crate::TreasuryOwnedLiquidity::create_tol(
    RuntimeOrigin::root(),
    0,
    AssetKind::Local(ASSET_A),
    AssetKind::Local(ASSET_FOREIGN),
    TOL_TOTAL_ALLOCATION
  ));

  // Ensure AssetConversion pool exists and add initial liquidity for Zap operations
  ensure_asset_conversion_pool(ASSET_NATIVE, AssetKind::Local(ASSET_FOREIGN));
  add_liquidity(
    RuntimeOrigin::signed(ALICE),
    ASSET_NATIVE,
    AssetKind::Local(ASSET_FOREIGN),
    LIQUIDITY_AMOUNT,
    LIQUIDITY_AMOUNT,
    MIN_LIQUIDITY,
    MIN_LIQUIDITY,
    &ALICE,
  )
  .map_err(|_| "failed to seed liquidity for zap operations")?;

  Ok(())
}
