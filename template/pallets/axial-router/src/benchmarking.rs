extern crate alloc;

use crate::{types::BenchmarkHelper, *};
use polkadot_sdk::frame_benchmarking::v2::*;
use polkadot_sdk::frame_system::RawOrigin;
use polkadot_sdk::sp_runtime::traits::{SaturatedConversion, Zero};
use primitives::AssetKind;

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn swap() {
    let caller: T::AccountId = whitelisted_caller();
    let from = AssetKind::Local(1);
    let to = T::NativeAsset::get();
    let amount_in = T::MinSwapForeign::get().saturating_mul(1000u32.into());
    let min_amount_out = Zero::zero();
    let recipient = caller.clone();
    let deadline = 10000u32.into();

    // Setup assets and liquidity
    T::BenchmarkHelper::create_asset(from).expect("Failed to create asset");
    T::BenchmarkHelper::mint_asset(to, &caller, 1_000_000_000_000_000_000u128.saturated_into())
      .expect("Failed to mint native");
    T::BenchmarkHelper::mint_asset(
      from,
      &caller,
      1_000_000_000_000_000_000u128.saturated_into(),
    )
    .expect("Failed to mint foreign");

    T::BenchmarkHelper::create_pool(to, from).expect("Failed to create pool");
    T::BenchmarkHelper::add_liquidity(
      &caller,
      to,
      from,
      100_000_000_000_000_000u128.saturated_into(),
      100_000_000_000_000_000u128.saturated_into(),
    )
    .expect("Failed to add liquidity");

    #[extrinsic_call]
    swap(
      RawOrigin::Signed(caller),
      from,
      to,
      amount_in,
      min_amount_out,
      recipient,
      deadline,
    );
  }

  #[benchmark]
  fn add_tracked_asset() {
    let asset = AssetKind::Local(100);

    #[extrinsic_call]
    add_tracked_asset(RawOrigin::Root, asset);

    assert!(TrackedAssets::<T>::get().contains(&asset));
  }

  #[benchmark]
  fn update_router_fee() {
    let new_fee = polkadot_sdk::sp_runtime::Permill::from_percent(1);

    #[extrinsic_call]
    update_router_fee(RawOrigin::Root, new_fee);
  }

  #[cfg(test)]
  use crate::mock::{new_test_ext, Test};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
