# TMCTOL Changelog

All notable changes to the TMCTOL (Token Minting Curve + Treasury-Owned Liquidity) framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
