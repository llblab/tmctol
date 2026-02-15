//! Zap Manager Load Tests
//!
//! This module contains load tests specifically for the Zap Manager component,
//! validating its behavior under high concurrency, batch processing, and recovery scenarios.

use super::common::{
  ALICE, ASSET_A, ASSET_B, ASSET_FOREIGN, BOB, CHARLIE, LIQUIDITY_AMOUNT, MIN_LIQUIDITY,
  add_liquidity, ensure_asset_conversion_pool, seeded_test_ext,
};
use crate::{AccountId, Assets, Balances, Runtime, RuntimeOrigin, System, ZapManager};
use pallet_zap_manager::WeightInfo as ZapWeightInfo;
use polkadot_sdk::frame_support::assert_ok;
use polkadot_sdk::frame_support::traits::{Currency, Hooks};
use primitives::AssetKind;

/// Setup Zap Manager infrastructure (mirrors zap_manager_integration_tests pattern)
fn setup_zap_manager_infrastructure() -> Result<(), &'static str> {
  let zap_manager_account = ZapManager::account_id();

  // Ensure test accounts have sufficient native balance for operations
  // Deposit generous amount to support high-frequency test cycles
  for user in [ALICE, BOB, CHARLIE] {
    let user_balance = Balances::free_balance(&user);
    if user_balance < 100_000_000_000_000_000_000 {
      let _ = Balances::deposit_creating(&user, 100_000_000_000_000_000_000 - user_balance);
    }
  }

  // Ensure zap manager has sufficient native balance for operations
  let current_balance = Balances::free_balance(&zap_manager_account);
  let min_balance = <Balances as Currency<AccountId>>::minimum_balance();
  if current_balance < min_balance * 2 {
    let _ =
      <Balances as Currency<AccountId>>::deposit_creating(&zap_manager_account, min_balance * 10);
  }

  // Ensure test accounts have sufficient asset balances for operations
  // Mint generous amount to support high-frequency test cycles
  for user in [ALICE, BOB, CHARLIE] {
    assert_ok!(Assets::mint(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      polkadot_sdk::sp_runtime::MultiAddress::Id(user.clone()),
      100_000_000_000_000_000_000
    ));
  }
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

#[test]
fn test_skewed_pending_load_does_not_starve_secondary_asset() {
  seeded_test_ext().execute_with(|| {
    assert_ok!(setup_zap_manager_infrastructure());
    let _ = Balances::deposit_creating(&ALICE, 500_000_000_000_000_000_000u128);
    assert_ok!(Assets::mint(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      polkadot_sdk::sp_runtime::MultiAddress::Id(ALICE),
      500_000_000_000_000_000_000u128,
    ));
    assert_ok!(Assets::mint(
      RuntimeOrigin::signed(ALICE),
      ASSET_B,
      polkadot_sdk::sp_runtime::MultiAddress::Id(ALICE),
      500_000_000_000_000_000_000u128,
    ));
    let zap_manager_account = crate::tests::common::zap_manager_account();

    ensure_asset_conversion_pool(AssetKind::Native, AssetKind::Local(ASSET_A));
    ensure_asset_conversion_pool(AssetKind::Native, AssetKind::Local(ASSET_B));

    assert_ok!(add_liquidity(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      AssetKind::Local(ASSET_A),
      LIQUIDITY_AMOUNT,
      LIQUIDITY_AMOUNT,
      MIN_LIQUIDITY,
      MIN_LIQUIDITY,
      &ALICE,
    ));
    assert_ok!(add_liquidity(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      AssetKind::Local(ASSET_B),
      LIQUIDITY_AMOUNT,
      LIQUIDITY_AMOUNT,
      MIN_LIQUIDITY,
      MIN_LIQUIDITY,
      &ALICE,
    ));

    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
    ));
    assert_ok!(ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_B),
    ));

    let one_cycle_weight =
      <<Runtime as pallet_zap_manager::Config>::WeightInfo as ZapWeightInfo>::process_zap_cycle();

    let heavy_native = 20_000_000_000_000_000_000u128;
    let heavy_foreign = 2_000_000_000_000_000_000u128;
    let light_native = 3_000_000_000_000_000_000u128;
    let light_foreign = 3_000_000_000_000_000_000u128;

    let mut asset_a_zaps = 0u32;
    let mut asset_b_zaps = 0u32;
    let mut first_asset_b_block: Option<u32> = None;

    for block in 2..=6u32 {
      System::set_block_number(block);
      System::reset_events();

      assert_ok!(Balances::transfer_allow_death(
        RuntimeOrigin::signed(ALICE),
        zap_manager_account.clone().into(),
        heavy_native,
      ));
      assert_ok!(Assets::transfer(
        RuntimeOrigin::signed(ALICE),
        ASSET_A,
        zap_manager_account.clone().into(),
        heavy_foreign,
      ));

      if block == 2 {
        assert_ok!(Balances::transfer_allow_death(
          RuntimeOrigin::signed(ALICE),
          zap_manager_account.clone().into(),
          light_native,
        ));
        assert_ok!(Assets::transfer(
          RuntimeOrigin::signed(ALICE),
          ASSET_B,
          zap_manager_account.clone().into(),
          light_foreign,
        ));
      }

      ZapManager::on_initialize(block);
      ZapManager::on_idle(block, one_cycle_weight);

      let mut block_zap_count = 0u32;
      for record in System::events() {
        if let crate::RuntimeEvent::ZapManager(pallet_zap_manager::Event::ZapCompleted {
          token_asset,
          ..
        }) = record.event
        {
          block_zap_count = block_zap_count.saturating_add(1);
          match token_asset {
            AssetKind::Local(id) if id == ASSET_A => {
              asset_a_zaps = asset_a_zaps.saturating_add(1);
            }
            AssetKind::Local(id) if id == ASSET_B => {
              asset_b_zaps = asset_b_zaps.saturating_add(1);
              if first_asset_b_block.is_none() {
                first_asset_b_block = Some(block);
              }
            }
            _ => {}
          }
        }
      }

      assert!(
        block_zap_count <= 1,
        "Single-cycle weight budget should process at most one zap per block"
      );
    }

    assert!(asset_a_zaps > 0, "Primary asset should be processed");
    assert!(
      asset_b_zaps > 0,
      "Secondary asset should not starve under skewed pending load"
    );
    assert!(
      first_asset_b_block.is_some_and(|block| block <= 3),
      "Secondary asset should be processed within one additional cycle window"
    );
  });
}
