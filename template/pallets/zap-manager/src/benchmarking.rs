extern crate alloc;

use crate::AssetConversionApi;
use crate::*;
use polkadot_sdk::frame_benchmarking::{account, v2::*};
use polkadot_sdk::frame_support::traits::{
  fungible::Mutate as NativeMutate, fungibles::Inspect as FungiblesInspect,
  fungibles::Mutate as FungiblesMutate, EnsureOrigin, Hooks,
};
use polkadot_sdk::sp_runtime::traits::One;
use primitives::AssetKind;

#[benchmarks]
mod benches {
  use super::*;

  #[benchmark]
  fn process_zap_cycle() {
    let asset_id = 1;
    let asset = AssetKind::Local(asset_id);
    let native = AssetKind::Native;
    let zap_account = Pallet::<T>::account_id();
    let lp_provider: T::AccountId = account("lp_provider", 0, 0);

    // 1. Enable Asset
    EnabledAssets::<T>::insert(asset, ());

    // 2. Mint tokens to accounts
    // Native and Asset for LP provider to create pool
    let liquidity_amount: u128 = 1_000_000_000_000_000; // 1000 units
    let _ = T::Currency::mint_into(&lp_provider, liquidity_amount);
    let _ = T::Assets::mint_into(asset_id, &lp_provider, liquidity_amount);

    // Native and Asset for Zap Manager to process
    let zap_amount: u128 = 100_000_000_000_000; // 100 units
    let _ = T::Currency::mint_into(&zap_account, zap_amount);
    let _ = T::Assets::mint_into(asset_id, &zap_account, zap_amount);

    // 3. Create Pool and Add Liquidity
    // We rely on T::AssetConversion to handle the pool creation logic.
    // We assume Asset 1 exists in the benchmarking environment.
    let _ = T::AssetConversion::create_pool(native, asset);
    let _ = T::AssetConversion::add_liquidity(
      &lp_provider,
      native,
      asset,
      liquidity_amount / 2,
      liquidity_amount / 2,
      1,
      1,
    );

    #[block]
    {
      Pallet::<T>::on_initialize(
        polkadot_sdk::frame_system::pallet_prelude::BlockNumberFor::<T>::one(),
      );
    }
  }

  #[benchmark]
  fn enable_asset() {
    let asset = AssetKind::Local(1);
    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    #[extrinsic_call]
    enable_asset(origin, asset);

    assert!(EnabledAssets::<T>::contains_key(asset));
  }

  #[benchmark]
  fn disable_asset() {
    let asset = AssetKind::Local(1);
    EnabledAssets::<T>::insert(asset, ());
    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    #[extrinsic_call]
    disable_asset(origin, asset);

    assert!(!EnabledAssets::<T>::contains_key(asset));
  }

  #[benchmark]
  fn sweep_trigger() {
    let asset_id = 1;
    let asset = AssetKind::Local(asset_id);
    let zap_account = Pallet::<T>::account_id();
    let origin =
      T::AdminOrigin::try_successful_origin().expect("AdminOrigin must have a successful origin");

    let amount: u128 = 10_000;

    // We assume the asset exists in the testing environment (like mock.rs)
    // or the runtime has been seeded.
    let _ = T::Assets::mint_into(asset_id, &zap_account, amount);

    #[extrinsic_call]
    sweep_trigger(origin, asset);

    let remaining = T::Assets::balance(asset_id, &zap_account);
    let min_balance = T::Assets::minimum_balance(asset_id);

    assert_eq!(remaining, min_balance);
  }

  #[cfg(test)]
  use crate::mock::{new_test_ext, Test};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
