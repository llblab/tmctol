#![cfg(feature = "runtime-benchmarks")]

use super::*;
use polkadot_sdk::frame_benchmarking::v2::*;
use polkadot_sdk::frame_support::traits::fungibles::Mutate;
use polkadot_sdk::frame_system::RawOrigin;
use primitives::AssetKind;

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn create_curve() {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Local(2);
    let initial_price: u128 = 1000;
    let slope: u128 = 1;

    // Treasury account must be the caller for create_curve
    let caller = T::TreasuryAccount::get();

    #[extrinsic_call]
    create_curve(
      RawOrigin::Signed(caller),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    );

    assert!(TokenCurves::<T>::contains_key(token_asset));
  }

  #[benchmark]
  fn mint_tokens() {
    // Setup: Create a curve first
    let token_id = 1;
    let foreign_id = 2;
    let token_asset = AssetKind::Local(token_id);
    let foreign_asset = AssetKind::Local(foreign_id);
    let initial_price: u128 = 1000;
    let slope: u128 = 1000;

    // Treasury creates the curve
    let treasury = T::TreasuryAccount::get();
    Pallet::<T>::create_curve(
      RawOrigin::Signed(treasury).into(),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    )
    .unwrap();

    let foreign_amount: u128 = 10000;

    // Regular user mints tokens
    let caller: T::AccountId = whitelisted_caller();

    // Mint foreign assets to caller
    T::Assets::mint_into(foreign_id, &caller, foreign_amount * 2).unwrap();

    #[extrinsic_call]
    mint_tokens(RawOrigin::Signed(caller), token_asset, foreign_amount);
  }

  #[benchmark]
  fn update_curve() {
    // Setup: Create a curve first
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Local(2);
    let initial_price: u128 = 1000;
    let slope: u128 = 1000;

    // Treasury account creates the curve
    let treasury = T::TreasuryAccount::get();

    Pallet::<T>::create_curve(
      RawOrigin::Signed(treasury).into(),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    )
    .unwrap();

    let new_slope: u128 = 2000;

    // AdminOrigin required - use Root
    #[extrinsic_call]
    update_curve(RawOrigin::Root, token_asset, new_slope);
  }

  #[benchmark]
  fn burn_tokens() {
    // Setup: Create a curve first
    let token_id = 1;
    let foreign_id = 2;
    let token_asset = AssetKind::Local(token_id);
    let foreign_asset = AssetKind::Local(foreign_id);
    let initial_price: u128 = 1000;
    let slope: u128 = 1000;

    // Treasury creates the curve
    let treasury = T::TreasuryAccount::get();
    Pallet::<T>::create_curve(
      RawOrigin::Signed(treasury.clone()).into(),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    )
    .unwrap();

    // Mint some tokens to burn
    let burn_amount: u128 = 5000;
    T::Assets::mint_into(token_id, &treasury, burn_amount * 2).unwrap();

    // Treasury burns tokens
    let caller = treasury;

    #[extrinsic_call]
    burn_tokens(RawOrigin::Signed(caller), token_asset, burn_amount);
  }

  #[benchmark]
  fn pause_minting() {
    // Setup: Create a curve first
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Local(2);
    let initial_price: u128 = 1000;
    let slope: u128 = 1000;

    let caller = T::TreasuryAccount::get();
    Pallet::<T>::create_curve(
      RawOrigin::Signed(caller.clone()).into(),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    )
    .unwrap();

    #[extrinsic_call]
    pause_minting(RawOrigin::Root);
  }

  #[benchmark]
  fn unpause_minting() {
    // Setup: Create a curve first and pause it
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Local(2);
    let initial_price: u128 = 1000;
    let slope: u128 = 1000;

    let caller = T::TreasuryAccount::get();
    Pallet::<T>::create_curve(
      RawOrigin::Signed(caller.clone()).into(),
      token_asset,
      foreign_asset,
      initial_price,
      slope,
    )
    .unwrap();

    Pallet::<T>::unpause_minting(RawOrigin::Root.into()).unwrap();

    #[extrinsic_call]
    unpause_minting(RawOrigin::Root);
  }

  impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
