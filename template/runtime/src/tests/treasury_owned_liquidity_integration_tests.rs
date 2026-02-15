//! Treasury-Owned Liquidity runtime integration tests
//!
//! Focus: validate TOL LP intake, unwind behavior, and bucket purity invariants

use crate::{
  AccountId, AssetConversion, Assets, Runtime, RuntimeOrigin, System, TreasuryOwnedLiquidity,
};
use alloc::boxed::Box;
use polkadot_sdk::frame_support::traits::Hooks;
use polkadot_sdk::frame_support::weights::Weight;
use polkadot_sdk::frame_support::{PalletId, assert_ok};
use polkadot_sdk::pallet_asset_conversion::{self, PoolLocator};
use polkadot_sdk::sp_runtime::traits::AccountIdConversion;
use primitives::assets::TYPE_LP;
use primitives::{AssetInspector, AssetKind, ecosystem};

use super::common::{
  ALICE, ASSET_A, LIQUIDITY_AMOUNT, MIN_LIQUIDITY, TOL_TOTAL_ALLOCATION, add_liquidity,
  burning_manager_account, setup_basic_test_environment, tol_ingress_account_for_tol_id,
  tol_treasury_account, zap_manager_account,
};

fn bucket_b_account() -> AccountId {
  PalletId(*ecosystem::pallet_ids::BUCKET_B_ID).into_account_truncating()
}

fn bucket_b_account_for_tol(tol_id: u32) -> AccountId {
  if tol_id == 0 {
    return bucket_b_account();
  }
  PalletId(*ecosystem::pallet_ids::TOL_PALLET_ID).into_sub_account_truncating((*b"tolbb", tol_id))
}

fn setup_bucket_lp_distribution(lp_amount: u128) -> (u32, AccountId) {
  assert_ok!(TreasuryOwnedLiquidity::create_tol(
    RuntimeOrigin::root(),
    0,
    AssetKind::Local(ASSET_A),
    AssetKind::Native,
    TOL_TOTAL_ALLOCATION,
  ));

  assert_ok!(AssetConversion::create_pool(
    RuntimeOrigin::signed(ALICE),
    Box::new(AssetKind::Native),
    Box::new(AssetKind::Local(ASSET_A)),
  ));

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

  let pool_id = <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(
    &AssetKind::Native,
    &AssetKind::Local(ASSET_A),
  )
  .expect("pool id should resolve");
  let lp_token_id = pallet_asset_conversion::Pools::<Runtime>::get(pool_id)
    .expect("pool should exist")
    .lp_token;

  assert_ok!(Assets::transfer(
    RuntimeOrigin::signed(ALICE),
    lp_token_id,
    tol_treasury_account().into(),
    lp_amount,
  ));

  assert_ok!(TreasuryOwnedLiquidity::receive_lp_tokens(
    RuntimeOrigin::signed(zap_manager_account()),
    AssetKind::Local(lp_token_id),
    lp_amount,
  ));

  let bucket_b = bucket_b_account();
  assert!(Assets::balance(lp_token_id, &bucket_b) > 0);

  (lp_token_id, bucket_b)
}

#[test]
fn lp_tokens_in_bucket_can_be_force_transferred_by_asset_admin() {
  setup_basic_test_environment().execute_with(|| {
    let (lp_token_id, bucket_b) = setup_bucket_lp_distribution(1_000_000_000_000);

    assert_ok!(Assets::force_asset_status(
      RuntimeOrigin::root(),
      lp_token_id,
      ALICE.into(),
      ALICE.into(),
      ALICE.into(),
      ALICE.into(),
      1,
      true,
      false,
    ));

    let bucket_before = Assets::balance(lp_token_id, &bucket_b);
    let alice_before = Assets::balance(lp_token_id, &ALICE);

    assert_ok!(Assets::force_transfer(
      RuntimeOrigin::signed(ALICE),
      lp_token_id,
      bucket_b.clone().into(),
      ALICE.into(),
      1,
    ));

    assert_eq!(Assets::balance(lp_token_id, &bucket_b), bucket_before - 1);
    assert_eq!(Assets::balance(lp_token_id, &ALICE), alice_before + 1);
  });
}

#[test]
fn unwind_bucket_liquidity_can_debit_any_lp_in_allowed_bucket() {
  setup_basic_test_environment().execute_with(|| {
    let (lp_token_id, bucket_b) = setup_bucket_lp_distribution(1_000_000_000_000);

    let bucket_balance_before = Assets::balance(lp_token_id, &bucket_b);
    let unwind_amount = bucket_balance_before / 2;
    assert!(unwind_amount > 0);

    assert_ok!(TreasuryOwnedLiquidity::unwind_bucket_liquidity(
      RuntimeOrigin::root(),
      1,
      AssetKind::Local(lp_token_id),
      unwind_amount,
      ALICE,
    ));

    let bucket_balance_after = Assets::balance(lp_token_id, &bucket_b);
    assert_eq!(bucket_balance_after, bucket_balance_before - unwind_amount);
  });
}

#[test]
fn non_lp_assets_in_bucket_are_swept_to_burning_manager_on_idle() {
  setup_basic_test_environment().execute_with(|| {
    let (_lp_token_id, bucket_b) = setup_bucket_lp_distribution(1_000_000_000_000);
    let burning_manager = burning_manager_account();
    let non_lp_amount = 123_456_789u128;

    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      ASSET_A,
      bucket_b.clone().into(),
      non_lp_amount,
    ));

    let bm_before = Assets::balance(ASSET_A, &burning_manager);

    System::set_block_number(2);
    let _ = TreasuryOwnedLiquidity::on_idle(2, Weight::MAX);

    assert_eq!(Assets::balance(ASSET_A, &bucket_b), 0);
    assert_eq!(
      Assets::balance(ASSET_A, &burning_manager),
      bm_before + non_lp_amount
    );
  });
}

#[test]
fn bound_token_domain_routes_lp_distribution_from_bound_ingress() {
  setup_basic_test_environment().execute_with(|| {
    let tol_id = 7u32;
    let lp_amount = 1_000_000_000_000u128;

    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      0,
      AssetKind::Local(ASSET_A),
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));
    assert_ok!(TreasuryOwnedLiquidity::create_tol(
      RuntimeOrigin::root(),
      tol_id,
      AssetKind::Local(ASSET_A),
      AssetKind::Native,
      TOL_TOTAL_ALLOCATION,
    ));

    assert_ok!(TreasuryOwnedLiquidity::bind_token_to_tol(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_A),
      tol_id,
    ));

    let default_ingress = tol_treasury_account();
    let bound_ingress = tol_ingress_account_for_tol_id(tol_id);

    assert_ok!(AssetConversion::create_pool(
      RuntimeOrigin::signed(ALICE),
      Box::new(AssetKind::Native),
      Box::new(AssetKind::Local(ASSET_A)),
    ));

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

    let pool_id = <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(
      &AssetKind::Native,
      &AssetKind::Local(ASSET_A),
    )
    .expect("pool id should resolve");
    let lp_token_id = pallet_asset_conversion::Pools::<Runtime>::get(pool_id)
      .expect("pool should exist")
      .lp_token;

    assert_ok!(Assets::transfer(
      RuntimeOrigin::signed(ALICE),
      lp_token_id,
      bound_ingress.clone().into(),
      lp_amount,
    ));

    assert_ok!(TreasuryOwnedLiquidity::receive_lp_tokens(
      RuntimeOrigin::signed(zap_manager_account()),
      AssetKind::Local(lp_token_id),
      lp_amount,
    ));

    assert_eq!(Assets::balance(lp_token_id, &bound_ingress), 0);
    assert_eq!(Assets::balance(lp_token_id, &default_ingress), 0);
    assert!(Assets::balance(lp_token_id, &bucket_b_account_for_tol(tol_id)) > 0);
  });
}

#[test]
fn lp_ids_start_in_type_lp_namespace_on_clean_genesis() {
  setup_basic_test_environment().execute_with(|| {
    let next_lp = pallet_asset_conversion::NextPoolAssetId::<Runtime>::get()
      .expect("next LP id should be initialized");
    assert_eq!(next_lp & TYPE_LP, TYPE_LP);

    assert_ok!(AssetConversion::create_pool(
      RuntimeOrigin::signed(ALICE),
      Box::new(AssetKind::Native),
      Box::new(AssetKind::Local(ASSET_A)),
    ));

    let pool_id = <Runtime as pallet_asset_conversion::Config>::PoolLocator::pool_id(
      &AssetKind::Native,
      &AssetKind::Local(ASSET_A),
    )
    .expect("pool id should resolve");
    let lp_token_id = pallet_asset_conversion::Pools::<Runtime>::get(pool_id)
      .expect("pool should exist")
      .lp_token;

    assert!(AssetKind::Local(lp_token_id).is_lp());
  });
}
