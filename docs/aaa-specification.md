# AAA Specification

- **Component:** `pallet-aaa` (Account Abstraction Actor)
- **Version:** `0.42.0`
- **Date:** February 2026
- **Status:** Normative; implementation target

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in RFC 2119.

---

## 0. Stability Contract

### 0.1 Hard requirements

1. **Determinism:** Identical chain state + block context MUST produce identical AAA behavior across all nodes.
2. **Bounded work:** Every runtime path (`on_initialize`, `on_idle`, extrinsics) MUST be O(1) or O(K) with strict `Max*` constants. No unbounded loops over unbounded storage.
3. **No chain-expense griefing:** Terminal handling MUST NOT force the chain to subsidize transfer costs for insolvent actors.
4. **Immediate destruction:** Terminal actors are refunded and destroyed in one atomic operation. No intermediate `Dormant` state, no deferred pruning queues.
5. **Stateless steps:** Steps within a pipeline MUST be independent. No ephemeral state is passed between steps. Each step reads on-chain state at the moment of its execution.
6. **Predictable failure:** Failures MUST resolve to one of: **deferral**, **step skip**, **cycle abort**, or **terminal refund**. No actor may become stuck in an unrecoverable non-terminal state.
7. **Saturating arithmetic:** All fee, rent, and balance computations MUST use saturating arithmetic. Overflow or underflow MUST NOT cause panics.

---

## 1. Actor Model

### 1.1 Instance

An AAA instance MUST store:

```rust
AaaInstance {
    // Identity
    aaa_id: u64,
    sovereign_account: AccountId,   // sovereign derived account
    owner: AccountId,               // creator/controller
    owner_slot: u16,                // deterministic slot in owner's namespace

    // Classification
    aaa_type: AaaType,              // User | System
    mutability: Mutability,         // Immutable | Mutable

    // Lifecycle
    is_paused: bool,
    pause_reason: Option<PauseReason>,

    // Scheduling
    schedule: Schedule,
    schedule_window: Option<ScheduleWindow>,

    // Pipeline
    pipeline: BoundedVec<Step, MaxSteps>,

    // Execution state
    cycle_nonce: u64,
    consecutive_failures: u32,
    manual_trigger_pending: bool,
    last_cycle_block: BlockNumber,

    // Economics
    refund_assets: BoundedVec<AssetId, MaxRefundableAssets>,
    refund_to: AccountId,           // User: owner; System: explicit target
    last_rent_block: BlockNumber,
    policy: AaaPolicy,

    // Metadata
    created_at: BlockNumber,
    updated_at: BlockNumber,
}
```

There is no `lifecycle_state` enum. An actor either exists in storage (active or paused) or has been destroyed. The `is_paused` flag distinguishes the two live states.

### 1.1A Sovereign account derivation and owner slots

AAA sovereign addresses MUST be deterministic and owner-scoped.

For each new actor of a given owner, the runtime MUST allocate `owner_slot` as the **first free slot starting from `0`** within that owner's active slot namespace.

```text
for slot in 0..MaxOwnerSlots:
    if (owner, slot) is free:
        owner_slot = slot
        break
else:
    fail OwnerSlotCapacityExceeded
```

Sovereign derivation MUST use an owner-scoped hash seed:

```text
seed = Blake2_256( SCALE(owner) || b"aaa" || LE(owner_slot) )
namespace = Mix(AAA_PalletId, seed)            // deterministic namespace hardening
sovereign_account = namespace.into_sub_account_truncating(owner_slot)
```

Rules:

1. At most one active actor MAY exist for the same `(owner, owner_slot)` pair.
2. On destruction, the `(owner, owner_slot)` binding MUST be removed, making the slot reusable.
3. This slot reuse is the emergency recovery path for orphan balances: recreating an actor and re-obtaining the same slot re-attaches protocol control to that sovereign address.
4. No dedicated owner-claim recovery extrinsic is required.

### 1.2 Types

- **User AAA:** Subject to rent, evaluation, and execution fees (§3). Created by any account.
- **System AAA:** Governance-created. Exempt from all User fee rules. Always Mutable (governance-mutable).

### 1.3 Mutability

Mutability is set at creation and MUST NOT change.

**Mutable** actors:

- Owner MUST be able to: `pause_aaa`, `resume_aaa`, `update_schedule`, `update_policy`.
- Owner MUST be able to: `fund_aaa`, `manual_trigger`, `refund_and_close`.

**Immutable** actors:

- Owner MUST NOT be able to modify schedule, policy, or pipeline.
- Owner MUST be able to: `fund_aaa`, `manual_trigger`, `refund_and_close`.
- Attempts to call `pause_aaa`, `resume_aaa`, `update_schedule`, `update_policy` MUST return `Error::ImmutableActor`.

**Error policy restriction:** `PauseActor` is not available as an error policy (see §5.4). This eliminates the Immutable downgrade problem entirely — no actor can enter a paused state via pipeline failure that its owner cannot recover from.

### 1.4 Lifecycle: Immediate Destruction

```
Active  → Paused                    (Manual pause; Mutable only)
Paused  → Active                    (Manual resume; Mutable only)
Active  → Refunded & Destroyed      (owner refund_and_close)
Active  → Refunded & Destroyed      (auto-terminal condition)
Paused  → Refunded & Destroyed      (auto-terminal on touch: rent insolvency)
```

There is **no** `Dormant` state and **no** prune queue. Upon reaching a terminal condition, the runtime MUST atomically:

1. Execute the terminal refund logic (§4.2).
2. Remove the `AaaInstance` and all associated index entries from storage.
3. Emit `AAADestroyed`.

The sovereign `sovereign_account` continues to exist natively on-chain as long as any balance exceeds Existential Deposit. Any balances outside configured refund handling remain at that address and are out of active protocol control until owner-slot reuse re-attaches control (§1.1A).

### 1.5 Auto-terminal conditions

A User AAA MUST be terminally refunded and destroyed if **any** of the following hold:

| Condition                                       | Trigger point       | RefundReason          |
| ----------------------------------------------- | ------------------- | --------------------- |
| `rent_due > native_balance` after lazy charge   | Any touch           | `RentInsolvent`       |
| `native_balance < MinUserBalance`               | Before cycle start  | `BalanceExhausted`    |
| `consecutive_failures > MaxConsecutiveFailures` | After cycle failure | `ConsecutiveFailures` |
| `schedule_window.end < current_block`           | Any touch or sweep  | `WindowExpired`       |

System AAA: exempt from rent/fee-based terminal conditions. `consecutive_failures` limit and `schedule_window` expiry still apply; governance MAY configure alternative thresholds.

### 1.6 Consecutive failures

1. `consecutive_failures` MUST increment by one when a cycle completes with a pipeline failure (any step fails and `on_error = AbortCycle`).
2. `consecutive_failures` MUST reset to zero on any successful cycle completion (all steps executed or skipped without abort).
3. `consecutive_failures` MUST NOT increment on deferred cycles (cycle never started).

---

## 2. Adapter Traits

AAA MUST execute tasks exclusively through typed adapter traits. AAA MUST NOT dispatch arbitrary extrinsics or call untyped runtime APIs.

### 2.1 AssetOps

```rust
trait AssetOps<AccountId, AssetId, Balance> {
    fn transfer(from: &AccountId, to: &AccountId, asset: AssetId, amount: Balance)
        -> Result<(), DispatchError>;
    fn burn(who: &AccountId, asset: AssetId, amount: Balance)
        -> Result<(), DispatchError>;
    fn mint(to: &AccountId, asset: AssetId, amount: Balance)
        -> Result<(), DispatchError>;
    fn balance(who: &AccountId, asset: AssetId) -> Balance;
}
```

`mint` origin guard: The pallet MUST verify `aaa_type == System` before dispatching any `Mint` task. A User AAA pipeline containing `Mint` MUST be rejected at creation with `Error::MintNotAllowedForUserAaa`.

### 2.2 DexOps

DexOps implementations MUST be bounded. No unbounded routing or pool discovery.

Bounded O(K) logic is allowed when all of the following hold:

1. `K` is explicitly capped by a runtime constant (`MaxK`).
2. The cap is part of runtime configuration and governance visibility.
3. Weights/benchmarks are generated against worst-case `K = MaxK`.
4. The implementation behavior is deterministic under equal state.

Pure O(1) adapters remain RECOMMENDED where practical, but are not REQUIRED.

```rust
trait DexOps<AccountId, AssetId, Balance> {
    fn swap_exact_in(who: &AccountId, asset_in: AssetId, asset_out: AssetId,
        amount_in: Balance, min_out: Balance) -> Result<Balance, DispatchError>;
    fn swap_exact_out(who: &AccountId, asset_in: AssetId, asset_out: AssetId,
        amount_out: Balance, max_in: Balance) -> Result<Balance, DispatchError>;
    fn get_quote(asset_in: AssetId, asset_out: AssetId, amount_in: Balance)
        -> Option<Balance>;
    fn add_liquidity(who: &AccountId, asset_a: AssetId, asset_b: AssetId,
        amount_a: Balance, amount_b: Balance)
        -> Result<(Balance, Balance, Balance), DispatchError>;
    fn remove_liquidity(who: &AccountId, lp_asset: AssetId, lp_amount: Balance)
        -> Result<(Balance, Balance), DispatchError>;
    fn get_pool_reserves(asset_a: AssetId, asset_b: AssetId)
        -> Option<(Balance, Balance)>;
}
```

### 2.3 Runtime integration

```rust
impl pallet_aaa::Config for Runtime {
    type AssetId = AssetKind;
    type Balance = u128;
    type AssetOps = TmctolAssetOpsAdapter;
    type DexOps = TmctolDexOpsAdapter;
}
```

### 2.4 ABI stability

Breaking adapter trait changes REQUIRE an explicit version bump and migration plan. The runtime MUST NOT silently reinterpret existing method semantics across upgrades.

### 2.5 Task weight contract

For every task kind `T`, the runtime MUST expose:

```rust
fn weight_upper_bound(task: T, params: TaskParams) -> Weight
```

This MUST be a **true worst-case upper bound** that is:

- Deterministic (function of `T` and `params` only, not runtime state).
- Constant under configured `Max*` limits.
- Greater than or equal to actual execution weight in all cases.

If a pipeline step uses a dynamic amount (e.g., `AllBalance`), `weight_upper_bound` MUST resolve to the worst-case bounded constant for that task type under configured `Max*` limits (including adapter `MaxK`), irrespective of the dynamic payload.

If actual execution weight exceeds the declared upper bound, this is a **critical implementation bug**.

---

## 3. Economics (User AAA)

System AAA is exempt from all fee rules in this section.

### 3.1 Fee layers and charging order

AAA uses a three-layer fee model. Layers are evaluated in strict order per step.

| Layer          | When                            | Amount                                        | Insufficient →                  |
| -------------- | ------------------------------- | --------------------------------------------- | ------------------------------- |
| **Rent**       | Lazy on touch (once per cycle)  | `min(elapsed × RentPerBlock, MaxRentAccrual)` | Auto-refund (`RentInsolvent`)   |
| **Evaluation** | Before conditions for each step | `StepBaseFee + ConditionReadFee × N`          | `StepError` → `on_error` policy |
| **Execution**  | After conditions pass, per step | `WeightToFee(weight_upper_bound(task))`       | `StepError` → `on_error` policy |

**Within a cycle:**

```
on_initialize (readiness evaluation):
    1. Lazy rent charge
       → if insolvent: auto-refund & destroy, exit
    2. MinUserBalance gate
       → if below: auto-refund & destroy, exit

on_idle (cycle execution):
    3. Pre-flight fee admission (cycle_fee_upper, §3.7)
       → if insufficient: CycleDeferred(InsufficientBudget), exit
    4. Reserve fee budget (reserved_fee_remaining)
    5. Increment cycle_nonce
    For each step:
        6. Charge evaluation fee
           → if insufficient: StepError, apply on_error
        7. Evaluate conditions
           → if false: StepSkipped, advance
           → if error: StepError, apply on_error (fail-closed)
        8. Charge execution fee
           → if insufficient: StepError, apply on_error
        9. Execute task
           → if failure: StepError, apply on_error
```

This order ensures: (1) storage costs are always covered first, (2) condition reads are paid for even when they result in a skip, (3) execution fees are only charged when the task will actually dispatch.

### 3.2 Rent

```
blocks_elapsed = saturating_sub(current_block, last_rent_block)
raw_rent       = saturating_mul(blocks_elapsed, RentPerBlock)
rent_due       = min(raw_rent, MaxRentAccrual)

if native_balance >= rent_due:
    native_balance -= rent_due
    last_rent_block = current_block
else:
    → auto-refund & destroy with reason RentInsolvent
```

**Rent ceiling rationale:** Without a cap, an actor untouched for millions of blocks could accumulate rent exceeding any reasonable storage cost. `MaxRentAccrual` bounds the maximum single-charge rent. The zombie sweep (§8.5) ensures actors are touched frequently enough that the ceiling is rarely reached.

All arithmetic MUST use saturating operations per §0.1(7).

### 3.3 Evaluation fee

The evaluation fee protects the chain from actors that spam heavy condition reads without ever executing tasks:

```
eval_fee = StepBaseFee + (ConditionReadFee × conditions.len())
```

- Charged per step, before conditions are evaluated.
- Fee is deposited to FeeSink regardless of condition outcome.
- If `native_balance < eval_fee`: the step MUST fail with `InsufficientEvaluationFee` and the per-step `on_error` policy MUST apply. This is a step error, NOT a terminal condition — the actor may still have funds for subsequent steps.

**Rationale:** Replaces the previous flat `AdmissionFee` model with a granular per-step charge proportional to the actual read work. Actors with zero conditions pay only `StepBaseFee`. Actors with 4 balance reads pay proportionally more.

### 3.4 Execution fee

```
execution_fee = WeightToFee(weight_upper_bound(task, params))
```

- Charged per step, after conditions pass, before dispatch.
- MUST use the same runtime `WeightToFee` conversion contract used by transaction-payment.
- Fee is deposited to FeeSink even if the task subsequently fails (pay for compute attempt).
- If `native_balance < execution_fee`: the step MUST fail with `InsufficientExecutionFee` and the per-step `on_error` policy MUST apply. This is a step error, NOT a terminal condition.

**Rationale:** User pays the same fee whether executing via AAA or directly. Fee insufficiency at the step level is handled by `on_error` policy, not instant destruction — this preserves multi-step pipeline viability when some steps are routinely skipped via conditions.

### 3.5 Fee destination (FeeSink)

All collected fees (evaluation + execution) and forfeited non-native assets MUST be deposited into a dedicated FeeSink account.

The FeeSink SHOULD be implemented as a System AAA with the following canonical pipeline:

1. **Condition:** `BalanceAbove(Native, Dust)`
2. **Task:** `SplitTransfer` (AllBalance)
   - 50% → `BurningManager` (System AAA)
   - 50% → Staking Rewards Pot

This is a RECOMMENDATION, not a REQUIREMENT. Alternative FeeSink architectures are permitted provided all fees are routed to a deterministic, auditable destination.

### 3.6 Configurable parameters

| Parameter                | Default                       | Scope       | Notes                           |
| ------------------------ | ----------------------------- | ----------- | ------------------------------- |
| `StepBaseFee`            | 0.001 Native                  | User        | Per-step flat evaluation cost   |
| `ConditionReadFee`       | 0.0005 Native                 | User        | Per-condition balance read cost |
| `RentPerBlock`           | 0.0001 Native                 | User        | Linear accrual                  |
| `MaxRentAccrual`         | 1000 × `RentPerBlock`         | User        | Per-touch ceiling               |
| `MinUserBalance`         | 5 × ED                        | User        | Pre-cycle gate                  |
| `MaxConsecutiveFailures` | 10                            | User/System | Terminal threshold              |
| `MaxSweepPerBlock`       | 5                             | Global      | Zombie sweep bound              |
| `RefundTransferCost`     | `WeightToFee(TransferWeight)` | Global      | Per-asset refund cost           |

### 3.7 Pre-flight admission and fee reservation

Before starting a User cycle, the runtime MUST compute a worst-case cycle fee upper bound:

```
cycle_fee_upper = rent_due + Σ_i(eval_fee_i + exec_fee_upper_i)
exec_fee_upper_i = WeightToFee(weight_upper_bound(task_i, params_i))
```

Admission MUST satisfy both constraints:

1. Weight admission (§8.4)
2. `native_balance >= cycle_fee_upper`

If `native_balance < cycle_fee_upper`, the cycle MUST be deferred with
`CycleDeferred { reason: InsufficientBudget }`, and `cycle_nonce` MUST NOT increment.
This deferral MUST NOT increment `consecutive_failures`.

During cycle execution, the runtime MUST maintain `reserved_fee_remaining` and MUST NOT allow
native-spending tasks to consume reserved fee balance.

For native-denominated amount resolution (`AllBalance(Native)`, `PercentOfBalance(Native, ..)`,
or equivalent native spend paths), implementations MUST use:

```
spendable_native = max(native_balance - reserved_fee_remaining, 0)
```

This guarantees that a cycle admitted by pre-flight cannot fail later due only to
internal fee starvation caused by earlier native-spending steps.

---

## 4. Refunds and Anti-Griefing

### 4.1 Refund assets set

1. At creation, the runtime MUST derive `refund_assets` from the union of all asset IDs referenced in the pipeline's task parameters.
2. The native asset MUST always be included.
3. Updates to `refund_assets` MUST be append-only (new assets added, none removed).
4. Governance MAY call `update_refund_assets(aaa_id, additional_assets)` for System AAA actors with omnivorous patterns (e.g., BurningManager).

### 4.2 Terminal handling: no chain-expense griefing

Define:

```
refund_threshold = len(refund_assets) × RefundTransferCost
```

On terminal destruction (auto-refund or `refund_and_close`), the runtime MUST apply:

**Solvent case** (`native_balance >= refund_threshold`):

- Transfer all assets in `refund_assets` to `refund_to`.
- Any remaining native balance after transfer costs: included in transfer.

**Insolvent case** (`native_balance < refund_threshold`):

- Burn remaining native balance (dust destruction).
- Sweep all non-native assets in `refund_assets` to FeeSink (forfeiture).
- Non-native assets become protocol revenue, not a subsidized refund.

This rule ensures terminal actors cannot force the chain to pay for expensive multi-asset transfers. The insolvent path converts potential griefing into protocol revenue.

### 4.3 Orphaned assets

Assets held by the actor's derived `sovereign_account` but **not** in `refund_assets` are orphaned. They are NOT automatically refunded or forfeited.

Because the derived `sovereign_account` continues to exist natively on-chain after `AaaInstance` deletion (as long as any balance exceeds ED), orphaned assets remain at that address.

AAA does **not** provide direct post-destruction claim extrinsics for orphaned assets.
Recovery is slot-based and indirect: owner recreates AAA, runtime allocates first free slot from `0`, and control is recovered when the same `(owner, owner_slot)` is obtained again (§1.1A).

**Operational policy:** refund MUST be treated as a happy-path flow.

1. **Pre-terminal (required practice):** For Mutable actors, owner SHOULD add all expected assets to `refund_assets` before closure. For System AAA, governance SHOULD do the same.
2. **Post-destruction (emergency path):** Assets not covered by `refund_assets` remain on the former sovereign address until owner slot reuse re-attaches control.

### 4.4 No dead-owner registry in stable contract

The protocol MUST NOT maintain post-destruction ownership tracking storage for destroyed AAA instances.
`DeadAaaOwners` and TTL-based pruning are excluded from the stable contract.

The protocol MAY maintain bounded **active** slot bindings (`owner + owner_slot`) required for deterministic sovereign derivation and slot reuse (§1.1A).

**Rationale:** This keeps terminal handling minimal and deterministic, avoids long-lived dead-owner registries, and provides bounded emergency recovery via deterministic owner-slot reuse.

### 4.5 Refund event

```rust
AAARefunded {
    aaa_id: AaaId,
    reason: RefundReason,
    solvent: bool,
    to: AccountId,
    assets_refunded: BoundedVec<(AssetId, Balance), MaxRefundableAssets>,
    assets_forfeited: BoundedVec<(AssetId, Balance), MaxRefundableAssets>,
    native_burned: Balance,
}
```

---

## 5. Pipeline

### 5.1 Structure

```
trigger → step_0 → step_1 → ... → step_n
```

One task per step. Full pipeline traversal = one cycle. Steps are fully independent — each reads on-chain state at its moment of execution.

**Bounds:**

- User AAA: `MaxSteps` MUST be ≤ 3.
- System AAA: `MaxSteps` MUST be ≤ 10 (configurable).
- Empty pipelines MUST be rejected at creation (`Error::EmptyPipeline`).

### 5.2 Step

```rust
Step {
    conditions: BoundedVec<Condition, MaxConditionsPerStep>,
    task: Task,
    on_error: ErrorPolicy,
}
```

### 5.3 Conditions

```rust
enum Condition<AssetId, Balance> {
    BalanceAbove(AssetId, Balance),
    BalanceBelow(AssetId, Balance),
    BalanceEquals(AssetId, Balance),
    BalanceNotEquals(AssetId, Balance),
}
```

1. All conditions MUST be AND-composed.
2. Empty conditions = unconditional execution.
3. Any condition evaluating to false → `StepSkipped`.
4. Any condition evaluation error → `StepError` (fail-closed).

**Branching** is achieved via complementary conditions on separate steps:

```
Step 0: [BalanceAbove(Native, dust), BalanceAbove(Foreign, dust)] → AddLiquidity
Step 1: [BalanceAbove(Foreign, dust), BalanceBelow(Native, dust)] → SwapToNative
```

### 5.4 Error policies

Per-step `on_error`:

- **`AbortCycle`** (default): Stop pipeline, emit `PipelineFailed`, increment `consecutive_failures`.
- **`ContinueNextStep`**: Skip failed step, advance to next. Does NOT increment `consecutive_failures` (the cycle may still succeed).

`PauseActor` is NOT available as an error policy. Actors pause only via explicit owner action (`pause_aaa`). This eliminates the Immutable downgrade problem and prevents actors from entering states requiring manual recovery after automated failures.

On the next trigger after `AbortCycle` or a resolved cycle, the pipeline re-runs from step 0.

### 5.5 Atomicity

- **Task-level:** Atomic. A failed task MUST NOT persist partial effects.
- **Pipeline-level:** Non-atomic. Successful prior steps are NOT rolled back on later failure.

**Design guidance (non-normative):** Operators SHOULD design idempotent pipelines:

- Place high-risk steps (swaps with slippage) early so that failure aborts before further side effects.
- Use `ContinueNextStep` only when subsequent steps are safe regardless of prior step outcomes.
- Use conditions on later steps to gate execution on expected intermediate state.
- Since `AbortCycle` restarts from step 0 on next trigger, conditions MUST guard against re-executing steps whose effects already persisted.

### 5.6 Amount resolution

Each task's amount parameter MUST resolve via one of:

```rust
enum AmountResolution<Balance> {
    Fixed(Balance),
    PercentOfBalance(AssetId, Perbill),
    AllBalance(AssetId),
}
```

All resolution reads current on-chain state at the moment of step execution. When step N-1 mutates a balance (e.g., via swap), step N's `PercentOfBalance` or `AllBalance` naturally reflects the updated balance without requiring ephemeral state passing.

### 5.7 Cycle nonce

1. `cycle_nonce` MUST increment exactly once per cycle start (after readiness checks pass, before step 0).
2. Deferred cycles MUST NOT increment the nonce.
3. `cycle_nonce` MUST NOT wrap. On reaching `u64::MAX`:
   - User AAA: MUST auto-refund and destroy with `RefundReason::CycleNonceExhausted`.
   - System AAA: MUST pause with `pause_reason = CycleNonceExhausted`. Governance decides disposition.

---

## 6. Tasks

### 6.1 Stable task set

The following tasks MUST be supported:

| Task              | Adapter                    | Description                              |
| ----------------- | -------------------------- | ---------------------------------------- |
| `Transfer`        | `AssetOps::transfer`       | Single-recipient transfer                |
| `SplitTransfer`   | `AssetOps::transfer`       | Bounded multi-recipient fan-out (atomic) |
| `Burn`            | `AssetOps::burn`           | Burn any asset                           |
| `Mint`            | `AssetOps::mint`           | Mint tokens (**System AAA only**)        |
| `SwapExactIn`     | `DexOps::swap_exact_in`    | Swap with minimum output guard           |
| `SwapExactOut`    | `DexOps::swap_exact_out`   | Swap with maximum input guard            |
| `AddLiquidity`    | `DexOps::add_liquidity`    | Opportunistic LP provisioning            |
| `RemoveLiquidity` | `DexOps::remove_liquidity` | LP removal                               |
| `Noop`            | —                          | No-op; condition-gated observation       |

`Mint` MUST be gated to System AAA. User AAA pipelines containing `Mint` MUST be rejected at creation.

`Noop` has zero execution fee and minimal weight (`BaseWeight` only). Useful for condition-gated event emission, pipeline padding, and placeholder steps in Mutable actors. Evaluation fee still applies (conditions are still read).

### 6.2 SplitTransfer validation

`SplitTransfer` uses explicit rational shares:

```rust
SplitTransfer {
    asset: AssetId,
    amount: AmountResolution,
    total_shares: u32,
    legs: BoundedVec<SplitLeg, MaxSplitTransferLegs>,
    remainder_to: Option<AccountId>,
}

SplitLeg {
    to: AccountId,
    share: u32,
}
```

Creation-time validation MUST enforce:

- Zero-share legs → `Error::ZeroShareLeg`
- Fewer than 2 legs → `Error::InsufficientSplitLegs`
- More than `MaxSplitTransferLegs` legs → `Error::TooManySplitLegs`
- Duplicate recipients → `Error::DuplicateRecipient`
- `total_shares > 0` and `sum(share_i) == total_shares` (no implicit normalization)

`AmountResolution::Fixed(value)` is the canonical absolute-amount mode.
`PercentOfBalance` and `AllBalance` remain supported as relative modes.

**Allocation formula:**

```
leg_i_amount = floor(total × share_i / total_shares)
remainder = total - Σ(leg_i_amount)
```

Remainder handling:

- If `remainder_to` is set, remainder MUST be assigned to that recipient
- Else remainder MUST be assigned to leg at index `0`

Fan-out is atomic at the task level. All legs succeed or all fail.

### 6.3 Task contract

Every task MUST define:

- Typed inputs and validation rules.
- Deterministic error set.
- Deterministic `weight_upper_bound(params)`.
- Explicit side-effect surface (which adapter methods are called).

No task MAY call adapter methods not declared in its side-effect surface.

---

## 7. Triggers

### 7.1 Supported triggers (stable)

1. **`ProbabilisticTimer`** — Every `N` blocks with probability `P`. Randomness MUST be derived from a deterministic canonical source (VRF/pallet). No external entropy, no oracle dependency.
2. **`OnAddressEvent`** — Trigger on inbound balance events to AAA account. Supports `IncludeOnly{assets}` / `Exclude{assets}` filters. Bounded inbox.
3. **`ManualTrigger`** — Owner or governance. Bypasses schedule timing only. MUST NOT bypass rent evaluation, fee charging, or cooldown.

### 7.2 OnAddressEvent inbox

Bounded per-key summary state `(aaa_id, matched_asset)`:

```rust
InboxState {
    pending_count: u32,     // saturating, max MaxAddressEventInboxCount
    saturated: bool,        // true when pending_count hit max
    last_event_block: BlockNumber,
}
```

Trigger ready: `pending_count > 0 || saturated`.

**Drain modes:**

```rust
enum InboxDrainMode {
    Single,         // consume one event per cycle (default)
    Batch(u32),     // consume min(pending_count, max) per cycle
    Drain,          // consume all pending in one cycle
}
```

- `Single`: one unit consumed per cycle start. Suitable for per-deposit processing.
- `Batch(max)`: consume `min(pending_count, max)` events. Suitable for batch-processing actors.
- `Drain`: consume all pending. Bounded by `MaxAddressEventInboxCount` (the counter's saturation cap). Suitable for actors triggered by "any activity" semantics.

`drain_mode` is set at creation and follows mutability rules. `Batch.max` bounded by `MaxAddressEventInboxCount`.

**Saturation handling:** When `saturated == true`, exact count is lost. `Drain` and `Batch` treat saturation as "consume all and reset." `Single` consumes one unit and leaves `saturated == true`.

**Source filtering:**

```rust
enum SourceFilter {
    Any,
    OwnerOnly,
    RefundAddressOnly,
    Whitelist(BoundedVec<AccountId, MaxWhitelistSize>),
}
```

All source filter variants are available to both User and System AAA. The bounded inbox with saturation cap (`MaxAddressEventInboxCount`) prevents DoS regardless of source filter configuration.

### 7.3 Schedule window

Bounded activation range (REQUIRED support):

```rust
struct ScheduleWindow {
    start: BlockNumber,  // actor eligible from this block (inclusive)
    end: BlockNumber,    // actor ineligible after this block; next touch → auto-refund
}
```

**Semantics:**

- `current_block < start` → actor is not ready (skipped in scheduling; rent still accrues).
- `start <= current_block <= end` → normal operation.
- `current_block > end` → auto-refund and destroy at next touch with `RefundReason::WindowExpired`.

**Creation-time validation MUST enforce:**

- `end > start` (else `Error::InvalidScheduleWindow`)
- `end - start >= MinWindowLength` (else `Error::WindowTooShort`)
- `start >= current_block` (no retroactive windows)

**Update rules (Mutable only):** `update_schedule` MAY modify the window subject to the same validation. Shortening is allowed. Extension requires `new_end >= current_block`.

**Interaction with Immutable actors:** Provides a natural termination condition. An Immutable timelock actor can set `end` to the desired unlock block, guaranteeing automatic refund without owner intervention.

---

## 8. Scheduler

### 8.1 Block budget

```
AAA_BUDGET = AAA_MAX_BLOCK_SHARE × TotalBlockWeight
```

The scheduler MUST NOT execute work exceeding `AAA_BUDGET` in any single block. This is a hard ceiling, not a target.

### 8.2 Ordering and fairness

1. Scheduling MUST be deterministic (reproducible across nodes given identical state).
2. Class arbitration MUST use weighted round-robin between System and User classes.
3. Per-class execution remains bounded by `MaxSystemExecutionsPerBlock` and `MaxUserExecutionsPerBlock`.
4. If one class has no ready actors, its share is skipped (no synthetic work injection).
5. No priority fee lane. No paid acceleration.

### 8.3 Queue

Implementations MUST expose deterministic class-aware ready scheduling for weighted arbitration (§8.2).
This may be implemented as:

- two physical class queues (`ReadyRingSystem`, `ReadyRingUser`), or
- one physical queue with deterministic class partition views

provided observable behavior is equivalent.

Total queued readiness MUST remain bounded by `MaxReadyRingLength`.

Overflow → `Deferred` with reason `QueueOverflow`. Deferred actors MUST be retried via a bounded rotating cursor. The cursor MUST guarantee that every deferred actor is re-evaluated within a bounded number of blocks (starvation-safe).

### 8.4 Admission

```
required_weight = weight_upper_bound(full_cycle)
weight_admit    = (aaa_budget_remaining >= required_weight)

required_fee_upper = cycle_fee_upper (§3.7)
fee_admit          = (native_balance >= required_fee_upper)

admit = weight_admit && fee_admit
```

- If `weight_admit == false` → cycle deferred (`InsufficientBudget`). `cycle_nonce` MUST NOT increment.
- If `fee_admit == false` → cycle deferred (`InsufficientBudget`). `cycle_nonce` MUST NOT increment.
- Deferral on admission MUST NOT increment `consecutive_failures`.
- Weight bounds are strict. No slack tolerance.

### 8.5 Ready predicate

An actor is ready iff ALL of:

- `is_paused == false`
- `GlobalCircuitBreaker == false`
- Cooldown elapsed (`current_block - last_cycle_block >= cooldown`)
- Rent OK (no insolvency on lazy evaluation)
- Trigger satisfied OR `manual_trigger_pending == true`
- Not already executing this block
- Within `schedule_window` (if set)

### 8.6 Zombie sweep

The runtime MUST implement a bounded per-block sweep for rent and lifecycle evaluation:

1. A persistent cursor iterates over all AAA IDs.
2. Per block, at most `MaxSweepPerBlock` actors are evaluated.
3. Evaluation performs lazy rent charge and checks auto-terminal conditions.
4. The cursor wraps around, ensuring every actor is eventually touched.

### 8.7 Permissionless sweep

The runtime MUST provide:

```rust
fn permissionless_sweep(aaa_id: AaaId) -> DispatchResult
```

- O(1) operation forcing the same rent/lifecycle evaluation path used by zombie sweep.
- Callable by any account.
- Charged as a normal extrinsic (prevents spam).
- MUST NOT bypass any economic checks.

Rationale: At scale (100k+ actors), the zombie sweep cursor may take days to complete a full revolution. Permissionless sweep allows any interested party to force-evaluate an actor's liveness without waiting for background rotation.

### 8.8 Cooldowns

- User AAA: 3–5 blocks (configurable).
- System AAA: 50–300 blocks (configurable).

---

## 9. Extrinsics

### 9.1 User extrinsics

| Extrinsic                                                                             | Mutability   | Description                     |
| ------------------------------------------------------------------------------------- | ------------ | ------------------------------- |
| `create_user_aaa(mutability, schedule, schedule_window, pipeline, policy, refund_to)` | —            | Create actor                    |
| `pause_aaa(aaa_id)`                                                                   | Mutable only | Pause actor                     |
| `resume_aaa(aaa_id)`                                                                  | Mutable only | Resume paused actor             |
| `manual_trigger(aaa_id)`                                                              | Any          | Set manual trigger flag         |
| `fund_aaa(aaa_id, amount)`                                                            | Any          | Deposit native to actor         |
| `refund_and_close(aaa_id)`                                                            | Any          | Owner-initiated terminal refund |
| `update_policy(aaa_id, policy)`                                                       | Mutable only | Update error policy             |
| `update_schedule(aaa_id, schedule, schedule_window)`                                  | Mutable only | Update schedule/window          |

`create_user_aaa` MUST allocate `owner_slot` as first free slot for that owner starting from `0` (§1.1A), then derive `sovereign_account` from `hash(concat(owner, b"aaa", owner_slot))`.

### 9.2 Governance extrinsics

| Extrinsic                                         | Description                        |
| ------------------------------------------------- | ---------------------------------- |
| `create_system_aaa(...)`                          | Create System AAA (always Mutable) |
| `set_global_circuit_breaker(paused: bool)`        | Halt/resume all AAA execution      |
| `update_refund_assets(aaa_id, additional_assets)` | Append to System AAA refund set    |

`create_system_aaa` MUST apply the same deterministic owner-slot allocation policy for the provided owner (§1.1A).

### 9.3 Tooling extrinsics

| Extrinsic                      | Status      | Description                     |
| ------------------------------ | ----------- | ------------------------------- |
| `permissionless_sweep(aaa_id)` | REQUIRED    | Force rent/lifecycle evaluation |
| `dry_run_cycle(aaa_id)`        | RECOMMENDED | Read-only cycle simulation      |

**`dry_run_cycle`:** Simulates one full cycle without mutating state. Returns:

```rust
DryRunResult {
    would_admit: bool,
    would_admit_weight: bool,
    would_admit_fee: bool,
    rent_due: Balance,
    cycle_fee_upper: Balance,
    steps: BoundedVec<DryRunStepResult, MaxSteps>,
}

DryRunStepResult {
    conditions_met: bool,
    estimated_eval_fee: Balance,
    estimated_execution_fee_upper: Balance,
}
```

Limitations: DEX quotes may differ at actual execution time. Condition evaluations reflect current-block state only. `dry_run_cycle` is charged as a normal extrinsic to prevent abuse.

### 9.4 Circuit breaker

1. The runtime MUST implement a global circuit breaker (`set_global_circuit_breaker`).
2. When active, the scheduler MUST NOT enqueue new ready actors and MUST NOT execute AAA cycles.
3. `create_user_aaa` and `create_system_aaa` MUST fail with `Error::GlobalCircuitBreakerActive` while breaker is active.
4. Cleanup and operational control paths MUST remain functional during breaker:
   `fund_aaa`, `refund_and_close`, `dry_run_cycle`, `permissionless_sweep`,
   and governance control extrinsics except creation calls.

---

## 10. Runtime Hooks

### 10.1 `on_initialize`

MUST:

- Perform bounded lifecycle cleanup work required for safety invariants
  (including schedule-window lifecycle checks).
- When circuit breaker is inactive:
  - Evaluate trigger readiness (bounded by `MaxReadyRingLength`)
  - Consume `OnAddressEvent` inbox entries (bounded by drain mode caps)
  - Enqueue ready actors into class-aware ready scheduling queues (§8.3)
- When circuit breaker is active:
  - MUST NOT enqueue ready actors

MUST NOT contain unbounded loops.

### 10.2 `on_idle`

MUST:

- When circuit breaker is inactive: execute cycles within `AAA_BUDGET` using weighted round-robin (§8.2)
- Perform bounded zombie sweep (`MaxSweepPerBlock`)

MUST NOT contain unbounded loops.

---

## 11. Events

Implementations MUST emit the following events:

### 11.1 Lifecycle

```rust
AAACreated { aaa_id, owner, owner_slot, aaa_type, mutability, sovereign_account }
AAAFunded { aaa_id, amount }
AAAPaused { aaa_id, reason: PauseReason }
AAAResumed { aaa_id }
AAARefunded { aaa_id, reason, solvent, to, assets_refunded, assets_forfeited, native_burned }
AAADestroyed { aaa_id }
```

### 11.2 Execution

```rust
CycleDeferred { aaa_id, reason: DeferReason }
CycleStarted { aaa_id, cycle_nonce }
StepSkipped { aaa_id, cycle_nonce, step_index }
StepFailed { aaa_id, cycle_nonce, step_index, error: DispatchError }
PipelineExecuted { aaa_id, cycle_nonce }
PipelineFailed { aaa_id, cycle_nonce, failed_step, error: DispatchError }
```

### 11.3 Task effects

```rust
TransferExecuted { aaa_id, asset, amount, to }
SplitTransferExecuted { aaa_id, asset, total, legs: u32 }
SwapExecuted { aaa_id, asset_in, asset_out, amount_in, amount_out }
BurnExecuted { aaa_id, asset, amount }
MintExecuted { aaa_id, asset, amount }
LiquidityAdded { aaa_id, asset_a, asset_b, lp_minted }
LiquidityRemoved { aaa_id, lp_asset, amount_a, amount_b }
```

### 11.4 Administrative

```rust
PolicyUpdated { aaa_id }
ScheduleUpdated { aaa_id }
GlobalCircuitBreakerSet { paused: bool }
ManualTriggerSet { aaa_id }
```

---

## 12. Enums

### 12.1 RefundReason

```rust
enum RefundReason {
    OwnerInitiated,
    RentInsolvent,
    BalanceExhausted,
    ConsecutiveFailures,
    WindowExpired,
    CycleNonceExhausted,
}
```

### 12.2 DeferReason

```rust
enum DeferReason {
    QueueOverflow,
    InsufficientBudget,
}
```

`InsufficientBudget` is used for both weight-admission failures and pre-flight fee-admission failures (§8.4, §3.7).

### 12.3 PauseReason

```rust
enum PauseReason {
    Manual,
    CycleNonceExhausted,
}
```

Note: `StepFailure` is removed as a pause reason. Failed steps apply `AbortCycle` or `ContinueNextStep`; neither results in a paused state.

---

## 13. Errors

Implementations MUST define, at minimum:

```rust
enum Error {
    // Lookup
    AaaNotFound,

    // Permission
    NotOwner,
    ImmutableActor,
    NotGovernance,

    // Creation validation
    EmptyPipeline,
    PipelineTooLong,
    MintNotAllowedForUserAaa,
    ZeroShareLeg,
    InsufficientSplitLegs,
    TooManySplitLegs,
    DuplicateRecipient,
    InvalidSplitShareTotal,
    InvalidScheduleWindow,
    WindowTooShort,
    OwnerSlotCapacityExceeded,
    SovereignAccountCollision,

    // Runtime
    InsufficientBalance,
    InsufficientEvaluationFee,
    InsufficientExecutionFee,
    QueueOverflow,
    CycleNonceExhausted,
    GlobalCircuitBreakerActive,
    TaskExecutionFailed(DispatchError),

    // Recovery/derivation
    // (no direct post-destruction claim extrinsic; slot-based recovery via recreate)
}
```

---

## 14. Storage

| Storage                | Type                                  | Purpose                                           |
| ---------------------- | ------------------------------------- | ------------------------------------------------- |
| `AaaInstances`         | `Map<AaaId, AaaInstance>`             | Active actor state                                |
| `OwnerIndex`           | `Map<AccountId, BoundedVec<AaaId>>`   | Lookup active actors by owner                     |
| `OwnerSlots`           | `Map<(AccountId, u16), AaaId>`        | Active slot binding per owner (`first-free` scan) |
| `SovereignIndex`       | `Map<AccountId, AaaId>`               | Collision guard: active sovereign uniqueness      |
| `ReadyRing*`           | implementation-defined bounded queues | Class-aware ready scheduling                      |
| `AddressEventInbox`    | `Map<(AaaId, AssetId), InboxState>`   | Per-actor event tracking                          |
| `SweepCursor`          | `AaaId`                               | Position in zombie sweep                          |
| `GlobalCircuitBreaker` | `bool`                                | Pallet-wide execution halt                        |

All collections MUST be bounded by their respective `Max*` constants.

Removed from earlier iterations: `DormantQueue`, `PruneCursor`.
Removed from stable contract scope: `DeadAaaOwners` (no post-destruction owner registry).

---

## 15. Safety Invariants

An implementation is compliant iff **all** of the following hold:

| #   | Invariant                                          | Verification                                                                                                                     |
| --- | -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Budget cap** respected every block               | `AAA_BUDGET` never exceeded in cycle execution paths                                                                             |
| 2   | **All queues and loops bounded**                   | `Max*` constants enforce O(K) bounds on all iteration                                                                            |
| 3   | **Determinism** preserved                          | Identical state + block → identical behavior. No external entropy                                                                |
| 4   | **No chain-expense refunds**                       | Insolvent terminal handling burns native dust, forfeits non-native to FeeSink                                                    |
| 5   | **Immediate destruction**                          | Terminal conditions atomically refund and destroy instance                                                                       |
| 6   | **Stateless steps**                                | No ephemeral state passes between steps. Each step reads on-chain state directly                                                 |
| 7   | **Task weight contract** holds                     | `weight_upper_bound` ≥ actual weight for all tasks, always                                                                       |
| 8   | **Permissionless sweep** is O(1) and safe          | Any account can force evaluation; no bypass of economic checks                                                                   |
| 9   | **Saturating arithmetic** everywhere               | No panics from overflow/underflow in fee, rent, or balance math                                                                  |
| 10  | **Consecutive failures** correctly tracked         | Increment on cycle failure, reset on success, never on deferral                                                                  |
| 11  | **Circuit breaker** halts enqueue+execution        | No new ready enqueue and no cycle execution while breaker is active                                                              |
| 12  | **Circuit breaker** preserves cleanup paths        | Funding, refund, sweep, and governance control remain available during breaker                                                   |
| 13  | **No mid-block retries**                           | Failed steps apply `on_error` immediately. Retries occur via next scheduled cycle                                                |
| 14  | **No dead-owner registry; slot recovery only**     | No `DeadAaaOwners`-style tracking is maintained; recovery occurs only via deterministic owner-slot reuse                         |
| 15  | **Rent ceiling** prevents single-touch drain       | `MaxRentAccrual` caps rent per lazy evaluation                                                                                   |
| 16  | **Pre-flight fee reservation is respected**        | Admitted cycles cannot fail later due only to internal fee starvation from earlier native spend                                  |
| 17  | **Weighted fairness is deterministic**             | Class arbitration follows weighted round-robin under fixed config and state                                                      |
| 18  | **Owner slot allocation is bounded/deterministic** | First-free scan starts at slot `0`, is capped by `MaxOwnerSlots`, and derives sovereign from `hash(concat(owner, b"aaa", slot))` |

---

## 16. Runtime Constants

| Constant                         | Recommended                   | Notes                                             |
| -------------------------------- | ----------------------------- | ------------------------------------------------- |
| `AAA_MAX_BLOCK_SHARE`            | 20–35%                        | Hard ceiling for AAA weight per block             |
| `MAX_USER_PIPELINE_STEPS`        | 3                             | User AAA step limit                               |
| `MAX_SYSTEM_PIPELINE_STEPS`      | 10                            | System AAA step limit                             |
| `MaxReadyRingLength`             | 128                           | Total ready queue bound                           |
| `MaxSystemExecutionsPerBlock`    | 8–32                          | Hard cap for System class per block               |
| `MaxUserExecutionsPerBlock`      | 8–64                          | Hard cap for User class per block                 |
| `FairnessWeightSystem`           | 1                             | Weighted RR class share                           |
| `FairnessWeightUser`             | 2–4                           | Weighted RR class share                           |
| `MaxRefundableAssets`            | 16                            | Per-actor refund set                              |
| `MaxConditionsPerStep`           | 4                             | AND-composed conditions per step                  |
| `MaxSplitTransferLegs`           | 8                             | Fan-out recipients                                |
| `MaxSweepPerBlock`               | 5                             | Zombie sweep throughput                           |
| `MaxAddressEventInboxCount`      | 64                            | Per-key saturation cap                            |
| `MaxWhitelistSize`               | 16                            | Source filter allowlist                           |
| `MaxConsecutiveFailures`         | 10                            | Terminal failure threshold                        |
| `MaxOwnerSlots`                  | 64–1024                       | Per-owner slot namespace cap (`first-free` scan)  |
| `MinWindowLength`                | 100 blocks                    | Minimum schedule window                           |
| `MaxRentAccrual`                 | 1000 × `RentPerBlock`         | Per-touch rent ceiling                            |
| `StepBaseFee`                    | 0.001 Native                  | Per-step flat evaluation cost                     |
| `ConditionReadFee`               | 0.0005 Native                 | Per-condition balance read cost                   |
| `RefundTransferCost`             | `WeightToFee(TransferWeight)` | Per-asset refund cost                             |
| `MaxK*` (adapter-specific scans) | runtime-specific              | Upper bound for permitted O(K) adapter operations |
| User cooldown                    | 3–5 blocks                    | Minimum inter-cycle gap                           |
| System cooldown                  | 50–300 blocks                 | Minimum inter-cycle gap                           |

---

## 17. Canonical Scenarios (Non-normative)

These scenarios illustrate intended usage. They are NOT normative requirements.

### User DCA

Periodic swap from stable to target token. `ProbabilisticTimer`, fixed or percentage amount. Single-step pipeline.

### Timelock Transfer

Immutable actor with `ScheduleWindow(start=unlock_block, end=unlock_block+grace)`. Single `Transfer` step. Guaranteed execution window with automatic refund if unclaimed.

### Revocable Payroll

Mutable actor, recurring `Transfer` to employee. Owner can pause/update/close at will.

### Opportunistic Zap-to-LP (System)

3-step stateless pipeline:

1. `[BalanceAbove(Native, dust), BalanceAbove(Foreign, dust)]` → `AddLiquidity(AllBalance(Native), AllBalance(Foreign))`
2. `[BalanceAbove(Foreign, dust), BalanceBelow(Native, dust)]` → `SwapExactIn(AllBalance(Foreign))` Foreign→Native
3. `[BalanceAbove(LP, dust)]` → `SplitTransfer(AllBalance(LP))` to buckets `[3,1,1,1]`

Each step reads current balances directly. Step 1's `AddLiquidity` leaves residual balances; step 2 reads those residuals via `AllBalance`. No ephemeral state needed. Branching via complementary conditions ensures only the applicable path executes.

### Burn Actor (System)

`OnAddressEvent` trigger with `Drain` inbox mode. Single `Burn` step. Governance maintains `refund_assets`.

### Observation Actor

`Noop`-only pipeline with conditions. Emits `StepSkipped`/`CycleStarted` for off-chain monitoring without side effects.

### Protocol FeeSink (System)

System AAA consuming collected fees:

1. **Condition:** `BalanceAbove(Native, Dust)`
2. **Task:** `SplitTransfer` (AllBalance) — 50% → BurningManager, 50% → Staking Rewards Pot

---

_End of specification._
