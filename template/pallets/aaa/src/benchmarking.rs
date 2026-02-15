#![cfg(feature = "runtime-benchmarks")]

extern crate alloc;

use crate::*;
use alloc::vec;
use frame::prelude::*;
use polkadot_sdk::frame_benchmarking::{account, v2::*};
use polkadot_sdk::frame_support::traits::Hooks;
use polkadot_sdk::frame_system::RawOrigin;

#[benchmarks]
mod benches {
  use super::*;

  fn make_pipeline<T: Config>(recipient: T::AccountId) -> PipelineOf<T> {
    let step = Step {
      conditions: BoundedVec::default(),
      task: TaskKind::Transfer {
        to: recipient,
        asset: T::NativeAssetId::get(),
        amount: AmountSpec::AllBalance,
      },
      on_error: PipelineErrorPolicy::AbortCycle,
    };
    BoundedVec::try_from(vec![step]).unwrap_or_default()
  }

  fn make_remove_liquidity_pipeline<T: Config>(
    lp_asset: T::AssetId,
    amount: T::Balance,
  ) -> PipelineOf<T> {
    let step = Step {
      conditions: BoundedVec::default(),
      task: TaskKind::RemoveLiquidity {
        lp_asset,
        amount: AmountSpec::Fixed(amount),
      },
      on_error: PipelineErrorPolicy::AbortCycle,
    };
    BoundedVec::try_from(vec![step]).unwrap_or_default()
  }

  fn bench_create_user<T: Config>(caller: T::AccountId) -> AaaId {
    let dummy =
      T::AccountId::decode(&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes())
        .unwrap();
    let pipeline = make_pipeline::<T>(dummy);
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 10,
    };
    Pallet::<T>::create_user_aaa(
      RawOrigin::Signed(caller).into(),
      Mutability::Mutable,
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    )
    .unwrap();
    NextAaaId::<T>::get().saturating_sub(1)
  }

  fn prefill_owner_slots_for_worst_case<T: Config>(owner: &T::AccountId) -> u16 {
    let max_slots = T::MaxOwnerSlots::get();
    assert!(max_slots > 0, "MaxOwnerSlots must be greater than zero");
    let target_slot = max_slots.saturating_sub(1);
    for slot in 0..target_slot {
      OwnerSlots::<T>::insert(owner.clone(), slot, u64::MAX.saturating_sub(slot as u64));
    }
    target_slot
  }

  #[benchmark]
  fn create_user_aaa() {
    let caller: T::AccountId = whitelisted_caller();
    let expected_slot = prefill_owner_slots_for_worst_case::<T>(&caller);
    let dummy =
      T::AccountId::decode(&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes())
        .unwrap();
    let pipeline = make_pipeline::<T>(dummy);
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 10,
    };
    #[extrinsic_call]
    create_user_aaa(
      RawOrigin::Signed(caller),
      Mutability::Mutable,
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    );
    let inst = AaaInstances::<T>::get(0u64).expect("AAA instance must exist after create_user_aaa");
    assert_eq!(inst.owner_slot, expected_slot);
  }

  #[benchmark]
  fn create_system_aaa() {
    let owner: T::AccountId = whitelisted_caller();
    let expected_slot = prefill_owner_slots_for_worst_case::<T>(&owner);
    let dummy =
      T::AccountId::decode(&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes())
        .unwrap();
    let pipeline = make_pipeline::<T>(dummy);
    let refund_to = owner.clone();
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 100,
    };
    #[extrinsic_call]
    create_system_aaa(
      RawOrigin::Root,
      owner,
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      refund_to,
    );
    let inst =
      AaaInstances::<T>::get(0u64).expect("AAA instance must exist after create_system_aaa");
    assert_eq!(inst.owner_slot, expected_slot);
  }

  #[benchmark]
  fn pause_aaa() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    #[extrinsic_call]
    pause_aaa(RawOrigin::Signed(caller), aaa_id);
    let inst = AaaInstances::<T>::get(aaa_id).unwrap();
    assert!(inst.is_paused);
  }

  #[benchmark]
  fn resume_aaa() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    Pallet::<T>::pause_aaa(RawOrigin::Signed(caller.clone()).into(), aaa_id).unwrap();
    #[extrinsic_call]
    resume_aaa(RawOrigin::Signed(caller), aaa_id);
    let inst = AaaInstances::<T>::get(aaa_id).unwrap();
    assert!(!inst.is_paused);
  }

  #[benchmark]
  fn manual_trigger() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    #[extrinsic_call]
    manual_trigger(RawOrigin::Signed(caller), aaa_id);
    let inst = AaaInstances::<T>::get(aaa_id).unwrap();
    assert!(inst.manual_trigger_pending);
  }

  #[benchmark]
  fn fund_aaa() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    let amount = T::MinUserBalance::get().saturating_add(1u32.into());
    T::AssetOps::mint(&caller, T::NativeAssetId::get(), amount).unwrap();
    #[extrinsic_call]
    fund_aaa(RawOrigin::Signed(caller), aaa_id, amount);
    assert!(AaaInstances::<T>::get(aaa_id).is_some());
  }

  #[benchmark]
  fn refund_and_close() {
    let caller: T::AccountId = whitelisted_caller();
    let dummy =
      T::AccountId::decode(&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes())
        .unwrap();
    let pipeline = make_pipeline::<T>(dummy.clone());
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 10,
    };
    Pallet::<T>::create_user_aaa(
      RawOrigin::Signed(caller.clone()).into(),
      Mutability::Mutable,
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      Some(dummy),
    )
    .unwrap();
    let aaa_id = 0u64;
    #[extrinsic_call]
    refund_and_close(RawOrigin::Signed(caller), aaa_id);
    assert!(AaaInstances::<T>::get(aaa_id).is_none());
  }

  #[benchmark]
  fn update_policy() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    let new_policy = AaaPolicy {
      default_error_policy: PipelineErrorPolicy::ContinueNextStep,
    };
    #[extrinsic_call]
    update_policy(RawOrigin::Signed(caller), aaa_id, new_policy);
    let inst = AaaInstances::<T>::get(aaa_id).unwrap();
    assert!(matches!(
      inst.policy.default_error_policy,
      PipelineErrorPolicy::ContinueNextStep
    ));
  }

  #[benchmark]
  fn update_schedule() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    let new_schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 20,
    };
    #[extrinsic_call]
    update_schedule(RawOrigin::Signed(caller), aaa_id, new_schedule, None);
    let inst = AaaInstances::<T>::get(aaa_id).unwrap();
    assert_eq!(inst.schedule.cooldown_blocks, 20);
  }

  #[benchmark]
  fn update_refund_assets() {
    let owner: T::AccountId = whitelisted_caller();
    let dummy =
      T::AccountId::decode(&mut polkadot_sdk::sp_runtime::traits::TrailingZeroInput::zeroes())
        .unwrap();
    let pipeline = make_pipeline::<T>(dummy.clone());
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 100,
    };
    Pallet::<T>::create_system_aaa(
      RawOrigin::Root.into(),
      owner.clone(),
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      dummy,
    )
    .unwrap();
    let aaa_id = 0u64;
    let additional: BoundedVec<T::AssetId, T::MaxRefundableAssets> = BoundedVec::default();
    #[extrinsic_call]
    update_refund_assets(RawOrigin::Root, aaa_id, additional);
    assert!(AaaInstances::<T>::get(aaa_id).is_some());
  }

  #[benchmark]
  fn set_global_circuit_breaker() {
    #[extrinsic_call]
    set_global_circuit_breaker(RawOrigin::Root, true);
    assert!(GlobalCircuitBreaker::<T>::get());
  }

  #[benchmark]
  fn permissionless_sweep() {
    let caller: T::AccountId = whitelisted_caller();
    let aaa_id = bench_create_user::<T>(caller.clone());
    let actor = AaaInstances::<T>::get(aaa_id)
      .map(|i| i.sovereign_account)
      .unwrap();
    let fee_sink = T::FeeSink::get();
    // Ensure fee sink account exists so sub-ED rent transfers do not fail on first touch.
    T::AssetOps::mint(&fee_sink, T::NativeAssetId::get(), T::MinUserBalance::get()).unwrap();
    let reserve = T::MaxRentAccrual::get()
      .saturating_add(T::MinUserBalance::get())
      .saturating_add(1u32.into());
    T::AssetOps::mint(&actor, T::NativeAssetId::get(), reserve).unwrap();
    #[extrinsic_call]
    permissionless_sweep(RawOrigin::Signed(caller), aaa_id);
    assert!(AaaInstances::<T>::get(aaa_id).is_some());
  }

  #[benchmark]
  fn process_remove_liquidity_max_k() {
    let caller: T::AccountId = whitelisted_caller();
    let max_scan = T::MaxAdapterScan::get();
    assert!(max_scan > 0, "MaxAdapterScan must be greater than zero");
    let (lp_asset, lp_amount) = T::BenchmarkHelper::setup_remove_liquidity_max_k(&caller, max_scan)
      .expect("benchmark helper must seed worst-case remove-liquidity state");
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 10,
    };
    let pipeline = make_remove_liquidity_pipeline::<T>(lp_asset, lp_amount);
    Pallet::<T>::create_user_aaa(
      RawOrigin::Signed(caller.clone()).into(),
      Mutability::Mutable,
      schedule,
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    )
    .unwrap();
    let aaa_id = NextAaaId::<T>::get().saturating_sub(1);
    let actor = AaaInstances::<T>::get(aaa_id)
      .map(|i| i.sovereign_account)
      .expect("AAA actor must exist after create");
    T::AssetOps::transfer(&caller, &actor, lp_asset, lp_amount).unwrap();
    let reserve = T::MaxRentAccrual::get()
      .saturating_add(T::MinUserBalance::get())
      .saturating_add(1u32.into());
    T::AssetOps::mint(&actor, T::NativeAssetId::get(), reserve).unwrap();
    frame_system::Pallet::<T>::set_block_number(1u32.into());
    Pallet::<T>::manual_trigger(RawOrigin::Signed(caller).into(), aaa_id).unwrap();
    #[block]
    {
      let _ = Pallet::<T>::on_idle(1u32.into(), Weight::from_parts(u64::MAX, u64::MAX));
    }
    let inst = AaaInstances::<T>::get(aaa_id).expect("AAA actor must survive benchmark cycle");
    assert_eq!(inst.cycle_nonce, 1);
    assert_eq!(inst.consecutive_failures, 0);
  }

  // Worst-case deferred retry path: ready ring is full, then exactly MaxDeferredRetriesPerBlock slots open.
  #[benchmark]
  fn process_deferred_retry_max_retries() {
    let max_ready = T::MaxReadyRingLength::get();
    let max_retries = T::MaxDeferredRetriesPerBlock::get().max(1);
    let total = max_ready.saturating_add(max_retries);
    for idx in 0u32..total {
      let owner: T::AccountId = account("aaa-owner", idx, 0);
      let _ = bench_create_user::<T>(owner);
    }
    let deferred_before = DeferredRing::<T>::get().len() as u32;
    assert!(deferred_before >= max_retries);
    ReadyRing::<T>::mutate(|ring| {
      let mut freed = 0u32;
      while freed < max_retries && !ring.is_empty() {
        ring.remove(0);
        freed = freed.saturating_add(1);
      }
    });
    frame_system::Pallet::<T>::set_block_number(1u32.into());
    let ready_before = ReadyRing::<T>::get().len() as u32;
    let deferred_before = DeferredRing::<T>::get().len() as u32;
    #[block]
    {
      let _ = Pallet::<T>::on_idle(1u32.into(), Weight::from_parts(1, 1));
    }
    let ready_after = ReadyRing::<T>::get().len() as u32;
    let deferred_after = DeferredRing::<T>::get().len() as u32;
    let moved = ready_after.saturating_sub(ready_before);
    assert!(moved <= max_retries);
    assert_eq!(deferred_before.saturating_sub(deferred_after), moved);
  }

  #[cfg(test)]
  use crate::mock::{Test, new_test_ext};
  #[cfg(test)]
  impl_benchmark_test_suite!(Pallet, new_test_ext(), Test);
}
