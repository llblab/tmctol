extern crate alloc;

use crate::*;
use polkadot_sdk::frame_benchmarking::v2::*;
use polkadot_sdk::frame_support::traits::Hooks;
use polkadot_sdk::frame_support::weights::Weight;
use polkadot_sdk::frame_system::RawOrigin;
use polkadot_sdk::sp_runtime::traits::SaturatedConversion;
use primitives::AssetKind;

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn process_foreign_fees() {
    let caller: T::AccountId = whitelisted_caller();
    let burning_manager = Pallet::<T>::account_id();
    let asset = AssetKind::Local(1);
    let native = AssetKind::Native;

    T::BenchmarkHelper::create_asset(asset).expect("Failed to create asset");
    T::BenchmarkHelper::create_pool(native, asset).expect("Failed to create pool");

    let liquidity_amount: u128 = 1_000_000_000_000_000;
    // Fund 2x to keep ED buffer after add_liquidity transfers
    T::BenchmarkHelper::ensure_funded(&caller, native, (liquidity_amount * 2).saturated_into())
      .expect("Failed to fund caller");
    T::BenchmarkHelper::ensure_funded(&caller, asset, (liquidity_amount * 2).saturated_into())
      .expect("Failed to fund caller");
    T::BenchmarkHelper::add_liquidity(
      &caller,
      native,
      asset,
      liquidity_amount.saturated_into(),
      liquidity_amount.saturated_into(),
    )
    .expect("Failed to add liquidity");

    let fee_amount: u128 = 1_000_000_000_000;
    T::BenchmarkHelper::ensure_funded(&burning_manager, asset, fee_amount.saturated_into())
      .expect("Failed to fund burning manager");
    T::BenchmarkHelper::ensure_funded(&burning_manager, native, fee_amount.saturated_into())
      .expect("Failed to fund burning manager with native for ED");

    BurnableAssets::<T>::try_mutate(|assets| assets.try_push(asset)).unwrap();

    #[block]
    {
      Pallet::<T>::on_idle(
        polkadot_sdk::frame_system::Pallet::<T>::block_number(),
        Weight::from_parts(u64::MAX, u64::MAX),
      );
    }
  }

  #[benchmark]
  fn burn_native_tokens() {
    let caller: T::AccountId = whitelisted_caller();
    let burning_manager = Pallet::<T>::account_id();
    let amount: u128 = MinBurnNative::<T>::get().saturating_add(10000u32.into());

    T::BenchmarkHelper::ensure_funded(&burning_manager, AssetKind::Native, amount.saturated_into())
      .expect("Failed to fund burning manager");

    #[extrinsic_call]
    burn_native_tokens(RawOrigin::Signed(caller), amount);
  }

  #[benchmark]
  fn add_burnable_asset() {
    let asset = AssetKind::Local(100);

    #[extrinsic_call]
    add_burnable_asset(RawOrigin::Root, asset);

    assert!(BurnableAssets::<T>::get().contains(&asset));
  }

  #[benchmark]
  fn update_min_burn_native() {
    let new_min: u128 = 20000;

    #[extrinsic_call]
    update_min_burn_native(RawOrigin::Root, new_min);
  }

  #[benchmark]
  fn update_dust_threshold() {
    let new_threshold: u128 = 1000;

    #[extrinsic_call]
    update_dust_threshold(RawOrigin::Root, new_threshold);
  }

  #[benchmark]
  fn update_slippage_tolerance() {
    let new_tolerance = polkadot_sdk::sp_runtime::Permill::from_percent(5);

    #[extrinsic_call]
    update_slippage_tolerance(RawOrigin::Root, new_tolerance);
  }

  #[cfg(test)]
  use crate::mock::{Test, new_test_ext};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
