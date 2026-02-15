use crate::{
  AaaPolicy, AaaType, AmountSpec, Condition, DeferReason, Error, Event, Mutability, OwnerSlots,
  PauseReason, PipelineErrorPolicy, Schedule, SourceFilter, SplitLeg, Step, TaskKind, Trigger,
  mock::*,
};
use polkadot_sdk::frame_support::{
  BoundedVec, assert_noop, assert_ok,
  traits::{ConstU32, Get, Hooks},
};
use polkadot_sdk::{frame_system, sp_runtime::Weight};

const RENT: Balance = 1_000_000;

fn default_schedule() -> Schedule<TestAsset, AccountId> {
  Schedule {
    trigger: Trigger::Manual,
    cooldown_blocks: 0,
  }
}

fn make_step(
  task: TaskKind<TestAsset, Balance, AccountId>,
) -> Step<TestAsset, Balance, AccountId, ConstU32<4>> {
  Step {
    conditions: BoundedVec::default(),
    task,
    on_error: PipelineErrorPolicy::AbortCycle,
  }
}

fn make_step_cond(
  conditions: Vec<Condition<TestAsset, Balance>>,
  task: TaskKind<TestAsset, Balance, AccountId>,
  on_error: PipelineErrorPolicy,
) -> Step<TestAsset, Balance, AccountId, ConstU32<4>> {
  Step {
    conditions: BoundedVec::try_from(conditions).unwrap(),
    task,
    on_error,
  }
}

fn transfer_step(
  to: AccountId,
  amount: Balance,
) -> Step<TestAsset, Balance, AccountId, ConstU32<4>> {
  make_step(TaskKind::Transfer {
    to,
    asset: TestAsset::Native,
    amount: AmountSpec::Fixed(amount),
  })
}

fn single_pipeline() -> crate::PipelineOf<Test> {
  vec![transfer_step(BOB, 100)].try_into().unwrap()
}

fn fund(aaa_id: u64, amount: Balance) {
  assert_ok!(AAA::fund_aaa(RuntimeOrigin::signed(ALICE), aaa_id, amount));
}

fn create_user(owner: AccountId) -> u64 {
  let id = AAA::next_aaa_id();
  assert_ok!(AAA::create_user_aaa(
    RuntimeOrigin::signed(owner),
    Mutability::Mutable,
    default_schedule(),
    None,
    single_pipeline(),
    AaaPolicy::default(),
    None,
  ));
  id
}

fn create_system() -> u64 {
  let id = AAA::next_aaa_id();
  assert_ok!(AAA::create_system_aaa(
    RuntimeOrigin::root(),
    ALICE,
    default_schedule(),
    None,
    single_pipeline(),
    AaaPolicy::default(),
    ALICE,
  ));
  id
}

fn sovereign(aaa_id: u64) -> AccountId {
  AAA::aaa_instances(aaa_id)
    .map(|inst| inst.sovereign_account)
    .expect("AAA must exist")
}

fn run_idle(block: u64) {
  AAA::on_idle(block, Weight::from_parts(1_000_000_000, 100_000));
}

fn run_block(n: u64) {
  frame_system::Pallet::<Test>::set_block_number(n);
  AAA::on_initialize(n);
  run_idle(n);
}

#[test]
fn create_user_aaa_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let inst = AAA::aaa_instances(id).unwrap();
    assert_eq!(inst.owner, ALICE);
    assert_eq!(inst.aaa_type, AaaType::User);
    assert!(!inst.is_paused);
    assert_eq!(inst.pause_reason, None);
    assert_eq!(inst.cycle_nonce, 0);
    assert_eq!(inst.consecutive_failures, 0);
    assert!(!inst.manual_trigger_pending);
    assert_eq!(AAA::next_aaa_id(), id + 1);
    assert!(AAA::ready_ring().contains(&id));
    assert!(inst.refund_assets.contains(&TestAsset::Native));
    assert!(inst.sovereign_account == sovereign(id));
  });
}

#[test]
fn owner_slots_allocate_from_zero_first_free() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);

    let id0 = create_user(ALICE);
    let id1 = create_user(ALICE);
    let id2 = create_user(ALICE);

    assert_eq!(AAA::aaa_instances(id0).unwrap().owner_slot, 0);
    assert_eq!(AAA::aaa_instances(id1).unwrap().owner_slot, 1);
    assert_eq!(AAA::aaa_instances(id2).unwrap().owner_slot, 2);
  });
}

#[test]
fn owner_slot_reused_after_destroy() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);

    let id0 = create_user(ALICE);
    let id1 = create_user(ALICE);

    let slot0 = AAA::aaa_instances(id0).unwrap().owner_slot;
    let slot1 = AAA::aaa_instances(id1).unwrap().owner_slot;
    assert_eq!(slot0, 0);
    assert_eq!(slot1, 1);

    fund(id1, 500);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id1));

    let id2 = create_user(ALICE);
    let slot2 = AAA::aaa_instances(id2).unwrap().owner_slot;
    assert_eq!(slot2, 1, "must reuse first free slot after destruction");
  });
}

#[test]
fn owner_slot_capacity_is_enforced() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);

    for slot in 0..64u16 {
      OwnerSlots::<Test>::insert(ALICE, slot, slot as u64 + 1);
    }

    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::OwnerSlotCapacityExceeded
    );
  });
}

#[test]
fn create_user_aaa_emits_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let sv = sovereign(id);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::AAACreated {
      aaa_id: id,
      owner: ALICE,
      owner_slot: 0,
      aaa_type: AaaType::User,
      mutability: Mutability::Mutable,
      sovereign_account: sv,
    }));
  });
}

#[test]
fn create_user_aaa_rejects_pipeline_too_long() {
  new_test_ext().execute_with(|| {
    // MaxUserPipelineSteps = 3; long pipeline = 4 steps
    let pipeline: crate::PipelineOf<Test> = vec![
      transfer_step(BOB, 10),
      transfer_step(BOB, 20),
      transfer_step(BOB, 30),
      transfer_step(BOB, 40),
    ]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::PipelineTooLong
    );
  });
}

#[test]
fn create_user_aaa_rejects_mint_task() {
  new_test_ext().execute_with(|| {
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Mint {
      asset: TestAsset::Local(1),
      amount: AmountSpec::Fixed(100),
    })]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::MintNotAllowedForUserAaa
    );
  });
}

#[test]
fn mint_task_is_rejected_for_user_even_if_injected_post_creation() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    crate::pallet::AaaInstances::<Test>::mutate(id, |maybe| {
      if let Some(inst) = maybe.as_mut() {
        inst.pipeline = vec![make_step(TaskKind::Mint {
          asset: TestAsset::Local(99),
          amount: AmountSpec::Fixed(500),
        })]
        .try_into()
        .unwrap();
      }
    });
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(get_total_minted(TestAsset::Local(99)), 0);
    assert_eq!(AAA::aaa_instances(id).unwrap().consecutive_failures, 1);
  });
}

#[test]
fn create_user_aaa_rejects_empty_pipeline() {
  new_test_ext().execute_with(|| {
    let pipeline: crate::PipelineOf<Test> = vec![].try_into().unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::EmptyPipeline
    );
  });
}

#[test]
fn create_system_aaa_allows_mint_task() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Mint {
      asset: TestAsset::Local(1),
      amount: AmountSpec::Fixed(500),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_system_aaa(
      RuntimeOrigin::root(),
      ALICE,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      ALICE,
    ));
  });
}

#[test]
fn create_user_aaa_rejected_when_breaker_active() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::GlobalCircuitBreakerActive
    );
  });
}

#[test]
fn create_system_aaa_rejected_when_breaker_active() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    assert_noop!(
      AAA::create_system_aaa(
        RuntimeOrigin::root(),
        ALICE,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        ALICE,
      ),
      Error::<Test>::GlobalCircuitBreakerActive
    );
  });
}

#[test]
fn create_system_aaa_rejects_refund_assets_overflow() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let mut steps = Vec::new();
    for i in 0u32..8 {
      let asset_in = TestAsset::Local(100 + i * 2);
      let asset_out = TestAsset::Local(101 + i * 2);
      steps.push(make_step(TaskKind::SwapExactIn {
        asset_in,
        asset_out,
        amount_in: AmountSpec::Fixed(1),
        min_out: 1,
      }));
    }
    let pipeline: crate::PipelineOf<Test> = steps.try_into().unwrap();
    assert_noop!(
      AAA::create_system_aaa(
        RuntimeOrigin::root(),
        ALICE,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        ALICE,
      ),
      Error::<Test>::RefundAssetsOverflow
    );
  });
}

#[test]
fn create_multiple_aaas_increments_id() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id0 = create_user(ALICE);
    let id1 = create_user(ALICE);
    let id2 = create_user(BOB);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(AAA::owner_index(ALICE).len(), 2);
    assert_eq!(AAA::owner_index(BOB).len(), 1);
  });
}

#[test]
fn pause_aaa_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id));
    let inst = AAA::aaa_instances(id).unwrap();
    assert!(inst.is_paused);
    assert_eq!(inst.pause_reason, Some(PauseReason::Manual));
    assert!(!AAA::ready_ring().contains(&id));
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::AAAPaused {
      aaa_id: id,
      reason: PauseReason::Manual,
    }));
  });
}

#[test]
fn pause_already_paused_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id));
    assert_noop!(
      AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id),
      Error::<Test>::AlreadyPaused
    );
  });
}

#[test]
fn resume_aaa_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id));
    assert_ok!(AAA::resume_aaa(RuntimeOrigin::signed(ALICE), id));
    let inst = AAA::aaa_instances(id).unwrap();
    assert!(!inst.is_paused);
    assert_eq!(inst.pause_reason, None);
    assert!(AAA::ready_ring().contains(&id));
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::AAAResumed {
      aaa_id: id,
    }));
  });
}

#[test]
fn resume_not_paused_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_noop!(
      AAA::resume_aaa(RuntimeOrigin::signed(ALICE), id),
      Error::<Test>::NotPaused
    );
  });
}

#[test]
fn non_owner_cannot_pause() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_noop!(
      AAA::pause_aaa(RuntimeOrigin::signed(BOB), id),
      Error::<Test>::NotOwner
    );
  });
}

#[test]
fn immutable_aaa_cannot_be_paused() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Immutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    assert_noop!(
      AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id),
      Error::<Test>::ImmutableActor
    );
  });
}

#[test]
fn root_can_control_system_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_system();
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::root(), id));
    assert!(AAA::aaa_instances(id).unwrap().is_paused);
    assert_ok!(AAA::resume_aaa(RuntimeOrigin::root(), id));
    assert!(!AAA::aaa_instances(id).unwrap().is_paused);
  });
}

#[test]
fn manual_trigger_sets_flag() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    assert!(AAA::aaa_instances(id).unwrap().manual_trigger_pending);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::ManualTriggerSet {
      aaa_id: id,
    }));
  });
}

#[test]
fn manual_trigger_on_paused_aaa_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id));
    assert_noop!(
      AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id),
      Error::<Test>::AlreadyPaused
    );
  });
}

#[test]
fn fund_aaa_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let sv = sovereign(id);
    let alice_before = Balances::free_balance(ALICE);
    fund(id, 500);
    assert_eq!(Balances::free_balance(sv), 500);
    assert_eq!(Balances::free_balance(ALICE), alice_before - 500);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::AAAFunded {
      aaa_id: id,
      from: ALICE,
      amount: 500,
    }));
  });
}

#[test]
fn fund_aaa_zero_fails() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_noop!(
      AAA::fund_aaa(RuntimeOrigin::signed(ALICE), id, 0),
      Error::<Test>::AmountZero
    );
  });
}

#[test]
fn refund_and_close_solvent_path() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      Some(BOB),
    ));
    let sv = sovereign(id);
    // Fund above refund threshold (1 asset × TestRefundTransferCost=101)
    fund(id, 500);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id));
    assert!(AAA::aaa_instances(id).is_none());
    assert_eq!(Balances::free_balance(sv), 0);
    assert_eq!(Balances::free_balance(BOB), bob_before + 500);
    assert!(!AAA::ready_ring().contains(&id));
  });
}

#[test]
fn refund_and_close_insolvent_path_burns_native() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      Some(BOB),
    ));
    // Fund below threshold — 1 asset needs 101 native
    fund(id, 50);
    let sv = sovereign(id);
    let bob_before = Balances::free_balance(BOB);
    let fee_sink_before = Balances::free_balance(999);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id));
    assert!(AAA::aaa_instances(id).is_none());
    // Insolvent: native burned, bob gets nothing
    assert_eq!(Balances::free_balance(BOB), bob_before);
    // sovereign drained
    assert_eq!(Balances::free_balance(sv), 0);
    let _ = fee_sink_before; // fee_sink not affected for native burn
  });
}

#[test]
fn refund_and_close_emits_destroyed_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 500);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id));
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::AAADestroyed {
      aaa_id: id,
    }));
  });
}

#[test]
fn non_owner_cannot_refund_and_close() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    assert_noop!(
      AAA::refund_and_close(RuntimeOrigin::signed(BOB), id),
      Error::<Test>::NotOwner
    );
  });
}

#[test]
fn update_policy_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let new_policy = AaaPolicy {
      default_error_policy: PipelineErrorPolicy::ContinueNextStep,
    };
    assert_ok!(AAA::update_policy(
      RuntimeOrigin::signed(ALICE),
      id,
      new_policy
    ));
    assert_eq!(
      AAA::aaa_instances(id).unwrap().policy.default_error_policy,
      PipelineErrorPolicy::ContinueNextStep
    );
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::PolicyUpdated {
      aaa_id: id,
    }));
  });
}

#[test]
fn update_policy_rejected_for_immutable() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Immutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    assert_noop!(
      AAA::update_policy(RuntimeOrigin::signed(ALICE), id, AaaPolicy::default()),
      Error::<Test>::ImmutableActor
    );
  });
}

#[test]
fn update_schedule_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let sched = Schedule {
      trigger: Trigger::ProbabilisticTimer {
        every_blocks: 5,
        probability_ppm: 1_000_000,
      },
      cooldown_blocks: 3,
    };
    assert_ok!(AAA::update_schedule(
      RuntimeOrigin::signed(ALICE),
      id,
      sched.clone(),
      None
    ));
    let inst = AAA::aaa_instances(id).unwrap();
    assert_eq!(inst.schedule.cooldown_blocks, 3);
    assert!(matches!(
      inst.schedule.trigger,
      Trigger::ProbabilisticTimer {
        every_blocks: 5,
        ..
      }
    ));
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::ScheduleUpdated {
      aaa_id: id,
    }));
  });
}

#[test]
fn update_schedule_rejected_for_immutable() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Immutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    assert_noop!(
      AAA::update_schedule(RuntimeOrigin::signed(ALICE), id, default_schedule(), None),
      Error::<Test>::ImmutableActor
    );
  });
}

#[test]
fn update_refund_assets_appends_system_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_system();
    let before = AAA::aaa_instances(id).unwrap().refund_assets.len();
    let extra: BoundedVec<TestAsset, ConstU32<16>> = vec![TestAsset::Local(42)].try_into().unwrap();
    assert_ok!(AAA::update_refund_assets(RuntimeOrigin::root(), id, extra));
    assert_eq!(
      AAA::aaa_instances(id).unwrap().refund_assets.len(),
      before + 1
    );
    assert!(
      AAA::aaa_instances(id)
        .unwrap()
        .refund_assets
        .contains(&TestAsset::Local(42))
    );
  });
}

#[test]
fn update_refund_assets_no_duplicates() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_system();
    let before = AAA::aaa_instances(id).unwrap().refund_assets.len();
    let same: BoundedVec<TestAsset, ConstU32<16>> = vec![TestAsset::Native].try_into().unwrap();
    assert_ok!(AAA::update_refund_assets(RuntimeOrigin::root(), id, same));
    assert_eq!(AAA::aaa_instances(id).unwrap().refund_assets.len(), before);
  });
}

#[test]
fn update_refund_assets_rejected_for_user_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let extra: BoundedVec<TestAsset, ConstU32<16>> = vec![TestAsset::Local(1)].try_into().unwrap();
    assert_noop!(
      AAA::update_refund_assets(RuntimeOrigin::root(), id, extra),
      Error::<Test>::NotGovernance
    );
  });
}

#[test]
fn global_circuit_breaker_blocks_execution() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    run_idle(1);
    // Nonce stays 0 — no execution
    assert_eq!(AAA::aaa_instances(id).unwrap().cycle_nonce, 0);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(
      Event::GlobalCircuitBreakerSet { paused: true },
    ));
  });
}

#[test]
fn global_circuit_breaker_toggle() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    assert!(AAA::global_circuit_breaker());
    assert_ok!(AAA::set_global_circuit_breaker(
      RuntimeOrigin::root(),
      false
    ));
    assert!(!AAA::global_circuit_breaker());
  });
}

#[test]
fn cycle_executes_transfer_via_manual_trigger() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 5_000);
    let bob_before = Balances::free_balance(BOB);

    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);

    // Transfer of 100 native to BOB
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
    let inst = AAA::aaa_instances(id).unwrap();
    assert_eq!(inst.cycle_nonce, 1);
    assert!(!inst.manual_trigger_pending);
  });
}

#[test]
fn cycle_increments_nonce_each_run() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 100_000);

    for expected_nonce in 1u64..=3 {
      assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
      run_idle(expected_nonce);
      assert_eq!(AAA::aaa_instances(id).unwrap().cycle_nonce, expected_nonce);
    }
  });
}

#[test]
fn cycle_emits_started_and_executed_events() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::CycleStarted {
      aaa_id: id,
      cycle_nonce: 1,
    }));
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::PipelineExecuted {
      aaa_id: id,
      cycle_nonce: 1,
      steps_executed: 1,
    }));
  });
}

#[test]
fn paused_aaa_not_executed() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    assert_ok!(AAA::pause_aaa(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(AAA::aaa_instances(id).unwrap().cycle_nonce, 0);
  });
}

#[test]
fn condition_balance_above_skips_when_false() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 5_000);
    let sv = sovereign(id);

    // pipeline: transfer only if Local(1) balance > 9999 (not met)
    let asset = TestAsset::Local(1);
    set_asset_balance(sv, asset, 100);
    let pipeline: crate::PipelineOf<Test> = vec![make_step_cond(
      vec![Condition::BalanceAbove {
        asset,
        threshold: 9999,
      }],
      TaskKind::Transfer {
        to: BOB,
        asset: TestAsset::Native,
        amount: AmountSpec::Fixed(100),
      },
      PipelineErrorPolicy::AbortCycle,
    )]
    .try_into()
    .unwrap();

    let id2 = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(BOB),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id2, 5_000);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(BOB), id2));
    run_idle(1);
    // Step skipped — no transfer
    assert_eq!(Balances::free_balance(BOB), bob_before);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::StepSkipped {
      aaa_id: id2,
      cycle_nonce: 1,
      step: 0,
    }));
  });
}

#[test]
fn condition_balance_above_executes_when_true() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset = TestAsset::Local(1);
    let pipeline: crate::PipelineOf<Test> = vec![make_step_cond(
      vec![Condition::BalanceAbove {
        asset,
        threshold: 50,
      }],
      TaskKind::Transfer {
        to: BOB,
        asset: TestAsset::Native,
        amount: AmountSpec::Fixed(100),
      },
      PipelineErrorPolicy::AbortCycle,
    )]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    set_asset_balance(sovereign(id), asset, 200);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
  });
}

#[test]
fn multiple_conditions_and_semantics() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let a1 = TestAsset::Local(1);
    let a2 = TestAsset::Local(2);
    // Both must pass (AND): a1 > 50 AND a2 < 1000
    let pipeline: crate::PipelineOf<Test> = vec![make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: a1,
          threshold: 50,
        },
        Condition::BalanceBelow {
          asset: a2,
          threshold: 1000,
        },
      ],
      TaskKind::Transfer {
        to: BOB,
        asset: TestAsset::Native,
        amount: AmountSpec::Fixed(100),
      },
      PipelineErrorPolicy::AbortCycle,
    )]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let sv = sovereign(id);
    set_asset_balance(sv, a1, 200); // passes
    set_asset_balance(sv, a2, 2000); // fails
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // Second condition fails → step skipped
    assert_eq!(Balances::free_balance(BOB), bob_before);
  });
}

#[test]
fn error_policy_abort_cycle() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    // Step 0 fails (transfer 0 amount → AmountZero), AbortCycle → step 1 never runs
    let pipeline: crate::PipelineOf<Test> = vec![
      make_step(TaskKind::Transfer {
        to: BOB,
        asset: TestAsset::Native,
        amount: AmountSpec::Fixed(0),
      }),
      transfer_step(BOB, 100),
    ]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // No transfer because abort on step 0
    assert_eq!(Balances::free_balance(BOB), bob_before);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::PipelineFailed {
      aaa_id: id,
      cycle_nonce: 1,
      failed_step: 0,
      error: polkadot_sdk::sp_runtime::DispatchError::from(Error::<Test>::AmountZero),
    }));
  });
}

#[test]
fn error_policy_continue_next_step() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    // Step 0: fails (amount=0), ContinueNextStep
    // Step 1: succeeds (100 to BOB)
    let pipeline: crate::PipelineOf<Test> = vec![
      make_step_cond(
        vec![],
        TaskKind::Transfer {
          to: BOB,
          asset: TestAsset::Native,
          amount: AmountSpec::Fixed(0),
        },
        PipelineErrorPolicy::ContinueNextStep,
      ),
      transfer_step(BOB, 100),
    ]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // Step 1 executed despite step 0 failure
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
  });
}

#[test]
fn consecutive_failures_increment_and_auto_refund() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    // Pipeline always fails: transfer 0
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Transfer {
      to: BOB,
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(0),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    // Fund enough for rent + refund threshold
    fund(id, 50_000);
    // MaxConsecutiveFailures = 3; need > 3 failures → 4 cycles
    for i in 1u64..=4 {
      assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
      run_idle(i);
    }
    // Should be auto-refunded after > MaxConsecutiveFailures
    assert!(AAA::aaa_instances(id).is_none());
  });
}

#[test]
fn consecutive_failures_reset_on_success() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    // Pipeline: 2 steps - first always fails, second always succeeds (ContinueNextStep)
    let pipeline: crate::PipelineOf<Test> = vec![
      make_step_cond(
        vec![],
        TaskKind::Transfer {
          to: BOB,
          asset: TestAsset::Native,
          amount: AmountSpec::Fixed(0),
        },
        PipelineErrorPolicy::ContinueNextStep,
      ),
      transfer_step(BOB, 1),
    ]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 100_000);
    for i in 1u64..=5 {
      assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
      run_idle(i);
    }
    // Pipeline as a whole succeeded (step 1 ran) → consecutive_failures stays 0
    let inst = AAA::aaa_instances(id).unwrap();
    assert_eq!(inst.consecutive_failures, 0);
    assert_eq!(inst.cycle_nonce, 5);
  });
}

#[test]
fn rent_charged_on_cycle() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    // Fund enough for rent across many blocks (5 blocks × 1_000_000 = 5_000_000)
    // plus refund threshold (101) and transfer (100)
    fund(id, 100_000_000);
    let sv = sovereign(id);
    // Move to block 6 — 5 blocks of rent should accrue
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    frame_system::Pallet::<Test>::set_block_number(6);
    let before = Balances::free_balance(sv);
    run_idle(6);
    // 5 blocks × RENT = 5_000_000
    let expected_rent = 5 * RENT;
    let after = Balances::free_balance(sv);
    assert!(
      before.saturating_sub(after) >= expected_rent,
      "rent not charged: before={before} after={after} diff={} expected={expected_rent}",
      before.saturating_sub(after)
    );
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::RentCharged {
      aaa_id: id,
      blocks_elapsed: 5,
      rent_due: expected_rent,
      rent_paid: expected_rent,
      rent_debt: 0,
    }));
  });
}

#[test]
fn rent_ceiling_caps_accrual() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 2_000_000_000);
    frame_system::Pallet::<Test>::set_block_number(10_000);
    assert_ok!(AAA::permissionless_sweep(RuntimeOrigin::signed(BOB), id));
    assert!(AAA::aaa_instances(id).is_some());
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::RentCharged {
      aaa_id: id,
      blocks_elapsed: 9_999,
      rent_due: TestMaxRentAccrual::get(),
      rent_paid: TestMaxRentAccrual::get(),
      rent_debt: 0,
    }));
  });
}

#[test]
fn system_aaa_not_charged_rent() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_system();
    let sv = sovereign(id);
    // No balance needed
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    frame_system::Pallet::<Test>::set_block_number(10);
    run_idle(10);
    // No RentCharged event for system AAA
    let events: Vec<_> = frame_system::Pallet::<Test>::events()
      .into_iter()
      .filter(|e| matches!(&e.event, RuntimeEvent::AAA(Event::RentCharged { aaa_id, .. }) if *aaa_id == id))
      .collect();
    assert!(events.is_empty(), "system AAA should not be charged rent");
    let _ = sv;
  });
}

#[test]
fn rent_insolvent_triggers_auto_refund() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    // Fund exactly enough for refund threshold (1 asset × 101) but not rent
    fund(id, 200);
    // Skip many blocks to accrue rent > balance
    frame_system::Pallet::<Test>::set_block_number(500);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(500);
    // Rent insolvent → auto-refund (insolvent path since 200 < rent due)
    assert!(AAA::aaa_instances(id).is_none());
  });
}

#[test]
fn min_user_balance_triggers_auto_refund() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    // Fund below MinUserBalance (50) — exact 40
    fund(id, 40);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // Balance below minimum → BalanceExhausted refund
    assert!(AAA::aaa_instances(id).is_none());
  });
}

#[test]
fn refund_assets_computed_from_pipeline() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let pipeline: crate::PipelineOf<Test> = vec![
      make_step(TaskKind::SwapExactIn {
        asset_in: TestAsset::Native,
        asset_out: TestAsset::Local(5),
        amount_in: AmountSpec::Fixed(100),
        min_out: 1,
      }),
      make_step(TaskKind::Transfer {
        to: BOB,
        asset: TestAsset::Local(7),
        amount: AmountSpec::Fixed(50),
      }),
    ]
    .try_into()
    .unwrap();
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    let inst = AAA::aaa_instances(id).unwrap();
    assert!(inst.refund_assets.contains(&TestAsset::Native));
    assert!(inst.refund_assets.contains(&TestAsset::Local(5)));
    assert!(inst.refund_assets.contains(&TestAsset::Local(7)));
  });
}

#[test]
fn task_swap_exact_in_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset_out = TestAsset::Local(1);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SwapExactIn {
      asset_in: TestAsset::Native,
      asset_out,
      amount_in: AmountSpec::Fixed(100),
      min_out: 1,
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    set_pool_reserves(TestAsset::Native, asset_out, 10_000, 10_000);
    // Pre-fund the mock pool account with asset_out so transfer succeeds
    set_asset_balance(u64::MAX, asset_out, 10_000);
    let sv = sovereign(id);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    let out_bal = get_asset_balance(sv, asset_out);
    assert!(out_bal > 0, "should have received asset_out");
  });
}

#[test]
fn task_burn_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset = TestAsset::Local(1);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Burn {
      asset,
      amount: AmountSpec::Fixed(50),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let sv = sovereign(id);
    set_asset_balance(sv, asset, 200);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(get_asset_balance(sv, asset), 150);
    assert_eq!(get_total_burned(asset), 50);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::BurnExecuted {
      aaa_id: id,
      asset,
      amount: 50,
    }));
  });
}

#[test]
fn task_mint_executes_for_system_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(7);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Mint {
      asset,
      amount: AmountSpec::Fixed(500),
    })]
    .try_into()
    .unwrap();
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_system_aaa(
      RuntimeOrigin::root(),
      ALICE,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      ALICE,
    ));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(get_total_minted(asset), 500);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::MintExecuted {
      aaa_id: id,
      asset,
      amount: 500,
    }));
  });
}

#[test]
fn task_noop_executes_cleanly() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Noop)].try_into().unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // Noop step counts as executed (steps_executed incremented)
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::PipelineExecuted {
      aaa_id: id,
      cycle_nonce: 1,
      steps_executed: 1,
    }));
  });
}

#[test]
fn task_split_transfer_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg { to: BOB, share: 60 },
      SplitLeg {
        to: CHARLIE,
        share: 40,
      },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(100),
      total_shares: 100,
      legs,
      remainder_to: None,
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let bob_before = Balances::free_balance(BOB);
    let charlie_before = Balances::free_balance(CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 60);
    assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 40);
  });
}

#[test]
fn split_transfer_requires_at_least_two_legs() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![SplitLeg {
      to: BOB,
      share: 100,
    }]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(100),
      total_shares: 100,
      legs,
      remainder_to: None,
    })]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::InsufficientSplitLegs
    );
  });
}

#[test]
fn split_transfer_rejects_zero_share_leg() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg {
        to: BOB,
        share: 100,
      },
      SplitLeg {
        to: CHARLIE,
        share: 0,
      },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(100),
      total_shares: 100,
      legs,
      remainder_to: None,
    })]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::ZeroShareLeg
    );
  });
}

#[test]
fn split_transfer_rejects_duplicate_recipients() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg { to: BOB, share: 50 },
      SplitLeg { to: BOB, share: 50 },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(100),
      total_shares: 100,
      legs,
      remainder_to: None,
    })]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::DuplicateRecipient
    );
  });
}

#[test]
fn split_transfer_rejects_total_share_mismatch() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg { to: BOB, share: 60 },
      SplitLeg {
        to: CHARLIE,
        share: 40,
      },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(100),
      total_shares: 120,
      legs,
      remainder_to: None,
    })]
    .try_into()
    .unwrap();
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::SplitTransferInvalid
    );
  });
}

#[test]
fn split_transfer_remainder_to_target_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg { to: BOB, share: 50 },
      SplitLeg {
        to: CHARLIE,
        share: 50,
      },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset: TestAsset::Native,
      amount: AmountSpec::Fixed(101),
      total_shares: 100,
      legs,
      remainder_to: Some(CHARLIE),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let bob_before = Balances::free_balance(BOB);
    let charlie_before = Balances::free_balance(CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 50);
    assert_eq!(Balances::free_balance(CHARLIE), charlie_before + 51);
  });
}

#[test]
fn task_add_liquidity_works() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset_b = TestAsset::Local(1);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::AddLiquidity {
      asset_a: TestAsset::Native,
      asset_b,
      amount_a: AmountSpec::Fixed(100),
      amount_b: AmountSpec::Fixed(100),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let sv = sovereign(id);
    set_asset_balance(sv, asset_b, 1000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::LiquidityAdded {
      aaa_id: id,
      asset_a: TestAsset::Native,
      asset_b,
      amount_a: 100,
      amount_b: 100,
      lp_minted: 100,
    }));
  });
}

#[test]
fn task_amount_spec_percentage_resolves() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Transfer {
      to: BOB,
      asset: TestAsset::Native,
      amount: AmountSpec::Percentage(polkadot_sdk::sp_runtime::Permill::from_percent(50)),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 1000);
    let bob_before = Balances::free_balance(BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    // 50% of ~1000 (minus rent = 0 at block 1 since no blocks elapsed)
    let transferred = Balances::free_balance(BOB) - bob_before;
    assert!(transferred > 0, "should have transferred some");
  });
}

#[test]
fn task_amount_spec_all_balance_resolves() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset = TestAsset::Local(5);
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::Transfer {
      to: BOB,
      asset,
      amount: AmountSpec::AllBalance,
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    let sv = sovereign(id);
    set_asset_balance(sv, asset, 777);
    let bob_asset_before = get_asset_balance(BOB, asset);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert_eq!(get_asset_balance(BOB, asset), bob_asset_before + 777);
    assert_eq!(get_asset_balance(sv, asset), 0);
  });
}

#[test]
fn address_event_trigger_fires_on_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Single,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    AAA::notify_address_event(id, asset, &BOB);
    let bob_before = Balances::free_balance(BOB);
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
  });
}

#[test]
fn address_event_trigger_does_not_fire_without_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Single,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 5_000);
    // No notify_address_event → should not execute
    let bob_before = Balances::free_balance(BOB);
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before);
  });
}

#[test]
fn address_event_consume_decrements_count() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Single,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 50_000);
    AAA::notify_address_event(id, asset, &BOB);
    AAA::notify_address_event(id, asset, &BOB);
    let bob_before = Balances::free_balance(BOB);
    run_idle(1); // consumes 1 event
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
    run_idle(2); // second event available → fires again
    assert_eq!(Balances::free_balance(BOB), bob_before + 200);
  });
}

#[test]
fn address_event_batch_mode_consumes_up_to_batch_size() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Batch(2),
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 50_000);
    AAA::notify_address_event(id, asset, &BOB);
    AAA::notify_address_event(id, asset, &BOB);
    AAA::notify_address_event(id, asset, &BOB);
    let bob_before = Balances::free_balance(BOB);
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
    run_idle(2);
    assert_eq!(Balances::free_balance(BOB), bob_before + 200);
    run_idle(3);
    assert_eq!(Balances::free_balance(BOB), bob_before + 200);
  });
}

#[test]
fn address_event_drain_mode_consumes_all_pending() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Drain,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 50_000);
    AAA::notify_address_event(id, asset, &BOB);
    AAA::notify_address_event(id, asset, &BOB);
    AAA::notify_address_event(id, asset, &BOB);
    let bob_before = Balances::free_balance(BOB);
    run_idle(1);
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
    run_idle(2);
    assert_eq!(Balances::free_balance(BOB), bob_before + 100);
  });
}

#[test]
fn address_event_single_mode_keeps_saturation_flag() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Single,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 50_000);
    for _ in 0u32..10 {
      AAA::notify_address_event(id, asset, &BOB);
    }
    let before = AAA::event_inbox(id, asset).expect("inbox entry must exist after notify");
    assert!(before.saturated);
    let bob_before = Balances::free_balance(BOB);
    run_idle(1);
    let after = AAA::event_inbox(id, asset).expect("single mode keeps saturated entry");
    assert!(after.saturated);
    run_idle(2);
    assert_eq!(Balances::free_balance(BOB), bob_before + 200);
  });
}

#[test]
fn address_event_batch_mode_validates_bounds() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let asset = TestAsset::Local(1);
    let filter = crate::AssetFilter::IncludeOnly(vec![asset].try_into().unwrap());
    let bad_zero = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter.clone(),
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Batch(0),
      },
      cooldown_blocks: 0,
    };
    let bad_over = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: filter,
        source_filter: SourceFilter::Any,
        drain_mode: crate::InboxDrainMode::Batch(9),
      },
      cooldown_blocks: 0,
    };
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        bad_zero,
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::InvalidDrainMode
    );
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        bad_over,
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::InvalidDrainMode
    );
  });
}

#[test]
fn probabilistic_timer_100pct_fires_every_block() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let schedule = Schedule {
      trigger: Trigger::ProbabilisticTimer {
        every_blocks: 1,
        probability_ppm: 1_000_000,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    // Fund enough to survive rent across 3 blocks + transfers + refund threshold
    fund(id, 1_000_000_000);
    let bob_before = Balances::free_balance(BOB);
    run_block(1);
    run_block(2);
    run_block(3);
    // 3 blocks × 100 transfer = 300 to BOB
    assert_eq!(Balances::free_balance(BOB), bob_before + 300);
  });
}

#[test]
fn probabilistic_timer_0pct_never_fires() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let schedule = Schedule {
      trigger: Trigger::ProbabilisticTimer {
        every_blocks: 1,
        probability_ppm: 0,
      },
      cooldown_blocks: 0,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    // Fund enough to survive rent across 5 blocks (5 × 1_000_000 + refund threshold)
    fund(id, 1_000_000_000);
    let bob_before = Balances::free_balance(BOB);
    for b in 1u64..=5 {
      run_block(b);
    }
    assert_eq!(Balances::free_balance(BOB), bob_before);
    assert_eq!(AAA::aaa_instances(id).unwrap().cycle_nonce, 0);
  });
}

#[test]
fn cooldown_blocks_prevent_rapid_execution() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let schedule = Schedule {
      trigger: Trigger::ProbabilisticTimer {
        every_blocks: 1,
        probability_ppm: 1_000_000,
      },
      cooldown_blocks: 5,
    };
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 1_000_000_000);
    let bob_before = Balances::free_balance(BOB);
    run_block(1); // nonce 0 → no cooldown check
    let nonce_after_1 = AAA::aaa_instances(id).map(|i| i.cycle_nonce).unwrap_or(0);
    run_block(2); // only 1 block since last, cooldown=5
    run_block(3);
    run_block(4);
    run_block(5);
    let nonce_after_5 = AAA::aaa_instances(id).map(|i| i.cycle_nonce).unwrap_or(0);
    run_block(6);
    let nonce_after_6 = AAA::aaa_instances(id).map(|i| i.cycle_nonce).unwrap_or(0);
    // First cycle at block 1, next cooldown expires at block 6
    assert_eq!(nonce_after_1, 1);
    assert_eq!(
      nonce_after_5, 1,
      "cooldown prevents re-execution before block 6"
    );
    assert_eq!(nonce_after_6, 2);
    let _ = bob_before;
  });
}

#[test]
fn permissionless_sweep_triggers_rent_insolvent_refund() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    // Very low balance — won't survive rent
    fund(id, 150);
    frame_system::Pallet::<Test>::set_block_number(500);
    // Anyone can call permissionless_sweep
    assert_ok!(AAA::permissionless_sweep(RuntimeOrigin::signed(BOB), id));
    assert!(AAA::aaa_instances(id).is_none());
  });
}

#[test]
fn orphan_assets_remain_on_sovereign_after_refund() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 500);

    let sv = sovereign(id);
    let orphan_asset = TestAsset::Local(1);
    set_asset_balance(sv, orphan_asset, 300);

    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id));

    assert!(AAA::aaa_instances(id).is_none());
    assert_eq!(get_asset_balance(sv, orphan_asset), 300);
  });
}

#[test]
fn zombie_sweep_runs_each_block() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    // Create several actors
    let _id0 = create_user(ALICE);
    let _id1 = create_user(BOB);
    // Fund enough for 10+ blocks of rent (10 × 1_000_000 = 10_000_000) plus threshold
    fund(_id0, 1_000_000_000);
    fund(_id1, 1_000_000_000);
    // Run many blocks — sweep cursor should advance
    for b in 2u64..=10 {
      run_block(b);
    }
    // Both should still be alive (have plenty of funds)
    assert!(AAA::aaa_instances(_id0).is_some());
    assert!(AAA::aaa_instances(_id1).is_some());
  });
}

#[test]
fn zombie_sweep_destroys_rent_insolvent_actor() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 150); // very small — swept after a few blocks of rent
    // Advance blocks until sweep catches it
    for b in 2u64..=200 {
      run_block(b);
      if AAA::aaa_instances(id).is_none() {
        break;
      }
    }
    assert!(
      AAA::aaa_instances(id).is_none(),
      "should be swept by zombie sweep"
    );
  });
}

#[test]
fn zombie_sweep_destroys_window_expired_actor() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let window = crate::ScheduleWindow { start: 2, end: 120 };
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      Some(window),
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    let id = AAA::next_aaa_id().saturating_sub(1);
    fund(id, 1_000_000_000);
    frame_system::Pallet::<Test>::set_block_number(121);
    run_idle(121);
    assert!(AAA::aaa_instances(id).is_none());
    assert!(System::events().into_iter().any(|record| {
      matches!(record.event,
        RuntimeEvent::AAA(Event::AAARefunded { aaa_id, reason: crate::RefundReason::WindowExpired, .. }) if aaa_id == id)
    }));
  });
}

#[test]
fn zombie_sweep_destroys_balance_exhausted_actor() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    run_idle(1);
    assert!(AAA::aaa_instances(id).is_none());
    assert!(System::events().into_iter().any(|record| {
      matches!(record.event,
        RuntimeEvent::AAA(Event::AAARefunded { aaa_id, reason: crate::RefundReason::BalanceExhausted, .. }) if aaa_id == id)
    }));
  });
}

#[test]
fn immutable_aaa_cannot_update_policy() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Immutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    assert_noop!(
      AAA::update_policy(RuntimeOrigin::signed(ALICE), id, AaaPolicy::default()),
      Error::<Test>::ImmutableActor
    );
  });
}

#[test]
fn immutable_aaa_can_be_funded_and_refunded() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Immutable,
      default_schedule(),
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 500);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id));
    assert!(AAA::aaa_instances(id).is_none());
  });
}

#[test]
fn cycle_nonce_exhausted_pauses_system_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_system();
    // Manually set nonce to max
    crate::pallet::AaaInstances::<Test>::mutate(id, |maybe| {
      if let Some(inst) = maybe.as_mut() {
        inst.cycle_nonce = u64::MAX;
      }
    });
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    let inst = AAA::aaa_instances(id).unwrap();
    assert!(inst.is_paused);
    assert_eq!(inst.pause_reason, Some(PauseReason::CycleNonceExhausted));
  });
}

#[test]
fn cycle_nonce_exhausted_destroys_user_aaa() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    fund(id, 50_000);
    // Manually set nonce to max
    crate::pallet::AaaInstances::<Test>::mutate(id, |maybe| {
      if let Some(inst) = maybe.as_mut() {
        inst.cycle_nonce = u64::MAX;
      }
    });
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    assert!(AAA::aaa_instances(id).is_none());
  });
}

// Scheduler fairness and bounded queue behavior.
#[test]
fn queue_overflow_emits_defer_event() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    // MaxReadyRingLength = 32, create 33 actors to overflow
    for _ in 0u32..33 {
      let id = AAA::next_aaa_id();
      if id < 32 {
        // first 32 fill the ring during create
        let _ = AAA::create_user_aaa(
          RuntimeOrigin::signed(ALICE),
          Mutability::Mutable,
          default_schedule(),
          None,
          single_pipeline(),
          AaaPolicy::default(),
          None,
        );
      } else {
        // 33rd should emit CycleDeferred::QueueOverflow
        let _ = AAA::create_user_aaa(
          RuntimeOrigin::signed(ALICE),
          Mutability::Mutable,
          default_schedule(),
          None,
          single_pipeline(),
          AaaPolicy::default(),
          None,
        );
        frame_system::Pallet::<Test>::assert_has_event(RuntimeEvent::AAA(Event::CycleDeferred {
          aaa_id: id,
          reason: DeferReason::QueueOverflow,
        }));
        break;
      }
    }
  });
}

#[test]
fn weighted_rr_is_deterministic_across_classes() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let user_a = create_user(ALICE);
    let user_b = create_user(BOB);
    let system = create_system();
    fund(user_a, 5_000);
    fund(user_b, 5_000);
    fund(system, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), user_a));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(BOB), user_b));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), system));
    run_idle(1);
    let started: Vec<u64> = frame_system::Pallet::<Test>::events()
      .into_iter()
      .filter_map(|record| match record.event {
        RuntimeEvent::AAA(Event::CycleStarted { aaa_id, .. }) => Some(aaa_id),
        _ => None,
      })
      .collect();
    assert_eq!(started, vec![system, user_a, user_b]);
  });
}

#[test]
fn per_class_caps_are_enforced() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let system_a = create_system();
    let system_b = create_system();
    let user_a = create_user(ALICE);
    let user_b = create_user(BOB);
    let user_c = create_user(CHARLIE);
    fund(system_a, 5_000);
    fund(system_b, 5_000);
    fund(user_a, 5_000);
    fund(user_b, 5_000);
    fund(user_c, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), system_a));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), system_b));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), user_a));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(BOB), user_b));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(CHARLIE), user_c));
    run_idle(1);
    let system_executed = [system_a, system_b]
      .into_iter()
      .filter(|id| AAA::aaa_instances(*id).unwrap().cycle_nonce == 1)
      .count();
    let user_executed = [user_a, user_b, user_c]
      .into_iter()
      .filter(|id| AAA::aaa_instances(*id).unwrap().cycle_nonce == 1)
      .count();
    assert_eq!(system_executed, 1);
    assert_eq!(user_executed, 2);
  });
}

#[test]
fn deferred_queue_retries_after_capacity_frees() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    for i in 0u32..33 {
      let owner = match i % 3 {
        0 => ALICE,
        1 => BOB,
        _ => CHARLIE,
      };
      assert_ok!(AAA::create_user_aaa(
        RuntimeOrigin::signed(owner),
        Mutability::Mutable,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ));
    }
    let deferred_id = 32u64;
    assert!(AAA::deferred_ring().contains(&deferred_id));
    assert!(!AAA::ready_ring().contains(&deferred_id));
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), 0));
    run_idle(1);
    assert!(AAA::ready_ring().contains(&deferred_id));
    assert!(!AAA::deferred_ring().contains(&deferred_id));
  });
}

#[test]
fn deferred_retry_respects_max_retries_per_block() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    for i in 0u32..48 {
      let owner = match i % 3 {
        0 => ALICE,
        1 => BOB,
        _ => CHARLIE,
      };
      assert_ok!(AAA::create_user_aaa(
        RuntimeOrigin::signed(owner),
        Mutability::Mutable,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ));
    }
    let max_retries =
      <<Test as crate::Config>::MaxDeferredRetriesPerBlock as Get<u32>>::get() as usize;
    assert!(AAA::deferred_ring().len() >= max_retries);
    for id in 0u64..max_retries as u64 {
      let owner = match id % 3 {
        0 => ALICE,
        1 => BOB,
        _ => CHARLIE,
      };
      assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(owner), id));
    }
    let ready_before = AAA::ready_ring().len();
    let deferred_before = AAA::deferred_ring().len();
    // retry_deferred_queue() runs before cycle budgeting, so tiny remaining_weight still exercises retry bound.
    let _ = AAA::on_idle(1, Weight::from_parts(1, 1));
    let ready_after = AAA::ready_ring().len();
    let deferred_after = AAA::deferred_ring().len();
    let moved = ready_after.saturating_sub(ready_before);
    assert!(moved <= max_retries);
    assert_eq!(deferred_before.saturating_sub(deferred_after), moved);
    let started = frame_system::Pallet::<Test>::events()
      .into_iter()
      .filter(|record| matches!(record.event, RuntimeEvent::AAA(Event::CycleStarted { .. })))
      .count();
    assert_eq!(started, 0);
  });
}

#[test]
fn ready_and_deferred_rings_remain_bounded_under_pressure() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let max_ready = <<Test as crate::Config>::MaxReadyRingLength as Get<u32>>::get() as usize;
    let max_deferred = <<Test as crate::Config>::MaxDeferredRingLength as Get<u32>>::get() as usize;
    for i in 0u64..120 {
      let owner = 1_000u64.saturating_add(i);
      let aaa_id = AAA::next_aaa_id();
      assert_ok!(AAA::create_user_aaa(
        RuntimeOrigin::signed(owner),
        Mutability::Mutable,
        default_schedule(),
        None,
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ));
      // Keep actors above MinUserBalance so queue bound checks are not masked by terminal refunds.
      fund(aaa_id, 100);
      assert!(AAA::ready_ring().len() <= max_ready);
      assert!(AAA::deferred_ring().len() <= max_deferred);
    }
    for block in 1u64..=10 {
      let _ = AAA::on_idle(block, Weight::from_parts(1, 1));
      assert!(AAA::ready_ring().len() <= max_ready);
      assert!(AAA::deferred_ring().len() <= max_deferred);
    }
  });
}

// Execution invariants that are easy to regress when scheduler internals evolve.
#[test]
fn no_mid_block_retries_for_always_ready_actor() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let schedule = Schedule {
      trigger: Trigger::ProbabilisticTimer {
        every_blocks: 0,
        probability_ppm: 1_000_000,
      },
      cooldown_blocks: 0,
    };
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      schedule,
      None,
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    fund(id, 10_000);
    run_idle(1);
    let started_count = frame_system::Pallet::<Test>::events()
      .into_iter()
      .filter(|record| {
        matches!(record.event, RuntimeEvent::AAA(Event::CycleStarted { aaa_id, .. }) if aaa_id == id)
      })
      .count();
    assert_eq!(started_count, 1);
    assert_eq!(AAA::aaa_instances(id).unwrap().cycle_nonce, 1);
  });
}

#[test]
fn steps_are_stateless_across_cycles() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let original_pipeline = AAA::aaa_instances(id).unwrap().pipeline;
    fund(id, 100_000_000);
    for block in 1u64..=3 {
      assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
      run_idle(block);
      assert_eq!(
        AAA::aaa_instances(id).unwrap().pipeline,
        original_pipeline.clone()
      );
    }
  });
}

#[test]
fn saturating_arithmetic_handles_large_split_amounts() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = AAA::next_aaa_id();
    let asset = TestAsset::Local(42);
    let amount = u128::MAX / 4;
    let legs: BoundedVec<SplitLeg<AccountId>, ConstU32<16>> = vec![
      SplitLeg { to: BOB, share: 1 },
      SplitLeg {
        to: CHARLIE,
        share: 1,
      },
    ]
    .try_into()
    .unwrap();
    let pipeline: crate::PipelineOf<Test> = vec![make_step(TaskKind::SplitTransfer {
      asset,
      amount: AmountSpec::Fixed(amount),
      total_shares: 2,
      legs,
      remainder_to: Some(BOB),
    })]
    .try_into()
    .unwrap();
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      None,
      pipeline,
      AaaPolicy::default(),
      None,
    ));
    let sv = sovereign(id);
    set_asset_balance(sv, asset, amount);
    fund(id, 5_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    run_idle(1);
    let bob_received = get_asset_balance(BOB, asset);
    let charlie_received = get_asset_balance(CHARLIE, asset);
    assert_eq!(bob_received.saturating_add(charlie_received), amount);
    assert_eq!(get_asset_balance(sv, asset), 0);
  });
}

#[test]
fn operations_on_nonexistent_aaa_fail() {
  new_test_ext().execute_with(|| {
    assert_noop!(
      AAA::pause_aaa(RuntimeOrigin::signed(ALICE), 999),
      Error::<Test>::AaaNotFound
    );
    assert_noop!(
      AAA::resume_aaa(RuntimeOrigin::signed(ALICE), 999),
      Error::<Test>::AaaNotFound
    );
    assert_noop!(
      AAA::manual_trigger(RuntimeOrigin::signed(ALICE), 999),
      Error::<Test>::AaaNotFound
    );
    assert_noop!(
      AAA::fund_aaa(RuntimeOrigin::signed(ALICE), 999, 100),
      Error::<Test>::AaaNotFound
    );
    assert_noop!(
      AAA::refund_and_close(RuntimeOrigin::signed(ALICE), 999),
      Error::<Test>::AaaNotFound
    );
  });
}

// Sovereign derivation and schedule-window regressions are kept at file tail as cross-cutting checks.
#[test]
fn sovereign_accounts_are_unique_per_owner_slot() {
  new_test_ext().execute_with(|| {
    let s0 = AAA::sovereign_account_id(&ALICE, 0);
    let s1 = AAA::sovereign_account_id(&ALICE, 1);
    let s2 = AAA::sovereign_account_id(&BOB, 0);
    assert_ne!(s0, s1);
    assert_ne!(s0, s2);
    assert_ne!(s1, s2);
  });
}

#[test]
fn sovereign_account_stored_in_instance() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(1);
    let id = create_user(ALICE);
    let inst = AAA::aaa_instances(id).unwrap();
    assert_eq!(
      inst.sovereign_account,
      AAA::sovereign_account_id(&inst.owner, inst.owner_slot)
    );
  });
}

#[test]
fn schedule_window_validation_and_lifecycle() {
  new_test_ext().execute_with(|| {
    frame_system::Pallet::<Test>::set_block_number(100);
    // 1. Creation validation
    // end < start
    let invalid_window = crate::ScheduleWindow {
      start: 150,
      end: 100,
    };
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        Some(invalid_window),
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::InvalidScheduleWindow
    );
    // too short
    let short_window = crate::ScheduleWindow {
      start: 100,
      end: 150,
    };
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        Some(short_window),
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::WindowTooShort
    );
    // retroactive
    let retro_window = crate::ScheduleWindow {
      start: 50,
      end: 200,
    };
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        default_schedule(),
        Some(retro_window),
        single_pipeline(),
        AaaPolicy::default(),
        None,
      ),
      Error::<Test>::InvalidScheduleWindow
    );
    // 2. Successful creation and ready-gate
    let valid_window = crate::ScheduleWindow {
      start: 120,
      end: 300,
    };
    assert_ok!(AAA::create_user_aaa(
      RuntimeOrigin::signed(ALICE),
      Mutability::Mutable,
      default_schedule(),
      Some(valid_window),
      single_pipeline(),
      AaaPolicy::default(),
      None,
    ));
    let id = crate::NextAaaId::<Test>::get() - 1;
    // Initially not ready because now < start (100 < 120)
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), id));
    assert!(!AAA::is_ready_for_execution(
      &AAA::aaa_instances(id).unwrap()
    ));
    // Fast forward to start
    frame_system::Pallet::<Test>::set_block_number(120);
    assert!(AAA::is_ready_for_execution(
      &AAA::aaa_instances(id).unwrap()
    ));
    // Fast forward past end
    frame_system::Pallet::<Test>::set_block_number(301);
    // It should auto-refund on permissionless sweep
    assert_ok!(AAA::permissionless_sweep(RuntimeOrigin::signed(BOB), id));
    // Actor should be destroyed
    assert!(AAA::aaa_instances(id).is_none());
    System::assert_has_event(RuntimeEvent::AAA(Event::AAARefunded {
      aaa_id: id,
      reason: crate::RefundReason::WindowExpired,
      solvent: false,
      to: ALICE,
      assets_refunded: BoundedVec::try_from(alloc::vec![]).unwrap(),
      assets_forfeited: BoundedVec::try_from(alloc::vec![]).unwrap(),
      native_burned: 0,
    }));
  });
}
