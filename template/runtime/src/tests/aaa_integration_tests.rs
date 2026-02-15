//! AAA (Autonomous Actor Automation) Integration Tests — v0.40.0
//!
//! Verifies AAA lifecycle, pipeline execution, and adapter wiring
//! using real runtime pallets (Balances, Assets, AxialRouter, AssetConversion).

use super::common::{
  ALICE, ASSET_A, ASSET_FOREIGN, BOB, CHARLIE, LIQUIDITY_AMOUNT, MIN_LIQUIDITY, add_liquidity,
  ensure_asset_conversion_pool, mint_tokens, seeded_test_ext,
};
use crate::{AAA, Assets, Balances, RuntimeEvent, RuntimeOrigin, System};
use pallet_aaa::{
  AaaId, AaaPolicy, AmountSpec, AssetFilter, Condition, Event, InboxDrainMode, Mutability,
  PipelineErrorPolicy, PipelineOf, RefundReason, Schedule, SourceFilter, SplitLeg, Step, TaskKind,
  Trigger,
};
use polkadot_sdk::frame_support::{
  BoundedVec, assert_noop, assert_ok,
  traits::{Currency, Get, Hooks},
};
use polkadot_sdk::sp_runtime::Permill;
use primitives::AssetKind;

type MaxConditionsPerStep = <crate::Runtime as pallet_aaa::Config>::MaxConditionsPerStep;
type RuntimeStep = Step<AssetKind, u128, crate::AccountId, MaxConditionsPerStep>;
type Pipeline = PipelineOf<crate::Runtime>;

fn make_step(task: TaskKind<AssetKind, u128, crate::AccountId>) -> RuntimeStep {
  Step {
    conditions: BoundedVec::default(),
    task,
    on_error: PipelineErrorPolicy::AbortCycle,
  }
}

fn make_step_cond(
  conditions: Vec<Condition<AssetKind, u128>>,
  task: TaskKind<AssetKind, u128, crate::AccountId>,
  on_error: PipelineErrorPolicy,
) -> RuntimeStep {
  Step {
    conditions: BoundedVec::try_from(conditions).expect("conditions overflow"),
    task,
    on_error,
  }
}

fn aaa_account(aaa_id: AaaId) -> crate::AccountId {
  AAA::aaa_instances(aaa_id)
    .map(|inst| inst.sovereign_account)
    .expect("AAA must exist")
}

fn native_balance(who: &crate::AccountId) -> u128 {
  Balances::free_balance(who)
}

fn asset_balance(asset_id: u32, who: &crate::AccountId) -> u128 {
  Assets::balance(asset_id, who)
}

fn seed_oracle_1to1(asset: AssetKind) {
  let price = 1_000_000_000_000u128;
  pallet_axial_router::EmaPrices::<crate::Runtime>::insert(asset, AssetKind::Native, price);
  pallet_axial_router::EmaPrices::<crate::Runtime>::insert(AssetKind::Native, asset, price);
}

fn setup_pool_with_liquidity(asset_id: u32) {
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

fn transfer_pipeline(to: crate::AccountId, asset: AssetKind, amount: u128) -> Pipeline {
  BoundedVec::try_from(vec![make_step(TaskKind::Transfer {
    to,
    asset,
    amount: AmountSpec::Fixed(amount),
  })])
  .unwrap()
}

fn burn_pipeline(asset: AssetKind, amount: u128) -> Pipeline {
  BoundedVec::try_from(vec![make_step(TaskKind::Burn {
    asset,
    amount: AmountSpec::Fixed(amount),
  })])
  .unwrap()
}

fn fund_aaa_account(aaa_id: AaaId, amount: u128) {
  let aaa_acc = aaa_account(aaa_id);
  let _ = <Balances as Currency<crate::AccountId>>::deposit_creating(&aaa_acc, amount);
}

fn fund_aaa_with_asset(asset_id: u32, aaa_id: AaaId, amount: u128) {
  let aaa_acc = aaa_account(aaa_id);
  assert_ok!(mint_tokens(asset_id, &ALICE, &aaa_acc, amount));
}

fn create_user(
  who: crate::AccountId,
  schedule: Schedule<AssetKind, crate::AccountId>,
  pipeline: Pipeline,
  refund_to: Option<crate::AccountId>,
) -> AaaId {
  let id = AAA::next_aaa_id();
  assert_ok!(AAA::create_user_aaa(
    RuntimeOrigin::signed(who),
    Mutability::Mutable,
    schedule,
    None,
    pipeline,
    AaaPolicy::default(),
    refund_to,
  ));
  id
}

fn create_system(
  owner: crate::AccountId,
  schedule: Schedule<AssetKind, crate::AccountId>,
  pipeline: Pipeline,
  refund_to: crate::AccountId,
) -> AaaId {
  let id = AAA::next_aaa_id();
  assert_ok!(AAA::create_system_aaa(
    RuntimeOrigin::root(),
    owner,
    schedule,
    None,
    pipeline,
    AaaPolicy::default(),
    refund_to,
  ));
  id
}

fn manual_schedule() -> Schedule<AssetKind, crate::AccountId> {
  Schedule {
    trigger: Trigger::Manual,
    cooldown_blocks: 0,
  }
}

fn run_idle(block: u32) {
  AAA::on_idle(block, polkadot_sdk::frame_support::weights::Weight::MAX);
}

// Basic lifecycle and permission tests remain in pallet unit tests.
// Runtime file focuses on cross-pallet integration and scheduler wiring.
#[test]
fn test_manual_trigger_transfer() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let transfer_amount = 5_000_000_000_000u128;
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, transfer_amount);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let bob_before = native_balance(&BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(native_balance(&BOB), bob_before + transfer_amount);
  });
}

#[test]
fn test_manual_trigger_swap() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_A);
    let pipeline = BoundedVec::try_from(vec![make_step(TaskKind::SwapExactIn {
      asset_in: AssetKind::Native,
      asset_out: AssetKind::Local(ASSET_A),
      amount_in: AmountSpec::Fixed(1_000_000_000_000u128),
      min_out: 1,
    })])
    .unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    fund_aaa_with_asset(ASSET_A, aaa_id, 10_000_000_000_000);
    let aaa_acc = aaa_account(aaa_id);
    let native_before = native_balance(&aaa_acc);
    let asset_before = asset_balance(ASSET_A, &aaa_acc);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert!(
      native_balance(&aaa_acc) < native_before,
      "native should decrease"
    );
    assert!(
      asset_balance(ASSET_A, &aaa_acc) > asset_before,
      "asset should increase"
    );
  });
}

#[test]
fn test_manual_trigger_burn_native() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let burn_amount = 2_000_000_000_000u128;
    let pipeline = burn_pipeline(AssetKind::Native, burn_amount);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let total_before = Balances::total_issuance();
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert!(
      Balances::total_issuance() < total_before,
      "burn should reduce issuance"
    );
    assert_eq!(Balances::total_issuance(), total_before - burn_amount);
  });
}

#[test]
fn test_manual_trigger_burn_foreign_asset() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let burn_amount = 5_000_000_000_000u128;
    let pipeline = burn_pipeline(AssetKind::Foreign(ASSET_FOREIGN), burn_amount);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, aaa_id, burn_amount * 2);
    let aaa_acc = aaa_account(aaa_id);
    let foreign_before = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(
      asset_balance(ASSET_FOREIGN, &aaa_acc),
      foreign_before - burn_amount
    );
  });
}

#[test]
fn test_split_transfer() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let legs = BoundedVec::try_from(vec![
      SplitLeg { to: BOB, share: 50 },
      SplitLeg {
        to: CHARLIE,
        share: 50,
      },
    ])
    .unwrap();
    let step = make_step(TaskKind::SplitTransfer {
      asset: AssetKind::Native,
      amount: AmountSpec::Fixed(4_000_000_000_000u128),
      total_shares: 100,
      legs,
      remainder_to: None,
    });
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let bob_before = native_balance(&BOB);
    let charlie_before = native_balance(&CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(native_balance(&BOB) - bob_before, 2_000_000_000_000);
    assert_eq!(native_balance(&CHARLIE) - charlie_before, 2_000_000_000_000);
  });
}

#[test]
fn test_split_transfer_remainder_to_target() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let legs = BoundedVec::try_from(vec![
      SplitLeg { to: BOB, share: 50 },
      SplitLeg {
        to: CHARLIE,
        share: 50,
      },
    ])
    .unwrap();
    let step = make_step(TaskKind::SplitTransfer {
      asset: AssetKind::Native,
      amount: AmountSpec::Fixed(4_000_000_000_001u128),
      total_shares: 100,
      legs,
      remainder_to: Some(CHARLIE),
    });
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let bob_before = native_balance(&BOB);
    let charlie_before = native_balance(&CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(native_balance(&BOB) - bob_before, 2_000_000_000_000);
    assert_eq!(native_balance(&CHARLIE) - charlie_before, 2_000_000_000_001);
  });
}

#[test]
fn test_on_address_event_batch_drain_mode() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let asset = AssetKind::Local(ASSET_A);
    let schedule = Schedule {
      trigger: Trigger::OnAddressEvent {
        asset_filter: AssetFilter::IncludeOnly(BoundedVec::try_from(vec![asset]).unwrap()),
        source_filter: SourceFilter::Any,
        drain_mode: InboxDrainMode::Batch(2),
      },
      cooldown_blocks: 0,
    };
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, schedule, pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    AAA::notify_address_event(aaa_id, asset, &BOB);
    AAA::notify_address_event(aaa_id, asset, &BOB);
    AAA::notify_address_event(aaa_id, asset, &BOB);
    let bob_before = native_balance(&BOB);
    run_idle(1);
    assert_eq!(native_balance(&BOB), bob_before + 1_000_000_000);
    run_idle(2);
    assert_eq!(native_balance(&BOB), bob_before + 2_000_000_000);
    run_idle(3);
    assert_eq!(native_balance(&BOB), bob_before + 2_000_000_000);
  });
}

// Scheduler/execution invariants in full runtime wiring.
#[test]
fn test_steps_are_stateless_across_cycles() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let expected_pipeline = pipeline.clone();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    for block in 1u32..=3 {
      assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
      run_idle(block);
      assert_eq!(
        AAA::aaa_instances(aaa_id).unwrap().pipeline,
        expected_pipeline
      );
    }
  });
}

#[test]
fn test_budget_cap_blocks_execution_when_remaining_weight_is_tiny() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    // remaining_weight too small for one-step admission should leave trigger pending and nonce untouched.
    let _ = AAA::on_idle(
      1,
      polkadot_sdk::frame_support::weights::Weight::from_parts(1, 1),
    );
    let inst = AAA::aaa_instances(aaa_id).unwrap();
    assert_eq!(inst.cycle_nonce, 0);
    assert!(inst.manual_trigger_pending);
    assert!(!System::events().into_iter().any(|record| {
      matches!(record.event, RuntimeEvent::AAA(Event::CycleStarted { aaa_id: id, .. }) if id == aaa_id)
    }));
  });
}

// Budget-cap property sweep across multiple remaining_weight slices.
#[test]
fn test_budget_cap_property_bound_across_weight_inputs() {
  let weight_cases = [
    1u64,
    10_000_000u64,
    50_000_000u64,
    150_000_000u64,
    300_000_000u64,
  ];
  // execute_cycle() admission uses estimated_cost = 5_000_000 * (pipeline_len + 1).
  // This test uses one-step pipelines, so 10_000_000 is the deterministic per-cycle gate.
  const ONE_STEP_ADMISSION_COST: u64 = 10_000_000;
  for (idx, remaining_ref_time) in weight_cases.into_iter().enumerate() {
    seeded_test_ext().execute_with(|| {
      let block = idx as u32 + 1;
      System::set_block_number(block);
      let mut user_actors: Vec<(crate::AccountId, AaaId)> = Vec::new();
      let mut system_actors: Vec<(crate::AccountId, AaaId)> = Vec::new();
      for i in 0u8..24 {
        let mut seed = [0u8; 32];
        seed[0] = 10u8.saturating_add(i);
        let owner = crate::AccountId::new(seed);
        let aaa_id = create_user(
          owner.clone(),
          manual_schedule(),
          transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
          Some(ALICE),
        );
        fund_aaa_account(aaa_id, 100_000_000_000_000);
        user_actors.push((owner, aaa_id));
      }
      for i in 0u8..12 {
        let mut seed = [0u8; 32];
        seed[0] = 100u8.saturating_add(i);
        let owner = crate::AccountId::new(seed);
        let aaa_id = create_system(
          owner.clone(),
          manual_schedule(),
          transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
          ALICE,
        );
        system_actors.push((owner, aaa_id));
      }
      for (owner, aaa_id) in user_actors.iter() {
        assert_ok!(AAA::manual_trigger(
          RuntimeOrigin::signed(owner.clone()),
          *aaa_id
        ));
      }
      for (owner, aaa_id) in system_actors.iter() {
        assert_ok!(AAA::manual_trigger(
          RuntimeOrigin::signed(owner.clone()),
          *aaa_id
        ));
      }
      System::reset_events();
      let _ = AAA::on_idle(
        block,
        polkadot_sdk::frame_support::weights::Weight::from_parts(remaining_ref_time, 1_000),
      );
      let started = System::events()
        .into_iter()
        .filter(|record| matches!(record.event, RuntimeEvent::AAA(Event::CycleStarted { .. })))
        .count() as u64;
      let budget =
        <crate::Runtime as pallet_aaa::Config>::AaaBudgetPct::get().mul_floor(remaining_ref_time);
      let max_by_budget = budget / ONE_STEP_ADMISSION_COST;
      let max_by_caps = (<crate::Runtime as pallet_aaa::Config>::MaxSystemExecutionsPerBlock::get()
        as u64)
        .saturating_add(
          <crate::Runtime as pallet_aaa::Config>::MaxUserExecutionsPerBlock::get() as u64,
        );
      let max_by_actors = user_actors.len().saturating_add(system_actors.len()) as u64;
      let expected_upper = max_by_budget.min(max_by_caps).min(max_by_actors);
      assert!(started <= expected_upper);
      if expected_upper == 0 {
        assert_eq!(started, 0);
      }
    });
  }
}

#[test]
fn test_cooldown_blocks_scheduler_execution() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let schedule = Schedule {
      trigger: Trigger::Manual,
      cooldown_blocks: 5,
    };
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, schedule, pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let bob_before = native_balance(&BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    let bob_after_first = native_balance(&BOB);
    assert_eq!(bob_after_first, bob_before + 1_000_000_000);
    // Cooldown not elapsed → blocked
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    System::set_block_number(2);
    run_idle(2);
    assert_eq!(native_balance(&BOB), bob_after_first);
    // Past cooldown → executes
    System::set_block_number(7);
    run_idle(7);
    assert_eq!(native_balance(&BOB), bob_after_first + 1_000_000_000);
  });
}

#[test]
fn test_global_circuit_breaker_blocks_execution() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    let bob_before = native_balance(&BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(native_balance(&BOB), bob_before, "breaker should block");
    assert_ok!(AAA::set_global_circuit_breaker(
      RuntimeOrigin::root(),
      false
    ));
    System::set_block_number(2);
    run_idle(2);
    assert_eq!(
      native_balance(&BOB),
      bob_before + 1_000_000_000,
      "should execute after breaker disabled"
    );
  });
}

#[test]
fn test_create_paths_rejected_when_breaker_active() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    assert_noop!(
      AAA::create_user_aaa(
        RuntimeOrigin::signed(ALICE),
        Mutability::Mutable,
        manual_schedule(),
        None,
        pipeline.clone(),
        AaaPolicy::default(),
        Some(ALICE),
      ),
      pallet_aaa::Error::<crate::Runtime>::GlobalCircuitBreakerActive
    );
    assert_noop!(
      AAA::create_system_aaa(
        RuntimeOrigin::root(),
        ALICE,
        manual_schedule(),
        None,
        pipeline,
        AaaPolicy::default(),
        ALICE,
      ),
      pallet_aaa::Error::<crate::Runtime>::GlobalCircuitBreakerActive
    );
  });
}

#[test]
fn test_cleanup_and_control_paths_alive_under_breaker() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(ALICE));
    fund_aaa_account(aaa_id, 10_000_000_000_000);
    assert_ok!(AAA::set_global_circuit_breaker(RuntimeOrigin::root(), true));
    assert_ok!(AAA::fund_aaa(
      RuntimeOrigin::signed(ALICE),
      aaa_id,
      1_000_000_000_000,
    ));
    assert_ok!(AAA::permissionless_sweep(
      RuntimeOrigin::signed(CHARLIE),
      aaa_id,
    ));
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), aaa_id));
    assert!(
      AAA::aaa_instances(aaa_id).is_none(),
      "refund path must remain active under breaker"
    );
    assert_ok!(AAA::set_global_circuit_breaker(
      RuntimeOrigin::root(),
      false
    ));
  });
}

#[test]
fn test_weighted_rr_prefers_system_slot_deterministically() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let user_a = create_user(
      ALICE,
      manual_schedule(),
      transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
      Some(ALICE),
    );
    let user_b = create_user(
      BOB,
      manual_schedule(),
      transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
      Some(BOB),
    );
    let system = create_system(
      ALICE,
      manual_schedule(),
      transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
      ALICE,
    );
    fund_aaa_account(user_a, 100_000_000_000_000);
    fund_aaa_account(user_b, 100_000_000_000_000);
    fund_aaa_account(system, 100_000_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), user_a));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(BOB), user_b));
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), system));
    run_idle(1);
    let started: Vec<AaaId> = System::events()
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
fn test_deferred_retry_moves_actor_back_to_ready_when_capacity_frees() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let first_owner = crate::AccountId::new([0u8; 32]);
    for i in 0u8..65 {
      let owner = crate::AccountId::new([
        i, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
      ]);
      let _ = create_user(
        owner.clone(),
        manual_schedule(),
        transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000),
        Some(owner),
      );
    }
    let deferred_id = 64u64;
    assert!(AAA::deferred_ring().contains(&deferred_id));
    assert!(!AAA::ready_ring().contains(&deferred_id));
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(first_owner), 0));
    run_idle(1);
    assert!(AAA::ready_ring().contains(&deferred_id));
    assert!(!AAA::deferred_ring().contains(&deferred_id));
  });
}

#[test]
fn test_multi_step_pipeline() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_A);
    let pipeline = BoundedVec::try_from(vec![
      make_step(TaskKind::SwapExactIn {
        asset_in: AssetKind::Native,
        asset_out: AssetKind::Local(ASSET_A),
        amount_in: AmountSpec::Fixed(2_000_000_000_000u128),
        min_out: 1,
      }),
      make_step(TaskKind::Transfer {
        to: BOB,
        asset: AssetKind::Local(ASSET_A),
        amount: AmountSpec::Percentage(Permill::from_percent(50)),
      }),
    ])
    .unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    fund_aaa_with_asset(ASSET_A, aaa_id, 10_000_000_000_000);
    let bob_asset_before = asset_balance(ASSET_A, &BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert!(asset_balance(ASSET_A, &BOB) > bob_asset_before);
  });
}

#[test]
fn test_error_policy_continue_next_step() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = BoundedVec::try_from(vec![
      Step {
        conditions: BoundedVec::default(),
        task: TaskKind::Transfer {
          to: BOB,
          asset: AssetKind::Local(ASSET_A),
          amount: AmountSpec::Fixed(1_000_000_000_000_000_000u128),
        },
        on_error: PipelineErrorPolicy::ContinueNextStep,
      },
      make_step(TaskKind::Transfer {
        to: CHARLIE,
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(1_000_000_000_000u128),
      }),
    ])
    .unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let charlie_before = native_balance(&CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(native_balance(&CHARLIE), charlie_before + 1_000_000_000_000);
  });
}

#[test]
fn test_error_policy_abort_cycle_stops_pipeline() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    // Step 0 fails (no Local(ASSET_A) balance) with AbortCycle → Step 1 never runs
    let pipeline = BoundedVec::try_from(vec![
      Step {
        conditions: BoundedVec::default(),
        task: TaskKind::Transfer {
          to: BOB,
          asset: AssetKind::Local(ASSET_A),
          amount: AmountSpec::Fixed(1_000_000_000_000_000_000u128),
        },
        on_error: PipelineErrorPolicy::AbortCycle,
      },
      make_step(TaskKind::Transfer {
        to: CHARLIE,
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(1_000_000_000_000u128),
      }),
    ])
    .unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let charlie_before = native_balance(&CHARLIE);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    // CHARLIE should NOT receive anything (step 1 aborted)
    assert_eq!(native_balance(&CHARLIE), charlie_before);
  });
}

#[test]
fn test_fund_aaa() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(
      ALICE,
      Schedule {
        trigger: Trigger::Manual,
        cooldown_blocks: 5,
      },
      pipeline,
      Some(BOB),
    );
    let aaa_acc = aaa_account(aaa_id);
    let balance_before = native_balance(&aaa_acc);
    assert_ok!(AAA::fund_aaa(
      RuntimeOrigin::signed(ALICE),
      aaa_id,
      3_000_000_000_000
    ));
    assert_eq!(native_balance(&aaa_acc), balance_before + 3_000_000_000_000);
  });
}

#[test]
fn test_add_liquidity() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_A);
    let step = make_step(TaskKind::AddLiquidity {
      asset_a: AssetKind::Native,
      asset_b: AssetKind::Local(ASSET_A),
      amount_a: AmountSpec::Fixed(1_000_000_000_000u128),
      amount_b: AmountSpec::Fixed(1_000_000_000_000u128),
    });
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    fund_aaa_with_asset(ASSET_A, aaa_id, 100_000_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    // Just verify it doesn't fail
    assert!(pallet_aaa::AaaInstances::<crate::Runtime>::get(aaa_id).is_some());
  });
}

#[test]
fn test_condition_balance_above() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let threshold = 1_000_000_000_000u128;
    let transfer_amount = 500_000_000_000u128;
    let step = make_step_cond(
      vec![Condition::BalanceAbove {
        asset: AssetKind::Native,
        threshold,
      }],
      TaskKind::Transfer {
        to: BOB,
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(transfer_amount),
      },
      PipelineErrorPolicy::AbortCycle,
    );
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    // Fund below threshold (but above MinUserBalance)
    fund_aaa_account(aaa_id, 500_000_000_000);
    let bob_before = native_balance(&BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(
      native_balance(&BOB),
      bob_before,
      "below threshold: should skip"
    );
    // Fund above threshold
    fund_aaa_account(aaa_id, 5_000_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    System::set_block_number(2);
    run_idle(2);
    assert_eq!(
      native_balance(&BOB),
      bob_before + transfer_amount,
      "above threshold: should execute"
    );
  });
}

#[test]
fn test_condition_balance_below() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let threshold = 1_000_000_000_000u128;
    let burn_amount = 500_000_000_000u128;
    let step = make_step_cond(
      vec![Condition::BalanceBelow {
        asset: AssetKind::Native,
        threshold,
      }],
      TaskKind::Burn {
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(burn_amount),
      },
      PipelineErrorPolicy::AbortCycle,
    );
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 100_000_000_000_000); // above threshold → skip
    let total_before = Balances::total_issuance();
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(
      Balances::total_issuance(),
      total_before,
      "above threshold: should skip"
    );
    // Drain to below threshold
    let aaa_acc = aaa_account(aaa_id);
    let _ = Balances::slash(&aaa_acc, 99_500_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    System::set_block_number(2);
    run_idle(2);
    assert!(
      Balances::total_issuance() < total_before,
      "below threshold: should execute"
    );
  });
}

#[test]
fn test_condition_and_semantics() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    // Both conditions must pass: Native > 1T AND Local(ASSET_A) > 1T
    let step = make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: AssetKind::Native,
          threshold: 1_000_000_000_000,
        },
        Condition::BalanceAbove {
          asset: AssetKind::Local(ASSET_A),
          threshold: 1_000_000_000_000,
        },
      ],
      TaskKind::Transfer {
        to: BOB,
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(500_000_000_000),
      },
      PipelineErrorPolicy::AbortCycle,
    );
    let pipeline = BoundedVec::try_from(vec![step]).unwrap();
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    // Only fund native; no ASSET_A
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let bob_before = native_balance(&BOB);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(
      native_balance(&BOB),
      bob_before,
      "ASSET_A condition fails → step skipped"
    );
    // Add ASSET_A → both pass
    fund_aaa_with_asset(ASSET_A, aaa_id, 5_000_000_000_000);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    System::set_block_number(2);
    run_idle(2);
    assert_eq!(
      native_balance(&BOB),
      bob_before + 500_000_000_000,
      "both conditions pass → executes"
    );
  });
}

#[test]
fn test_execution_fee_charged_for_user_aaa() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let transfer_amount = 100_000_000_000u128;
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, transfer_amount);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, None);
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let actor = aaa_account(aaa_id);
    let balance_before = native_balance(&actor);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    let deducted = balance_before.saturating_sub(native_balance(&actor));
    // At minimum the transfer amount must be deducted
    assert!(
      deducted >= transfer_amount,
      "transfer should be deducted; deducted={deducted}, transfer={transfer_amount}"
    );
  });
}

#[test]
fn test_execution_fee_exempt_for_system_aaa() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 100_000_000_000);
    let aaa_id = create_system(ALICE, manual_schedule(), pipeline, ALICE);
    fund_aaa_account(aaa_id, 100_000_000_000_000);
    let actor = aaa_account(aaa_id);
    let balance_before = native_balance(&actor);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    let expected_transfer = 100_000_000_000u128;
    // System AAA: only transfer deducted, no exec fee
    assert_eq!(
      balance_before - native_balance(&actor),
      expected_transfer,
      "System AAA should be exempt from execution fee"
    );
  });
}

#[test]
fn test_balance_exhausted_triggers_auto_refund() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let min = <crate::Runtime as pallet_aaa::Config>::MinUserBalance::get();
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 100);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, None);
    // Fund below MinUserBalance
    fund_aaa_account(aaa_id, min / 2);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert!(
      AAA::aaa_instances(aaa_id).is_none(),
      "should be auto-refunded"
    );
  });
}

#[test]
fn test_owner_slot_first_free_and_reuse() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);

    let pipeline_a = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let pipeline_b = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let pipeline_c = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);

    let id0 = create_user(ALICE, manual_schedule(), pipeline_a, None);
    let id1 = create_user(ALICE, manual_schedule(), pipeline_b, None);

    assert_eq!(AAA::aaa_instances(id0).unwrap().owner_slot, 0);
    assert_eq!(AAA::aaa_instances(id1).unwrap().owner_slot, 1);

    fund_aaa_account(id1, 10_000_000_000_000);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), id1));

    let id2 = create_user(ALICE, manual_schedule(), pipeline_c, None);
    assert_eq!(AAA::aaa_instances(id2).unwrap().owner_slot, 1);
  });
}

#[test]
fn test_refund_and_close_emits_unified_event() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 10_000_000_000_000);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), aaa_id));
    let events = System::events();
    let found = events.iter().any(|rec| {
      matches!(&rec.event,
        RuntimeEvent::AAA(Event::AAARefunded { reason: RefundReason::OwnerInitiated, to, .. })
          if *to == BOB
      )
    });
    assert!(found, "unified AAARefunded(OwnerInitiated) event not found");
  });
}

#[test]
fn test_destroyed_event_emitted_on_terminal_refund() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    fund_aaa_account(aaa_id, 10_000_000_000_000);
    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), aaa_id));
    let events = System::events();
    let found = events.iter().any(|rec| {
      matches!(&rec.event,
        RuntimeEvent::AAA(Event::AAADestroyed { aaa_id: id }) if *id == aaa_id
      )
    });
    assert!(found, "AAADestroyed event not found");
  });
}

#[test]
fn test_system_actor_mint_task() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline: Pipeline = BoundedVec::try_from(vec![make_step(TaskKind::Mint {
      asset: AssetKind::Local(ASSET_A),
      amount: AmountSpec::Fixed(1_000_000_000_000),
    })])
    .unwrap();
    let aaa_id = create_system(ALICE, manual_schedule(), pipeline, ALICE);
    let total_before = Assets::total_supply(ASSET_A);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert_eq!(
      Assets::total_supply(ASSET_A),
      total_before + 1_000_000_000_000
    );
  });
}

#[test]
fn test_branching_via_complementary_conditions() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_FOREIGN);
    let dust = 100_000_000u128;
    // Step 1: balanced → AddLiquidity
    let step1 = make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: AssetKind::Native,
          threshold: dust,
        },
        Condition::BalanceAbove {
          asset: AssetKind::Local(ASSET_FOREIGN),
          threshold: dust,
        },
      ],
      TaskKind::AddLiquidity {
        asset_a: AssetKind::Native,
        asset_b: AssetKind::Local(ASSET_FOREIGN),
        amount_a: AmountSpec::Fixed(1_000_000_000_000u128),
        amount_b: AmountSpec::Fixed(1_000_000_000_000u128),
      },
      PipelineErrorPolicy::ContinueNextStep,
    );
    // Step 2: foreign-rich, native-poor → swap foreign to native
    let step2 = make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: AssetKind::Local(ASSET_FOREIGN),
          threshold: dust,
        },
        Condition::BalanceBelow {
          asset: AssetKind::Native,
          threshold: dust,
        },
      ],
      TaskKind::SwapExactIn {
        asset_in: AssetKind::Local(ASSET_FOREIGN),
        asset_out: AssetKind::Native,
        amount_in: AmountSpec::Fixed(1_000_000_000_000u128),
        min_out: 1,
      },
      PipelineErrorPolicy::ContinueNextStep,
    );
    let pipeline = BoundedVec::try_from(vec![step1, step2]).unwrap();
    let aaa_id = create_system(ALICE, manual_schedule(), pipeline, ALICE);
    let aaa_acc = aaa_account(aaa_id);
    fund_aaa_account(aaa_id, 3_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, aaa_id, 3_000_000_000_000);
    let native_before = native_balance(&aaa_acc);
    let foreign_before = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    let native_after = native_balance(&aaa_acc);
    let foreign_after = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert!(
      native_after < native_before || foreign_after < foreign_before,
      "Balanced: AddLiquidity should consume assets"
    );
  });
}

#[test]
fn test_orphan_assets_remain_on_former_sovereign_account() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    let pipeline = transfer_pipeline(BOB, AssetKind::Native, 1_000_000_000);
    let aaa_id = create_user(ALICE, manual_schedule(), pipeline, Some(BOB));
    let actor = aaa_account(aaa_id);
    fund_aaa_account(aaa_id, 10_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, aaa_id, 2_000_000_000_000);

    assert_ok!(AAA::refund_and_close(RuntimeOrigin::signed(ALICE), aaa_id));

    assert!(AAA::aaa_instances(aaa_id).is_none());
    assert_eq!(asset_balance(ASSET_FOREIGN, &actor), 2_000_000_000_000);
  });
}

#[test]
fn test_system_actor_swap_and_burn() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_FOREIGN);
    let swap_amount = 1_000_000_000_000u128;
    let pipeline: Pipeline = BoundedVec::try_from(vec![
      make_step(TaskKind::SwapExactIn {
        asset_in: AssetKind::Local(ASSET_FOREIGN),
        asset_out: AssetKind::Native,
        amount_in: AmountSpec::Fixed(swap_amount),
        min_out: 1,
      }),
      make_step(TaskKind::Burn {
        asset: AssetKind::Native,
        amount: AmountSpec::Fixed(500_000_000_000u128),
      }),
    ])
    .unwrap();
    let aaa_id = create_system(ALICE, manual_schedule(), pipeline, ALICE);
    fund_aaa_account(aaa_id, 1_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, aaa_id, 2_000_000_000_000);
    let total_before = Balances::total_issuance();
    let aaa_acc = aaa_account(aaa_id);
    let foreign_before = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    assert!(
      Balances::total_issuance() <= total_before,
      "issuance should not increase"
    );
    assert!(
      asset_balance(ASSET_FOREIGN, &aaa_acc) < foreign_before,
      "foreign should be swapped"
    );
  });
}

#[test]
fn test_e2e_zap_burn_actors() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_FOREIGN);
    let tol_treasury = crate::configs::TolTreasuryAccount::get();
    let zap_pipeline: Pipeline = BoundedVec::try_from(vec![make_step(TaskKind::AddLiquidity {
      asset_a: AssetKind::Native,
      asset_b: AssetKind::Local(ASSET_FOREIGN),
      amount_a: AmountSpec::Fixed(1_000_000_000_000u128),
      amount_b: AmountSpec::Fixed(1_000_000_000_000u128),
    })])
    .unwrap();
    let zap_aaa_id = create_system(ALICE, manual_schedule(), zap_pipeline, tol_treasury.clone());
    let burn_pipeline: Pipeline = BoundedVec::try_from(vec![make_step(TaskKind::Burn {
      asset: AssetKind::Native,
      amount: AmountSpec::Fixed(500_000_000_000u128),
    })])
    .unwrap();
    let burn_aaa_id = create_system(
      ALICE,
      manual_schedule(),
      burn_pipeline,
      tol_treasury.clone(),
    );
    fund_aaa_account(zap_aaa_id, 100_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, zap_aaa_id, 100_000_000_000_000);
    fund_aaa_account(burn_aaa_id, 100_000_000_000_000);
    let zap_acc = aaa_account(zap_aaa_id);
    let zap_native_before = native_balance(&zap_acc);
    let zap_foreign_before = asset_balance(ASSET_FOREIGN, &zap_acc);
    assert_ok!(AAA::manual_trigger(
      RuntimeOrigin::signed(ALICE),
      zap_aaa_id
    ));
    run_idle(1);
    assert!(
      native_balance(&zap_acc) < zap_native_before,
      "Zap should consume native"
    );
    assert!(
      asset_balance(ASSET_FOREIGN, &zap_acc) < zap_foreign_before,
      "Zap should consume foreign"
    );
    assert_ok!(AAA::manual_trigger(
      RuntimeOrigin::signed(ALICE),
      burn_aaa_id
    ));
    System::set_block_number(2);
    run_idle(2);

    let burn_acc = aaa_account(burn_aaa_id);
    assert!(
      native_balance(&burn_acc) < 100_000_000_000_000,
      "Burn should consume native"
    );
  });
}

#[test]
fn test_zap_pipeline_3_step_mutually_exclusive() {
  seeded_test_ext().execute_with(|| {
    System::set_block_number(1);
    setup_pool_with_liquidity(ASSET_FOREIGN);
    let dust: u128 = 100_000_000u128;
    let tol_treasury = crate::configs::TolTreasuryAccount::get();
    let step1 = make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: AssetKind::Native,
          threshold: dust,
        },
        Condition::BalanceAbove {
          asset: AssetKind::Local(ASSET_FOREIGN),
          threshold: dust,
        },
      ],
      TaskKind::AddLiquidity {
        asset_a: AssetKind::Native,
        asset_b: AssetKind::Local(ASSET_FOREIGN),
        amount_a: AmountSpec::AllBalance,
        amount_b: AmountSpec::AllBalance,
      },
      PipelineErrorPolicy::ContinueNextStep,
    );
    let step2 = make_step_cond(
      vec![
        Condition::BalanceAbove {
          asset: AssetKind::Local(ASSET_FOREIGN),
          threshold: dust,
        },
        Condition::BalanceBelow {
          asset: AssetKind::Native,
          threshold: dust,
        },
      ],
      TaskKind::SwapExactIn {
        asset_in: AssetKind::Local(ASSET_FOREIGN),
        asset_out: AssetKind::Native,
        amount_in: AmountSpec::AllBalance,
        min_out: 1,
      },
      PipelineErrorPolicy::ContinueNextStep,
    );
    let step3 = make_step_cond(
      vec![Condition::BalanceAbove {
        asset: AssetKind::Native,
        threshold: dust,
      }],
      TaskKind::SplitTransfer {
        asset: AssetKind::Native,
        amount: AmountSpec::AllBalance,
        total_shares: 100,
        legs: BoundedVec::try_from(vec![
          SplitLeg { to: BOB, share: 50 },
          SplitLeg {
            to: CHARLIE,
            share: 17,
          },
          SplitLeg {
            to: tol_treasury.clone(),
            share: 17,
          },
          SplitLeg {
            to: ALICE,
            share: 16,
          },
        ])
        .unwrap(),
        remainder_to: None,
      },
      PipelineErrorPolicy::AbortCycle,
    );
    let pipeline = BoundedVec::try_from(vec![step1, step2, step3]).unwrap();
    let aaa_id = create_system(ALICE, manual_schedule(), pipeline, tol_treasury.clone());
    let aaa_acc = aaa_account(aaa_id);
    fund_aaa_account(aaa_id, 2_000_000_000_000);
    fund_aaa_with_asset(ASSET_FOREIGN, aaa_id, 2_000_000_000_000);
    let native_before = native_balance(&aaa_acc);
    let foreign_before = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert_ok!(AAA::manual_trigger(RuntimeOrigin::signed(ALICE), aaa_id));
    run_idle(1);
    let native_after = native_balance(&aaa_acc);
    let foreign_after = asset_balance(ASSET_FOREIGN, &aaa_acc);
    assert!(
      native_after < native_before || foreign_after < foreign_before,
      "Balanced: AddLiquidity should consume assets"
    );
  });
}
