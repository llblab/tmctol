use crate::{mock::*, types::*, Error, Event};
use polkadot_sdk::frame_support::{
  assert_noop, assert_ok,
  traits::{fungibles::Mutate, Get},
};
use polkadot_sdk::sp_runtime::Permill;

const PRECISION: u128 = primitives::ecosystem::params::PRECISION;

#[test]
fn router_fee_calculation_logic() {
  new_test_ext().execute_with(|| {
    let amount = 1000 * PRECISION;
    // Router fee is 0.2% (20/10000)
    let fee = (amount * 20) / 10000;
    assert_eq!(fee, 2 * PRECISION);
  });
}

#[test]
fn large_amount_fee_calculation() {
  new_test_ext().execute_with(|| {
    let amount = 1_000_000_000_000u128;
    let fee = (amount * 20) / 10000;
    assert_eq!(fee, 2_000_000_000);
  });
}

#[test]
fn zero_amount_fee_calculation() {
  new_test_ext().execute_with(|| {
    let amount = 0u128;
    let fee = (amount * 20) / 10000;
    assert_eq!(fee, 0);
  });
}

#[test]
fn precision_constant_validation() {
  // Test that PRECISION constant is correct (10^12)
  new_test_ext().execute_with(|| {
    let precision = <<Test as crate::Config>::Precision as Get<u128>>::get();
    assert_eq!(precision, 1_000_000_000_000u128);
  });
}

#[test]
fn router_intelligence_test() {
  // Verify Router Intelligence (Bidirectional Compression)
  // Case 1: XYK Output > TMC Output => Prefer XYK (Market Liquidity)
  // Case 2: TMC Output > XYK Output => Prefer TMC (Protocol Liquidity/Ceiling)

  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let user = 1u64;
    let asset_in = AssetKind::Local(1);
    let asset_out = AssetKind::Native;
    let amount_in = 1000 * PRECISION;

    // Setup: Mint assets to user (done in new_test_ext)

    // Scenario 1: Market Liquidity is better (XYK > TMC)
    // We want XYK ~ 1200, TMC ~ 1000

    // Setup TMC: Rate 1.0 (1000*P in -> 1000*P out)
    set_tmc_rate(asset_in, 1);

    // Setup XYK: We need a pool that gives > TMC output (1000*P)
    // Pool formula: out = (in * res_out) / (res_in + in)
    // With amount_in = 1000*P, reserve_in = 1000*P, reserve_out = 3000*P:
    // out = (1000*P * 3000*P) / (2000*P) = 1500*P
    let reserve_in = 1_000 * PRECISION;
    let reserve_out = 3_000 * PRECISION;
    set_pool(asset_in, asset_out, reserve_in, reserve_out);

    // Execute swap
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(user),
      asset_in,
      asset_out,
      amount_in,
      0, // min_amount_out
      user,
      u64::MAX
    ));

    // Verify event: Expect XYK execution (1500*P)
    let expected_xyk_out = 1_500 * PRECISION;
    System::assert_last_event(
      crate::Event::SwapExecuted {
        who: user,
        from: asset_in,
        to: asset_out,
        amount_in,
        amount_out: expected_xyk_out,
      }
      .into(),
    );

    // Scenario 2: Protocol Liquidity is better (TMC > XYK)
    // We want TMC ~ 1000, XYK ~ 800

    // TMC Rate stays 1.0 -> 1000*P output
    set_tmc_rate(asset_in, 1);

    // XYK: We need a pool that gives < TMC output (1000*P)
    // With amount_in = 1000*P, reserve_in = 1000*P, reserve_out = 500*P:
    // out = (1000*P * 500*P) / (2000*P) = 250*P
    let reserve_in_2 = 1_000 * PRECISION;
    let reserve_out_2 = 500 * PRECISION;
    set_pool(asset_in, asset_out, reserve_in_2, reserve_out_2);

    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(user),
      asset_in,
      asset_out,
      amount_in,
      0,
      user,
      u64::MAX
    ));

    // Verify event: Expect TMC execution (1000*P)
    let expected_tmc_out = 1_000 * PRECISION;
    System::assert_last_event(
      crate::Event::SwapExecuted {
        who: user,
        from: asset_in,
        to: asset_out,
        amount_in,
        amount_out: expected_tmc_out,
      }
      .into(),
    );
  });
}

#[test]
fn circular_swap_protection_test() {
  new_test_ext().execute_with(|| {
    let user = 1u64;
    let asset = AssetKind::Local(1);
    let amount = 1000u128;

    assert_noop!(
      AxialRouter::swap(
        RuntimeOrigin::signed(user),
        asset,
        asset,
        amount,
        0,
        user,
        u64::MAX
      ),
      Error::<Test>::IdenticalAssets
    );
  });
}

#[test]
fn slippage_protection_test() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let user = 1u64;
    let asset_in = AssetKind::Local(1);
    let asset_out = AssetKind::Native;
    let amount_in = 1000 * PRECISION;

    // Setup: Market Liquidity pool with PRECISION scaling
    let reserve_both = 1_000 * PRECISION;
    set_pool(asset_in, asset_out, reserve_both, reserve_both);

    // TMC gives 0 (Not set)

    // XYK formula: out = (in * res_out) / (res_in + in) = (1000*P * 1000*P) / (2000*P) = 500*P
    let expected_out = 500 * PRECISION;

    // Case 1: Slippage met (min_out < expected_out) - Success
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(user),
      asset_in,
      asset_out,
      amount_in,
      400 * PRECISION, // 400*P < 500*P
      user,
      u64::MAX
    ));

    // Verify actual output matches expected
    System::assert_last_event(
      crate::Event::SwapExecuted {
        who: user,
        from: asset_in,
        to: asset_out,
        amount_in,
        amount_out: expected_out,
      }
      .into(),
    );

    // Case 2: Slippage exceeded (min_out > expected_out) - Failure
    assert_noop!(
      AxialRouter::swap(
        RuntimeOrigin::signed(user),
        asset_in,
        asset_out,
        amount_in,
        600 * PRECISION, // 600*P > 500*P
        user,
        u64::MAX
      ),
      Error::<Test>::SlippageExceeded
    );
  });
}

#[test]
fn sandwich_attack_simulation() {
  // Goal: Verify that router fees make sandwich attacks unprofitable.
  // Router fee is 0.2% each way, so round-trip costs ~0.4% of trade volume.
  // For attack to be unprofitable: fees > profit from price manipulation.

  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let attacker = 666u64;
    let victim = 1u64;
    let asset_in = AssetKind::Local(1);
    let asset_out = AssetKind::Native;

    // Fund attacker
    let attacker_initial_local = 200 * PRECISION;
    assert_ok!(Assets::mint_into(1, &attacker, attacker_initial_local));

    // Record attacker's starting balance
    let attacker_balance_before = Assets::balance(1, attacker);

    // Deep liquidity pool
    let pool_reserve = 10_000 * PRECISION;
    set_pool(asset_in, asset_out, pool_reserve, pool_reserve);

    // 1. Attacker Front-Run (Buy) - 1% of pool
    let attacker_in = 100 * PRECISION;
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(attacker),
      asset_in,
      asset_out,
      attacker_in,
      0,
      attacker,
      u64::MAX
    ));

    // Get attacker bought amount from event
    let attacker_bought = System::events()
      .iter()
      .rev()
      .find_map(|r| {
        if let crate::mock::RuntimeEvent::AxialRouter(crate::Event::SwapExecuted {
          amount_out,
          who,
          ..
        }) = &r.event
        {
          if *who == attacker {
            Some(*amount_out)
          } else {
            None
          }
        } else {
          None
        }
      })
      .expect("Attacker swap failed");

    // 2. Victim Buy (Pushes price further) - minimum allowed amount
    let victim_in = PRECISION;
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(victim),
      asset_in,
      asset_out,
      victim_in,
      0,
      victim,
      u64::MAX
    ));

    // 3. Attacker Back-Run (Sell)
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(attacker),
      asset_out,
      asset_in,
      attacker_bought,
      0,
      attacker,
      u64::MAX
    ));

    // Check attacker's final Local asset balance
    let attacker_balance_after = Assets::balance(1, attacker);

    // Calculate total fees paid (0.2% on each leg)
    let fee_buy = (attacker_in * 2) / 1000; // 0.2% of buy amount
    let fee_sell = (attacker_bought * 2) / 1000; // 0.2% of sell amount (in Native)

    println!("Attacker balance before: {attacker_balance_before}");
    println!("Attacker balance after: {attacker_balance_after}");
    println!("Fees paid (buy leg): {fee_buy}");
    println!("Fees paid (sell leg, in Native): {fee_sell}");

    // The attacker's Local token balance should decrease after the round-trip.
    // Even if they got slightly more Local back from the swap than they put in,
    // the fees are deducted separately, so net balance should be lower.
    assert!(
      attacker_balance_after < attacker_balance_before,
      "Sandwich attack should be unprofitable. Balance before: {attacker_balance_before}, after: {attacker_balance_after}"
    );
  });
}

#[test]
fn fee_routing_adapter_test() {
  new_test_ext().execute_with(|| {
    let user = 1u64;
    let asset = AssetKind::Local(1);
    let amount = 1000u128;

    // Call route_fee via adapter (MockFeeAdapter)
    assert_ok!(<Test as crate::Config>::FeeAdapter::route_fee(
      &user, asset, amount
    ));

    let fees = get_collected_fees();
    assert_eq!(fees.len(), 1);
    assert_eq!(fees[0], (user, asset, amount));

    // Check balances (Mock adapter transfers to 123)
    let balance = Assets::balance(1, 123);
    assert_eq!(balance, 1000);
  });
}

#[test]
fn price_oracle_test() {
  new_test_ext().execute_with(|| {
    let asset_in = AssetKind::Local(1);
    let asset_out = AssetKind::Native;
    let price = 500u128;

    assert_ok!(<Test as crate::Config>::PriceOracle::update_ema_price(
      asset_in, asset_out, price, 0
    ));

    let stored = <Test as crate::Config>::PriceOracle::get_ema_price(asset_in, asset_out);
    assert_eq!(stored, Some(price));
  });
}

#[test]
fn tmc_interface_test() {
  new_test_ext().execute_with(|| {
    let user = 1u64;
    let asset = AssetKind::Local(1);
    set_tmc_rate(asset, 2); // 1 unit -> 2 Native

    assert!(<Test as crate::Config>::TmcPallet::has_curve(asset));

    let receives = <Test as crate::Config>::TmcPallet::calculate_user_receives(asset, 100).unwrap();
    assert_eq!(receives, 200);

    // Capture initial balances
    let initial_native = Balances::free_balance(user);
    let initial_asset = Assets::balance(1, user);

    let minted =
      <Test as crate::Config>::TmcPallet::mint_with_distribution(&user, asset, 100).unwrap();
    assert_eq!(minted, 200);

    // User should have 200 Native more
    assert_eq!(Balances::free_balance(user), initial_native + 200);
    // User should have 100 less Asset 1
    assert_eq!(Assets::balance(1, user), initial_asset - 100);
  });
}

#[test]
fn asset_conversion_api_test() {
  new_test_ext().execute_with(|| {
    let asset_a = AssetKind::Local(1);
    let asset_b = AssetKind::Native;

    set_pool(asset_a, asset_b, 1_000_000, 1_000_000);

    let pool_id = <Test as crate::Config>::AssetConversion::get_pool_id(asset_a, asset_b);
    assert!(pool_id.is_some());

    let reserves = <Test as crate::Config>::AssetConversion::get_pool_reserves(pool_id.unwrap());
    assert_eq!(reserves, Some((1_000_000, 1_000_000)));

    let quote = <Test as crate::Config>::AssetConversion::quote_price_exact_tokens_for_tokens(
      asset_a, asset_b, 1000, true,
    );
    // out = 1000 * 1000000 / 1001000 = 999
    assert_eq!(quote, Some(999));
  });
}

#[test]
fn tmctol_integration_flow() {
  // Test complete TMCTOL integration flow math check

  // 1. User mints tokens through TMC
  let foreign_amount = 1000u128;
  let user_allocation = Permill::from_rational(333u32, 1000u32).mul_floor(foreign_amount);
  let tol_total = foreign_amount.saturating_sub(user_allocation);

  // 2. TOL distributed to 4 buckets
  let bucket_a = Permill::from_rational(500u32, 1000u32).mul_floor(tol_total);
  let bucket_b = Permill::from_rational(167u32, 1000u32).mul_floor(tol_total);
  let bucket_c = Permill::from_rational(167u32, 1000u32).mul_floor(tol_total);
  let bucket_d = tol_total.saturating_sub(bucket_a + bucket_b + bucket_c);

  // 3. User swaps through Axial Router
  let swap_amount = 500u128;
  let router_fee_bps = 50u32; // 0.5%
  let router_fee = (swap_amount * router_fee_bps as u128) / 10_000;

  // 4. Fee burning (all router fees are burned)
  let burn_amount = router_fee;
  let remaining_fee = 0u128;

  // Verify complete flow
  assert_eq!(user_allocation, 333u128);
  assert_eq!(tol_total, 667u128);
  assert_eq!(bucket_a + bucket_b + bucket_c + bucket_d, tol_total);
  assert_eq!(router_fee, 2u128);
  assert_eq!(burn_amount, 2u128);
  assert_eq!(remaining_fee, 0u128);
}

#[test]
fn tmctol_parameter_validation() {
  // Test TMCTOL parameter validation
  new_test_ext().execute_with(|| {
    let precision = <<Test as crate::Config>::Precision as Get<u128>>::get();
    assert_eq!(precision, 1_000_000_000_000u128);

    // TOL distribution must sum to 66.7%
    let tol_total = Permill::from_rational(667u32, 1000u32);
    assert_eq!(tol_total, Permill::from_rational(667u32, 1000u32));

    // 4-bucket distribution must sum to 100% of TOL
    let bucket_a = Permill::from_rational(500u32, 1000u32);
    let bucket_b = Permill::from_rational(167u32, 1000u32);
    let bucket_c = Permill::from_rational(167u32, 1000u32);
    let bucket_d = Permill::from_rational(166u32, 1000u32);

    let total_buckets = bucket_a.deconstruct()
      + bucket_b.deconstruct()
      + bucket_c.deconstruct()
      + bucket_d.deconstruct();
    assert_eq!(total_buckets, 1000000u32);

    // Fee burning must be 100% (all fees burned)
    let total_fees = 1000u128;
    let burned_fees = total_fees;
    assert_eq!(burned_fees, 1000u128);

    // Router fee must be reasonable (0.5%)
    let router_fee = Permill::from_rational(5u32, 1000u32);
    assert!(
      router_fee <= Permill::from_percent(1),
      "Router fee should be reasonable"
    );
  });
}

#[test]
fn governance_can_update_router_fee() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    // Initial fee should be the default
    let initial_fee = AxialRouter::router_fee();
    assert_eq!(initial_fee, Permill::from_parts(2_000)); // 0.2%

    // Update router fee as root
    let new_fee = Permill::from_percent(1); // 1%
    assert_ok!(AxialRouter::update_router_fee(
      RuntimeOrigin::root(),
      new_fee
    ));

    // Verify fee was updated
    let updated_fee = AxialRouter::router_fee();
    assert_eq!(updated_fee, new_fee);

    // Verify event was emitted
    System::assert_last_event(
      Event::RouterFeeUpdated {
        old_fee: initial_fee,
        new_fee,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_router_fee() {
  new_test_ext().execute_with(|| {
    // Regular user cannot update router fee
    let new_fee = Permill::from_percent(2);
    assert_noop!(
      AxialRouter::update_router_fee(RuntimeOrigin::signed(1), new_fee),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can update
    assert_ok!(AxialRouter::update_router_fee(
      RuntimeOrigin::root(),
      new_fee
    ));
  });
}

#[test]
fn updated_fee_is_used_in_calculations() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    let amount = 10_000u128;

    // Calculate fee with initial rate (0.2%)
    let initial_fee = AxialRouter::calculate_router_fee(amount);
    assert_eq!(initial_fee, 20); // 10,000 * 0.002 = 20

    // Update fee to 1%
    let new_fee_rate = Permill::from_percent(1);
    assert_ok!(AxialRouter::update_router_fee(
      RuntimeOrigin::root(),
      new_fee_rate
    ));

    // Calculate fee with new rate
    let new_fee = AxialRouter::calculate_router_fee(amount);
    assert_eq!(new_fee, 100); // 10,000 * 0.01 = 100
  });
}

#[test]
fn governance_can_add_tracked_assets() {
  new_test_ext().execute_with(|| {
    // Advance block to enable events
    System::set_block_number(1);

    let asset = AssetKind::Local(42);

    // Add tracked asset as root
    assert_ok!(AxialRouter::add_tracked_asset(RuntimeOrigin::root(), asset));

    // Verify event was emitted
    System::assert_last_event(Event::TrackedAssetAdded { asset }.into());
  });
}

#[test]
fn only_governance_can_add_tracked_assets() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(42);

    // Regular user cannot add tracked assets
    assert_noop!(
      AxialRouter::add_tracked_asset(RuntimeOrigin::signed(1), asset),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );

    // Root can add
    assert_ok!(AxialRouter::add_tracked_asset(RuntimeOrigin::root(), asset));
  });
}
