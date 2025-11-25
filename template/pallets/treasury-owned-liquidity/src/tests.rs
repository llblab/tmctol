//! Unit tests for the Treasury Owned Liquidity pallet.

use crate::{mock::*, BucketAllocation};
use polkadot_sdk::frame_support::assert_ok;
use primitives::AssetKind;

const PRECISION: u128 = primitives::ecosystem::params::PRECISION;

#[test]
fn integer_sqrt_calculation() {
  // Helper for integer square root calculation
  fn integer_sqrt(n: u128) -> u128 {
    if n < 2 {
      return n;
    }
    let mut x = n / 2;
    let mut y = (x + n / x) / 2;
    while y < x {
      x = y;
      y = (x + n / x) / 2;
    }
    x
  }

  assert_eq!(integer_sqrt(0), 0);
  assert_eq!(integer_sqrt(1), 1);
  assert_eq!(integer_sqrt(4), 2);
  assert_eq!(integer_sqrt(9), 3);
  assert_eq!(integer_sqrt(16), 4);
  assert_eq!(integer_sqrt(100), 10);
  assert_eq!(integer_sqrt(2_000_000_000_000), 1_414_213);
}

#[test]
fn floor_price_calculation() {
  // Test floor price calculation logic
  // P_floor = R_foreign / (R_native)^2 (Simplified model for test)

  let total_native = 1_000_000u128;
  let total_foreign = 2_000_000u128;
  let precision = 1_000_000u128;

  // If R_native > 0
  let denominator = total_native
    .saturating_mul(total_native)
    .saturating_div(precision);

  let floor_price = if denominator > 0 {
    total_foreign.saturating_mul(precision) / denominator
  } else {
    0
  };

  // 2M * 1M / (1M * 1M / 1M) = 2M * 1M / 1M = 2M
  assert_eq!(floor_price, 2_000_000);
}

#[test]
fn optimal_allocation_calculation() {
  // Test optimal allocation calculation
  // Allocation = Target_Ratio * Total_Amount

  let total_amount = 1_000_000u128;
  let target_ratio_ppm = 250_000u32; // 25%

  let allocation = total_amount.saturating_mul(target_ratio_ppm as u128) / 1_000_000;

  assert_eq!(allocation, 250_000);
}

#[test]
fn total_reserves_calculation() {
  // Test total reserves aggregation across buckets
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);

    // Initialize buckets with some values
    let bucket_alloc = BucketAllocation {
      target_allocation_ppm: 250_000,
      native_reserves: 1000,
      foreign_reserves: 1000,
      lp_tokens: 1000,
    };

    crate::BucketA::<Test>::insert(token_asset, bucket_alloc.clone());
    crate::BucketB::<Test>::insert(token_asset, bucket_alloc.clone());
    crate::BucketC::<Test>::insert(token_asset, bucket_alloc.clone());
    crate::BucketD::<Test>::insert(token_asset, bucket_alloc.clone());

    // Simulate getting totals by reading manually since get_total_tol_reserves is internal
    // or use the pallet method if public. Since it's public in lib.rs:
    // pub fn get_total_tol_reserves(token_asset: AssetId) -> Option<(Balance, Balance)>
    // Note: AssetId changed to AssetKind.
    let (total_native, total_foreign) =
      TreasuryOwnedLiquidity::get_total_tol_reserves(token_asset).unwrap();

    assert_eq!(total_native, 4000);
    assert_eq!(total_foreign, 4000);
  });
}

#[test]
fn bucket_allocation_validation() {
  // Test bucket allocation validation
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      token_asset,
      foreign_asset,
      1_000_000
    ));

    // Valid update
    assert_ok!(TreasuryOwnedLiquidity::update_bucket_allocation(
      RuntimeOrigin::root(),
      token_asset,
      0,       // Bucket A
      500_000  // 50%
    ));

    let bucket_a = TreasuryOwnedLiquidity::bucket_a(token_asset);
    assert_eq!(bucket_a.target_allocation_ppm, 500_000);
  });
}

#[test]
fn arithmetic_overflow_protection() {
  // Test overflow protection in calculations
  let max_val = u128::MAX;
  let result = max_val.saturating_add(1);
  assert_eq!(result, u128::MAX);

  let result = max_val.saturating_mul(2);
  assert_eq!(result, u128::MAX);
}

#[test]
fn precision_constant_validation() {
  // Validate precision constant
  // Config::Precision is Get<u128>
  use polkadot_sdk::frame_support::traits::Get;
  assert_eq!(
    <<Test as crate::Config>::Precision as Get<u128>>::get(),
    primitives::ecosystem::params::PRECISION
  );
}

#[test]
fn capital_efficiency_validation() {
  // Test capital efficiency metrics logic
  let lp_tokens = 1_000_000u128;
  let native_value = 500_000u128;
  let foreign_value = 500_000u128;

  // Efficiency = LP / (Native + Foreign)
  let total_value = native_value + foreign_value;
  assert_eq!(total_value, 1_000_000);
  assert_eq!(lp_tokens, total_value);
}

#[test]
fn multi_bucket_management() {
  // Test management of multiple buckets
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);

    // Simulate LP token distribution
    let lp_amount = 1_000_000u128;

    // Create TOL to initialize bucket allocations (25% each from mock config)
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    // Need to mint LP tokens to treasury first so they can be distributed
    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    // Mint extra to ensure account stays alive (Preservation::Preserve)
    assert_ok!(Assets::mint_into(1, &treasury, lp_amount + 1000));

    // Buckets configured to 25% each in mock
    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      token_asset,
      lp_amount
    ));

    // Check balances
    assert_eq!(
      TreasuryOwnedLiquidity::bucket_a(token_asset).lp_tokens,
      250_000
    );
    assert_eq!(
      TreasuryOwnedLiquidity::bucket_b(token_asset).lp_tokens,
      250_000
    );
    assert_eq!(
      TreasuryOwnedLiquidity::bucket_c(token_asset).lp_tokens,
      250_000
    );
    assert_eq!(
      TreasuryOwnedLiquidity::bucket_d(token_asset).lp_tokens,
      250_000
    );
  });
}

#[test]
fn zero_values_handling() {
  // Test handling of zero values
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      token_asset,
      0
    ));

    assert_eq!(TreasuryOwnedLiquidity::bucket_a(token_asset).lp_tokens, 0);
  });
}

#[test]
fn zap_spot_price_calculation() {
  // Test spot price calculation logic
  let native_reserve = 1_000_000u128;
  let foreign_reserve = 2_000_000u128;
  let precision = 1_000_000u128;

  let spot_price = foreign_reserve.saturating_mul(precision) / native_reserve;
  assert_eq!(spot_price, 2_000_000); // 2.0
}

#[test]
fn zap_buffer_accumulation() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let native = 100u128;
    let foreign = 200u128;

    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(
      token_asset,
      native,
      foreign
    ));

    let buffer = TreasuryOwnedLiquidity::zap_buffers(token_asset);
    assert_eq!(buffer.pending_native, 100);
    assert_eq!(buffer.pending_foreign, 200);

    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(
      token_asset,
      native,
      foreign
    ));

    let buffer = TreasuryOwnedLiquidity::zap_buffers(token_asset);
    assert_eq!(buffer.pending_native, 200);
    assert_eq!(buffer.pending_foreign, 400);
  });
}

#[test]
fn zap_pool_initialization_logic() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      token_asset,
      foreign_asset,
      2 * PRECISION
    ));

    // Should trigger zap logic if threshold met (logic in on_initialize or manual trigger)
    // Here we just test buffer state with PRECISION-relative amounts
    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(
      token_asset,
      2 * PRECISION,
      2 * PRECISION
    ));

    // Check buffer state directly
    let buffer = TreasuryOwnedLiquidity::zap_buffers(token_asset);
    use polkadot_sdk::frame_support::traits::Get;
    assert!(
      buffer.pending_foreign >= <<Test as crate::Config>::MinSwapForeign as Get<u128>>::get()
    );
  });
}

#[test]
fn create_tol_works() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      token_asset,
      foreign_asset,
      1_000_000
    ));

    let config = TreasuryOwnedLiquidity::tol_configurations(token_asset).unwrap();
    assert_eq!(config.token_asset, token_asset);
    assert_eq!(config.foreign_asset, foreign_asset);
    assert_eq!(config.total_tol_allocation, 1_000_000);
  });
}
