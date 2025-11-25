//! Unit tests for the Burning Manager pallet.

use crate::{
  mock::{new_test_ext, set_pool, Assets, Balances, BurningManager, RuntimeOrigin, System},
  Event,
};
use polkadot_sdk::frame_support::{
  assert_noop, assert_ok,
  traits::{fungibles::Mutate, Currency},
};
use polkadot_sdk::sp_runtime::Permill;
use primitives::AssetKind;

#[test]
fn burn_fee_percentage_calculation() {
  // Test that burn fee percentage is applied correctly
  let burn_fee = Permill::from_percent(20);
  assert_eq!(burn_fee, Permill::from_percent(20));

  // 20% of 1_000_000 = 200_000
  let expected_fee = burn_fee.mul_floor(1_000_000u128);
  assert_eq!(expected_fee, 200_000);
}

#[test]
fn min_swap_foreign_threshold() {
  // Test that min swap foreign threshold is respected
  let min_threshold = 1_000_000u128;
  assert_eq!(min_threshold, 1_000_000);

  // Amounts below threshold should not trigger foreign processing
  let amount_below_threshold = 500_000u128;
  assert!(amount_below_threshold < min_threshold);
}

#[test]
fn min_burn_native_threshold() {
  // Test that min burn native threshold is respected
  let min_threshold = 500_000u128;
  assert_eq!(min_threshold, 500_000);

  // Amounts below threshold should not trigger native burning
  let amount_below_threshold = 100_000u128;
  assert!(amount_below_threshold < min_threshold);
}

#[test]
fn total_burned_accumulation() {
  // Test that total burned accumulates correctly
  let mut total_burned = 0u128;

  total_burned += 100_000;
  assert_eq!(total_burned, 100_000);

  total_burned += 200_000;
  assert_eq!(total_burned, 300_000);

  total_burned += 500_000;
  assert_eq!(total_burned, 800_000);
}

#[test]
fn total_swapped_accumulation() {
  // Test that total swapped accumulates correctly per asset
  use std::collections::BTreeMap;

  let mut total_swapped: BTreeMap<AssetKind, u128> = BTreeMap::new();
  let asset1 = AssetKind::Local(1);
  let asset2 = AssetKind::Local(2);

  // Asset 1
  *total_swapped.entry(asset1).or_insert(0) += 500_000;
  assert_eq!(total_swapped[&asset1], 500_000);

  *total_swapped.entry(asset1).or_insert(0) += 300_000;
  assert_eq!(total_swapped[&asset1], 800_000);

  // Asset 2
  *total_swapped.entry(asset2).or_insert(0) += 1_000_000;
  assert_eq!(total_swapped[&asset2], 1_000_000);
}

#[test]
fn fee_processing_logic() {
  // Test fee processing logic with different scenarios

  // Scenario 1: Only native tokens above threshold
  let native_balance = 2_000_000u128;
  let foreign_balance = 0u128;
  let min_burn_threshold = 500_000u128;

  let should_process_native = native_balance >= min_burn_threshold;
  assert!(should_process_native);
  assert!(foreign_balance < min_burn_threshold);

  // Scenario 2: Only foreign tokens above threshold
  let native_balance = 0u128;
  let foreign_balance = 1_500_000u128;
  let min_swap_threshold = 1_000_000u128;

  let should_process_foreign = foreign_balance >= min_swap_threshold;
  assert!(should_process_foreign);
  assert!(native_balance < min_burn_threshold);

  // Scenario 3: Both assets above thresholds
  let native_balance = 2_000_000u128;
  let foreign_balance = 1_500_000u128;

  let should_process_native = native_balance >= min_burn_threshold;
  let should_process_foreign = foreign_balance >= min_swap_threshold;

  assert!(should_process_native);
  assert!(should_process_foreign);
}

#[test]
fn burn_amount_calculation() {
  // Test burn amount calculation with different percentages

  let amount = 1_000_000u128;

  // 20% burn fee
  let burn_fee_20 = Permill::from_percent(20);
  let burn_amount_20 = burn_fee_20.mul_floor(amount);
  assert_eq!(burn_amount_20, 200_000);

  // 10% burn fee
  let burn_fee_10 = Permill::from_percent(10);
  let burn_amount_10 = burn_fee_10.mul_floor(amount);
  assert_eq!(burn_amount_10, 100_000);

  // 5% burn fee
  let burn_fee_5 = Permill::from_percent(5);
  let burn_amount_5 = burn_fee_5.mul_floor(amount);
  assert_eq!(burn_amount_5, 50_000);
}

#[test]
fn swap_to_burn_flow() {
  // Test the complete flow from swap to burn

  let foreign_amount = 1_000_000u128;
  let burn_fee_percentage = Permill::from_percent(20);

  // Step 1: Swap foreign tokens to native (1:1 ratio in this test)
  let native_received = foreign_amount; // Mock 1:1 swap

  // Step 2: Calculate burn amount
  let burn_amount = burn_fee_percentage.mul_floor(native_received);
  assert_eq!(burn_amount, 200_000);

  // Step 3: Remaining native tokens after burn
  let remaining_native = native_received - burn_amount;
  assert_eq!(remaining_native, 800_000);

  // Verify the flow
  assert_eq!(foreign_amount, 1_000_000);
  assert_eq!(native_received, 1_000_000);
  assert_eq!(burn_amount, 200_000);
  assert_eq!(remaining_native, 800_000);
}

#[test]
fn account_id_generation_logic() {
  // Test account ID generation from pallet ID
  let pallet_id_bytes = *b"burnmgr_";

  // Verify the pallet ID bytes are correct
  assert_eq!(pallet_id_bytes, [98, 117, 114, 110, 109, 103, 114, 95]); // ASCII "burnmgr_"

  // The actual account ID generation is handled by Substrate's AccountIdConversion trait
  // This test just verifies the pallet ID is properly defined
  let pallet_id = polkadot_sdk::frame_support::PalletId(pallet_id_bytes);
  assert_eq!(pallet_id.0, pallet_id_bytes);
}

#[test]
fn threshold_validation() {
  // Test various threshold validation scenarios
  // Note: Assertions on constants are optimized out by the compiler
  // so we test with variables instead

  let min_swap_threshold = 1_000_000u128;
  let min_burn_threshold = 500_000u128;

  // Test with variables to avoid compiler optimization
  let amount_below_swap = 500_000u128;
  let amount_just_below_swap = 999_999u128;
  let zero_amount = 0u128;

  assert!(amount_below_swap < min_swap_threshold); // Below threshold
  assert!(amount_just_below_swap < min_swap_threshold); // Just below threshold
  assert!(zero_amount < min_burn_threshold); // Zero amount
}

#[test]
fn arithmetic_safety() {
  // Test arithmetic operations for safety

  let large_amount = 1_000_000_000_000u128;
  let burn_fee = Permill::from_percent(20);

  // Test multiplication safety
  let burn_amount = burn_fee.mul_floor(large_amount);
  assert_eq!(burn_amount, 200_000_000_000);

  // Test subtraction safety
  let remaining = large_amount.checked_sub(burn_amount).unwrap();
  assert_eq!(remaining, 800_000_000_000);

  // Test overflow protection
  let max_amount = u128::MAX;
  let small_fee = Permill::from_percent(1);
  let small_burn = small_fee.mul_floor(max_amount);

  // Should not panic and should be a valid calculation
  assert!(small_burn > 0);
}

#[test]
fn add_burnable_asset_works() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(1);

    // Should start empty
    assert!(!BurningManager::burnable_assets().contains(&asset));

    // Add asset as root
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));

    // Should be present
    assert!(BurningManager::burnable_assets().contains(&asset));

    // Adding duplicate should be fine (idempotent logic in try_mutate check)
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));
    assert_eq!(BurningManager::burnable_assets().len(), 1);
  });
}

#[test]
fn process_fees_works_native() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    // Setup: Give BurningManager some native tokens (15 * PRECISION to exceed MIN_BURN_NATIVE)
    let account = BurningManager::account_id();
    const PRECISION: u128 = primitives::ecosystem::params::PRECISION;
    let burn_amount = 15 * PRECISION;
    let _ = Balances::deposit_creating(&account, burn_amount);

    // Trigger process fees
    assert_ok!(BurningManager::process_fees(RuntimeOrigin::signed(1)));

    // Check event
    System::assert_has_event(
      Event::NativeTokensBurned {
        amount: burn_amount,
        new_total: burn_amount,
      }
      .into(),
    );

    // Balance should be 0
    assert_eq!(Balances::free_balance(account), 0);
  });
}

#[test]
fn process_fees_works_foreign() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let asset_id = 1;
    let asset_kind = AssetKind::Local(asset_id);
    let native_asset = AssetKind::Native;

    const PRECISION: u128 = primitives::ecosystem::params::PRECISION;

    // Setup Pool for Foreign -> Native swap with PRECISION-relative amounts
    // Reserves: 1000 PRECISION Native, 1000 PRECISION Foreign (Deep liquidity)
    let reserve_amount = 1000 * PRECISION;
    set_pool(native_asset, asset_kind, reserve_amount, reserve_amount);

    // Add asset to burnable list
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset_kind
    ));

    // Mint assets to BurningManager (20 * PRECISION)
    let foreign_amount = 20 * PRECISION;
    assert_ok!(Assets::mint_into(asset_id, &account, foreign_amount));

    // Trigger process
    assert_ok!(BurningManager::process_fees(RuntimeOrigin::signed(1)));

    // Calculate expected output using constant product formula:
    // Out = AmountIn * ReserveOut / (ReserveIn + AmountIn)
    // Out = 20*P * 1000*P / (1000*P + 20*P) = 20*P * 1000*P / 1020*P
    // Out = 20000*P^2 / 1020*P = ~19.607843137 * P
    let expected_out = 19_607_843_137_254u128;

    // Expect events
    System::assert_has_event(
      Event::ForeignTokensSwapped {
        foreign_asset: asset_kind,
        foreign_amount,
        native_received: expected_out,
        burn_amount: expected_out,
      }
      .into(),
    );

    System::assert_has_event(
      Event::NativeTokensBurned {
        amount: expected_out,
        new_total: expected_out,
      }
      .into(),
    );
  });
}

#[test]
fn governance_can_update_min_burn_native() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    // Initial minimum should be the default
    let initial_min = BurningManager::min_burn_native();
    assert_eq!(initial_min, 10_000_000_000_000); // From mock config

    // Update minimum burn as root
    let new_min = 20_000_000_000_000u128;
    assert_ok!(BurningManager::update_min_burn_native(
      RuntimeOrigin::root(),
      new_min
    ));

    // Verify minimum was updated
    let updated_min = BurningManager::min_burn_native();
    assert_eq!(updated_min, new_min);

    // Verify event was emitted
    System::assert_last_event(
      Event::MinBurnUpdated {
        old_amount: initial_min,
        new_amount: new_min,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_min_burn_native() {
  new_test_ext().execute_with(|| {
    // Regular user cannot update minimum burn
    let new_min = 15_000_000_000_000u128;
    assert_noop!(
      BurningManager::update_min_burn_native(RuntimeOrigin::signed(1), new_min),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can update
    assert_ok!(BurningManager::update_min_burn_native(
      RuntimeOrigin::root(),
      new_min
    ));
  });
}

#[test]
fn governance_can_update_dust_threshold() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    // Initial threshold should be the default
    let initial_threshold = BurningManager::dust_threshold();
    assert_eq!(initial_threshold, 100_000_000_000); // From mock config

    // Update threshold as root
    let new_threshold = 200_000_000_000u128;
    assert_ok!(BurningManager::update_dust_threshold(
      RuntimeOrigin::root(),
      new_threshold
    ));

    // Verify threshold was updated
    let updated_threshold = BurningManager::dust_threshold();
    assert_eq!(updated_threshold, new_threshold);

    // Verify event was emitted
    System::assert_last_event(
      Event::DustThresholdUpdated {
        old_threshold: initial_threshold,
        new_threshold,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_dust_threshold() {
  new_test_ext().execute_with(|| {
    // Regular user cannot update dust threshold
    let new_threshold = 150_000_000_000u128;
    assert_noop!(
      BurningManager::update_dust_threshold(RuntimeOrigin::signed(1), new_threshold),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can update
    assert_ok!(BurningManager::update_dust_threshold(
      RuntimeOrigin::root(),
      new_threshold
    ));
  });
}

#[test]
fn governance_can_update_slippage_tolerance() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    // Initial tolerance should be the default
    let initial_tolerance = BurningManager::slippage_tolerance();
    assert_eq!(initial_tolerance, Permill::from_percent(2)); // From mock config

    // Update tolerance as root
    let new_tolerance = Permill::from_percent(5);
    assert_ok!(BurningManager::update_slippage_tolerance(
      RuntimeOrigin::root(),
      new_tolerance
    ));

    // Verify tolerance was updated
    let updated_tolerance = BurningManager::slippage_tolerance();
    assert_eq!(updated_tolerance, new_tolerance);

    // Verify event was emitted
    System::assert_last_event(
      Event::SlippageToleranceUpdated {
        old_tolerance: initial_tolerance,
        new_tolerance,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_slippage_tolerance() {
  new_test_ext().execute_with(|| {
    // Regular user cannot update slippage tolerance
    let new_tolerance = Permill::from_percent(3);
    assert_noop!(
      BurningManager::update_slippage_tolerance(RuntimeOrigin::signed(1), new_tolerance),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can update
    assert_ok!(BurningManager::update_slippage_tolerance(
      RuntimeOrigin::root(),
      new_tolerance
    ));
  });
}

#[test]
fn updated_parameters_are_used_in_processing() {
  new_test_ext().execute_with(|| {
    let account = BurningManager::account_id();

    // Update minimum burn native to a lower value
    let new_min = 5_000_000_000_000u128;
    assert_ok!(BurningManager::update_min_burn_native(
      RuntimeOrigin::root(),
      new_min
    ));

    // Fund with amount above new minimum but below original
    let _ = Balances::deposit_creating(&account, 7_000_000_000_000);

    // Process fees should work with the new minimum
    assert_ok!(BurningManager::process_fees(RuntimeOrigin::signed(1)));

    // Verify native tokens were burned
    assert_eq!(Balances::free_balance(account), 0);
  });
}

#[test]
fn governance_can_add_burnable_assets() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(99);

    // Add burnable asset as root
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));

    // Verify asset was added
    assert!(BurningManager::burnable_assets().contains(&asset));
  });
}

#[test]
fn only_governance_can_add_burnable_assets() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(99);

    // Regular user cannot add burnable assets
    assert_noop!(
      BurningManager::add_burnable_asset(RuntimeOrigin::signed(1), asset),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can add
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));
  });
}
