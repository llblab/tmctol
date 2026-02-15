# Project Context & Meta-Protocol

> Capturing layered abstractions, implementation insights, and evolutionary optimization

## 0. Meta-Protocol Principles

This document is a living protocol for continuous, intelligent self-improvement and optimal knowledge evolution. Its core principles govern how this context system manages itself and are universally applicable to any project:

- `Mandatory Self-Improvement`: Every task ends with context updates, creating self-reinforcing knowledge accumulation. Protocols enforce their own effectiveness through positive feedback loops.
- `Protocol Evolution`: Rules improve when better workflows emerge. Feedback loops create emergent intelligence; context embodies the design principles it mandates.
- `Test-Driven Evolution`: Comprehensive validation enables clean evolution through systematic patterns and self-correcting feedback loops.
- `Context Optimization`: Systematically prevent infinite growth through proactive cleanup. Transform tactical experiences into strategic wisdom. Evolution includes both addition AND consolidation.
- `Change History Rotation`: Fixed entry limits through intelligent consolidation. Newer entries provide tactical details, older entries preserve strategic patterns.
- `Non-Duplication Enforcement`: Information exists in only one authoritative section. Create hierarchical navigation instead of duplicating content.
- `Decreasing Abstraction Structure`: Organize from general to specific, mirroring optimal cognitive processing patterns.
- `Validation Infrastructure`: Automated validation of structure, cross-references, and information architecture. Pre-task preparation and completion protocols ensure quality gates are never skipped.
- `Living Hierarchical Documentation`: Documentation mirrors system architecture and evolves with implementation. Design intent stays synchronized with actual behavior.
- `Boundary Clarity`: Meta-principles govern context evolution; project conventions govern domain. Protocol ≠ project; distinct evolutionary pathways with cognitive firewalls preventing contamination.
- `Emergent Elegance`: Multiple iterations reveal constraints guiding toward patterns; complexity reduction emerges from understanding, not premature simplification.
- `Progressive Enhancement`: At 95%+ quality, targeted additions beat wholesale replacement; incremental improvement surpasses architectural revolution.
- `Emergent Property Validation`: Component interactions require explicit testing/documentation. Test organization mirrors architecture; structural symmetry creates multiplicative quality.
- `Morphological-First Decision Making`: Analyze solution space before implementing—map extremes, identify trade-offs. Dual-phase analysis reveals dimensional intersections invisible to single-phase.
- `Specification Maturity`: Documentation evolves exploratory → consolidated. Each abstraction layer enables deeper insight through progressive comprehension frameworks.
- `Constraint-Driven Evolution`: Evolution follows constraint discovery; constraints are catalysts not limitations. Evolved architectures solve discovered constraints; defend against simplification discarding hard-won insights.

---

## 1. Concept: The TMCTOL Standard

The project is a `Specification & Reference Framework`.

1.  `The Core (TMCTOL)`: A rigorous economic specification for a Token Minting Curve combined with Treasury-Owned Liquidity. It defines the mathematical laws of a self-sustaining economy.
2.  `The Framework (`/template`)`: A production-ready, forkable `Polkadot SDK Parachain Template`. It implements the TMCTOL standard using modern Polkadot SDK 2512 patterns (Omni Node, Frame V2).

`Goal`: Provide a "Foundation in a Box" for launching independent ecosystems with mathematically guaranteed liquidity infrastructure, achieving production-ready reliability through test-driven evolution with 100% validation.

---

## 2. Core Entities

### 2.1 Abstract Economic Actors (The Specification)

_Defined in `/simulator`, agnostic of blockchain framework._

- `TMC (Curve)`: The unidirectional emission engine ($P = P_0 + slope \cdot s$). Controls token emission through a deterministic economic machine.
- `TOL (Liquidity)`: The multi-bucket accumulator ensuring a rising price floor.
- `Gravity Well`: The emergent state where TOL accumulation stabilizes volatility (~15% MarketCap).
- `Elasticity Inversion`: The mathematical point where inflation strengthens the price floor.

### 2.2 Concrete Implementation Entities (The Framework)

_Implemented in `/template/pallets`, bound by Substrate logic._

- `Parachain Runtime`: The aggregator of pallets, utilizing `Runtime-as-Config` adapters and modern FRAME patterns.
- `Axial Router`: The execution gateway enforcing fee burning and optimal routing. It acts as an economic coordination actor determining "Efficiency Score" to arbitrate between Market Liquidity (XYK) and Protocol Liquidity (TMC).
- `Zap Manager`: Deterministic state machine for complex liquidity provisioning.
  - `"Omnivorous Intake"`: Scans balances (`on_initialize`) rather than relying on specific extrinsics, processing any asset that arrives at the system account.
  - `"Opportunistic Liquidity Provisioning"`: Maximizes liquidity addition in current pool ratios without pre-swap balancing.
  - `"Patriotic Accumulation"`: Explicitly prefers holding Native surplus over selling it for Foreign assets, treating Native token as sovereign collateral.
  - `Resilience`: Uses `RetryCooldown` to prevent resource exhaustion during oracle unavailability.
- `Burning Manager`: Consumes native tokens and swaps foreign tokens through token-driven fee architecture.
- `Adaptors`: The translation layer mapping Abstract Actors to Substrate types (e.g., mapping `PPM` math to `Permill` arithmetic).
- `Asset Conversion`: Uniswap V2-like DEX for automated market making, utilizing `AssetKind` with Bitmask-based classification.
- `Omni Node`: Primary deployment architecture eliminating node boilerplate.

---

## 3. Architectural Decisions

### 3.1 Meta-Architecture: Spec vs. Impl

- `Specification (JS)`: Uses infinite precision (BigInt) and `PPM` (Parts Per Million) for ratios. Focuses on _ideal_ behavior.
- `Implementation (Rust)`: Uses `u128`, `Permill`/`Perbill` (Substrate primitives), and `FixedU128`. Focuses on _determinism_ and _weight safety_.
- `Bridge`: We explicitly validate that Implementation drift (due to rounding/types) remains within safe bounds defined by the Specification.
- `Manifesto-Driven Architecture`: Explicit separation of L1 Strategy (Mathematical Sovereignty via TMC) from L2 Tactics (Democratic Allocation via Buckets). "Bad Politics cannot kill Good Physics."

### 3.2 Engineering Architecture (`/template`)

- `Polkadot SDK 2512 Standard`:
  - `Omni Node`: Deployment architecture eliminating node boilerplate. Node-level features (DHT bootnode discovery, `ParachainTracingExecuteBlock`, `collator_peer_id`) are handled by the Omni Node binary — no custom `node/` directory.
  - `Frame V2`: Strictly typed `#[frame::pallet]`, `frame_benchmarking::v2`.
  - `Token-Driven Coordination`: State transitions are triggered by asset movement (Substrate hooks), not Signed Extrinsics. This ensures origin-agnostic security.
  - `Runtime-as-Config`: Business logic (Pallets) is generic. Configuration (Runtime) injects specific behavior via Adapters following SDK patterns for clean separation.
  - `Unified Primitives`: `primitives/src/ecosystem.rs` is the single source of truth for constants, avoiding magic numbers. `AssetKind` uses bitmask classification for O(1) type inspection.
  - `Type System Discipline`: Enforced strict `sp_arithmetic::Permill` for ecosystem parameters. Adopted `sp_core::U256` for bonding curve calculations to prevent intermediate overflows.
  - `assets-common Rejected`: The `assets-common` crate (Location-as-AssetId, TrustBacked, ERC20/pallet-revive) is designed for Asset Hub pattern. Incompatible with TMCTOL's `u32` bitmask + `pallet-asset-registry` architecture. `pallet-asset-registry` already provides `MaybeEquivalence<Location, AssetId>`.
- `Stateful Testing`: Mocks use `RefCell<BTreeMap>` for realistic AMM simulation and TMC behavior, enabling "Mechanism Verification" over simple policy checks.

### 3.3 Economic Architecture

- `Unidirectional Minting`: A mathematical "Ratchet" preventing reserve extraction.
- `Bidirectional Compression`: Burning lowers the ceiling; TOL raises the floor. The price corridor compresses upwards.
- `Multi-Bucket Strategy`: 4-bucket system (50/16/16/16) ensures ~100% capital utilization while preserving governance segmentation.
- `Mechanism-Over-Policy`: The Router acts as a pure mechanism (XYK vs TMC) rather than a policy engine, reducing attack surface.

---

## 4. Project Structure

- `/docs/`: The Knowledge Base. Architecture guides, specs, mathematics.
- `/simulator/`: `The Source of Truth`. JavaScript/BigInt implementation of the Economic Standard.
- `/template/`: `The Reference Implementation`.
  - `/template/runtime/`: The Parachain assembly.
    - `/weights/`: The Bridge between generated benchmarks and runtime configuration.
  - `/template/pallets/`: Modular logic (`axial-router`, `tmctol`, `burning-manager`, `zap-manager`).
  - `/template/primitives/`: Unified types (`assets.rs`, `ecosystem.rs`).
  - `/template/node/`: (Minimal) Omni Node configuration.
- `/scripts/`: Automation (Simulation runners, Benchmark generators).
- `AGENTS.md`: `(You are here)`. The Cognitive Core.

---

## 5. Development & Evolution Conventions

### 5.1 The Three-Layer Validation

Truth is established in three stages. Skipping a stage is forbidden.

1.  `Simulation (Mathematical Truth)`:
    - _Location_: `/simulator`.
    - _Tooling_: JavaScript/BigInt, `PPM`.
    - _Purpose_: Verifies the formula is correct before any code is written.
2.  `Implementation (Behavioral Truth)`:
    - _Location_: `/template/pallets`.
    - _Tooling_: Rust, `Permill`, Unit Tests, Benchmarks.
    - _Purpose_: Verifies the Rust code matches the Math and fits within Block Weight limits.
3.  `Integration (Systemic Truth)`:
    - _Location_: `/template/runtime`.
    - _Tooling_: Integration Tests, XCM.
    - _Purpose_: Verifies components coordinate correctly (e.g., "Does a swap trigger the burn hook?").

### 5.2 Benchmarking Standard

_Context: Applied strictly within `/template`._

- `Syntax`: `frame_benchmarking::v2::*`.
- `Metrics`: Mandatory measurement of `RefTime` (Computation) AND `ProofSize` (Storage Access).
- `Complexity`: Explicit `Linear<Min, Max>` components.
- `Hygiene`: No assumptions. Mock the worst-case state (full storage) in `SETUP` using `whitelisted_caller()`.
- `Stateful Benchmarking`: Use `BenchmarkHelper` traits to bridge mock runtimes with benchmarking requirements.

### 5.3 Coding Standards

- `Zero Warnings`: Maintain zero clippy warnings. Resolve redundant pattern matching, collapsible ifs, useless conversions.
- `Clean Imports`: Use unified `polkadot_sdk::*` imports over fragmented crate-specific imports.
- `No License Headers`: Do not include license headers or copyright notices at the beginning of source files.
- `Complexity Resolution`: When facing integration challenges, simplify abstractions progressively. Substrate compilation failures are architectural feedback.
- `Antifragile Simplicity`: Default to the simplest deterministic rule that preserves invariants. Add complexity only when a concrete failure mode or constraint proves it necessary.
- `Workspace Lint Hygiene`: `Cargo.toml` must declare `unexpected_cfgs` with Substrate-specific cfg values (`substrate_runtime`). Clippy lints must track upstream parachain template — currently 25 rules.
- `SDK Version Tracking`: `substrate-wasm-builder` version must match polkadot-sdk umbrella (2512.1.0 → 31.1.0). `system_version` in `RuntimeVersion` must be `1` for SDK 2512.

### 5.4 Evolution Protocol

- `Forkability`: Changes in `/template` must maintain generic utility. Do not hardcode ecosystem-specific logic into the generic framework components.
- `Emergent Complexity`: Features like "Gravity Well" are `evolved complexity`. They are protected. Complicated spaghetti code is `accidental complexity`. It is destroyed.

### 5.5 Runtime Integration Protocol

_Context: Bridging Pallet Logic to Runtime Reality._

- `The Weight Bridge`: Generated weights (`/pallets/*/src/weights.rs`) are templates. Real weights live in `/runtime/src/weights/`. We copy, adapt, and expose them.
  - _Why_: Generated code often misses context-specific imports (e.g., `polkadot_sdk::*`) or trait bounds.
  - _Action_: Create a bridge module in runtime that implements the pallet's `WeightInfo` trait using the generated numbers.
- `Configuration-as-Code`: Runtime `configs/*.rs` must point to `crate::weights::pallet_name::WeightInfo`. Never leave `()` or placeholder implementations in production.
- `On_Idle Safety`: Verify `BlockWeights` configuration leaves sufficient margin (e.g., 75% Dispatch Ratio) for `on_idle` tasks (automatic cleanup/swapping).
- `Upstream Sync Protocol`: When syncing with `paritytech/polkadot-sdk-parachain-template`, classify each change: (a) `SDK-standard` — adopt (lint rules, build tooling, RuntimeVersion); (b) `Ecosystem-pattern` — evaluate against project architecture before adopting (`assets-common`, `ForeignCreators`); (c) `Business-logic` — skip (XCM routing, pallet composition).

### 5.6 Network Integration Protocol

_Context: Handling Foreign Assets and XCM._

- `Hybrid Registration Protocol`: Foreign assets use a "Hybrid Registry" pattern combining deterministic hashing with storage persistence.
  - _Mechanism_: Asset IDs are generated via `Blake2(Location)` ONLY upon initial registration. This mapping (`Location -> AssetId`) is permanently stored in `pallet-asset-registry`.
  - _Why_: Pure hashing is vulnerable to XCM version upgrades changing `Location` encoding (and thus hashes). Storage persistence allows migration (updating `Location` keys while preserving `AssetId` values) without breaking user balances.
  - _Action_: Governance calls `register_foreign_asset`, which generates the ID, locks the mapping, and initializes the asset in `pallet-assets`.
- `Token-Domain Bootstrap Protocol`: Runtime glue hooks on Asset Registry registration and TMC curve creation must remain idempotent and deterministic. Preferred default mapping is `tol_id = token_asset_id` for non-LP assets, with governance override available via explicit binding extrinsics.
- `Sovereign Liquidity`: The Parachain treats itself as a sovereign entity. It does not trust foreign chains to manage its liquidity; it pulls assets into its own local `pallet-assets` registry via XCM Reserve Transfers.

---

## 6. Pre-Task Preparation Protocol

`Before executing any task, the Agent must`:

1.  `Decompose`: Is this a Spec change (Math) or an Impl change (Code)?
2.  `Locate Truth`:
    - If Math: Consult `/simulator`.
    - If Code: Consult `/template` patterns and `AGENTS.md` conventions.
3.  `Documentation Review`:
    - Review `/docs/README.md`.
    - If touching specific entities (e.g., Axial Router), read their specific guides.
4.  `Context Check`: Ensure mental model aligns with current Architecture.

---

## 7. Task Completion Protocol

`The sequence for "Done"`:

1.  `Validation`:
    - Math holds? (`node ./simulator/tests.js`)
    - Code compiles? (`cargo check --workspace`)
    - Tests pass? (`cargo test`)
    - Weights valid? (`cargo test --features runtime-benchmarks`)
2.  `Hygiene`:
    - Zero Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
    - Code formatted.
3.  `Knowledge Sync`:
    - Update `/docs` if logic changed.
    - Update `AGENTS.md` if _patterns_ or _wisdom_ evolved.
    - Add substantive Change History entry with task, implementation, impact, insights
    - Change History structure: `[Current]` → `[Previous]` → `[Legacy-0]` → `[Legacy-1]` → `[Legacy-2]` (newest first)
    - Update `CHANGELOG.md`.
4.  `Garbage Collection` (if AGENTS.md exceeds 300 lines):
    - Trigger garbage collection phase
    - Change History: keep max 3 Legacy entries; drop oldest, extract lasting insights into sections 3-5
    - Analyze bloat sources: prune verbose sections outside Change History (redundant references, over-detailed patterns)
    - Preserve: architectural decisions rationale, philosophical foundations, active conventions
    - Remove: implementation minutiae superseded by code, resolved open questions, dated references

---

## 8. Change History

- `[Current]`: AAA Phase 7+8+9 закрыты по спецификации без backward-compat/migration слоя (проект в pre-release) + закрыт Phase 10 (Validation/Hardening). `SplitTransfer` переведён на рациональную модель (`SplitLeg { to, share }`, `total_shares`, `remainder_to`) с жёсткой валидацией `sum(share_i) == total_shares`; remainder маршрутизируется в `remainder_to` либо fallback leg[0], а fee upper-bound держит worst-case запас (`legs+1`). Для `OnAddressEvent` добавлен `InboxDrainMode::{Single,Batch,Drain}`: `Batch(max)` валидируется (`0 < max <= MaxAddressEventInboxCount`), `Batch/Drain` трактуют saturation как consume-all+reset, `Single` consume-one с сохранением saturation flag. В zombie sweep добавлены lifecycle проверки `WindowExpired` и `MinUserBalance` для parity с permissionless liveness path при сохранении bounded cursor semantics (`MaxSweepPerBlock`); отдельно подтверждён rent ceiling (`MaxRentAccrual`) юнит-тестом на large block delta. По Phase 10 дополнительно закрыты инварианты `No mid-block retries`, `Stateless steps`, `Saturating arithmetic`, `Budget cap`, `All cycles/queues bounded`: добавлены runtime property-style budget tests по множеству `remaining_weight`, unit-тесты на bounded deferred retry/queue pressure и benchmark guard `process_deferred_retry_max_retries` для worst-case deferred cursor path. Финальный audit-pass: удалён дублирующий Noop-тест, приведены секции тестов/бенчмарков к однородной структуре и добавлены комментарии для неочевидных regression guards. Полная валидация (`simulator`, `cargo check/test`, `runtime-benchmarks`, `clippy`) зелёная.
- `[Previous]`: AAA owner-slot sovereign rollout реализован end-to-end + закрыт Phase 6.5. В pallet/runtime добавлены `owner_slot`, `OwnerSlots(owner,slot)`, `SovereignIndex(sovereign)`, `MaxOwnerSlots`, ошибки `OwnerSlotCapacityExceeded`/`SovereignAccountCollision`; create-path использует first-free scan от `slot=0`, destroy-path освобождает slot/index bindings, `AAACreated` расширен `owner_slot`. Добавлены unit+integration тесты на детерминированную раздачу слотов и reuse после destroy/recreate. Бенчмарки `create_user_aaa`/`create_system_aaa` переведены на worst-case slot scan (`slot=MaxOwnerSlots-1`), create-weights обновлены с учётом линейного DB-read cost по `MaxOwnerSlots`, runtime `pallet_aaa` weights перегенерированы через `frame-omni-bencher`. Для стабильности runtime-bench добавлен pre-seed fee sink в `permissionless_sweep` benchmark setup (иначе sub-ED rent transfer в новый fee sink мог удалять actor в setup/assert path). Выявлен и закрыт runtime-интеграционный риск: sub-ED `ExecutionFeePerStep` падал при пустом fee sink (`PipelineFailed`/`Other("")`); исправлено предсозданием/funding AAA fee sink в test env и chain spec genesis.
- `[Legacy-0]`: AAA spec+impl sync: отказ от `TTL + DeadAaaOwners` в stable-контракте реализован end-to-end. После terminal-refund orphan-активы вне `refund_assets` остаются на бывшем sovereign-адресе и выходят из-под protocol control; recovery-registry/extrinsic удалены из pallet/runtime/tests/weights/benchmarks. Сохранены: строгая `WeightToFee(weight_upper_bound)` fee-модель, `ScheduleWindow` REQUIRED, pre-flight `cycle_fee_upper` + fee reserve, breaker stop enqueue+execution, weighted fairness, bounded O(K) adapter contract с `MaxK`, нормализованный `SplitTransfer` (`share_i + total_shares`, `remainder_to`). Валидация: simulator + workspace check/test/bench/clippy зелёные.
- `[Legacy-1]`: AAA hardening + runtime adapter safety. Добавлено: строгая валидация `SplitTransfer` (`InsufficientSplitLegs`, `ZeroShareLeg`, `DuplicateRecipient`) на create+execute, runtime guard для `Mint` (только `System`), fail-fast для переполнения `refund_assets` (`RefundAssetsOverflow`), отдельные веса/бенчмарки для `permissionless_sweep` и `recover_dead_aaa_funds`. Исправлено: преждевременное удаление `DeadAaaOwners` после частичного recovery (блокировало последующие claim). Runtime `aaa_config`: O(1) lookup LP-баланса + bounded scan (`AaaMaxPoolScan`) для LP→pair резолва вместо unbounded iter. Runtime AAA integration suite: 31→26 (базовые lifecycle/access-control кейсы оставлены в unit). 85 pallet tests, 155 runtime integration tests, 0 warnings.
- `[Legacy-2]`: AAA v0.40.0 full spec alignment complete. Удалены: `LifecycleState`, `AaaMode`, `PauseActor`, `DormantQueue`, `SystemExecutionPaused`, `AdmissionFee`, `actor_account_id`. Добавлены: `is_paused: bool`, `DeadAaaOwners`, `StepBaseFee`/`ConditionReadFee`/`MaxRentAccrual`, `Noop` task, `permissionless_sweep`, `recover_dead_aaa_funds`, `AAADestroyed` event, `CycleNonceExhausted` (pause System / destroy User). Инсайт: ring double-firing prevented by deferring re-enqueue to post-loop. Суверенный аккаунт: XOR-derivation prevents AccountId collision for small AccountId types. 79 pallet tests, 160 integration tests, 382 workspace tests, 0 warnings.
