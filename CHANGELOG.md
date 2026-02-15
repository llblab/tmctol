# TMCTOL Changelog

All notable changes to the TMCTOL (Token Minting Curve + Treasury-Owned Liquidity) framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `AAA Final Test-Suite Audit`: Performed final hygiene pass on AAA test surfaces: removed duplicated Noop coverage (`noop_does_not_transfer`), kept scheduler/boundedness invariants in a single coherent block, normalized section comments for non-obvious regression guards, and aligned spacing/formatting across pallet/runtime tests and AAA benchmarking module.
- `AAA Phase 10 Validation Closure`: Closed remaining matrix gaps for `Budget cap` and `All cycles/queues bounded`. Added runtime property-style budget tests across multiple `remaining_weight` slices (plus tiny-weight admission block case), pallet unit tests for deferred-retry bound (`<= MaxDeferredRetriesPerBlock`) and queue bound preservation under sustained pressure, and a dedicated runtime-benchmark guard (`process_deferred_retry_max_retries`) for worst-case deferred cursor retry path. Phase 10.1/10.2 are now marked complete in implementation plan and matrix.
- `AAA Phase 9 Sweep Lifecycle Completeness`: Zombie sweep now applies the same terminal lifecycle checks for active User actors as permissionless liveness path (`WindowExpired`, rent insolvency, `MinUserBalance` exhaustion). Added dedicated pallet coverage for window-expired and balance-exhausted destruction via background sweep while preserving bounded `MaxSweepPerBlock` cursor semantics. Added rent-ceiling unit coverage (`MaxRentAccrual`) to confirm capped lazy rent accrual under large block deltas.
- `AAA Phase 8 OnAddressEvent Drain Modes`: Implemented `InboxDrainMode::{Single,Batch,Drain}` in `Trigger::OnAddressEvent`, added schedule validation (`Batch(max)` requires `0 < max <= MaxAddressEventInboxCount`), and integrated drain semantics into cycle execution. Implemented saturation behavior per spec: `Batch/Drain` clear saturated entries as "consume all", `Single` consumes one and keeps saturation flag. Added pallet and runtime integration coverage for batch draining and saturation behavior.
- `AAA Phase 7 SplitTransfer Share Model`: Migrated `SplitTransfer` task payload from permill legs to explicit rational shares (`SplitLeg { to, share }`, `total_shares`, `remainder_to`). Added strict `sum(share_i) == total_shares` validation and deterministic remainder routing (`remainder_to` or leg[0] fallback). Updated pallet+runtime tests to cover mismatch rejection and remainder-target execution. Runtime migration/backward-compat path intentionally skipped because pallet/framework remains pre-release.
- `AAA Phase 5.2 Adapter Benchmark Guard`: Added runtime-visible adapter bound plumbing (`MaxAdapterScan`) and wired task upper-bound pricing to adapter `MaxK` for `RemoveLiquidity` in `TaskWeightInfo`. Added runtime-benchmark benchmark `process_remove_liquidity_max_k` that executes `on_idle` with a seeded worst-case LP resolution path (`K = MaxAdapterScan`) via a new benchmark helper contract (`BenchmarkHelper::setup_remove_liquidity_max_k`) implemented in both mock and runtime config.
- `AAA Phase 4 Weighted Fairness Scheduler`: Added deterministic weighted RR arbitration with runtime-configurable class weights/caps (`FairnessWeightSystem/User`, `MaxSystemExecutionsPerBlock`, `MaxUserExecutionsPerBlock`), bounded deferred queue/cursor retry (`DeferredRing`, `DeferredCursor`, `MaxDeferredRingLength`, `MaxDeferredRetriesPerBlock`), and class-aware pop semantics over bounded ready ring. Added pallet+runtime tests for deterministic class ordering, cap enforcement, and deferred retry progress once capacity is freed.
- `AAA Phase 3 Breaker Semantics`: Finished breaker admission/control semantics. `create_user_aaa` and `create_system_aaa` now fail with `GlobalCircuitBreakerActive` while breaker is on. Added pallet+runtime coverage that execution remains halted under breaker while cleanup/control extrinsics (`fund_aaa`, `permissionless_sweep`, `refund_and_close`, governance breaker toggle) remain active.
- `AAA Phase 2 Fee Model`: Completed WeightToFee-based execution pricing with runtime-bound `task -> weight_upper_bound` contract (`TaskWeightInfo`), pre-flight `cycle_fee_upper` admission, and in-cycle `reserved_fee_remaining` to protect fee budget from native spend starvation. Removed fixed `ExecutionFeePerStep` path from pallet/runtime config and aligned all fee calculations to `WeightToFee(weight_upper_bound(task))`.
- `AAA Owner-Slot Sovereign Rollout`: Implemented owner-scoped sovereign identity and slot lifecycle in pallet/runtime. Added `owner_slot` to `AaaInstance`, `OwnerSlots(owner, slot) -> aaa_id`, `SovereignIndex(sovereign_account) -> aaa_id`, `MaxOwnerSlots`, and errors `OwnerSlotCapacityExceeded`/`SovereignAccountCollision`. Creation now uses first-free slot allocation from `0`; destruction frees slot/index bindings. `AAACreated` now includes `owner_slot`. Added pallet+runtime tests for deterministic slot allocation and slot reuse after destroy/recreate. Updated AAA benchmarks to force worst-case slot scan (`slot = MaxOwnerSlots - 1`) and refreshed pallet/runtime create weights to include slot-scan DB cost. Runtime `pallet_aaa` weights were regenerated with `frame-omni-bencher`; benchmark setup now pre-seeds fee sink in `permissionless_sweep` to avoid sub-ED rent-transfer false negatives. Also hardened runtime test/genesis setup by endowing AAA fee sink account so sub-ED execution fees do not fail on first transfer.
- `AAA Spec/Impl sync (post-v0.41.1 policy)`: Updated orphan-handling policy — full rejection of `TTL + DeadAaaOwners` post-destruction ownership registry. After terminal refund, non-covered orphan balances remain on the former sovereign address and are outside protocol control; refund flow is explicitly treated as a happy-path discipline via pre-terminal `refund_assets` maintenance. Removed owner-claim recovery surface from pallet/runtime (`recover_dead_aaa_funds`, `DeadAaaOwners`, `OrphanClaimTTL`, related errors/events, weights, benchmarks, and tests), and updated runtime integration tests to assert orphan balances remain on former sovereign accounts.
- `AAA Spec v0.41.0`: Consolidated spec/code evolution plans and released new AAA specification version with clarified normative contract: strict `WeightToFee(weight_upper_bound)` execution fee model, mandatory `ScheduleWindow` support, pre-flight cycle fee admission + reserved fee budget, breaker semantics (`enqueue+execution` halted, cleanup/recovery alive), weighted fair scheduling, bounded O(K) adapter policy with `MaxK`, `DeadAaaOwners` TTL-only pruning semantics, and normalized `SplitTransfer` share model (`share_i + total_shares` with explicit remainder policy). Added companion docs: implementation plan (`docs/aaa-plan.md`), Spec↔Test matrix appendix, and incremental upgrade playbook.
- `AAA Hardening & Runtime Adapter Safety`: Added strict SplitTransfer validation (`InsufficientSplitLegs`, `ZeroShareLeg`, `DuplicateRecipient`) at creation and execution time, added runtime mint guard in `execute_task` (System-only), made refund asset derivation fail-fast on overflow, and made `update_refund_assets` fail instead of silently truncating. Added dedicated weights/benchmarks for `permissionless_sweep` and `recover_dead_aaa_funds`, and fixed extrinsic annotations to use those weights. Fixed orphan-recovery lockout bug by keeping `DeadAaaOwners` entry after partial recovery. In runtime `TmctolDexOps`, replaced unbounded LP balance scan with O(1) pool lookup and bounded LP reverse lookup (`AaaMaxPoolScan`) to keep adapter work deterministic. Trimmed runtime AAA integration suite from 31 to 26 tests by removing unit-level lifecycle/access-control duplicates already covered in pallet tests.
- `AAA v0.40.0 Full Spec Alignment`: Complete pallet-aaa overhaul to match spec v0.40.0. **Removed**: `LifecycleState` enum → `is_paused: bool` + `pause_reason: Option<PauseReason>`. `AaaMode` enum. `PipelineErrorPolicy::PauseActor`. `DormantQueue`, `SystemExecutionPaused`, `DeferredCursor` storage. `AdmissionFee` → replaced by `StepBaseFee` + `ConditionReadFee` + `ExecutionFeePerStep`. `actor_account_id` → `sovereign_account_id`. **Added**: `DeadAaaOwners` storage (orphan recovery §4.4). `Noop` task. `permissionless_sweep` extrinsic (§8.7). `recover_dead_aaa_funds` extrinsic (§4.4). `AAADestroyed` event. `MaxRentAccrual` cap. `OrphanClaimTTL`. `CycleNonceExhausted` terminal handling (pause System, destroy User). **Fixed**: insolvent refund path (native burned, non-native → FeeSink §4.2). Double-firing prevention (re-enqueue deferred to post-loop). Sovereign account XOR-derivation for small AccountId types. **Result**: 79 pallet tests, 160 integration tests, 382 workspace tests, 0 warnings, benchmarks compile.

- `AAA Spec v0.34.0`: Major semantic changes — (1) Rent debt → auto-refund (not pause), no zombie actors. (2) New mutability model: `Immutable` (pipeline frozen, no pause/resume/update) vs `Mutable` (owner-controlled). (3) `PauseReason::RentInsufficient` removed. (4) Lifecycle updated: balance exhaustion is terminal. (5) 6 new Design Decisions marked ⏳ Planned: auto-refund, multi-asset, mutability, zombie sweep, execution fees, failures guard.
- `AAA Release Plan`: Created `/docs/aaa-release-plan.md` with 12 work items across 3 tiers. Priority: mutability → auto-refund → multi-asset → sweep → fees → guards. Estimate: ~485 LOC code + ~365 LOC tests.
- `AAA Unit Tests`: Refactored mock to production-realistic fees (`AdmissionFee=10B`, `RentPerBlock=1M`). Fixed 29 tests for fee-awareness. Added 7 new tests: 3 admission fee, 4 rent model. Total: 47 unit tests (was 38), 353 overall, 0 warnings.
- `AAA Integration Tests`: Removed misplaced rent tests (moved to unit tests). 163 integration tests remain.

- `AAA Spec v0.33.0`: Added Design Decisions Log table clarifying: (1) Admission fee only for User AAA (System exempt), (2) Condition limit fully runtime-configurable via `MaxConditionsPerStep` type parameter, (3) Rent model as MVP with admission fee as DDoS protection, (4) StepFailed event includes `DispatchError` for debugging.
- `AAA Step Generic Bound`: `Step` struct now parameterized by `MaxConditions: Get<u32>` using runtime's `Config::MaxConditionsPerStep`. Manual trait implementations (`Clone`, `Debug`, `PartialEq`, `Eq`) avoid over-constrained derive bounds. `#[scale_info(skip_type_params)]` used for TypeInfo.
- `AAA Admission Fee`: Implemented per §3.1 - User AAA charged `AdmissionFee` before cycle start, System AAA exempt. Insufficient balance → deferred with `AdmissionFeeInsufficient` reason. Events: `AdmissionFeeCharged`, `CycleDeferred`. Config: `AdmissionFee`, `FeeRecipient`.
- `AAA Rent Model (§3.2)`: Lazy charging on touch. Config: `RentPerBlock` (default 0.000001 native/block ≈ 0.144/day). Storage: `last_rent_block` in `AaaInstance`. Logic: `rent_due = blocks_elapsed * rent_per_block`. If balance < rent_due → debt, pause with `RentInsufficient`. System AAA exempt. Event: `RentCharged { aaa_id, blocks_elapsed, rent_due, rent_paid, rent_debt }`.
- `AAA Source Filtering (§5.1B)`: `SourceFilter` enum for OnAddressEvent trigger. Variants: `Any` (all sources), `OwnerOnly` (only owner), `RefundAddressOnly` (only refund_to), `Whitelist(BoundedVec<AccountId>)`. `notify_address_event` now takes `source: &AccountId` parameter. `Trigger::OnAddressEvent` updated to include both `asset_filter` and `source_filter`. `Schedule` type now generic over `AccountId`. 4 new unit tests covering all filter modes.
- `AAA DNF Conditions`: Replaced recursive `Condition` tree (Simple/And/Or/Not with `Box`) with flat DNF (Disjunctive Normal Form). `Step.conditions: BoundedVec<BoundedVec<Condition, MaxPerClause>, MaxClauses>`. Eliminates `Box`, manual `MaxEncodedLen`, `MAX_CONDITION_DEPTH`. `MaxEncodedLen` now derivable. Config: `MaxConditionsPerClause` + `MaxClausesPerStep` replace `MaxConditionsPerStep`. Renamed `ConditionKind` → `Condition` (only type remaining). Spec §6.1B updated.
- `AAA Production Benchmarks`: Generated real weights via `frame-omni-bencher` on AMD Ryzen 7 4800H. 9 extrinsics benchmarked (50 steps, 20 repeats). Registered `pallet-aaa` in `define_benchmarks!`.
- `AAA Proxy Section Removal`: `docs/aaa-specification.md` advanced to `v0.27.0` by removing non-semantic proxy sections `6A/6B/6C` entirely (no external anchors), keeping only canonical normative loci (`§7.2` instruction contract, `§4.4` admission, `§6.6` effects).
- `AAA Spec Noise Cleanup`: `docs/aaa-specification.md` advanced to `v0.26.0` via no-behavior-change deduplication pass: canonical versioning in §2.6 (with §3A intent-only), instruction-contract duplication collapsed (6A/6B/6C reference §7.2/§4.4/§6.6), determinism consolidation reduced to checklist semantics in §13A, and event-definition ambiguity clarified (`Cycle*` lifecycle vs `Pipeline*` terminal outcomes).
- `AAA Docs Consistency`: Added `docs/aaa-specification.md` to `docs/README.md` index and replaced stale DripVault roadmap link in `docs/treasury-owned-liquidity-architecture.md` with AAA roadmap reference.
- `LP Ingress Validation`: TOL now enforces `lp_asset.is_lp()` at LP entry points (`receive_lp_tokens`, `distribute_lp_tokens_to_buckets`) and rejects non-LP inputs.
- `TOL Singleton Model`: Replaced per-asset TOL map model with singleton state (`TolConfiguration`, singleton bucket allocations, singleton zap buffer). Bucket policy is now configured once per runtime instance.
- `Token-Domain LP Coupling (Phase 1)`: Introduced `TokenTolBindings (AssetId -> TolId)` and token-resolved TOL ingress routing. Zap Manager now resolves LP destination via runtime `TolAccountResolver` instead of static treasury account wiring.
- `Token-Domain LP Coupling (Phase 1.1)`: Extended resolver behavior for LP-asset sweeps by resolving destination via LP pair metadata + token binding fallback. This keeps `sweep_trigger` aligned with token-domain ingress policy.
- `TOL Domain-Scoped State (Phase 2A)`: Migrated TOL core state to `tol_id` domains (`TolConfigurations`, `ActiveTolDomains`, `Bucket*`, `PendingRequests`, `ZapBufferState`), added domain-aware queue draining and sweeps, and switched domain account custody to deterministic per-domain bucket sub-accounts for `tol_id > 0`.
- `TOL Extrinsic Surface (Phase 2A)`: `create_tol` now accepts `tol_id`; `update_bucket_allocation` and `withdraw_buffer` are domain-aware; `receive_mint_allocation` resolves domain from `token_asset`; LP intake/unwind paths resolve domain by LP pair binding.
- `Phase 2B Runtime Glue (Trait-Based)`: Added runtime adapter-driven lifecycle glue instead of a new orchestrator pallet. Asset registration and TMC curve activation now trigger idempotent token-domain bootstrap and deterministic routing setup.
- `Token-Domain ID Alignment`: `TolId` widened to `u32` and aligned with token-keyed defaults (`tol_id = token_asset_id` for non-LP assets), preserving governance override via explicit bind calls.
- `Deterministic Ensure Semantics`: `ensure_domain_for_token` now respects existing governance overrides (`bind_token_to_tol`) instead of silently overwriting them, while still hardening the bound domain idempotently.
- `Lifecycle Planning and Risk Envelope`: Added `docs/plan.md` with checkpoint statuses, phased delivery plan (P0-P3), antifragile risk register, and strict anti-overengineering guardrails.
- `SDK 2512 Positioning Clarified`: Expanded `docs/polkadot-sdk-2512-insights.md` with a focused comparison baseline vs 2503/2506-era runtime assumptions.
- `Paseo Guide Runtime Sync`: Updated `docs/paseo-testnet-guide.ru.md` to current runtime surface (`tokenCurves`, domain-keyed bucket queries, no `RouteSelected`, BM `on_idle` flow, `bindTokenToTol` token-domain routing checks).
- `TOL Call Surface Simplified`: `update_bucket_allocation`, `receive_mint_allocation`, and `unwind_bucket_liquidity` now use singleton semantics and no longer require a per-token storage key path.
- `Bucket Purity Automation`: TOL now runs periodic `on_idle` sweeps that move non-LP assets from bucket accounts to `BurningManager`, keeping buckets LP-only by design.
- `Architecture Docs Sync`: Updated TOL/Core/roadmap architecture docs to reflect singleton TOL state, sweep automation, LP ingress validation, and clean-slate LP namespace initialization.
- `LP Detection by Namespace`: TOL LP intake paths and bucket sweeps classify LP assets via `AssetKind::is_lp()` bitmask semantics, consistent with LP namespace initialization.
- `Terminology Consistency Cleanup`: Unified wording across code/docs to `TOL pallet account` and singleton TOL semantics, replaced stale `ZapBuffers` references with `ZapBufferState`, and documented BM as the bounded sink for TOL non-LP bucket sweeps.
- `TOL Bucket Account Liveness`: `genesis_build` now increments providers for Bucket A/B/C/D accounts so LP account creation in buckets works deterministically, and bucket distribution transfers use `Preservation::Expendable` to allow full-balance distribution from the TOL account.
- `SDK 2512 Alignment`: Updated `substrate-wasm-builder` 25.0.0→31.1.0, added `unexpected_cfgs` lint config for Rust 2024 compatibility, expanded workspace clippy lints to 25 rules (matching upstream template), bumped `system_version` 0→1.
- `Bucket Semantics Documentation`: Clarified that B/C automatic unwind is roadmap, not current runtime behavior. Updated architecture/spec docs to separate implemented governance-manual unwind from planned DripVault automation.
- `TOL Architecture Doc Sync`: `docs/treasury-owned-liquidity-architecture.md` aligned with runtime reality: `MaxTolRequestsPerBlock=10`, LP flow semantics, current TMC→ZM wiring, non-wired `receive_lp_tokens` integration path, exact TOL event fields, and current status of ratio/oracle config usage.
- `Abstract vs Runtime Wording Sync`: `docs/l2-tol.en.md` and `docs/l2-tol.ru.md` removed runtime-looking pseudocode from LP management section and now describe layer-agnostic invariants with hook-driven runtime note.
- `TMC Architecture Wording Sync`: `docs/token-minting-curve-architecture.md` now reflects current fee flow ownership (AxialRouter + BurningManager), active ZapManager recipient semantics, and current event/call surface wording.
- `Account Terminology Cleanup`: Replaced ambiguous `TOL Treasury` wording with `TOL pallet account` / `treasury destination account` terminology across core architecture docs.
- `L2 TOL Conceptual Doc Cleanup`: Removed DripVault pseudocode from `docs/l2-tol.en.md` and `docs/l2-tol.ru.md`, replaced with concept-level descriptions, normalized non-code fenced blocks to prose/bullets, and aligned EN/RU terminology (including GovXP 1x→5x range).
- `TMCTOL Intro Matrix Refactor`: Updated `docs/tmctol.en.md` and `docs/tmctol.ru.md` with an abbreviation-only 2×2 matrix (`TBC/TMC × POL/TOL`), added base-term glossary, and added explicit feasibility/property profiles for all four combinations.
- `TOL Roadmap Clarification`: `docs/treasury-owned-liquidity-architecture.md` now explicitly separates current singleton behavior from post-release token-scoped (`tol_id`) plans, including canonical `$BLDR` profile goals and cross-pallet TMC/Zap/TOL domain coupling intent.
- `Cross-Doc Evolution Sync`: `token-minting-curve-architecture.md` and `zap-manager-architecture.md` now include post-release token-scoped domain coupling notes (`AssetId -> TolId`) and `$BLDR` profile intent, clearly marked as planned behavior.
- `Lifecycle Placement Decision`: `core-architecture.md` now hosts token lifecycle orchestration (registration vs economic activation, instance coupling intent, canonical `$BLDR` profile), while `asset-registry-architecture.md` explicitly scopes registry to identity/namespace concerns only.
- `Zap Pending Fair Scheduling`: `on_idle` now reinserts all deferred `PendingZaps` entries when weight is exhausted and rotates execution by `ZapExecutionCursor` for deterministic round-robin fairness across pending assets.
- `Burning Manager Scope Clarified`: Architecture docs now explicitly fix Burning Manager as a singleton global burn domain (no BM per-token domain instancing).
- `DripVault Instance Direction Clarified`: Roadmap docs now explicitly describe DripVault as future multi-instance account abstraction.

### Added

- `AAA Scenarios Catalog (RU)`: Added `docs/aaa-scenarios.ru.md` as the canonical Source-of-Truth catalog for AAA usage scenarios required for implementation and testing. Includes config examples for all task/trigger/policy primitives and high-level multi-AAA mesh compositions.
- `LP Ingress Guard`: TOL validates `lp_asset.is_lp()` on LP intake paths and rejects non-LP inputs before bucket distribution.
- `Zap Fairness Unit Coverage`: Added pallet tests for pending preservation under zero weight and round-robin cursor progression under single-cycle budget.
- `Zap Fairness Stress Coverage`: Added runtime load test `test_skewed_pending_load_does_not_starve_secondary_asset` to validate non-starvation under skewed pending load with one-cycle `on_idle` budget.
- `Asset Registry TokenDomainHook`: Registration/link flows can notify runtime glue to auto-bootstrap token-domain state.
- `TMC DomainGlueHook`: `create_curve` now supports runtime glue hook for deterministic token onboarding side effects (domain ensure + Zap enable).
- `TokenDomainEnsured Event`: Added compact lifecycle observability event in TOL with action states (`Created/Rebound/Noop`) and foreign-asset mutation visibility (`previous_foreign_asset` → `foreign_asset`).
- `Token Lifecycle Runbook`: Integrated checkpoint-by-checkpoint onboarding runbook into `docs/core-architecture.md` (§3.6) to keep lifecycle operations in the core canonical document.
- `Phase 2B Integration Coverage`: Added runtime tests for registry-driven TOL bootstrap and curve-activation auto glue behavior.
- `Idempotency Coverage`: Added unit/runtime tests proving repeated lifecycle checkpoints do not duplicate active domains and keep deterministic bindings stable.
- `Capacity Boundary Coverage`: Added `MaxTolDomains` saturation test proving fail-fast on new domain creation at cap while allowing hardening of already-bound existing domains.
- `Negative-Path Glue Coverage`: Added runtime tests for hook fail-fast behavior on domain-cap saturation and missing-domain binding failure during curve activation, plus LP/invalid-asset rejection coverage for domain ensure path.
- `Onboarding Contract Ops Note`: Added canonical token onboarding contract rules to `docs/governance-operations.md`.
- `LP Namespace Initialization`: Clean-slate genesis now initializes `pallet-asset-conversion::NextPoolAssetId` into `TYPE_LP` space so newly created LP tokens start in the LP bitmask namespace.
- `Bucket LP Integration Tests`: Runtime integration tests verify (1) policy-driven LP handling in bucket accounts, (2) governance unwind remains functional for allowed buckets, (3) non-LP bucket assets are swept to BurningManager, and (4) LP IDs are minted in `TYPE_LP` namespace.
- `Token-Domain Routing Integration Tests`: Added runtime tests for (1) bound-token LP distribution from non-default ingress accounts, (2) Zap LP transfer routing under token binding, and (3) LP `sweep_trigger` routing by LP pair domain.
- `TOL Domain Unit Coverage`: Added/updated TOL unit tests for domain binding and LP ingress resolution through domain-aware helpers.
- `TOL Manual Bucket Unwind`: Added governance-only `unwind_bucket_liquidity` for buckets B/C/D. Bucket A unwind is now explicitly blocked at pallet level.
- `DripVault Roadmap`: Added `docs/drip-vault-roadmap.md` with staged delivery plan for B/C gradual unwind, BLDR policy fallback, and user/system DripVault account abstraction.
- `Documentation Index Sync`: `/docs/README.md` now indexes all 22 docs — zero orphans, zero phantoms, zero duplicate references. Previously 8 docs were not indexed.
- `Router-Through Architecture`: BM and ZM now swap through the axial router via public `execute_swap_for()` instead of bypassing to `pallet-asset-conversion` directly. Oracle updates, optimal routing, and price protection apply to all system swaps.
- `Fee Exemption for System Accounts`: New `is_fee_exempt()` checks router, BM, and ZM accounts. System pallet-to-pallet swaps pay zero router fees.
- `ZapManagerAccount Config`: Axial router `Config` trait now includes `ZapManagerAccount: Get<Self::AccountId>` for fee exemption.
- `ED-Free Pallet Accounts`: BM, ZM, Router, TOL, and TMC accounts get `inc_providers` in `genesis_build`, surviving zero native balance without account reaping. 5/5 pallets with `PalletId` now ED-free.
- `keep_alive Awareness`: `execute_direct_swap` and `execute_optimal_route` accept `keep_alive` parameter. System accounts can drain balances; user accounts remain keep-alive.
- `Benchmark Runner Script`: `scripts/06-run-benchmarks.sh` — runs all 7 pallet benchmarks via `frame-omni-bencher`, auto-normalizes generated files (`SubstrateWeight<T>`, `polkadot_sdk::*` imports).
- `Asset Registry Runtime Weights`: Created `runtime/src/weights/pallet_asset_registry.rs` bridge, registered `pallet_asset_registry` in `define_benchmarks!`. All 7 custom pallets now have runtime weight bridges.
- `Architecture Documentation`: Created digital twin docs for TMC (`token-minting-curve-architecture.md`), TOL (`treasury-owned-liquidity-architecture.md`), and Asset Registry (`asset-registry-architecture.md`). All 6 custom pallets now documented.
- `Multi-Hop Mock`: `MockAssetConversionAdapter::swap_exact_tokens_for_tokens` now supports multi-hop paths (iterates `path.windows(2)` processing each hop's XYK pool sequentially). Previously hard-failed for `path.len() > 2`.
- `Multi-Hop Unit Tests`: 8 new tests covering end-to-end multi-hop swap, output math verification, slippage protection, route preference, missing intermediate pool, fee collection, and pool reserve updates.
- `Multi-Hop Integration Tests`: 3 new runtime integration tests — real ASSET_A → Native → ASSET_B swap via `pallet_asset_conversion`, fee-collected-once verification, and missing second pool error.
- `Asset Registry Benchmarks`: V2 benchmarks for all 4 extrinsics (`register_foreign_asset`, `register_foreign_asset_with_id`, `link_existing_asset`, `migrate_location_key`) with `WeightInfo` trait. 6/6 pallets now benchmarked.

### Changed

- `EMA Oracle: Time-Weighted Alpha`: `PriceOracleImpl::update_ema_price` now uses `EmaLastUpdate` storage to compute elapsed blocks, producing a time-weighted `alpha = elapsed / (half_life + elapsed)` instead of the previous fixed `1 / (half_life + 1)`. EMA now converges faster when swaps are infrequent.
- `PairTvl Storage Removed`: TVL is read directly from pool reserves via `get_pool_reserves()` during routing — always fresh, no EMA smoothing needed for TVL.
- `BM/ZM/TOL dev_mode Removed`: All three pallets no longer use `#[frame::pallet(dev_mode)]`. All extrinsics have proper `T::WeightInfo::*` weight annotations. BM: 4 hardcoded `#[pallet::weight(10_000)]` replaced. ZM: `on_idle` weight proxy corrected to `process_zap_cycle()`. TOL: already had WeightInfo, just needed dev_mode removal.
- `Router Defensive Unwraps`: Replaced `path.first().unwrap()` / `path.last().unwrap()` with `.ok_or(Error::NoRouteFound)?` in `validate_price_protection`. Zero `.unwrap()` in all pallet production code.
- `BM Benchmark Fixed`: `process_foreign_fees` benchmark now exercises `on_idle` directly (the actual production code path) instead of the removed `process_pending_fees()`. Fixed `NotExpendable` error by funding benchmark accounts with 2x liquidity amount.
- `ZM Benchmark Fixed`: `process_zap_cycle` benchmark now exercises `on_idle` (the heavy zap processing) instead of only `on_initialize` (the lightweight scan phase).
- `TOL Benchmark Fixed`: Added `BenchmarkHelper` trait for asset creation in runtime benchmark context. `receive_lp_tokens` and `withdraw_buffer` now properly create assets via `force_create` before minting.
- `Router Swap Benchmark Fixed`: Fixed `ZeroAmount` error — `pallet_asset_conversion` rejects `min_amount_out=0`. Router now passes `min_amount_out.max(1)` to underlying swap. Benchmark funds BM account for fee routing.
- `Real Weight Generation`: All 7 custom pallets now have real hardware-benchmarked weights (AMD Ryzen 7 4800H) via `frame-omni-bencher v0.17.0`. Replaced all placeholder weight files.
- `Generated Weight File Normalization`: Auto-generated weight files use `SubstrateWeight<T>` struct name and `polkadot_sdk::*` imports for consistency with the project's unified import standard.
- `Asset Registry WeightInfo`: Runtime Config now uses `SubstrateWeight<Runtime>` instead of `()` — includes proper DB cost adjustments.
- `PriceOracle Trait Simplified`: Removed `tvl` parameter from `update_ema_price` — TVL is not oracle-smoothed.
- `Mock Router Fee → 0.5%`: Aligned mock `DefaultRouterFee` from 0.2% to production's 0.5% (`Permill::from_parts(5_000)` sourced from `ecosystem::params::AXIAL_ROUTER_FEE`). All unit test assertions updated.
- `Mock MaxPriceDeviation → 20%`: Aligned from 1% to production's 20% (`ecosystem::params::MAX_PRICE_DEVIATION`).

### Removed

- `Execution Plan Surface`: Removed `docs/plan.md` and dropped active roadmap/TODO surface from docs index in favor of architecture-first canonical docs.
- `Per-Asset TOL Maps`: Removed `TolConfigurations`, per-token bucket storage maps, and per-token zap buffer maps in favor of singleton TOL state.
- `Bucket LP Debit Guard Storage`: Removed `AuthorizedBucketLpDebits` and runtime `BucketLpFreezer` coupling in favor of direct LP ingress validation and policy-driven unwind behavior.
- `TOL Implementation Guide Consolidated`: Merged unique math content (floor price formula, spot price, swap calculations, Gravity Well convergence, security model) from `tol-implementation-guide.md` into `treasury-owned-liquidity-architecture.md`. Deleted the redundant impl guide.
- `Dead Code Cleanup`: Removed `RouterConfiguration` storage + `RouterConfig` struct + `Route` struct + `BestPriceQuote` struct (all unused). Removed `EmaPriceUpdated` and `PriceDeviationDetected` events (declared but never emitted). Removed stale `use core::convert::TryInto` import (Rust 2021 prelude). Simplified `GenesisConfig` to `tracked_assets` only.
- `Cross-Pallet Dead Events`: Removed 3 dead events from `token-minting-curve` (`TokensMinted`, `TolDistributed`, `ZapError`) and 4 from `treasury-owned-liquidity` (`LiquidityAdded`, `LiquidityZapped`, `EconomicMetricsRecorded`, `BufferAnalyticsRecorded`) — all declared but never emitted.
- `Dead Functions`: Removed `get_tracked_assets()` from axial-router (never called). Removed `should_trigger_zap()` from treasury-owned-liquidity (never called).
- `Dead Test Constants`: Removed 5 unused `LP_ASSET_*` constants and `TYPE_LP` import from runtime test common module.
- `Stale Test Comments`: Replaced hardcoded `0.2%` formulas with `calculate_router_fee()` calls in fee and sandwich tests.
- `Asset Registry: WeightInfo Trait`: Replaced inline `T::DbWeight::get().reads_writes()` with proper `WeightInfo` pattern. Placeholder weights pending production benchmarks.
- `LP Token Unwinding`: Burning Manager detects LP tokens via bitmask, unwraps into constituent tokens, burns native and swaps+burns foreign.
- `PendingZaps Storage`: Zap Manager uses two-phase architecture — `on_initialize` scans, `on_idle` executes with weight budget.
- `PendingNativeBurn Storage`: Burning Manager flags pending burns in `on_initialize`, executes in `on_idle`.
- `Pool Auto-Creation Event`: Zap Manager emits `PoolCreated` event for observability when pools are auto-created.
- `Bounded TrackedAssets`: Router's `TrackedAssets` now uses `BoundedVec` with configurable `MaxTrackedAssets` limit.
- `Conservation Property Test`: TMC validates `user + zap == total` for all minted amounts.

### Changed

- `TMC: TotalIssuance-Based Supply`: Removed internal `current_supply`/`total_minted` counters. Price formula now reads live `TotalIssuance` — any standard burn auto-compresses the ceiling.
- `TMC: Single Mint Path`: Removed `mint_tokens` and `burn_tokens` extrinsics. `mint_with_distribution` is the only mint path (called by Router via `TmcInterface`).
- `TMC: Governance-Only Curve Creation`: `create_curve` now requires `AdminOrigin` instead of any signed user.
- `TMC: dev_mode Removed`: All extrinsics have proper benchmarked weights and call indices.
- `Router: Fee Deducted Before Swap`: Fee calculated once in `swap()`, deducted before execution. Eliminated double-dipping where users paid `amount_in × 1.005`.
- `Router: PalletId from Primitives`: Replaced hardcoded `*b"axialrt0"` with configurable `T::PalletId` sourced from `ecosystem::pallet_ids`.
- `Router: Benchmarked Weights`: All extrinsics use `T::WeightInfo` instead of hardcoded constants.
- `Burning Manager → on_idle`: Native burns and foreign swaps moved from `on_initialize` to `on_idle` with proper `remaining_weight` tracking.
- `Burning Manager: process_fees Removed`: Autonomous-only operation via hooks. No public extrinsic needed.
- `TOL: Global Bucket Allocation`: `distribute_lp_tokens_to_buckets` uses Config constants instead of per-asset storage lookup (fixes key mismatch bug).
- `XYK LP Fee: 0.3% → 0.0%`: All fee revenue flows through Router to Burning Manager.
- `TolZapAdapter: Error Propagation`: Transfer failures now revert the mint instead of silently losing funds.
- `Zap Manager → on_idle`: Heavy zap execution moved to `on_idle`; `on_initialize` is lightweight scan only.

### Fixed

- `TMC Conservation Invariant`: Canonical remainder pattern (`user = ratio.mul_floor(total); zap = total - user`) eliminates rounding loss.
- `TMC Storage Key`: `mint_with_distribution` now uses `token_asset` (not `foreign_asset`) for `TokenCurves` lookup.
- `Burning Manager Mock`: Replaced hardcoded pool reserves with deterministic Blake2-based pool ID mapping using actual POOLS state.
- `Core Architecture Doc`: Updated §5.3 to reflect actual two-phase hook architecture (no per-transfer callback in pallet-assets).
- `Ecosystem Constants`: Updated AXIAL_ROUTER_FEE comment to reflect 0% XYK fee.

### Removed

- `docs/pallet-improvement-plan.md`: All 20 items + 4 spec corrections completed.
- `runtime/src/tests/xcm_e2e_tests.rs`: Deleted empty placeholder tests (`assert!(true)`).
- `test_ceiling_arbitrage`: Deleted — tested TMC burn/redemption path removed in this release.

### Security

- `No Silent Fund Loss`: TolZapAdapter errors propagate to revert mints, preventing token evaporation.
- `Router Fee Integrity`: Single fee deduction eliminates economic invariant violation.
- `Bounded Storage`: TrackedAssets cannot grow unboundedly (DoS protection).
- `Weight-Bounded Hooks`: All `on_idle` implementations respect `remaining_weight` parameter.

---

- `Parachain Template Framework`: Introduced `/template`, a complete production-ready Polkadot SDK parachain implementation of the TMCTOL standard (Omni Node, Frame V2).
  - `Pallets`: `axial-router`, `burning-manager`, `token-minting-curve`, `treasury-owned-liquidity`, `zap-manager`, and `asset-registry`.
  - `Runtime`: Configured with "Runtime-as-Config" pattern, `LazyBlock` APIs, and `SingleBlockMigrations`.
  - `Infrastructure`: Docker-ready node configuration, Zombienet tests, and benchmarking tooling.

- `Polkadot SDK 2512 Integration`: Full synchronization with Polkadot SDK v1.21.0.
  - Migrated workspace dependencies and `rust-toolchain.toml` (Rust 1.88).
  - Implemented `XCM v5` configuration and `cumulus-pallet-xcmp-queue` migration.
  - Updated `pallet-assets` with `ReserveData` support.

- `Foreign Asset Reception`: Complete XCM inbound infrastructure.
  - `ForeignAssetsTransactor`: Handles incoming assets via `pallet-assets`.
  - `Hybrid Registry`: Deterministic `LocationToAssetId` mapping with persistent storage for Foreign (0xF...) assets.
  - `Trust Filters`: `ReserveAssetsFrom` enabled for Relay Chain and Sibling Parachains.

- `Comprehensive Governance`: Implemented `AdminOrigin` controls across all economic pallets for parameter management (fees, thresholds, pausing).

- `Stateful Benchmarking`: Added `BenchmarkHelper` implementations for realistic weight generation in custom pallets.

### Changed

- `Economic Precision`: Standardized all pallets to use `PRECISION` (10^12) and ecosystem constants from `primitives::ecosystem::params`.
- `Testing Architecture`: achieved 100% test coverage (113 runtime tests) including economic invariant checks and load testing.
- `Governance Logic`: Moved from hardcoded constants to on-chain storage parameters updateable via governance.

### Fixed

- `Consensus Configuration`: Resolved block production issues by adopting `FixedVelocityConsensusHook` and correct Async Backing parameters.
- `Runtime API Completeness`: Fixed silent failures by implementing missing APIs like `RelayParentOffsetApi` and `AuraUnincludedSegmentApi`.

---

## [1.0.1] - 2025-12-12

### Fixed

#### Documentation Errata (v1)

- `Floor Formula Correction`: Updated floor price formula to correctly reflect constant product invariant ($P_{floor} = k / (R_{native} + S_{sold})^2$) in EN/RU docs
- `Approximation Logic`: Replaced misleading $(1+1.5s)^2$ approximation with formally derived $P_{floor} / P_{ceiling} \approx 1 / (1 + s/a)^2$ where $a$ is floor support fraction
- `Protection Semantics`: Clarified min/max protection scenarios (25% = all buckets supporting, 11% = only Bucket_A supporting) correcting inverted definitions
- `Artifact Removal`: Cleaned up stray text in L2 TOL documentation and duplicate entries in Test Mirror

#### Semantic Clarification

- `Scenario Definitions`: Renamed scenarios to "User Exit" and "System Exit" to reflect actor behavior rather than emotional states
- `Supply Logic`: Explicitly defined "33% Sold" as total user exit and "66% Sold" as catastrophic failure requiring treasury leakage
- `Dependency Clarity`: Added explicit $a=33\%$ assumption to scenario tables

#### Mechanism Formalization

- `Ratchet Effect`: Replaced vague proportional claims with rigorous derivation showing how supply contraction reduces max potential pool balance, forcing floor elevation ($P'_{floor} > P_{floor}$)
- `Bidirectional Compression`: Explicitly defined the simultaneous lowering of Ceiling (via curve) and raising of Floor (via burn)

#### Equilibrium Logic

- `Dimensional Correction`: Fixed $P_{eq}$ formula to $P \approx \sqrt{R_{foreign} \cdot m}$, resolving unit mismatch ($[Price] = \sqrt{[Foreign] \cdot [Foreign/Native^2]}$)
- `Backing Definition`: Defined Equilibrium as the point of 100% Foreign Reserve backing for the curve-implied capitalization

#### Verification & Testing

- `Floor Logic Validation`: Added Test 41 (`Floor Formula & Scenario Verification`) confirming simulator behavior matches theoretical floor derivation ($P_{floor} = k/(R+S)^2$)
- `Scenario Ratio Check`: Verified that the floor/ceiling ratio scales correctly with support fraction ($1/(1+s/a)^2$ approximation verified within 0.5% tolerance)
- `Test Synchronization`: Updated `simulator/tests.md` mirror to include new validation coverage

---

## [1.0.0] - 2025-11-01

### Added

#### Core Framework

- `Consolidated Specification`: Unified `tmctol.en.md` integrating Foundation → Architecture → Mathematics → Economics → Dynamics → Implementation → Trade-offs
- `L2 TOL Governance`: Constant protection model with declining voting power (10x → 1x linear decay for direct holders)
- `Invoice Voting Mechanics`: DOUBLE/APPROVE/REDUCE/VETO voting system with binary VETO threshold (>50% blocks)
- `Axial Router Specification`: Comprehensive routing gateway with fee burning and price discovery
- `Emergent Properties Documentation`: Bootstrap gravity well (~15% TOL/market-cap stability threshold), supply elasticity inversion, price ratchet effect
- `Dimensional Type System`: Physical types encoding units (`Price [Foreign/Native]`, `Slope [Foreign/Native²]`) preventing categorical errors

#### Mathematical Guarantees

- `Fee Consistency Principle`: XYK quote and execution apply fees identically using "fee on input" model (amount_in × (1 - fee)); prevents routing logic breakage when fees activated from default zero
- `Fair Rounding Strategy`: Largest remainder method eliminates systematic distribution bias; remainder allocated to party with maximum fractional part
- `Internal Slippage Protection`: Fee manager foreign→native conversion uses 10% slippage tolerance; prevents price manipulation attacks on burn mechanism with graceful degradation (buffering)
- `XYK Constant Product Necessity`: Validated that constant product (x·y=k) guarantees liquidity at all price levels; XYK "inefficiency" is precisely its strength for floor protection

#### Testing & Validation

- `Simulator as Ground Truth`: JavaScript/BigInt formal verification environment defining implementation truth (54 comprehensive tests)
- `Structural Symmetry Testing`: RADB (Recursive Abstraction Decomposition) test organization mirroring system architecture
- `Economic Attack Simulation`: Governance resilience, cross-chain independence, sandwich attack resistance scenarios
- `Legitimacy Phase Documentation`: Academic rigor with conditional guarantees throughout entire specification

#### Architectural Patterns

- `Architectural Refactoring Patterns`: New section 6.9 in AGENTS.md documenting symmetry violations, rounding bias, YAGNI in architecture, state ownership questions, distribute-collect anti-pattern, return value redundancy, and single source of truth
- `Buffer Ownership Clarity`: Documented principle that buffers live where concepts live—Tol buffer (awaiting zap) vs Bucket LP (owned liquidity)
- `Method Return Minimalism`: Principle that methods should return only what callers actually use; side-effect methods returning detailed decomposition create redundancy when state is directly queryable
- `Layered Abstraction Insights`: Cognitive scaffolding patterns for progressive comprehension

---

_Changelog maintained according to [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) standards._
