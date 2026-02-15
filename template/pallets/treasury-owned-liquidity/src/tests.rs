//! Unit tests for the Treasury Owned Liquidity pallet.

use crate::{BucketAllocation, DomainEnsureAction, mock::*};
use polkadot_sdk::frame_support::{assert_noop, assert_ok, traits::Get};
use primitives::{AssetKind, ecosystem::params::PRECISION};

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
    // Initialize buckets with some values
    let bucket_alloc = BucketAllocation {
      target_allocation_ppm: 250_000,
      native_reserves: 1000,
      foreign_reserves: 1000,
      lp_tokens: 1000,
    };
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      AssetKind::Local(1),
      AssetKind::Native,
      1_000_000
    ));
    crate::BucketA::<Test>::insert(0, bucket_alloc.clone());
    crate::BucketB::<Test>::insert(0, bucket_alloc.clone());
    crate::BucketC::<Test>::insert(0, bucket_alloc.clone());
    crate::BucketD::<Test>::insert(0, bucket_alloc.clone());
    let (total_native, total_foreign) = TreasuryOwnedLiquidity::get_total_tol_reserves();
    assert_eq!(total_native, 4000);
    assert_eq!(total_foreign, 4000);
  });
}

#[test]
fn reserves_are_tracked_when_lp_tokens_are_distributed() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let lp_amount = 1_000_000u128;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    assert_ok!(Assets::mint_into(
      TEST_LP_ASSET_ID,
      &treasury,
      lp_amount + 1_000
    ));

    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      AssetKind::Local(TEST_LP_ASSET_ID),
      lp_amount
    ));

    let (native_total, foreign_total) = TreasuryOwnedLiquidity::get_total_tol_reserves();
    assert!(native_total > 0);
    assert!(foreign_total > 0);
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
      0,
      token_asset,
      foreign_asset,
      1_000_000
    ));
    // Valid update
    assert_ok!(TreasuryOwnedLiquidity::update_bucket_allocation(
      RuntimeOrigin::root(),
      0,
      0,       // Bucket A
      500_000  // 50%
    ));
    let bucket_a = TreasuryOwnedLiquidity::bucket_a(0);
    assert_eq!(bucket_a.target_allocation_ppm, 500_000);
  });
}

#[test]
fn updated_bucket_allocation_is_used_for_distribution() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let lp_amount = 1_000_000u128;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    // 40 / 30 / 20 / 10 split (Bucket D receives remainder).
    assert_ok!(TreasuryOwnedLiquidity::update_bucket_allocation(
      RuntimeOrigin::root(),
      0,
      0,
      400_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::update_bucket_allocation(
      RuntimeOrigin::root(),
      0,
      1,
      300_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::update_bucket_allocation(
      RuntimeOrigin::root(),
      0,
      2,
      200_000
    ));

    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    assert_ok!(Assets::mint_into(
      TEST_LP_ASSET_ID,
      &treasury,
      lp_amount + 1_000
    ));

    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      AssetKind::Local(TEST_LP_ASSET_ID),
      lp_amount
    ));

    assert_eq!(TreasuryOwnedLiquidity::bucket_a(0).lp_tokens, 400_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_b(0).lp_tokens, 300_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_c(0).lp_tokens, 200_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_d(0).lp_tokens, 100_000);
  });
}

#[test]
fn bucket_a_unwind_is_disabled() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_noop!(
      TreasuryOwnedLiquidity::unwind_bucket_liquidity(
        RuntimeOrigin::root(),
        0,
        AssetKind::Local(TEST_LP_ASSET_ID),
        1,
        42
      ),
      crate::Error::<Test>::BucketAUnwindDisabled
    );
  });
}

#[test]
fn governance_can_unwind_bucket_c_liquidity_manually() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let lp_amount = 1_000_000u128;
    let unwind_amount = 100_000u128;
    let destination = 42u64;
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    assert_ok!(Assets::mint_into(
      TEST_LP_ASSET_ID,
      &treasury,
      lp_amount + 1_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      AssetKind::Local(TEST_LP_ASSET_ID),
      lp_amount
    ));
    let native_before = Balances::free_balance(destination);
    let foreign_before = Assets::balance(1, destination);
    assert_ok!(TreasuryOwnedLiquidity::unwind_bucket_liquidity(
      RuntimeOrigin::root(),
      2,
      AssetKind::Local(TEST_LP_ASSET_ID),
      unwind_amount,
      destination
    ));
    assert_eq!(
      TreasuryOwnedLiquidity::bucket_c(0).lp_tokens,
      150_000,
      "Bucket C should lose the requested LP amount"
    );
    assert!(
      Balances::free_balance(destination) > native_before,
      "Destination should receive native from LP unwind"
    );
    assert!(
      Assets::balance(1, destination) > foreign_before,
      "Destination should receive foreign/local asset from LP unwind"
    );
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
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    // Need to mint LP tokens to treasury first so they can be distributed
    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    // Mint extra to ensure account stays alive (Preservation::Preserve)
    assert_ok!(Assets::mint_into(
      TEST_LP_ASSET_ID,
      &treasury,
      lp_amount + 1000
    ));
    // Buckets configured to 25% each in mock
    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      AssetKind::Local(TEST_LP_ASSET_ID),
      lp_amount
    ));
    // Check balances
    assert_eq!(TreasuryOwnedLiquidity::bucket_a(0).lp_tokens, 250_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_b(0).lp_tokens, 250_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_c(0).lp_tokens, 250_000);
    assert_eq!(TreasuryOwnedLiquidity::bucket_d(0).lp_tokens, 250_000);
  });
}

#[test]
fn zero_values_handling() {
  // Test handling of zero values
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(
      AssetKind::Local(TEST_LP_ASSET_ID),
      0
    ));
    assert_eq!(TreasuryOwnedLiquidity::bucket_a(0).lp_tokens, 0);
  });
}

#[test]
fn non_lp_assets_are_rejected_for_lp_distribution() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    let treasury = TreasuryOwnedLiquidity::account_id();
    use polkadot_sdk::frame_support::traits::fungibles::Mutate;
    assert_ok!(Assets::mint_into(1, &treasury, 1_000_000));

    assert_noop!(
      TreasuryOwnedLiquidity::distribute_lp_tokens_to_buckets(AssetKind::Local(1), 100_000),
      crate::Error::<Test>::InvalidAsset
    );
  });
}

#[test]
fn ensure_domain_for_token_rejects_lp_asset_class() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);

    assert_noop!(
      TreasuryOwnedLiquidity::ensure_domain_for_token(
        AssetKind::Local(TEST_LP_ASSET_ID),
        AssetKind::Native,
        1_000_000
      ),
      crate::Error::<Test>::InvalidAsset
    );
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
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      AssetKind::Local(1),
      AssetKind::Native,
      1_000_000
    ));
    let native = 100u128;
    let foreign = 200u128;
    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(native, foreign));
    let buffer = TreasuryOwnedLiquidity::zap_buffer(0);
    assert_eq!(buffer.pending_native, 100);
    assert_eq!(buffer.pending_foreign, 200);
    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(native, foreign));
    let buffer = TreasuryOwnedLiquidity::zap_buffer(0);
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
      0,
      token_asset,
      foreign_asset,
      2 * PRECISION
    ));
    // Should trigger zap logic if threshold met (logic in on_initialize or manual trigger)
    // Here we just test buffer state with PRECISION-relative amounts
    assert_ok!(TreasuryOwnedLiquidity::add_to_zap_buffer(
      2 * PRECISION,
      2 * PRECISION
    ));
    // Check buffer state directly
    let buffer = TreasuryOwnedLiquidity::zap_buffer(0);
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
      0,
      token_asset,
      foreign_asset,
      1_000_000
    ));
    let config = TreasuryOwnedLiquidity::tol_configuration(0).unwrap();
    assert_eq!(config.token_asset, token_asset);
    assert_eq!(config.foreign_asset, foreign_asset);
    assert_eq!(config.total_tol_allocation, 1_000_000);
  });
}

#[test]
fn ensure_domain_for_token_reports_created_noop_and_rebound() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let token_asset = AssetKind::Local(11);

    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_eq!(
      TreasuryOwnedLiquidity::token_tol_binding(token_asset),
      Some(11)
    );
    let events = System::events();
    assert!(
      events.iter().any(|r| {
        matches!(
          r.event,
          RuntimeEvent::TreasuryOwnedLiquidity(crate::Event::TokenDomainEnsured {
            token_asset: e_token,
            tol_id,
            action: DomainEnsureAction::Created,
            previous_foreign_asset: None,
            foreign_asset: AssetKind::Native,
          }) if e_token == token_asset && tol_id == 11
        )
      }),
      "events: {:?}",
      events
    );

    System::reset_events();
    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert!(System::events().iter().any(|r| {
      matches!(
        r.event,
        RuntimeEvent::TreasuryOwnedLiquidity(crate::Event::TokenDomainEnsured {
          token_asset: e_token,
          tol_id,
          action: DomainEnsureAction::Noop,
          previous_foreign_asset: Some(AssetKind::Native),
          foreign_asset: AssetKind::Native,
        }) if e_token == token_asset && tol_id == 11
      )
    }));

    System::reset_events();
    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      token_asset,
      AssetKind::Foreign(77),
      1_000_000
    ));
    let updated = TreasuryOwnedLiquidity::tol_configuration(11).unwrap();
    assert_eq!(updated.foreign_asset, AssetKind::Foreign(77));
    assert!(System::events().iter().any(|r| {
      matches!(
        r.event,
        RuntimeEvent::TreasuryOwnedLiquidity(crate::Event::TokenDomainEnsured {
          token_asset: e_token,
          tol_id,
          action: DomainEnsureAction::Rebound,
          previous_foreign_asset: Some(AssetKind::Native),
          foreign_asset: AssetKind::Foreign(77),
        }) if e_token == token_asset && tol_id == 11
      )
    }));
  });
}

#[test]
fn ensure_domain_for_token_respects_manual_binding_override() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let token_asset = AssetKind::Local(2);
    let custom_tol_id = 99u32;

    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      custom_tol_id,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      token_asset,
      custom_tol_id
    ));

    System::reset_events();
    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      token_asset,
      AssetKind::Foreign(42),
      1_000_000
    ));

    assert_eq!(
      TreasuryOwnedLiquidity::token_tol_binding(token_asset),
      Some(custom_tol_id)
    );
    assert_eq!(
      TreasuryOwnedLiquidity::tol_configuration(custom_tol_id)
        .expect("custom domain exists")
        .foreign_asset,
      AssetKind::Foreign(42)
    );
    assert_eq!(
      TreasuryOwnedLiquidity::tol_configuration(2)
        .expect("default domain exists")
        .foreign_asset,
      AssetKind::Native
    );
    let events = System::events();
    assert!(
      events.iter().any(|r| {
        matches!(
          r.event,
          RuntimeEvent::TreasuryOwnedLiquidity(crate::Event::TokenDomainEnsured {
            token_asset: e_token,
            tol_id,
            action: DomainEnsureAction::Rebound,
            previous_foreign_asset: Some(AssetKind::Native),
            foreign_asset: AssetKind::Foreign(42),
          }) if e_token == token_asset && tol_id == custom_tol_id
        )
      }),
      "events: {:?}",
      events
    );
  });
}

#[test]
fn max_tol_domains_capacity_blocks_new_domain_creation_but_allows_existing_domain_hardening() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);

    let max_domains = <Test as crate::Config>::MaxTolDomains::get();
    for i in 0..max_domains {
      let tol_id = i + 1;
      let token_asset = AssetKind::Local(1_000 + tol_id);
      assert_ok!(TreasuryOwnedLiquidity::create_tol(
        RuntimeOrigin::root(),
        tol_id,
        token_asset,
        AssetKind::Native,
        1_000_000
      ));
    }

    assert_eq!(
      TreasuryOwnedLiquidity::active_tol_domains().len() as u32,
      max_domains
    );

    let overflow_tol_id = max_domains + 100;
    let overflow_token = AssetKind::Local(9_999_000 + overflow_tol_id);
    assert_noop!(
      TreasuryOwnedLiquidity::create_tol(
        RuntimeOrigin::root(),
        overflow_tol_id,
        overflow_token,
        AssetKind::Native,
        1_000_000
      ),
      crate::Error::<Test>::TooManyTolDomains
    );
    assert!(
      TreasuryOwnedLiquidity::tol_configuration(overflow_tol_id).is_none(),
      "failed create_tol must not persist partial domain state"
    );

    let existing_token = AssetKind::Local(1_001);
    assert_ok!(TreasuryOwnedLiquidity::ensure_domain_for_token(
      existing_token,
      AssetKind::Foreign(77),
      1_000_000
    ));

    assert_noop!(
      TreasuryOwnedLiquidity::ensure_domain_for_token(
        AssetKind::Local(123_456),
        AssetKind::Native,
        1_000_000
      ),
      crate::Error::<Test>::TooManyTolDomains
    );
  });
}

#[test]
fn governance_can_bind_token_to_tol_domain() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let tol_id = 7u32;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      tol_id,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      token_asset,
      tol_id
    ));

    assert_eq!(
      TreasuryOwnedLiquidity::token_tol_binding(token_asset),
      Some(tol_id)
    );

    let bound_ingress = TreasuryOwnedLiquidity::ingress_account_for_tol_id(tol_id);
    assert_eq!(
      TreasuryOwnedLiquidity::ingress_account_for_token(token_asset),
      bound_ingress
    );
  });
}

#[test]
fn lp_ingress_resolution_uses_bound_token_domain() {
  new_test_ext().execute_with(|| {
    let token_asset = AssetKind::Local(1);
    let tol_id = 9u32;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      tol_id,
      token_asset,
      AssetKind::Native,
      1_000_000
    ));

    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      token_asset,
      tol_id
    ));

    let resolved =
      TreasuryOwnedLiquidity::ingress_account_for_lp_asset(AssetKind::Local(TEST_LP_ASSET_ID));
    assert_eq!(
      resolved,
      Some(TreasuryOwnedLiquidity::ingress_account_for_tol_id(tol_id))
    );
  });
}
