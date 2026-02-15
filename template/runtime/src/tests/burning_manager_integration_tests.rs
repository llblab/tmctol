//! Burning Manager Integration Tests
//!
//! Verifies BM on_idle cycle: native burning, foreign swap→burn, and the
//! full router-fee→BM→burn pipeline using real runtime pallets.

use super::common::{
  ALICE, ASSET_A, ASSET_FOREIGN, INITIAL_BALANCE, LIQUIDITY_AMOUNT, MIN_LIQUIDITY, add_liquidity,
  burning_manager_account, ensure_asset_conversion_pool, new_test_ext, seeded_test_ext,
};
use crate::{Assets, AxialRouter, Balances, BurningManager, RuntimeOrigin, System};
use polkadot_sdk::frame_support::assert_ok;
use polkadot_sdk::frame_support::traits::{Currency, Hooks, fungibles::Mutate};
use primitives::AssetKind;

fn seed_oracle_1to1(asset: AssetKind) {
  let price = 1_000_000_000_000u128;
  pallet_axial_router::EmaPrices::<crate::Runtime>::insert(asset, AssetKind::Native, price);
  pallet_axial_router::EmaPrices::<crate::Runtime>::insert(AssetKind::Native, asset, price);
}

fn setup_pool_and_oracle(asset_id: u32) {
  let asset = AssetKind::Local(asset_id);
  ensure_asset_conversion_pool(AssetKind::Native, asset);
  assert_ok!(add_liquidity(
    RuntimeOrigin::signed(ALICE),
    AssetKind::Native,
    asset,
    LIQUIDITY_AMOUNT,
    LIQUIDITY_AMOUNT,
    MIN_LIQUIDITY,
    MIN_LIQUIDITY,
    &ALICE,
  ));
  seed_oracle_1to1(asset);
}

fn register_burnable(asset: AssetKind) {
  assert_ok!(BurningManager::add_burnable_asset(
    RuntimeOrigin::root(),
    asset
  ));
}

#[test]
fn test_bm_burns_native_on_idle() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let bm = burning_manager_account();
    let deposit = 500_000_000_000_000u128;
    let _ = Balances::deposit_creating(&bm, deposit);
    let total_before = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    BurningManager::on_idle(1, polkadot_sdk::frame_support::weights::Weight::MAX);
    let total_after = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    assert!(total_after > total_before, "TotalBurned must increase");
  });
}

#[test]
fn test_bm_swaps_foreign_then_burns_native() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    // Register burnable BEFORE we trigger on_idle
    register_burnable(AssetKind::Local(ASSET_FOREIGN));
    let bm = burning_manager_account();
    // Drain pre-seeded foreign balance and re-seed with a small controlled amount
    let pre_seeded = Assets::balance(ASSET_FOREIGN, &bm);
    if pre_seeded > 0 {
      use polkadot_sdk::frame_support::traits::tokens::{Fortitude, Precision, Preservation};
      let _ = <Assets as polkadot_sdk::frame_support::traits::fungibles::Mutate<
        crate::AccountId,
      >>::burn_from(
        ASSET_FOREIGN,
        &bm,
        pre_seeded,
        Preservation::Expendable,
        Precision::BestEffort,
        Fortitude::Force,
      );
    }

    // Set up pool with deep liquidity
    let pool_liquidity = INITIAL_BALANCE * 10;
    let _ = Balances::deposit_creating(&ALICE, pool_liquidity * 2);
    assert_ok!(Assets::mint_into(ASSET_FOREIGN, &ALICE, pool_liquidity * 2));
    ensure_asset_conversion_pool(AssetKind::Native, AssetKind::Local(ASSET_FOREIGN));
    assert_ok!(add_liquidity(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      AssetKind::Local(ASSET_FOREIGN),
      pool_liquidity,
      pool_liquidity,
      MIN_LIQUIDITY,
      MIN_LIQUIDITY,
      &ALICE,
    ));
    seed_oracle_1to1(AssetKind::Local(ASSET_FOREIGN));
    // Give BM a small foreign amount (< 1% of pool for minimal price impact)
    let bm_foreign_amount = pool_liquidity / 200; // 0.5% of pool
    assert_ok!(Assets::mint_into(ASSET_FOREIGN, &bm, bm_foreign_amount));
    let foreign_before = Assets::balance(ASSET_FOREIGN, &bm);
    assert!(foreign_before > 0, "BM should have foreign tokens");
    let total_burned_before = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    BurningManager::on_idle(1, polkadot_sdk::frame_support::weights::Weight::MAX);
    let foreign_after = Assets::balance(ASSET_FOREIGN, &bm);
    assert!(
      foreign_after < foreign_before,
      "Foreign tokens must be swapped via router"
    );
    let total_burned_after = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    assert!(
      total_burned_after > total_burned_before,
      "Swap proceeds + pre-seeded native must be burned"
    );
  });
}

#[test]
fn test_router_fee_flows_to_bm_and_burns() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_and_oracle(ASSET_A);
    let bm = burning_manager_account();
    // Drain pre-seeded native from BM
    BurningManager::on_idle(1, polkadot_sdk::frame_support::weights::Weight::MAX);
    System::set_block_number(2);
    let bm_native_before = Balances::free_balance(&bm);
    let swap_amount = INITIAL_BALANCE / 20;
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      AssetKind::Local(ASSET_A),
      swap_amount,
      1,
      ALICE,
      1000
    ));
    let bm_native_after_swap = Balances::free_balance(&bm);
    let fee_received = bm_native_after_swap.saturating_sub(bm_native_before);
    assert!(
      fee_received > 0,
      "BM must receive native fee from router swap"
    );
    let total_burned_before = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    // Lower threshold so small fee amounts are burned in test
    pallet_burning_manager::MinBurnNative::<crate::Runtime>::put(0u128);
    System::set_block_number(3);
    BurningManager::on_idle(3, polkadot_sdk::frame_support::weights::Weight::MAX);
    let total_burned_after = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    assert!(
      total_burned_after >= total_burned_before + fee_received,
      "BM must burn at least the fee amount received"
    );
  });
}

#[test]
fn test_bm_skips_empty_account() {
  new_test_ext().execute_with(|| {
    System::set_block_number(1);
    let total_before = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    BurningManager::on_idle(1, polkadot_sdk::frame_support::weights::Weight::MAX);
    let total_after = pallet_burning_manager::TotalBurned::<crate::Runtime>::get();
    assert_eq!(
      total_before, total_after,
      "Nothing to burn on empty BM account"
    );
  });
}

#[test]
fn test_full_e2e_mint_zap_fee_burn() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    // Setup: pool, oracle, TMC curve, ZM whitelist, BM burnable
    setup_pool_and_oracle(ASSET_FOREIGN);
    register_burnable(AssetKind::Local(ASSET_FOREIGN));
    assert_ok!(crate::TokenMintingCurve::create_curve(
      RuntimeOrigin::root(),
      AssetKind::Native,
      AssetKind::Local(ASSET_FOREIGN),
      1_000_000_000_000,
      1_000_000_000,
    ));
    assert_ok!(crate::ZapManager::enable_asset(
      RuntimeOrigin::root(),
      AssetKind::Local(ASSET_FOREIGN)
    ));
    // Drain pre-seeded BM native
    BurningManager::on_idle(1, polkadot_sdk::frame_support::weights::Weight::MAX);
    System::set_block_number(2);
    // Stage 1: TMC mint
    let mint_foreign = 100_000_000_000_000u128;
    let alice_native_before = Balances::free_balance(&ALICE);
    let minted = crate::TokenMintingCurve::mint_with_distribution(
      &ALICE,
      AssetKind::Native,
      AssetKind::Local(ASSET_FOREIGN),
      mint_foreign,
    )
    .expect("TMC mint must succeed");
    assert!(minted > 0);
    let user_received = Balances::free_balance(&ALICE).saturating_sub(alice_native_before);
    assert!(user_received > 0, "User must receive 1/3 allocation");
    let zap_account = crate::ZapManager::account_id();
    let zap_foreign = Assets::balance(ASSET_FOREIGN, &zap_account);
    assert!(zap_foreign > 0, "ZM must receive foreign tokens");
    // Stage 2: ZM zap → LP
    System::set_block_number(5);
    crate::ZapManager::on_initialize(5);
    crate::ZapManager::on_idle(5, polkadot_sdk::frame_support::weights::Weight::MAX);
    let zap_foreign_after = Assets::balance(ASSET_FOREIGN, &zap_account);
    assert!(
      zap_foreign_after < zap_foreign,
      "ZM must consume foreign during zap"
    );
    // Stage 3: Router swap → fee → BM
    let bm = burning_manager_account();
    let bm_native_pre = Balances::free_balance(&bm);
    let swap_amount = INITIAL_BALANCE / 20;
    assert_ok!(AxialRouter::swap(
      RuntimeOrigin::signed(ALICE),
      AssetKind::Native,
      AssetKind::Local(ASSET_FOREIGN),
      swap_amount,
      1,
      ALICE,
      1000
    ));
    let fee_collected = Balances::free_balance(&bm).saturating_sub(bm_native_pre);
    assert!(fee_collected > 0, "Router fee must reach BM");
    // Stage 4: BM burns
    // Lower threshold so small fee amounts are burned in test
    pallet_burning_manager::MinBurnNative::<crate::Runtime>::put(0u128);
    let issuance_before = Balances::total_issuance();
    System::set_block_number(6);
    BurningManager::on_idle(6, polkadot_sdk::frame_support::weights::Weight::MAX);
    let issuance_after = Balances::total_issuance();
    assert!(
      issuance_after < issuance_before,
      "Total issuance must decrease after BM burn"
    );
    assert!(pallet_burning_manager::TotalBurned::<crate::Runtime>::get() > 0);
  });
}
