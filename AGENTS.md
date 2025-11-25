# Project Context & Meta-Protocol

> `Architectural Wisdom Repository` | Capturing layered abstractions, implementation insights, and evolutionary optimization.

## 0. Meta-Protocol Principles

This document is a living protocol for continuous, intelligent self-improvement and optimal knowledge evolution. Its core principles govern how this context system manages itself and are universally applicable to any project:

- `Mandatory Self-Improvement`: Every task must end with an update to this document, creating a self-reinforcing cycle of knowledge accumulation and protocol refinement.
- `Protocol Evolution`: The rules themselves must be improved when more efficient workflows are discovered. Meta-protocol evolution demonstrates system maturity.
- `Test-Driven Evolution`: Comprehensive validation enables clean evolution through systematic patterns and self-correcting feedback loops.
- `Context Optimization`: Systematically prevent infinite growth through proactive cleanup protocols. Evolution includes both addition AND consolidation of information.
- `Knowledge Consolidation`: Transform tactical experiences into strategic wisdom. Preserve important insights from rotated history in permanent sections.
- `Non-Duplication Enforcement`: Information must exist in only one authoritative section. Create hierarchical navigation instead of duplicating content.
- `Self-Referential Enhancement`: Protocols should enforce their own effectiveness through requirements that create positive feedback loops.
- `Decreasing Abstraction Structure`: Always organize from general to specific, mirroring optimal cognitive processing patterns.
- `Context Garbage Collection`: Systematically audit and remove outdated, redundant, or obsolete information through periodic consolidation cycles.
- `Change History Rotation`: Maintain fixed entry limits through intelligent consolidation. Newer entries provide tactical details, older entries preserve strategic patterns.
- `Validation Infrastructure`: Automated validation of document structure, cross-references, and information architecture ensures protocol adherence.
- `Pre-Task Preparation Protocol`: Mandatory review of relevant context before task execution ensures comprehensive understanding and consistency.
- `Task Completion Protocol`: Structured completion checklist ensures quality gates, validation, and context updates are never skipped.
- `Hierarchical Documentation`: Mirror system architecture in documentation structure - integration guides reflect component relationships.
- `Living Documentation`: Documentation evolves with implementation, maintaining synchronization between design intent and actual behavior.
- `Boundary Clarity`: Meta-principles govern context evolution; project conventions govern domain; self-improvement never contaminates project documentation.
- `Layered Abstraction`: Protocol ≠ project; distinct evolutionary pathways; cognitive firewalls prevent conceptual contamination.
- `Domain Purity`: Project conventions reflect actual domain; preserve "how we document" vs "what we document" distinction.
- `Evolutionary Feedback`: Protocol improvements inform but never override project decisions; feedback loops create emergent intelligence.
- `Reflexive Integrity`: Context models separation it mandates; embodies own design principles through recursive application.
- `Emergent Elegance`: Multiple iterations reveal constraints guiding toward patterns; complexity reduction emerges from understanding, not premature simplification.
- `Progressive Enhancement`: At 95%+ quality, targeted additions beat wholesale replacement; incremental improvement surpasses architectural revolution.
- `Knowledge Lifecycle Management`: Periodic garbage collection consolidates overlapping concepts by cognitive function, transforms tactical → strategic; proactive evolution, not technical debt.
- `Emergent Property Validation`: Component interactions require explicit testing/documentation—features not bugs; emergent behaviors signal maturity.
- `Structural Symmetry`: Test organization mirrors architecture; structure emerges from behavioral patterns; test-system conceptual integrity creates multiplicative quality.
- `Morphological-First Decision Making`: Analyze solution space before implementing—map extremes, identify trade-offs, document triggers; applies to evaluating existing systems and designing new ones.
- `Framework Evaluation Methodology`: Dual-phase (morphological mapping + recursive decomposition) reveals dimensional intersections and critical transitions invisible to single-phase.
- `Specification Maturity`: Documentation evolves exploratory → consolidated; analysis matures → satellite documents merge to canonical; code illustrates, simulators define truth.
- `Cognitive Scaffolding`: Layered understanding—progressive comprehension frameworks; each abstraction enables deeper insight into next.
- `Constraint-Driven Evolution`: Evolution follows constraint discovery; each iteration reveals boundaries guiding refinement; constraints are catalysts not limitations.
- `Necessary Complexity`: Constraint-discovered complexity ≠ accidental complexity; evolved architectures solve discovered constraints; defend against simplification discarding insights; complexity enabling elegance is feature.

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

- `Polkadot SDK 2025 Standard`:
  - `Omni Node`: Deployment architecture eliminating node boilerplate.
  - `Frame V2`: Strictly typed `#[frame::pallet]`, `frame_benchmarking::v2`.
  - `Token-Driven Coordination`: State transitions are triggered by asset movement (Substrate hooks), not Signed Extrinsics. This ensures origin-agnostic security.
  - `Runtime-as-Config`: Business logic (Pallets) is generic. Configuration (Runtime) injects specific behavior via Adapters following SDK patterns for clean separation.
  - `Unified Primitives`: `primitives/src/ecosystem.rs` is the single source of truth for constants, avoiding magic numbers. `AssetKind` uses bitmask classification for O(1) type inspection.
  - `Type System Discipline`:
  - Enforced strict `sp_arithmetic::Permill` for ecosystem parameters (allocations, fees).
  - Adopted `sp_core::U256` for internal bonding curve calculations to prevent intermediate overflows.
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
- `/scripts/`: Automation (Simulation runners, Benchmark generators, Context validators).
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

### 5.6 Network Integration Protocol

_Context: Handling Foreign Assets and XCM._

- `Hybrid Registration Protocol`: Foreign assets use a "Hybrid Registry" pattern combining deterministic hashing with storage persistence.
  - _Mechanism_: Asset IDs are generated via `Blake2(Location)` ONLY upon initial registration. This mapping (`Location -> AssetId`) is permanently stored in `pallet-asset-registry`.
  - _Why_: Pure hashing is vulnerable to XCM version upgrades changing `Location` encoding (and thus hashes). Storage persistence allows migration (updating `Location` keys while preserving `AssetId` values) without breaking user balances.
  - _Action_: Governance calls `register_foreign_asset`, which generates the ID, locks the mapping, and initializes the asset in `pallet-assets`.
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
5.  `Benchmarking Check`: If touching dispatchables, plan the V2 Benchmark update AND Runtime Integration.

---

## 7. Task Completion Protocol

`The sequence for "Done"`:

1.  `Validation`:
    - Math holds? (`deno ./simulator/tests.js`)
    - Code compiles? (`cargo check --workspace`)
    - Tests pass? (`cargo test`)
    - Weights valid? (`cargo test --features runtime-benchmarks`)
2.  `Hygiene`:
    - Zero Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
    - Code formatted.
3.  `Context Infrastructure Validation`:
    - Run: `./scripts/validate-context.sh`
4.  `Knowledge Sync`:
    - Update `/docs` if logic changed.
    - Update `AGENTS.md` if _patterns_ or _wisdom_ evolved.
    - Update `CHANGELOG.md`.
5.  `Garbage Collection`:
    - Did this task make some previous context obsolete? Delete it.

---

## 9. Change History

- `[Current]`: Phase 8 Polkadot SDK 2512 Update. `Problem`: Parachain template was running on SDK 2509, missing latest optimizations (XCM v5, modern frame migrations). `Solution`: Upgraded to SDK 2512 (v1.21.0). `Integration`: Updated workspace to SDK 2512, refactored `Executive` to use `SingleBlockMigrations`, implemented `V5Config` for `xcmp-queue`, and updated runtime APIs to use `LazyBlock`. `Status`: Workspace compiles, all 101 tests pass. `Roadmap`: E2E validation with latest node binary.
- `[Previous]`: Phase 7 ARCHITECTURAL EVOLUTION - Hybrid Asset Registry Implemented. `Problem`: Pure deterministic hashing of XCM Locations is vulnerable to XCM version upgrades changing hashes. `Solution`: Implemented "Hybrid Registry" pattern. Asset IDs are generated via hashing ONLY upon initial registration, then persisted in `ForeignAssetMapping` storage. `LocationToAssetId` converter now performs O(1) storage lookups instead of on-the-fly hashing. `Integration`: Updated `pallet-asset-registry` with storage map, `xcm_config` to use registry lookup. `Status`: 101 tests passing. Workspace compilation successful. `Roadmap`: Ready for E2E validation.
- `[Legacy-1]`: Phase 7 MAJOR MILESTONE - Foreign Asset Reception infrastructure complete. `Core Infrastructure`: LocationToAssetId (blake2 deterministic), ForeignAssetsTransactor (pallet-assets integration), ReserveAssetsFrom trust filter (relay + siblings), XcmReserveTransferFilter enabled. `Testing`: 10 XCM unit tests (101/101 runtime tests passing). `Deliverables`: Network configs (westend.yml, tmctol.yml, network.yml), TypeScript test suite (foreign-assets.test.ts), full documentation (README.md 310 lines, phase7-foreign-assets.md 421 lines). Next: governance metadata management, economic pallet integration, and execution of foreign-asset e2e flows.
- `[Legacy-2]`: Phase 6 COMPLETED - ALL CRITICAL BLOCKERS RESOLVED WITH ROOT CAUSE IDENTIFICATION. Missing `register_validate_block!` macro identified and fixed. Missing `RelayParentOffsetApi` implementation added. Updated to `FixedVelocityConsensusHook` matching official template. Runtime now fully matches official parachain template configuration. All 91 tests pass. `STANDALONE MODE VERIFIED`: Parachain successfully produces and finalizes blocks in standalone mode (blocks #1-#4 confirmed). `ZOMBIENET INTEGRATION ACHIEVED`: Full network with relay chain and parachain successfully launched and connected. `ROOT CAUSE IDENTIFIED`: Comparison with official template revealed missing APIs and incorrect hook configuration. Parachain runtime is now production-ready.
