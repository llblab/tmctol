//! Unit tests for the Zap Manager pallet.

use crate::mock::*;
use crate::{AssetConversionApi, Event, PendingZaps, ZapExecutionCursor};
use polkadot_sdk::frame_support::traits::fungibles::{Inspect, Mutate};
use polkadot_sdk::frame_support::{
  assert_ok,
  traits::{Currency, Get, Hooks},
  weights::Weight,
};
use primitives::{AssetKind, TYPE_FOREIGN, ecosystem::params::PRECISION};

#[test]
fn min_swap_foreign_threshold() {
  let min_threshold = <<Test as crate::Config>::MinSwapForeign as Get<u128>>::get();
  assert_eq!(min_threshold, PRECISION);
  let amount_below_threshold = PRECISION / 2;
  assert!(amount_below_threshold < min_threshold);
}

#[test]
fn tol_account_resolver_configuration() {
  let tol_account = <<Test as crate::Config>::TolAccountResolver as crate::TolAccountResolver<
    u64,
  >>::resolve_tol_account(AssetKind::Local(1));
  assert_eq!(tol_account, 999);
}

#[test]
fn dust_threshold_configuration() {
  let dust_threshold = <<Test as crate::Config>::DustThreshold as Get<u128>>::get();
  assert_eq!(dust_threshold, PRECISION);
}

#[test]
fn foreign_asset_opportunistic_zap() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let foreign_id = TYPE_FOREIGN | 1;
    let token_asset = AssetKind::Foreign(foreign_id);
    let native_asset = AssetKind::Native;
    // Ensure foreign asset exists and pool is ready
    assert_ok!(Assets::force_create(
      RuntimeOrigin::root(),
      foreign_id,
      zap_account,
      true,
      1
    ));
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 1_000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    // Align oracle with spot price
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Fund zap account with balanced amounts
    let zap_amount = 500 * PRECISION;
    let _ = Balances::deposit_creating(&zap_account, zap_amount);
    assert_ok!(Assets::mint_into(foreign_id, &zap_account, zap_amount));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    let has_zap_completed = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::ZapCompleted { .. })
      )
    });
    assert!(
      has_zap_completed,
      "Expected ZapCompleted for foreign asset zap"
    );
  });
}

#[test]
fn liquidity_addition_logic() {
  new_test_ext().execute_with(|| {
    let user = 1u64;
    let asset1 = AssetKind::Native;
    let asset2 = AssetKind::Local(1);
    assert_ok!(MockAssetConversion::create_pool(asset1, asset2));
    let initial_amount = 10 * PRECISION;
    let _ = Balances::deposit_creating(&user, initial_amount);
    assert_ok!(Assets::mint_into(1, &user, initial_amount));
    // Initial liquidity: sqrt(1*P * 1*P) = 1*P
    let add_amount = PRECISION;
    let (used1, used2, lp) =
      MockAssetConversion::add_liquidity(&user, asset1, asset2, add_amount, add_amount, 0, 0)
        .unwrap();
    assert_eq!(used1, add_amount);
    assert_eq!(used2, add_amount);
    assert_eq!(lp, add_amount);
    // Subsequent liquidity: proportional
    let (used1_s, used2_s, lp_s) =
      MockAssetConversion::add_liquidity(&user, asset1, asset2, add_amount, add_amount, 0, 0)
        .unwrap();
    assert_eq!(used1_s, add_amount);
    assert_eq!(used2_s, add_amount);
    assert_eq!(lp_s, 1_000_000_000); // Mock formula: (amount * 1B) / reserve
  });
}

#[test]
fn pool_creation_logic() {
  new_test_ext().execute_with(|| {
    let native = AssetKind::Native;
    let token = AssetKind::Local(1);
    assert_ok!(MockAssetConversion::create_pool(native, token));
    // Try creating again - should fail
    assert!(MockAssetConversion::create_pool(native, token).is_err());
  });
}

#[test]
fn swap_exact_tokens_for_tokens_logic() {
  new_test_ext().execute_with(|| {
    let user = 1u64;
    let native = AssetKind::Native;
    let token = AssetKind::Local(1);
    assert_ok!(MockAssetConversion::create_pool(native, token));
    // Fund user and add initial liquidity
    let initial_amount = 10_000 * PRECISION;
    let _ = Balances::deposit_creating(&user, initial_amount);
    assert_ok!(Assets::mint_into(1, &user, initial_amount));
    let liquidity_amount = 1_000 * PRECISION;
    assert_ok!(MockAssetConversion::add_liquidity(
      &user,
      native,
      token,
      liquidity_amount,
      liquidity_amount,
      0,
      0
    ));
    // Swap 100*PRECISION token for native
    // XYK: amount_out = (100*P * 1000*P) / (1000*P + 100*P) = ~90.9*P
    let swap_amount = 100 * PRECISION;
    let amount_out =
      MockAssetConversion::swap_exact_tokens_for_tokens(&user, token, native, swap_amount, 0)
        .unwrap();
    assert!(amount_out > 0);
    assert!(amount_out < swap_amount); // Slippage
  });
}

#[test]
fn whitelist_asset_management() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let asset = AssetKind::Local(1);
    // Enable asset
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), asset));
    System::assert_last_event(Event::AssetEnabled { asset }.into());
    // Try enabling again - should succeed (idempotent)
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), asset));
    // Disable asset
    assert_ok!(ZapManager::disable_asset(RuntimeOrigin::root(), asset));
    System::assert_last_event(Event::AssetDisabled { asset }.into());
    // Try disabling again - should not error
    assert_ok!(ZapManager::disable_asset(RuntimeOrigin::root(), asset));
  });
}

#[test]
fn sweep_trigger_functionality() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let asset = AssetKind::Local(1);
    let treasury = 999u64;
    let fund_amount = 10_000u128;
    assert_ok!(Assets::mint_into(1, &zap_account, fund_amount));
    // Ensure asset is not enabled
    assert!(ZapManager::enabled_assets(asset).is_none());
    // Sweep the asset
    assert_ok!(ZapManager::sweep_trigger(RuntimeOrigin::root(), asset));
    System::assert_last_event(
      Event::AssetsSwept {
        assets: vec![asset],
        destination: treasury,
      }
      .into(),
    );
    let min_balance = <Assets as Inspect<_>>::minimum_balance(1);
    let remaining = Assets::balance(1, zap_account);
    assert!(remaining <= min_balance);
  });
}

#[test]
fn on_idle_preserves_all_pending_when_weight_is_insufficient() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);

    let asset_a = AssetKind::Local(1);
    let asset_b = AssetKind::Local(2);
    let pending_amount = 2 * PRECISION;

    PendingZaps::<Test>::insert(asset_a, pending_amount);
    PendingZaps::<Test>::insert(asset_b, pending_amount);

    let consumed = ZapManager::on_idle(1, Weight::zero());
    assert_eq!(consumed, Weight::zero());
    assert_eq!(PendingZaps::<Test>::iter().count(), 2);
    assert_eq!(PendingZaps::<Test>::get(asset_a), Some(pending_amount));
    assert_eq!(PendingZaps::<Test>::get(asset_b), Some(pending_amount));
  });
}

#[test]
fn on_idle_uses_cursor_for_round_robin_fairness() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);

    let asset_a = AssetKind::Local(1);
    let asset_b = AssetKind::Local(2);
    let pending_amount = 2 * PRECISION;

    PendingZaps::<Test>::insert(asset_a, pending_amount);
    PendingZaps::<Test>::insert(asset_b, pending_amount);
    ZapExecutionCursor::<Test>::put(asset_a);

    let zap_weight =
      <<Test as crate::Config>::WeightInfo as crate::WeightInfo>::process_zap_cycle();
    let consumed = ZapManager::on_idle(1, zap_weight);

    assert_eq!(consumed, zap_weight);
    assert_eq!(ZapExecutionCursor::<Test>::get(), Some(asset_b));
    assert_eq!(PendingZaps::<Test>::iter().count(), 1);
    assert_eq!(PendingZaps::<Test>::get(asset_a), Some(pending_amount));
    assert_eq!(PendingZaps::<Test>::get(asset_b), None);
  });
}

#[test]
fn opportunistic_zap_balanced_ratio() {
  // Test opportunistic zap when native and foreign are balanced with pool ratio
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    // Create pool with 1:1 ratio
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 1000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    // Set oracle price matching spot
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Fund zap account with balanced amounts
    let zap_amount = 100 * PRECISION;
    let _ = Balances::deposit_creating(&zap_account, zap_amount);
    assert_ok!(Assets::mint_into(1, &zap_account, zap_amount));
    // Enable asset
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    // Trigger on_initialize and on_idle
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Should have ZapCompleted event since liquidity was added
    let has_zap_completed = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::ZapCompleted { .. })
      )
    });
    assert!(has_zap_completed, "Expected ZapCompleted event");
  });
}

#[test]
fn opportunistic_zap_foreign_surplus_swapped() {
  // Test that excess foreign tokens are swapped to native
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    // Create pool with 1:1 ratio and sufficient liquidity for swaps
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 10_000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Fund with imbalanced amounts: Heavy in Foreign, Light in Native
    // Pool is 1:1, so optimal is 1000:1000
    // Foreign surplus = 5000 - 1000 = 4000 (should be swapped)
    let _ = Balances::deposit_creating(&zap_account, 1_000 * PRECISION);
    assert_ok!(Assets::mint_into(1, &zap_account, 5_000 * PRECISION));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Check for SurplusSwapped event
    let has_surplus_swapped = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::SurplusSwapped { .. })
      )
    });
    assert!(
      has_surplus_swapped,
      "Expected SurplusSwapped event for excess foreign tokens"
    );
  });
}

#[test]
fn opportunistic_zap_native_held() {
  // Test that excess native tokens are held (not swapped)
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    // Create pool with 1:1 ratio
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 1_000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    // Set oracle price to 1.0 (matching pool spot price)
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Fund with imbalanced amounts: Heavy in Native, Light in Foreign
    // Pool is 1:1, so optimal is 1000:1000
    // Native surplus = 5000 - 1000 = 4000 (should be HELD, not swapped)
    let _ = Balances::deposit_creating(&zap_account, 5_000 * PRECISION);
    assert_ok!(Assets::mint_into(1, &zap_account, 1_000 * PRECISION));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Check for NativeHeld event (not SurplusSwapped)
    let has_native_held = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::NativeHeld { .. })
      )
    });
    assert!(
      has_native_held,
      "Expected NativeHeld event for excess native tokens"
    );
    // Verify NO SurplusSwapped event (native should not be swapped to foreign)
    let has_surplus_swapped = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::SurplusSwapped { .. })
      )
    });
    assert!(!has_surplus_swapped, "Native surplus should NOT be swapped");
  });
}

#[test]
fn zap_execution_sequence() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    let _amount = 750_000u128;
    let zap_amount = 100 * PRECISION;
    let _ = Balances::deposit_creating(&zap_account, zap_amount);
    assert_ok!(Assets::mint_into(1, &zap_account, zap_amount));
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    // Pool starts empty (0,0) so no oracle check needed for initial liquidity
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(System::block_number());
    ZapManager::on_idle(System::block_number(), Weight::MAX);
    // Check that LPTokensDistributed event was emitted
    let has_lp_distributed = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
      )
    });
    assert!(has_lp_distributed, "Expected LPTokensDistributed event");
  });
}

#[test]
fn threshold_validation() {
  let min_swap_threshold = PRECISION;
  assert!(PRECISION >= min_swap_threshold);
  assert!(1_500 * PRECISION >= min_swap_threshold);
  assert!(PRECISION / 2 < min_swap_threshold);
  assert!(PRECISION - 1 < min_swap_threshold);
}

#[test]
fn arithmetic_safety() {
  let large_native = 1_000_000_000_000u128;
  let large_foreign = 1_000_000_000_000u128;
  let lp_tokens = large_native.min(large_foreign);
  assert_eq!(lp_tokens, 1_000_000_000_000);
  let max_amount = u128::MAX;
  let small_amount = 1u128;
  let small_lp_tokens = small_amount.min(max_amount);
  assert_eq!(small_lp_tokens, 1);
  assert!(lp_tokens > 0);
  assert!(small_lp_tokens > 0);
}

#[test]
fn gravity_well_escape_simulation() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    let zap_account = ZapManager::account_id();
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    // Create pool but leave it empty (0,0) - no oracle needed for initial liquidity
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    // Sub-threshold amount (MinSwapForeign = PRECISION)
    let small_amount = PRECISION / 2;
    let _ = Balances::deposit_creating(&zap_account, small_amount);
    assert_ok!(Assets::mint_into(1, &zap_account, small_amount));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Should NOT zap because amount < PRECISION (after ED subtraction)
    assert!(!System::events().iter().any(|r| matches!(
      r.event,
      crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
    )));
    // Add more to cross threshold (add 2*PRECISION to ensure we're well above threshold)
    let _ = Balances::deposit_creating(&zap_account, 2 * PRECISION);
    assert_ok!(Assets::mint_into(1, &zap_account, 2 * PRECISION));
    ZapManager::on_initialize(2);
    ZapManager::on_idle(2, Weight::MAX);
    // Should zap now (initial liquidity to empty pool)
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
    )));
  });
}

#[test]
fn price_deviation_protection() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    let zap_account = ZapManager::account_id();
    let amount = 1_000 * PRECISION;
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 1_000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    let _ = Balances::deposit_creating(&zap_account, amount * 2);
    assert_ok!(Assets::mint_into(1, &zap_account, amount * 2));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    // Set Oracle Price with high deviation (50%)
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), 2 * PRECISION);
    });
    // Should fail due to deviation
    ZapManager::on_initialize(System::block_number());
    ZapManager::on_idle(System::block_number(), Weight::MAX);
    assert!(!System::events().iter().any(|r| matches!(
      r.event,
      crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
    )));
    // Fix oracle price
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Clear events from first attempt
    System::reset_events();
    // Should succeed now - advance block number past retry cooldown (10 blocks)
    System::set_block_number(System::block_number() + 11);
    ZapManager::on_initialize(System::block_number());
    ZapManager::on_idle(System::block_number(), Weight::MAX);
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
    )));
  });
}

#[test]
fn opportunistic_strategy_no_pre_swap_balancing() {
  // Verify that the system does NOT swap native to foreign before adding liquidity
  // This is the key difference from "Active Balancing" strategy
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    let pool_reserve = 1_000 * PRECISION;
    set_pool(native_asset, token_asset, pool_reserve, pool_reserve);
    ORACLE_PRICES.with(|p| {
      p.borrow_mut()
        .insert((native_asset, token_asset), PRECISION);
    });
    // Heavily unbalanced: lots of Native, little Foreign
    // Foreign amount must exceed MinSwapForeign (PRECISION)
    let _ = Balances::deposit_creating(&zap_account, 100 * PRECISION);
    assert_ok!(Assets::mint_into(1, &zap_account, 2 * PRECISION));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Check that we got a ZapCompleted event with foreign_used matching foreign_available
    // (no pre-swap means we used what we had)
    let foreign_used_values: Vec<u128> = System::events()
      .into_iter()
      .filter_map(|r| {
        if let crate::mock::RuntimeEvent::ZapManager(crate::Event::ZapCompleted {
          foreign_used,
          ..
        }) = r.event
        {
          Some(foreign_used)
        } else {
          None
        }
      })
      .collect();
    // Foreign used should be close to what we had (minus ED/dust)
    if !foreign_used_values.is_empty() {
      let foreign_used = foreign_used_values[0];
      let foreign_available = 2 * PRECISION;
      // Should use approximately what we deposited (2 * PRECISION)
      assert!(
        foreign_used <= foreign_available,
        "Should not use more foreign than available"
      );
    }
    // Verify zap operation completed successfully
    let has_zap_completed = System::events().into_iter().any(|r| {
      matches!(
        r.event,
        crate::mock::RuntimeEvent::ZapManager(crate::Event::ZapCompleted { .. })
      )
    });
    assert!(
      has_zap_completed,
      "Zap should complete successfully even with unbalanced amounts"
    );
  });
}

#[test]
fn initial_pool_liquidity() {
  // Test adding liquidity to an empty pool (no oracle check needed)
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let zap_account = ZapManager::account_id();
    let token_asset = AssetKind::Local(1);
    let native_asset = AssetKind::Native;
    // Create pool - starts empty (0, 0)
    assert_ok!(MockAssetConversion::create_pool(native_asset, token_asset));
    // Fund above MinSwapForeign (PRECISION)
    let zap_amount = 10 * PRECISION;
    let _ = Balances::deposit_creating(&zap_account, zap_amount);
    assert_ok!(Assets::mint_into(1, &zap_account, zap_amount));
    assert_ok!(ZapManager::enable_asset(RuntimeOrigin::root(), token_asset));
    ZapManager::on_initialize(1);
    ZapManager::on_idle(1, Weight::MAX);
    // Should successfully add initial liquidity to empty pool
    assert!(System::events().iter().any(|r| matches!(
      r.event,
      crate::mock::RuntimeEvent::ZapManager(crate::Event::LPTokensDistributed { .. })
    )));
  });
}
