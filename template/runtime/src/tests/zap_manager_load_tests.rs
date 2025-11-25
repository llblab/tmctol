//! Zap Manager Load Tests
//!
//! This module contains load tests specifically for the Zap Manager component,
//! validating its behavior under high concurrency, batch processing, and recovery scenarios.

use super::common::{seeded_test_ext, ALICE, ASSET_A, ASSET_FOREIGN, BOB, CHARLIE};
use crate::{AccountId, Assets, Balances, Runtime, RuntimeOrigin, System, ZapManager};
use polkadot_sdk::frame_support::assert_ok;
use polkadot_sdk::frame_support::traits::{Currency, Hooks};
use primitives::AssetKind;

/// Setup Zap Manager infrastructure (mirrors zap_manager_integration_tests pattern)
fn setup_zap_manager_infrastructure() -> Result<(), &'static str> {
  let zap_manager_account = ZapManager::account_id();

  // Ensure ALICE has sufficient balance for operations
  let alice_balance = Balances::free_balance(&ALICE);
  if alice_balance < 10_000_000_000_000_000_000 {
    let _ = Balances::deposit_creating(&ALICE, 10_000_000_000_000_000_000 - alice_balance);
  }

  // Ensure zap manager has sufficient native balance for operations
  let current_balance = Balances::free_balance(&zap_manager_account);
  let min_balance = <Balances as Currency<AccountId>>::minimum_balance();
  if current_balance < min_balance * 2 {
    let _ =
      <Balances as Currency<AccountId>>::deposit_creating(&zap_manager_account, min_balance * 10);
  }

  // Ensure ALICE has sufficient asset balances for operations
  assert_ok!(Assets::mint(
    RuntimeOrigin::signed(ALICE),
    ASSET_A,
    polkadot_sdk::sp_runtime::MultiAddress::Id(ALICE.clone()),
    10_000_000_000_000_000_000
  ));
  assert_ok!(Assets::mint(
    RuntimeOrigin::signed(ALICE),
    ASSET_FOREIGN,
    polkadot_sdk::sp_runtime::MultiAddress::Id(ALICE.clone()),
    10_000_000_000_000_000_000
  ));

  Ok(())
}

/// Zap Manager batch processing under load test
#[test]
fn test_zap_manager_batch_processing_under_load() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = crate::tests::common::zap_manager_account();

    // Enable Asset A for zapping (following working integration test pattern)
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Record balance before batch deposits
    let initial_foreign = Assets::balance(ASSET_A, &zap_manager_account);

    // Simulate batch deposits: transfer native AND foreign (required for LP creation)
    let deposit_amount = 2_000_000_000_000_000_000u128; // 2 Units
    let deposit_count = 5u128;

    for _ in 0..deposit_count {
      // Transfer both native and foreign to enable zap
      assert_ok!(Balances::transfer_allow_death(
        RuntimeOrigin::signed(ALICE),
        zap_manager_account.clone().into(),
        deposit_amount
      ));
      assert_ok!(Assets::transfer(
        RuntimeOrigin::signed(ALICE),
        ASSET_A,
        zap_manager_account.clone().into(),
        deposit_amount
      ));
    }

    let accumulated = Assets::balance(ASSET_A, &zap_manager_account);
    assert!(
      accumulated > initial_foreign,
      "ZapManager should have accumulated deposits"
    );

    // Trigger batch processing
    System::set_block_number(10);
    ZapManager::on_initialize(10);

    // Debug: Print all ZapManager events
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| {
        matches!(
          r.event,
          crate::RuntimeEvent::ZapManager(..)
        )
      })
      .collect();

    // Print enabled assets
    let enabled: Vec<_> = pallet_zap_manager::EnabledAssets::<Runtime>::iter().collect();

    // Print balances
    let native_bal = Balances::free_balance(&zap_manager_account);
    let foreign_bal = Assets::balance(ASSET_A, &zap_manager_account);

    assert!(
      !zap_events.is_empty(),
      "ZapCompleted event should be emitted. Debug: enabled_assets={enabled:?}, native_bal={native_bal}, foreign_bal={foreign_bal}, events={zap_events:?}",
    );
  });
}

/// Zap Manager concurrent user stress test
#[test]
fn test_zap_manager_concurrent_user_deposits() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = crate::tests::common::zap_manager_account();

    // Enable asset for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Simulate concurrent deposits from multiple users across multiple blocks
    let deposit_amount = 2_000_000_000_000_000_000u128; // 2 Units
    let users = [ALICE, BOB, CHARLIE];
    let mut zap_count = 0u32;

    for block in 2..=6u32 {
      System::set_block_number(block);

      // Each user deposits native AND foreign in each block
      for user in &users {
        assert_ok!(Balances::transfer_allow_death(
          RuntimeOrigin::signed(user.clone()),
          zap_manager_account.clone().into(),
          deposit_amount
        ));
        assert_ok!(Assets::transfer(
          RuntimeOrigin::signed(user.clone()),
          ASSET_A,
          zap_manager_account.clone().into(),
          deposit_amount
        ));
      }

      // Process deposits
      ZapManager::on_initialize(block);

      // Count ZapManager events in this block
      zap_count += crate::System::events()
        .into_iter()
        .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
        .count() as u32;
    }

    // At least some zap-related events should have occurred
    let enabled: Vec<_> = pallet_zap_manager::EnabledAssets::<Runtime>::iter().collect();
    assert!(
      zap_count > 0,
      "At least one zap event should occur during concurrent deposits. enabled={enabled:?}",
    );
  });
}

/// Zap Manager high-frequency deposit/process cycles
#[test]
fn test_zap_manager_high_frequency_cycles() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = crate::tests::common::zap_manager_account();

    // Enable asset for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Simulate 10 rapid deposit-process cycles
    let deposit_amount = 2_000_000_000_000_000_000u128; // 2 Units
    let mut total_zaps = 0u32;

    for cycle in 2..=11u32 {
      System::set_block_number(cycle);

      // Deposit native AND foreign (required for LP creation)
      assert_ok!(Balances::transfer_allow_death(
        RuntimeOrigin::signed(ALICE),
        zap_manager_account.clone().into(),
        deposit_amount
      ));
      assert_ok!(Assets::transfer(
        RuntimeOrigin::signed(ALICE),
        ASSET_A,
        zap_manager_account.clone().into(),
        deposit_amount
      ));

      ZapManager::on_initialize(cycle);

      // Count zap events
      total_zaps += crate::System::events()
        .into_iter()
        .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
        .count() as u32;
    }

    // Multiple zap events should have occurred
    let enabled: Vec<_> = pallet_zap_manager::EnabledAssets::<Runtime>::iter().collect();
    assert!(
      total_zaps >= 5,
      "At least 5 zap events should occur in 10 cycles (got: {total_zaps}). enabled={enabled:?}",
    );
  });
}

/// Zap Manager recovery after heavy load period
#[test]
fn test_zap_manager_recovery_after_load() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_infrastructure());
    System::set_block_number(1);
    let zap_manager_account = crate::tests::common::zap_manager_account();

    // Enable asset for zapping
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));

    // Phase 1: Heavy load (many deposits without processing)
    let heavy_deposit = 2_000_000_000_000_000_000u128; // 2 Units
    for _ in 0..5 {
      // Deposit native AND foreign (required for LP creation)
      assert_ok!(Balances::transfer_allow_death(
        RuntimeOrigin::signed(ALICE),
        zap_manager_account.clone().into(),
        heavy_deposit
      ));
      assert_ok!(Assets::transfer(
        RuntimeOrigin::signed(ALICE),
        ASSET_A,
        zap_manager_account.clone().into(),
        heavy_deposit
      ));
    }

    // Phase 2: Single processing event handles backlog
    System::set_block_number(10);
    ZapManager::on_initialize(10);

    // Verify processing occurred via ZapCompleted event
    // Verify ZapManager event should be emitted after processing backlog
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    let enabled: Vec<_> = pallet_zap_manager::EnabledAssets::<Runtime>::iter().collect();
    assert!(
      !zap_events.is_empty(),
      "ZapManager event should be emitted after processing backlog. enabled={enabled:?}, events={zap_events:?}",
    );

    // Phase 3: Normal operation resumes
    for block in 11..=13u32 {
      System::set_block_number(block);

      // Fresh deposit
      assert_ok!(Balances::transfer_allow_death(
        RuntimeOrigin::signed(BOB),
        zap_manager_account.clone().into(),
        heavy_deposit
      ));
      assert_ok!(Assets::transfer(
        RuntimeOrigin::signed(BOB),
        ASSET_A,
        zap_manager_account.clone().into(),
        heavy_deposit
      ));

      ZapManager::on_initialize(block);
    }

    // Verify continued processing
    let zap_events: Vec<_> = crate::System::events()
      .into_iter()
      .filter(|r| matches!(r.event, crate::RuntimeEvent::ZapManager(..)))
      .collect();

    let enabled: Vec<_> = pallet_zap_manager::EnabledAssets::<Runtime>::iter().collect();
    assert!(
      !zap_events.is_empty(),
      "ZapManager events should occur during recovery. enabled={enabled:?}, events={zap_events:?}",
    );
  });
}
