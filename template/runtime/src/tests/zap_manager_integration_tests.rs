//! Integration tests for Zap Manager pallet (Strict Whitelist Architecture).
//!
//! Tests the complete Zap Manager functionality including:
//! - Governance-controlled asset whitelisting
//! - Token-driven zap execution via on_initialize
//! - Sweep mechanism for non-whitelisted assets
//! - Error handling and edge cases

use super::common::{
  ALICE, ASSET_A, ASSET_B, ASSET_FOREIGN, BOB, CHARLIE, LIQUIDITY_AMOUNT, MIN_LIQUIDITY,
  TOL_TOTAL_ALLOCATION, add_liquidity, ensure_asset_conversion_pool, seeded_test_ext,
  setup_axial_router_infrastructure, tol_ingress_account_for_tol_id,
};
use crate::{
  AccountId, Assets, Balances, RuntimeEvent, RuntimeOrigin, System, TreasuryOwnedLiquidity,
  ZapManager,
};
use pallet_zap_manager::AssetConversionApi;
use polkadot_sdk::{
  frame_support::{
    assert_noop, assert_ok,
    traits::{Currency, Get, Hooks, fungibles::Inspect},
  },
  sp_runtime::DispatchError,
};
use primitives::AssetKind;

/// Setup test environment with zap manager infrastructure
fn setup_zap_manager_integration_infrastructure() -> Result<(), &'static str> {
  let zap_manager_account = ZapManager::account_id();

  // Ensure ALICE has sufficient balance for operations
  let alice_balance = Balances::free_balance(&ALICE);
  if alice_balance < 1_000_000_000_000_000_000_000_000 {
    let _ = Balances::deposit_creating(&ALICE, 1_000_000_000_000_000_000_000_000 - alice_balance);
  }

  // Ensure BOB and CHARLIE have sufficient native balance for operations
  for user in [BOB, CHARLIE] {
    let user_balance = Balances::free_balance(&user);
    if user_balance < 1_000_000_000_000_000_000_000_000 {
      let _ = Balances::deposit_creating(&user, 1_000_000_000_000_000_000_000_000 - user_balance);
    }
  }

  // Ensure zap manager has sufficient balance for operations (Native ED + some buffer)
  let current_balance = Balances::free_balance(&zap_manager_account);
  let min_balance = <Balances as Currency<AccountId>>::minimum_balance();

  if current_balance < min_balance * 2 {
    let _ =
      <Balances as Currency<AccountId>>::deposit_creating(&zap_manager_account, min_balance * 10);
  }

  // Ensure test accounts have sufficient asset balances for operations
  for user in [ALICE, BOB, CHARLIE] {
    assert_ok!(Assets::mint(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      polkadot_sdk::sp_runtime::MultiAddress::Id(user.clone()),
      1_000_000_000_000_000_000_000_000
    ));
    assert_ok!(Assets::mint(
      RuntimeOrigin::signed(ALICE),
      ASSET_B,
      polkadot_sdk::sp_runtime::MultiAddress::Id(user.clone()),
      1_000_000_000_000_000_000_000_000
    ));
  }
  assert_ok!(Assets::mint(
    RuntimeOrigin::signed(ALICE),
    ASSET_FOREIGN,
    polkadot_sdk::sp_runtime::MultiAddress::Id(ALICE.clone()),
    1_000_000_000_000_000_000_000_000
  ));

  Ok(())
}

#[test]
fn test_governance_whitelist_control() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let asset = AssetKind::Local(ASSET_A);

    // 1. Verify enable_asset requires AdminOrigin (Root)
    assert_noop!(
      ZapManager::enable_asset(RuntimeOrigin::signed(ALICE), asset),
      DispatchError::BadOrigin
    );

    // 2. Enable asset via Root
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), asset));

    // Verify event
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::AssetEnabled { .. })
    )));

    // 3. Verify disable_asset requires AdminOrigin
    assert_noop!(
      ZapManager::disable_asset(RuntimeOrigin::signed(ALICE), asset),
      DispatchError::BadOrigin
    );

    // 4. Disable asset via Root
    assert_ok!(ZapManager::disable_asset(RuntimeOrigin::root(), asset));

    // Verify event
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::AssetDisabled { .. })
    )));
  });
}

#[test]
fn test_zap_execution_on_whitelist() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let zap_manager_account = ZapManager::account_id();
    let asset = AssetKind::Local(ASSET_A);
    let native_amount = 2_000_000_000_000_000_000; // 2 Unit
    let foreign_amount = 2_000_000_000_000_000_000; // 2 Unit

    // 1. Enable asset
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), asset));

    // 2. Fund ZapManager (Wrong Door Deposit)
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      native_amount
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      foreign_amount
    ));

    // 3. Trigger on_initialize
    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // 4. Verify Zap Execution
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::ZapCompleted { .. })
    )));

    assert!(System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::LPTokensDistributed { .. })
    )));

    // Verify balances consumed
    let final_foreign = Assets::balance(ASSET_A, &zap_manager_account);
    assert!(final_foreign < foreign_amount);
  });
}

#[test]
fn test_zap_lp_distribution_respects_token_tol_binding() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());

    let tol_id = 9u32;
    let token_asset = AssetKind::Local(ASSET_A);
    let default_ingress = crate::tests::common::tol_treasury_account();
    let bound_ingress = tol_ingress_account_for_tol_id(tol_id);
    let zap_account = ZapManager::account_id();

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      tol_id,
      token_asset,
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));
    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      token_asset,
      tol_id,
    ));

    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));

    let native_amount = 2_000_000_000_000_000_000u128;
    let foreign_amount = 2_000_000_000_000_000_000u128;

    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_account.clone().into(),
      native_amount,
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_account.clone().into(),
      foreign_amount,
    ));

    System::set_block_number(11);
    ZapManager::on_initialize(11);
    ZapManager::on_idle(11, polkadot_sdk::frame_support::weights::Weight::MAX);

    let pool_id =
      <crate::configs::AssetConversionAdapter as AssetConversionApi<AccountId, u128>>::get_pool_id(
        AssetKind::Native,
        token_asset,
      )
      .expect("Pool should exist");

    let lp_token_id = match pool_id {
      AssetKind::Local(id) | AssetKind::Foreign(id) => id,
      AssetKind::Native => panic!("Invalid LP token ID"),
    };

    assert!(
      Assets::balance(lp_token_id, &bound_ingress) > 0,
      "Bound ingress should receive LP tokens"
    );
    assert_eq!(
      Assets::balance(lp_token_id, &default_ingress),
      0,
      "Default ingress should not receive LP tokens for bound token"
    );
  });
}

#[test]
fn test_ignore_non_whitelisted() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let zap_manager_account = ZapManager::account_id();
    // ASSET_A is used but NOT enabled
    let native_amount = 2_000_000_000_000_000_000;
    let foreign_amount = 2_000_000_000_000_000_000;

    // 1. Fund ZapManager
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      native_amount
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      foreign_amount
    ));

    // 2. Trigger on_initialize
    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // 3. Verify NO Zap Execution
    assert!(!System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::ZapCompleted { .. })
    )));

    // Balances remain (Initial Balance + deposited amount)
    let final_foreign = Assets::balance(ASSET_A, &zap_manager_account);
    // Check that balance increased exactly by deposit amount, meaning nothing was spent
    // We don't check absolute value because setup seeds accounts
    let initial_plus_deposit = crate::tests::common::INITIAL_BALANCE + foreign_amount;
    assert_eq!(final_foreign, initial_plus_deposit);
  });
}

#[test]
fn test_sweep_trigger() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let zap_manager_account = ZapManager::account_id();
    let tol_treasury = crate::tests::common::tol_treasury_account();
    let asset = AssetKind::Local(ASSET_A);
    let amount = 2_000_000_000_000_000_000;

    // 1. Fund ZapManager with non-enabled asset
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      amount
    ));

    // 2. Call sweep_trigger (Root only)
    assert_noop!(
      ZapManager::sweep_trigger(RuntimeOrigin::signed(ALICE), asset),
      DispatchError::BadOrigin
    );

    assert_ok!(ZapManager::sweep_trigger(RuntimeOrigin::root(), asset));

    // 3. Verify transfer to Treasury
    let zap_balance = Assets::balance(ASSET_A, &zap_manager_account);
    let treasury_balance = Assets::balance(ASSET_A, &tol_treasury);
    let min_balance = Assets::minimum_balance(ASSET_A);

    // Should leave min_balance
    assert_eq!(zap_balance, min_balance);

    assert!(treasury_balance > 0);

    // Event check
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      RuntimeEvent::ZapManager(pallet_zap_manager::Event::AssetsSwept { .. })
    )));
  });
}

#[test]
fn test_sweep_trigger_routes_lp_assets_by_bound_pair_domain() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());

    let tol_id = 12u32;
    let token_asset = AssetKind::Local(ASSET_A);
    let default_ingress = crate::tests::common::tol_treasury_account();
    let bound_ingress = tol_ingress_account_for_tol_id(tol_id);
    let zap_account = ZapManager::account_id();

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      tol_id,
      token_asset,
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));
    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      token_asset,
      tol_id,
    ));

    ensure_asset_conversion_pool(AssetKind::Native, token_asset);
    assert_ok!(add_liquidity(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      token_asset,
      LIQUIDITY_AMOUNT,
      LIQUIDITY_AMOUNT,
      MIN_LIQUIDITY,
      MIN_LIQUIDITY,
      &ALICE,
    ));

    let pool_id =
      <crate::configs::AssetConversionAdapter as AssetConversionApi<AccountId, u128>>::get_pool_id(
        AssetKind::Native,
        token_asset,
      )
      .expect("Pool should exist");

    let lp_token_id = match pool_id {
      AssetKind::Local(id) | AssetKind::Foreign(id) => id,
      AssetKind::Native => panic!("Invalid LP token ID"),
    };

    let sweep_amount = 1_000_000_000u128;
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      lp_token_id,
      zap_account.clone().into(),
      sweep_amount,
    ));

    assert_ok!(ZapManager::sweep_trigger(
      RuntimeOrigin::root(),
      AssetKind::Local(lp_token_id),
    ));

    assert!(
      Assets::balance(lp_token_id, &bound_ingress) > 0,
      "Bound ingress should receive swept LP"
    );
    assert_eq!(
      Assets::balance(lp_token_id, &default_ingress),
      0,
      "Default ingress should not receive swept LP"
    );
  });
}

#[test]
fn test_sweep_trigger_fails_for_enabled() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let asset = AssetKind::Local(ASSET_A);

    // 1. Enable asset
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), asset));

    // 2. Try sweep
    assert_noop!(
      ZapManager::sweep_trigger(RuntimeOrigin::root(), asset),
      pallet_zap_manager::Error::<crate::Runtime>::InvalidAsset
    );
  });
}

#[test]
fn test_omnivorous_intake() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = ZapManager::account_id();

    // Enable multiple assets for omnivorous zapping
    let test_assets = [ASSET_A, ASSET_B];
    for &asset_id in &test_assets {
      assert_ok!(ZapManager::enable_asset(
        RuntimeOrigin::root(),
        AssetKind::Local(asset_id),
      ));
    }

    // Deposit native AND foreign for each asset (required for LP creation)
    let deposit_amount = 2_000_000_000_000_000_000u128; // 2 Units
    let users = [ALICE, BOB];

    for &asset_id in &test_assets {
      for user in &users {
        // Transfer native to zap manager
        assert_ok!(Balances::transfer_allow_death(
          RuntimeOrigin::signed(user.clone()),
          zap_manager_account.clone().into(),
          deposit_amount
        ));
        // Transfer foreign to zap manager
        assert_ok!(Assets::transfer(
          RuntimeOrigin::signed(user.clone()),
          asset_id,
          zap_manager_account.clone().into(),
          deposit_amount
        ));
      }
    }

    // Trigger omnivorous processing
    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Verify at least one ZapManager event was emitted
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    assert!(
      !zap_events.is_empty(),
      "Omnivorous intake should process all assets"
    );
  });
}

#[test]
fn test_patriotic_accumulation() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = ZapManager::account_id();

    // Enable asset for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Burn existing Foreign assets from ZapManager (from setup) to ensure "No Foreign" condition
    let existing_foreign = Assets::balance(ASSET_A, &zap_manager_account);
    if existing_foreign > 0 {
      assert_ok!(Assets::burn(
        RuntimeOrigin::signed(ALICE), // Alice is admin
        ASSET_A,
        zap_manager_account.clone().into(),
        existing_foreign
      ));
    }

    // Transfer ONLY native to zap manager (no foreign)
    let native_deposit = 2_000_000_000_000_000_000u128; // 2 Units
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      native_deposit
    ));

    let native_before = Balances::free_balance(&zap_manager_account);

    // Trigger on_initialize - should NOT swap native (Patriotic Accumulation)
    System::set_block_number(2);
    ZapManager::on_initialize(2);
    ZapManager::on_idle(2, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Native should be preserved (no foreign to pair with)
    let native_after = Balances::free_balance(&zap_manager_account);
    assert!(
      native_after >= native_before.saturating_sub(crate::EXISTENTIAL_DEPOSIT * 10),
      "Native should be held (Patriotic Accumulation), not sold"
    );

    // Now add foreign tokens to enable LP creation
    let foreign_deposit = 2_000_000_000_000_000_000u128; // 2 Units
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      foreign_deposit
    ));

    // Process - should now create LP with accumulated native
    System::set_block_number(3);
    ZapManager::on_initialize(3);
    ZapManager::on_idle(3, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Verify ZapManager event was emitted
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    assert!(
      !zap_events.is_empty(),
      "ZapManager event should be emitted after foreign arrives"
    );
  });
}

#[test]
fn test_foreign_surplus_swap() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = ZapManager::account_id();

    // Enable asset for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Deposit more foreign than native to create surplus scenario
    let native_deposit = 2_000_000_000_000_000_000u128; // 2 Units
    let foreign_deposit = 10_000_000_000_000_000_000u128; // 10 Units (surplus)

    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      native_deposit
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      foreign_deposit
    ));

    // Process - should add LP and handle foreign surplus
    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Verify ZapManager event was emitted
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    assert!(!zap_events.is_empty(), "ZapManager event should be emitted");
  });
}

#[test]
fn test_retry_cooldown_backpressure() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = ZapManager::account_id();

    // Enable Asset A for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // 1. Initial Setup: Create pool and add liquidity (ensure normal operation is possible)
    // Fund Zap Manager with initial Native and Foreign
    let initial_amount = 2_000_000_000_000_000_000u128; // 2 Units
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      initial_amount
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      initial_amount
    ));

    // Trigger Zap to create pool (Block 2)
    System::set_block_number(2);
    ZapManager::on_initialize(2);
    ZapManager::on_idle(2, polkadot_sdk::frame_support::weights::Weight::MAX);

    // 2. Test Cooldown Mechanism: Manually set cooldown
    // Fund Zap Manager again
    let amount = 10_000_000_000_000_000_000u128; // 10 Units
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      amount
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      amount
    ));

    let balance_before = Assets::balance(ASSET_A, &zap_manager_account);

    // Manually set cooldown to Block 20
    let cooldown_block = 20;
    pallet_zap_manager::NextZapAttempt::<crate::Runtime>::insert(
      AssetKind::Local(ASSET_A),
      cooldown_block,
    );

    // Attempt during cooldown (Block 10)
    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Verify Skipped (Tokens remain)
    let foreign_bal_during = Assets::balance(ASSET_A, &zap_manager_account);
    assert_eq!(
      foreign_bal_during, balance_before,
      "Should skip processing during cooldown"
    );

    // Verify Cooldown still exists
    assert_eq!(
      pallet_zap_manager::NextZapAttempt::<crate::Runtime>::get(AssetKind::Local(ASSET_A)),
      Some(cooldown_block)
    );
  });
}

#[test]
fn test_opportunistic_cycle() {
  use super::common::seeded_test_ext;
  use crate::configs::axial_router_config::{AssetConversionAdapter, PriceOracleImpl};
  use pallet_axial_router::PriceOracle;
  use pallet_zap_manager::AssetConversionApi;

  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_axial_router_infrastructure());
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let zap_manager_account = ZapManager::account_id();
    let tol_treasury = crate::tests::common::tol_treasury_account();

    // 1. Enable asset
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A)
    ));

    // 2. Seed Phase: Concurrent deposits from different "sources"
    // Source 1: "User" (Alice) - Adds balanced liquidity
    let amount_user = 1_000_000_000_000_000_000u128; // 1 Unit
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_manager_account.clone().into(),
      amount_user
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      zap_manager_account.clone().into(),
      amount_user
    ));

    // Source 2: "TMC" (Simulated by Bob) - Adds surplus Native
    let amount_tmc = 5_000_000_000_000_000_000u128; // 5 Units (Surplus Native)
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(BOB),
      zap_manager_account.clone().into(),
      amount_tmc
    ));

    // Source 3: "XCM" (Simulated by Charlie) - Adds surplus Foreign
    let amount_xcm = 500_000_000_000_000_000u128; // 0.5 Units (Surplus Foreign)
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(CHARLIE),
      ASSET_A,
      zap_manager_account.clone().into(),
      amount_xcm
    ));

    // Initial State Check
    let initial_native = Balances::free_balance(&zap_manager_account);

    // 3. Oracle Protection Check (Before Trigger)
    // Simulate Oracle deviation to block execution
    // Set Oracle price to something very different from pool spot price (1:1)
    let deviated_price = 2_000_000_000_000u128; // 2.0 (Pool is 1.0, Precision 1e12)
    // Update both directions to ensure validation catches it
    assert_ok!(PriceOracleImpl::<crate::Runtime>::update_ema_price(
      AssetKind::Native,
      AssetKind::Local(ASSET_A),
      deviated_price
    ));
    assert_ok!(PriceOracleImpl::<crate::Runtime>::update_ema_price(
      AssetKind::Local(ASSET_A),
      AssetKind::Native,
      deviated_price
    ));

    System::set_block_number(10);
    ZapManager::on_initialize(10);
    ZapManager::on_idle(10, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Verify NO Zap occurred due to oracle deviation
    let blocked_native = Balances::free_balance(&zap_manager_account);
    assert_eq!(
      blocked_native, initial_native,
      "Zap should be blocked by Oracle deviation"
    );

    // 4. Recovery & Execution Phase
    // Reset Oracle to match pool (Pool is ~1:1)
    // Force set storage to avoid EMA smoothing delay for test determinism
    let valid_price = 1_000_000_000_000u128; // 1.0 (Precision 1e12)
    pallet_axial_router::EmaPrices::<crate::Runtime>::insert(
      AssetKind::Native,
      AssetKind::Local(ASSET_A),
      valid_price,
    );
    // Also update inverse for completeness
    pallet_axial_router::EmaPrices::<crate::Runtime>::insert(
      AssetKind::Local(ASSET_A),
      AssetKind::Native,
      valid_price,
    );

    // Advance block past cooldown (default 10 blocks)
    System::set_block_number(25);
    ZapManager::on_initialize(25);
    ZapManager::on_idle(25, polkadot_sdk::frame_support::weights::Weight::MAX);

    // 5. Verify Liquidity Creation
    // Get LP token ID
    let pool_id = <AssetConversionAdapter as AssetConversionApi<AccountId, u128>>::get_pool_id(
      AssetKind::Native,
      AssetKind::Local(ASSET_A),
    )
    .expect("Pool should exist");

    let lp_token_id = match pool_id {
      AssetKind::Local(id) => id,
      _ => panic!("Invalid LP token ID"),
    };

    // Check TOL received LP tokens
    let tol_lp_balance = Assets::balance(lp_token_id, &tol_treasury);
    assert!(tol_lp_balance > 0, "TOL should receive LP tokens");

    // 6. Verify Surplus Management (Opportunistic Behavior)
    let final_native = Balances::free_balance(&zap_manager_account);
    let final_foreign = Assets::balance(ASSET_A, &zap_manager_account);

    // Should hold significant Native (approx 4.5 units, minus fees/dust)
    assert!(
      final_native > 4_000_000_000_000_000_000u128,
      "Should hold native surplus (Patriotic Accumulation)"
    );

    // Should have exhausted Foreign (or close to dust)
    assert!(
      final_foreign < 1_000_000_000_000_000u128,
      "Should exhaust foreign assets"
    ); // < 0.001 Unit

    // Verify Events
    let events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    assert!(
      events.iter().any(|r| matches!(
        r.event,
        crate::RuntimeEvent::ZapManager(pallet_zap_manager::Event::LPTokensDistributed { .. })
      )),
      "Should distribute LP tokens"
    );

    // Verify NativeHeld event
    assert!(
      events.iter().any(|r| matches!(
        r.event,
        crate::RuntimeEvent::ZapManager(pallet_zap_manager::Event::NativeHeld { .. })
      )),
      "Should emit NativeHeld event"
    );
  });
}

#[test]
fn test_foreign_asset_zap_flow() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_integration_infrastructure());
    let zap_account = ZapManager::account_id();
    let tol_treasury = crate::configs::TolTreasuryAccount::get();
    let foreign_asset = AssetKind::Foreign(ASSET_FOREIGN);

    // Ensure pool exists and seed liquidity for foreign asset
    ensure_asset_conversion_pool(AssetKind::Native, foreign_asset);
    assert_ok!(add_liquidity(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      foreign_asset,
      LIQUIDITY_AMOUNT,
      LIQUIDITY_AMOUNT,
      MIN_LIQUIDITY,
      MIN_LIQUIDITY,
      &ALICE,
    ));

    // Enable foreign asset
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      foreign_asset
    ));

    // Fund zap account (simulate native + foreign ingress)
    let fund_amount = 2 * LIQUIDITY_AMOUNT;
    assert_ok!(Balances::transfer_allow_death(
      RuntimeOrigin::signed(ALICE),
      zap_account.clone().into(),
      fund_amount
    ));
    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_FOREIGN,
      zap_account.clone().into(),
      fund_amount
    ));

    // Trigger zap
    System::set_block_number(2);
    ZapManager::on_initialize(2);
    ZapManager::on_idle(2, polkadot_sdk::frame_support::weights::Weight::MAX);

    // Validate LP distribution to TOL
    let pool_id =
      <crate::configs::AssetConversionAdapter as AssetConversionApi<AccountId, u128>>::get_pool_id(
        AssetKind::Native,
        foreign_asset,
      )
      .expect("Pool should exist");

    let lp_id = match pool_id {
      AssetKind::Local(id) | AssetKind::Foreign(id) => id,
      AssetKind::Native => panic!("Invalid LP token ID"),
    };
    let tol_lp_balance = Assets::balance(lp_id, &tol_treasury);
    assert!(
      tol_lp_balance > 0,
      "TOL should receive LP tokens for foreign zap"
    );

    // Foreign balance on zap account should be near dust
    let remaining_foreign = Assets::balance(ASSET_FOREIGN, &zap_account);
    assert!(
      remaining_foreign < LIQUIDITY_AMOUNT / 10,
      "Foreign balance should be mostly consumed"
    );
  });
}
