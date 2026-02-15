extern crate alloc;

use crate::*;
use polkadot_sdk::frame_benchmarking::v2::*;
use polkadot_sdk::frame_support::traits::fungibles::Inspect;
use polkadot_sdk::frame_support::traits::{EnsureOrigin, Get};
use polkadot_sdk::frame_system::RawOrigin;
use primitives::{AssetKind, assets::TYPE_LP};

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn create_tol() {
    let tol_id: TolId = 0;
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    #[extrinsic_call]
    create_tol(origin, tol_id, token_asset, foreign_asset, total_allocation);

    assert!(TolConfigurations::<T>::get(tol_id).is_some());
  }

  #[benchmark]
  fn update_bucket_allocation() {
    let tol_id: TolId = 0;
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(
      origin.clone(),
      tol_id,
      token_asset,
      foreign_asset,
      total_allocation,
    )
    .unwrap();

    // Bucket B (index 1)
    let bucket_id: u8 = 1;
    let new_allocation: u32 = 500000; // 50%

    #[extrinsic_call]
    update_bucket_allocation(origin, tol_id, bucket_id, new_allocation);
  }

  #[benchmark]
  fn receive_mint_allocation() {
    let tol_id: TolId = 0;
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;
    let native_amount: u128 = 1000;
    let foreign_amount: u128 = 1000;

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(origin, tol_id, token_asset, foreign_asset, total_allocation).unwrap();

    // Caller must be Zap Manager
    let caller = T::ZapManagerAccount::get();

    #[extrinsic_call]
    receive_mint_allocation(
      RawOrigin::Signed(caller),
      token_asset,
      native_amount,
      foreign_amount,
    );
  }

  #[benchmark]
  fn receive_lp_tokens() {
    let tol_id: TolId = 0;
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;
    let lp_asset = AssetKind::Local(TYPE_LP | 1);

    // Create LP-classified asset first (required in runtime benchmark context)
    T::BenchmarkHelper::create_asset(TYPE_LP | 1).expect("Failed to create LP asset");
    let min_balance = T::Assets::minimum_balance(TYPE_LP | 1);
    let lp_amount = min_balance.saturating_mul(2);

    // Setup TOL first
    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(origin, tol_id, token_asset, foreign_asset, total_allocation).unwrap();

    // Zap Manager account needs to be the caller
    let caller = T::ZapManagerAccount::get();

    // Fund TOL account with LP tokens
    let tol_account = Pallet::<T>::account_id();
    T::BenchmarkHelper::fund_account(
      &tol_account,
      lp_asset,
      lp_amount.saturating_add(min_balance),
    )
    .expect("Failed to fund TOL account");

    #[extrinsic_call]
    receive_lp_tokens(RawOrigin::Signed(caller), lp_asset, lp_amount);
  }

  #[benchmark]
  fn withdraw_buffer() {
    let tol_id: TolId = 0;
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;
    let asset = AssetKind::Local(1);
    let destination: T::AccountId = whitelisted_caller();

    // Create asset first (required in runtime benchmark context)
    T::BenchmarkHelper::create_asset(1).expect("Failed to create asset");
    let min_balance = T::Assets::minimum_balance(1);
    let amount = min_balance.saturating_mul(2);

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(
      origin.clone(),
      tol_id,
      token_asset,
      foreign_asset,
      total_allocation,
    )
    .unwrap();

    // Fund treasury with native and target asset
    let treasury = Pallet::<T>::ingress_account_for_tol_id(tol_id);
    T::BenchmarkHelper::fund_account(&treasury, AssetKind::Native, 1_000_000_000_000u128)
      .expect("Failed to fund treasury native");
    T::BenchmarkHelper::fund_account(&treasury, asset, amount.saturating_add(min_balance))
      .expect("Failed to fund treasury asset");
    // Fund destination with ED
    T::BenchmarkHelper::fund_account(&destination, AssetKind::Native, 1_000_000_000_000u128)
      .expect("Failed to fund destination");

    #[extrinsic_call]
    withdraw_buffer(origin, tol_id, asset, amount, destination);
  }

  #[cfg(test)]
  use crate::mock::{Test, new_test_ext};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
