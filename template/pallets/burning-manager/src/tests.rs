//! Unit tests for the Burning Manager pallet.

use crate::{
  Event,
  mock::{
    Assets, Balances, BurningManager, RuntimeOrigin, System, new_test_ext, register_lp_token,
    set_oracle_price, set_pool,
  },
};
use polkadot_sdk::frame_support::{
  assert_noop, assert_ok,
  traits::{Currency, fungibles::Mutate},
  weights::Weight,
};
use polkadot_sdk::sp_runtime::Permill;
use primitives::{AssetKind, ecosystem::params::PRECISION};

#[test]
fn burn_native_reduces_total_issuance() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let burn_amount = 15 * PRECISION;
    let _ = Balances::deposit_creating(&account, burn_amount);
    let issuance_before = Balances::total_issuance();
    BurningManager::on_idle(1, Weight::MAX);
    let issuance_after = Balances::total_issuance();
    assert_eq!(issuance_before - issuance_after, burn_amount);
    assert_eq!(Balances::free_balance(account), 0);
    assert_eq!(BurningManager::total_burned(), burn_amount);
  });
}

#[test]
fn burn_native_event_reflects_actual_amount() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let burn_amount = 15 * PRECISION;
    let _ = Balances::deposit_creating(&account, burn_amount);
    BurningManager::on_idle(1, Weight::MAX);
    System::assert_has_event(
      Event::NativeTokensBurned {
        amount: burn_amount,
        new_total: burn_amount,
      }
      .into(),
    );
  });
}

#[test]
fn burn_native_accumulates_total_burned() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    let account = BurningManager::account_id();
    let first = 10 * PRECISION;
    let _ = Balances::deposit_creating(&account, first);
    BurningManager::on_idle(1, Weight::MAX);
    assert_eq!(BurningManager::total_burned(), first);
    let second = 20 * PRECISION;
    let _ = Balances::deposit_creating(&account, second);
    BurningManager::on_idle(2, Weight::MAX);
    assert_eq!(BurningManager::total_burned(), first + second);
  });
}

#[test]
fn min_burn_threshold_prevents_small_burns() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    let account = BurningManager::account_id();
    let min_burn = BurningManager::min_burn_native();
    let below_threshold = min_burn - 1;
    let _ = Balances::deposit_creating(&account, below_threshold);
    let issuance_before = Balances::total_issuance();
    BurningManager::on_idle(1, Weight::MAX);
    assert_eq!(Balances::total_issuance(), issuance_before);
    assert_eq!(Balances::free_balance(account), below_threshold);
    assert_eq!(BurningManager::total_burned(), 0);
  });
}

#[test]
fn min_burn_threshold_allows_exact_amount() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    let account = BurningManager::account_id();
    let min_burn = BurningManager::min_burn_native();
    let _ = Balances::deposit_creating(&account, min_burn);
    let issuance_before = Balances::total_issuance();
    BurningManager::on_idle(1, Weight::MAX);
    assert_eq!(issuance_before - Balances::total_issuance(), min_burn);
    assert_eq!(Balances::free_balance(account), 0);
  });
}

#[test]
fn updated_min_burn_is_enforced() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    let account = BurningManager::account_id();
    let new_min = 5 * PRECISION;
    assert_ok!(BurningManager::update_min_burn_native(
      RuntimeOrigin::root(),
      new_min
    ));
    // Below new threshold — should NOT burn
    let _ = Balances::deposit_creating(&account, new_min - 1);
    BurningManager::on_idle(1, Weight::MAX);
    assert_eq!(Balances::free_balance(account), new_min - 1);
    // Top up to reach threshold — should burn all
    let _ = Balances::deposit_creating(&account, 1);
    let issuance_before = Balances::total_issuance();
    BurningManager::on_idle(2, Weight::MAX);
    assert_eq!(issuance_before - Balances::total_issuance(), new_min);
    assert_eq!(Balances::free_balance(account), 0);
  });
}

#[test]
fn foreign_swap_then_burn_reduces_issuance() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let asset_id = 1;
    let asset_kind = AssetKind::Local(asset_id);
    let native_asset = AssetKind::Native;
    let reserve_amount = 1000 * PRECISION;
    set_pool(native_asset, asset_kind, reserve_amount, reserve_amount);
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset_kind
    ));
    let foreign_amount = 20 * PRECISION;
    assert_ok!(Assets::mint_into(asset_id, &account, foreign_amount));
    BurningManager::on_idle(1, Weight::MAX);
    // XYK: 20*P * 1000*P / (1000*P + 20*P) = ~19.607843137 * P
    let expected_native_out = 19_607_843_137_254u128;
    // TotalBurned must match the actual swap output (not the foreign input)
    assert_eq!(BurningManager::total_burned(), expected_native_out);
    // BM native balance should be zero after burn
    assert_eq!(Balances::free_balance(account), 0);
    System::assert_has_event(
      Event::ForeignTokensSwapped {
        foreign_asset: asset_kind,
        foreign_amount,
        native_received: expected_native_out,
      }
      .into(),
    );
    System::assert_has_event(
      Event::NativeTokensBurned {
        amount: expected_native_out,
        new_total: expected_native_out,
      }
      .into(),
    );
  });
}

#[test]
fn slippage_guard_rejects_large_price_impact() {
  new_test_ext().execute_with(|| {
    use polkadot_sdk::frame_support::traits::Hooks;

    System::set_block_number(1);

    let account = BurningManager::account_id();
    let foreign_asset = AssetKind::Local(1);

    // Pool output for 100*P input is ~90.9*P, which is below 98% of expected 100*P.
    let reserve_amount = 1_000 * PRECISION;
    set_pool(
      AssetKind::Native,
      foreign_asset,
      reserve_amount,
      reserve_amount,
    );
    set_oracle_price(foreign_asset, AssetKind::Native, PRECISION);

    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      foreign_asset
    ));

    let foreign_amount = 100 * PRECISION;
    assert_ok!(Assets::mint_into(1, &account, foreign_amount));

    BurningManager::on_idle(1, Weight::MAX);

    // Swap is rejected by slippage guard, so nothing is burned and foreign remains on BM.
    assert_eq!(BurningManager::total_burned(), 0);
    assert_eq!(Assets::balance(1, account), foreign_amount);
  });
}

#[test]
fn extrinsic_burn_reduces_issuance() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let caller = 1u64;
    let amount = 15 * PRECISION;
    let _ = Balances::deposit_creating(&account, amount);
    let issuance_before = Balances::total_issuance();
    assert_ok!(BurningManager::burn_native_tokens(
      RuntimeOrigin::signed(caller),
      amount
    ));
    assert_eq!(issuance_before - Balances::total_issuance(), amount);
    assert_eq!(BurningManager::total_burned(), amount);
  });
}

#[test]
fn extrinsic_burn_rejects_zero() {
  new_test_ext().execute_with(|| {
    assert_noop!(
      BurningManager::burn_native_tokens(RuntimeOrigin::signed(1), 0),
      crate::Error::<crate::mock::Test>::AmountTooSmall
    );
  });
}

#[test]
fn add_burnable_asset_works() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(1);
    assert!(!BurningManager::burnable_assets().contains(&asset));
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));
    assert!(BurningManager::burnable_assets().contains(&asset));
    // Idempotent
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));
    assert_eq!(BurningManager::burnable_assets().len(), 1);
  });
}

#[test]
fn only_governance_can_add_burnable_assets() {
  new_test_ext().execute_with(|| {
    let asset = AssetKind::Local(99);
    assert_noop!(
      BurningManager::add_burnable_asset(RuntimeOrigin::signed(1), asset),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );
    assert_ok!(BurningManager::add_burnable_asset(
      RuntimeOrigin::root(),
      asset
    ));
  });
}

#[test]
fn governance_can_update_min_burn_native() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let initial_min = BurningManager::min_burn_native();
    let new_min = 20_000_000_000_000u128;
    assert_ok!(BurningManager::update_min_burn_native(
      RuntimeOrigin::root(),
      new_min
    ));
    assert_eq!(BurningManager::min_burn_native(), new_min);
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
    assert_noop!(
      BurningManager::update_min_burn_native(RuntimeOrigin::signed(1), 1),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );
  });
}

#[test]
fn governance_can_update_dust_threshold() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let initial = BurningManager::dust_threshold();
    let new_val = 200_000_000_000u128;
    assert_ok!(BurningManager::update_dust_threshold(
      RuntimeOrigin::root(),
      new_val
    ));
    assert_eq!(BurningManager::dust_threshold(), new_val);
    System::assert_last_event(
      Event::DustThresholdUpdated {
        old_threshold: initial,
        new_threshold: new_val,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_dust_threshold() {
  new_test_ext().execute_with(|| {
    assert_noop!(
      BurningManager::update_dust_threshold(RuntimeOrigin::signed(1), 1),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );
  });
}

#[test]
fn governance_can_update_slippage_tolerance() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let initial = BurningManager::slippage_tolerance();
    let new_val = Permill::from_percent(5);
    assert_ok!(BurningManager::update_slippage_tolerance(
      RuntimeOrigin::root(),
      new_val
    ));
    assert_eq!(BurningManager::slippage_tolerance(), new_val);
    System::assert_last_event(
      Event::SlippageToleranceUpdated {
        old_tolerance: initial,
        new_tolerance: new_val,
      }
      .into(),
    );
  });
}

#[test]
fn only_governance_can_update_slippage_tolerance() {
  new_test_ext().execute_with(|| {
    assert_noop!(
      BurningManager::update_slippage_tolerance(RuntimeOrigin::signed(1), Permill::from_percent(3)),
      polkadot_sdk::sp_runtime::DispatchError::BadOrigin
    );
  });
}

#[test]
fn lp_unwinding_decomposes_and_burns() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let account = BurningManager::account_id();
    let native_asset = AssetKind::Native;
    let foreign_asset = AssetKind::Local(1);
    let lp_token_id: u32 = primitives::assets::TYPE_LP | 42;
    let lp_asset = AssetKind::Local(lp_token_id);
    register_lp_token(lp_token_id, native_asset, foreign_asset);
    let reserve = 1000 * PRECISION;
    set_pool(native_asset, foreign_asset, reserve, reserve);
    set_oracle_price(foreign_asset, native_asset, PRECISION);
    let _ = Assets::force_create(RuntimeOrigin::root(), lp_token_id, account, true, 1);
    let lp_amount = 100 * PRECISION;
    assert_ok!(Assets::mint_into(lp_token_id, &account, lp_amount));
    let _ = Balances::deposit_creating(&account, 1);
    assert_ok!(BurningManager::process_lp_unwinding(lp_asset, lp_amount));
    System::assert_has_event(
      Event::LpUnwound {
        lp_asset,
        lp_amount,
        asset1: native_asset,
        amount1: 50 * PRECISION,
        asset2: foreign_asset,
        amount2: 50 * PRECISION,
      }
      .into(),
    );
    assert_eq!(Balances::free_balance(account), 1 + 50 * PRECISION);
    assert_eq!(Assets::balance(1, account), 50 * PRECISION);
  });
}

#[test]
fn lp_unwinding_unknown_lp_fails_gracefully() {
  new_test_ext().execute_with(|| {
    let unknown_lp = AssetKind::Local(primitives::assets::TYPE_LP | 999);
    assert_noop!(
      BurningManager::process_lp_unwinding(unknown_lp, 1000),
      crate::Error::<crate::mock::Test>::SwapFailed
    );
  });
}
