extern crate alloc;

use crate::*;
use polkadot_sdk::frame_benchmarking::v2::*;
use polkadot_sdk::frame_support::traits::{fungibles::Mutate, Currency, EnsureOrigin, Get};
use polkadot_sdk::frame_system::RawOrigin;
use polkadot_sdk::sp_runtime::traits::SaturatedConversion;
use primitives::AssetKind;

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn create_tol() {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    #[extrinsic_call]
    create_tol(origin, token_asset, foreign_asset, total_allocation);

    assert!(TolConfigurations::<T>::contains_key(token_asset));
  }

  #[benchmark]
  fn update_bucket_allocation() {
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(origin.clone(), token_asset, foreign_asset, total_allocation).unwrap();

    // Bucket B (index 1)
    let bucket_id: u8 = 1;
    let new_allocation: u32 = 500000; // 50%

    #[extrinsic_call]
    update_bucket_allocation(origin, token_asset, bucket_id, new_allocation);
  }

  #[benchmark]
  fn receive_mint_allocation() {
    let token_asset = AssetKind::Local(1);
    let native_amount: u128 = 1000;
    let foreign_amount: u128 = 1000;

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
    let token_asset = AssetKind::Local(1);
    let foreign_asset = AssetKind::Native;
    let total_allocation: u128 = 1000;
    let lp_amount: u128 = 100;

    // Setup TOL first
    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");
    Pallet::<T>::create_tol(origin, token_asset, foreign_asset, total_allocation).unwrap();

    // Zap Manager account needs to be the caller
    let caller = T::ZapManagerAccount::get();

    // Fund TOL treasury with LP tokens
    let tol_treasury = Pallet::<T>::account_id();
    // Mint extra to cover Preservation
    let _ = T::Assets::mint_into(
      1,
      &tol_treasury,
      lp_amount.saturating_add(1_000_000u32.into()),
    );

    #[extrinsic_call]
    receive_lp_tokens(RawOrigin::Signed(caller), token_asset, lp_amount);
  }

  #[benchmark]
  fn withdraw_buffer() {
    let asset = AssetKind::Local(1);
    let amount: u128 = 1000;
    let destination: T::AccountId = whitelisted_caller();

    // Fund treasury with assets
    let treasury = Pallet::<T>::account_id();
    let _ = T::Currency::deposit_creating(&treasury, 1_000_000_000_000u128.saturated_into());
    // Mint extra to cover Preservation
    let _ = T::Assets::mint_into(1, &treasury, amount.saturating_add(1_000_000u32.into()));

    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    #[extrinsic_call]
    withdraw_buffer(origin, asset, amount, destination);
  }

  #[cfg(test)]
  use crate::mock::{new_test_ext, Test};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
