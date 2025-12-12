# Project Context

## Meta-Protocol Principles

Living protocol for continuous self-improvement and knowledge evolution:

- `Boundary Clarity`: Meta-principles govern context evolution; project conventions govern domain; self-improvement never contaminates project documentation
- `Layered Abstraction`: Protocol ≠ project; distinct evolutionary pathways; cognitive firewalls prevent conceptual contamination
- `Domain Purity`: Project conventions reflect actual domain; preserve "how we document" vs "what we document" distinction
- `Evolutionary Feedback`: Protocol improvements inform but never override project decisions; feedback loops create emergent intelligence
- `Reflexive Integrity`: Context models separation it mandates; embodies own design principles through recursive application
- `Emergent Elegance`: Multiple iterations reveal constraints guiding toward patterns; complexity reduction emerges from understanding, not premature simplification
- `Progressive Enhancement`: At 95%+ quality, targeted additions beat wholesale replacement; incremental improvement surpasses architectural revolution
- `Knowledge Lifecycle Management`: Periodic garbage collection consolidates overlapping concepts by cognitive function, transforms tactical → strategic; proactive evolution, not technical debt
- `Emergent Property Validation`: Component interactions require explicit testing/documentation—features not bugs; emergent behaviors signal maturity
- `Structural Symmetry`: Test organization mirrors architecture; structure emerges from behavioral patterns; test-system conceptual integrity creates multiplicative quality
- `Morphological-First Decision Making`: Analyze solution space before implementing—map extremes, identify trade-offs, document triggers; applies to evaluating existing systems and designing new ones
- `Framework Evaluation Methodology`: Dual-phase (morphological mapping + recursive decomposition) reveals dimensional intersections and critical transitions invisible to single-phase
- `Specification Maturity`: Documentation evolves exploratory → consolidated; analysis matures → satellite documents merge to canonical; code illustrates, simulators define truth
- `Cognitive Scaffolding`: Layered understanding—progressive comprehension frameworks; each abstraction enables deeper insight into next
- `Constraint-Driven Evolution`: Evolution follows constraint discovery; each iteration reveals boundaries guiding refinement; constraints are catalysts not limitations
- `Necessary Complexity`: Constraint-discovered complexity ≠ accidental complexity; evolved architectures solve discovered constraints; defend against simplification discarding insights; complexity enabling elegance is feature

---

## 1. Overall Concept

A token launch mechanism specification that combines minting curves with automatic treasury-owned liquidity generation through optimized Zap mechanics to create self-sustaining token economies with mathematically guaranteed price boundaries. The system exhibits emergent properties that enhance security and stability beyond designed mechanisms.

## 2. Core Entities

### 2.1 Primary Protocol:

- `TMCTOL`: Token Minting Curve + Treasury-Owned Liquidity with mathematically guaranteed price boundaries (See: `tmctol.en.md`)
- `L2 TOL`: Second-layer DAOs with declining voting power and constant L1 TOL protection (See: `l2-tol.en.md`)

### 2.2 Mechanisms:

- `TMC`: Linear bonding curve (P = P₀ + slope·s) for predictable emission
- `Axial Router`: Price discovery gateway routing TMC vs XYK optimally (See: `axial-router.en.md`)
- `TOL Multi-Bucket`: 66.6% minted supply → 4 independent buckets (~100% capital utilization through deployment cycles)
- `Fee Burning`: Router 0.5% → 100% burn for systematic deflation
- `Zap`: Intelligent liquidity addition handling price imbalances

### 2.3 Emergent Properties:

- `Price Ratchet`: TOL accumulation + fee burning → ever-rising floor
- `Bootstrap Gravity Well`: Critical TOL threshold (~15% market cap) → system stability transition
- `Supply Elasticity Inversion`: Post-threshold, inflation raises (not lowers) minimum price

---

## 3. Architectural Decisions

### 3.1 Core Mechanics

- `Unidirectional Minting`: One-way token creation prevents reserve extraction; creates mathematical lock-in for long-term alignment
- `Linear Pricing`: Fair, predictable progression (P = P₀ + slope·s); enables precise equilibrium calculations
- `Automatic TOL`: 66.6% mint allocation to protocol-owned liquidity; creates bootstrap gravity well and supply elasticity inversion

### 3.2 TOL Multi-Bucket Architecture

- `Multi-Bucket Strategy`: 4 independent buckets with varied governance thresholds (50%:16.667%:16.667%:16.666%); ~100% capital utilization through continuous deployment cycles (temporary buffers recycled into subsequent mints) vs 0% traditional treasuries (idle vaults); 50%+ failure resilience (survives loss of 3 smaller buckets)
- `Share-Based Withdrawal`: LP tokens managed as shares (not absolute amounts); mathematical correctness, no edge cases from pool state changes
- `Floor Protection Range`: Effective floor varies 11-25% based on bucket deployment decisions; governance explicitly trades floor strength for ecosystem development

### 3.3 Mathematical Foundations

- `Dimensional Type System`: Type system as physics engine—`Price [Foreign/Native²]` uses PRECISION (10¹²); dimensionless ratios use PPM (10⁶); prevents categorical errors at compile-time
- `Guaranteed Price Boundaries`: Calculable floor/ceiling formulas via permanent TOL accumulation; protection ranges 11-25% contingent on governance maintaining parameters
- `Bidirectional Compression`: Burning lowers ceiling, TOL raises floor → convergence creates mathematical security traps
- `XYK Constant Product Necessity`: Constant product (x·y=k) guarantees liquidity at all price levels; XYK "inefficiency" is precisely its strength for floor protection
- `Equilibrium Analysis`: System converges to P_eq ≈ √(R_TOL × m / PRECISION); equilibrium explicitly depends on governance-maintained parameters
- `Fee Consistency Principle`: Quote and execution must apply fees identically; XYK uses "fee on input" model (amount_in × (1 - fee)) in both paths; inconsistency invisible at fee=0 but breaks routing logic when activated
- `Fair Rounding Strategy`: Largest remainder method for distribution prevents systematic bias; ostatok (remainder) allocated to party with maximum fractional part; eliminates long-term drift from "remainder always goes to X" patterns

### 3.4 L2 TOL Governance

- `Declining Voting Power`: Direct holders 10x → 1x linear decay; prevents last-minute manipulation
- `Constant L1 TOL Protection`: L1 TOL maintains 10x (no decay); balanced ecosystem protection without extreme multipliers
- `Invoice Voting`: DOUBLE/APPROVE/REDUCE/VETO mechanics; VETO binary (>50% blocks), evaluations determine pricing multiplier
- `Progressive Rewards`: Active voters earn ~2x vs passive; economic incentive for participation

### 3.5 System Architecture

- `Router as Gateway`: Universal entry point ensures optimal price discovery, fee collection, consistent behavior
- `Zap-Based Liquidity`: Intelligent strategy handles price imbalances; maximizes depth when XYK lags TMC; excess foreign swapped to native (buffered for next mint maximizes long-term protocol profit, optimized computation, MEV-resistant determinism)
- `XYK Pool Mandatory`: Constant product (x·y=k) guarantees liquidity at ALL price levels; XYK "inefficiency" prevents complete depletion
- `Fee Burning`: Router 0.5% → 100% burn for deflation; creates friction preventing infinite mint-swap avalanches
- `Buffer Ownership Clarity`: Buffers live where concept lives—Tol buffer (awaiting zap) vs Bucket LP (owned liquidity); intermediate state without semantic value signals architectural redundancy; "distribute then immediately collect" pattern reveals evolved complexity
- `Method Return Minimalism`: Return only what callers use; if method has side effects (state modification), returning detailed decomposition of those effects creates redundancy when state is directly queryable; zap methods modify buckets (side effect) → return only aggregates, callers query buckets for details
- `Internal Slippage Protection`: Fee manager foreign→native conversion uses 10% slippage tolerance based on spot price; prevents price manipulation attacks on burn mechanism; graceful degradation (buffering) when threshold exceeded

### 3.6 Testing & Validation

- `Simulator as Verification`: JavaScript/BigInt pre-production validation; 55 hierarchical RADB tests (see Section 6.6: Testing Wisdom Patterns) reveal emergent properties through multi-actor flows
- `Attack Simulation`: Economic attack scenarios beyond mathematical correctness; governance resilience, cross-chain independence, sandwich attack resistance validated through multi-actor simulation
- `Validation Hierarchy`: Simulator (mathematical correctness) → Testnet (economic attack surface) → Mainnet (empirical parameter tuning); each layer addresses different constraint categories; production readiness requires coverage across all dimensions

---

## 4. Emergent Properties Layer

### 4.1 Critical System Transitions

System exhibits phase transitions at thresholds where qualitative behavior changes discontinuously:

- `Bootstrap Gravity Well`: ~15% TOL/market-cap threshold (fragile → stable); below: high volatility, market-driven; above: stability emergence, mechanism convergence; monitor ratio, alert at threshold; governance shifts from accumulation focus to strategic deployment
- `Supply Elasticity Inversion`: Critical supply where inflation raises (not lowers) floor; floor growth exceeds ceiling growth (TOL quadratic effect); post-inversion minting strengthens floor vs dilutes
- `Legitimacy Phase`: Exploratory → academic transition; conditional guarantees, dimensional analysis, governance dependencies; enables regulatory review; TMCTOL v1.0.0 complete
- `Decentralization Handoff`: Super-user → team veto → progressive reduction → DAO; premature=vulnerable, delayed=low ownership; monitor participation, proposal quality, attacks

### 4.2 Security Enhancements

- `Vesting Cliff Math Trap`: Team tokens mathematically difficult to exit (additional protection beyond smart contracts); monitor ceiling-floor gap, team fraction of TOL
- `Treasury Deadlock Security`: Governance paralysis increases effective lock ratio; system more secure during disputes; floor calculation includes governance state
- `Governance Attack Resilience`: Mathematical constraints protect against distribution manipulation; extreme changes cannot compromise floor (XYK invariant creates value extraction traps); monitor distribution changes, floor preservation
- `Cross-Chain Economic Independence`: Each chain autonomous despite bridge failures; price divergence, TOL concentration don't compromise floor (XYK invariant operates independently); monitor TOL distribution, individual chain floors

### 4.3 Economic Behaviors

- `Mint-Swap Feedback Loop`: TMC mints degrade XYK prices → TMC more attractive (self-reinforcing); router fees (0.5%) prevent infinite avalanches; monitor consecutive TMC routes, price divergence
- `Slope Efficiency Sublinearity`: Equilibrium ∝ √slope (not linear); diminishing returns on slope increases; P_eq ≈ √(R_TOL × m / PRECISION)
- `Price Ratchet Acceleration`: Deflation compounds floor growth (bidirectional compression: burning lowers ceiling, TOL raises floor); velocity ∝ burn_rate/(R_native)⁴; superlinear acceleration creates progressive stability

### 4.4 Dimensional Intersection Innovations

Framework occupies unique positions at dimensional intersections:

- `Security × Efficiency`: Floor (11-25%) WITHOUT idle capital; TOL in active XYK (~100% utilization) vs vaults (0%); resolves "protection requires reserves" vs "efficiency requires deployment"
- `Decentralization × Safety`: Progressive handoff WITHOUT binary transition; phased veto (super-user → team → DAO) maintains security while transferring control
- `Complexity × Elegance`: Multi-component (TMC+4-bucket+Router+Zap+Fee) WITH emergent simplification; necessary complexity creates emergent properties (gravity well, elasticity inversion) simplifying long-term behavior
- `Rigor × Flexibility`: Verifiable formulas WITH flexible deployment; hard constraints (XYK invariant, conservation) coexist with soft parameters (bucket allocation, fees); separates "what math guarantees" from "how governance deploys"
- `Discovery × Resistance`: Responsive oracle (half-life EMA) WITH attack resistance (TVL-weighted, deviation limits); avoids "slow secure" vs "fast vulnerable" dichotomy

Intersection positions create competitive moats through novel constraint resolution—simpler alternatives cannot reach these positions.

---

## 5. Project Structure

- `/docs/`: Specifications (tmctol/l2-tol/axial-router `.en.md` + `.ru.md`, README.md architecture guide)
- `/simulator/`: JavaScript/BigInt verification (model.js, tests.js 55 hierarchical tests, tests.md mirror)
- `AGENTS.md`: Meta-protocol + architectural decisions + conventions ("how we work")
- `CHANGELOG.md`: Evolution history ("what we learned")
- `README.md`: Project overview
- `LICENSE`: MIT

---

## 6. Development Conventions

### 6.1 Documentation Standards

- `Documentation`: Reflect changes with rationale | `Code Examples`: Rust; code illustrates, simulator defines | `Language`: English only | `Mathematical Precision`: Formulas validated with derivations/edge cases | `Implementation Fidelity`: Preserve correctness in optimizations | `Clarity`: Each concept once in logical context | `KISS`: Balance simplicity/accuracy; oversimplification worse than appropriate complexity | `Precision > Brevity`: Never sacrifice correctness | `Primacy`: Core spec contains math; consolidate when mature

### 6.2 Progressive Evolution: Legitimacy Phase

Continuous refinement toward academic legitimacy:

- `Eliminate Marketing`: "Revolutionary" → precise technical | `Conditional Guarantees`: Claims marked with dependencies; no absolutes | `Mathematical Rigor`: Formulas with dimensional analysis, boundaries, proofs | `Governance Transparency`: Properties labeled "governance-dependent" | `Framework vs Promises`: Bounds not outcomes | `Mechanism > Rhetoric`: HOW not promises | `Null Hypothesis`: Lead with failure modes | `Temporal Honesty`: Floor exists when conditions met | `Audit Trail`: Claims → proofs/tests

### 6.3 Technical Implementation

- `Iteration → Elegance`: Work first, elegance second; patterns emerge from constraints | `Simulation vs Production`: Fallbacks reveal inconsistencies demanding normalization | `Closure as Architecture`: JS closures solve dependency injection elegantly
- `Evolved vs Necessary Complexity`: Architectural debt accumulates as "intermediate state without semantic value"—if you distribute then immediately collect, question the distribution; complexity solving discovered constraints ≠ complexity from iterative accretion; example: bucket buffers (distributed pre-zap, collected for unified zap) vs single Tol buffer (awaiting zap); refactor trigger: "where does concept live?" guides state ownership

### 6.4 Dimensional Analysis Discipline

- `Physical Types`: Variables convey magnitude AND dimension (`Price [Foreign/Native]`, `Slope [Foreign/Native²]`) | `Scaling Semantics`: PPM (10^6) dimensionless vs PRECISION (10^12) physical | `Terminology`: Domain-specific (`Native`, `Foreign`) over generic | `Consistency`: Operations preserve dimensional correctness | `Derivations`: Verify dimensions each step

### 6.5 Naming Consistency Patterns

- `Type-First`: Lead with domain (`native_`, `foreign_`); never bare `fee` | `Context-Aware`: Local → simpler, returns → full context | `Dimensional Prefix`: Names reflect dimension; annotations balance clarity

### 6.6 Testing Wisdom Patterns

- `Deep Reading`: Testing lurks behind interfaces; outlines deceive—read originals | `Enhancement`: At 95%+ quality, surgical > rewrites | `Validation Hierarchy`: Simulator → Testnet → Mainnet; math≠security—each layer addresses different constraints | `RADB (Recursive Abstraction Decomposition with Behavioral grouping)`: Hierarchical test sections mirror abstraction layers; codes (Section.Position) enable navigation; structure communicates design—Math (1-3) → Core (4) → Integration (5-9) → Emergent (10) → Security (11-12); component placement reveals architecture | `Coverage`: Production spans math, behavior, invariants, integration, emergent, security | `Triple-Sync`: Code+tests+docs parallel; static → liability | `Failure Interpretation`: Bugs OR evolution—distinguish error from change | `Monitoring Gateway`: Infrastructure required before production; tests → logic, monitoring → runtime

### 6.7 Meta-Protocol Boundary Testing

- `Universal Applicability`: Meta never references project-specific | `Contamination`: Domain → Architectural Decisions | `Refinement`: Feedback reveals violations

### 6.8 Framework Assessment Patterns

- `Dimensional Positioning`: Map across solution space; reveals trade-offs/targets | `Intersection Innovation`: Unique positions achieving mutually exclusive properties; create moats | `Evolution Vectors`: Map paths; identify transitions; document monitoring | `Constraint Classification`: Hard (invariants) vs soft (parameters) vs evolutionary (emergent) | `Complexity Defense`: Document constraint → solution; defend against simplification discarding insights | `Empirical Gaps`: Simulator≠testnet≠mainnet; gap analysis guides validation

### 6.9 Architectural Refactoring Patterns

- `Symmetry Violations Signal Bugs`: Quote and execution paths must be identical; invisible at zero values, catastrophic when activated; test asymmetric code paths with non-zero parameters even if defaults are zero
- `Rounding Bias Accumulation`: "Remainder always goes to X" creates systematic drift over many operations; largest remainder method (allocate to max fractional part) achieves mathematical fairness without state
- `YAGNI in Architecture`: Single-client abstractions without demonstrated constraint = premature; ZapManager seemed clean but solved no problem; deep integration + no reuse = keep together; extraction criteria: (1) second client exists, (2) clarity significantly improves, (3) discovered constraint demands separation
- `State Ownership Questions`: Ask "where does concept live?" not "how to organize code?"; buffer = "awaiting transformation" → lives in transformer; bucket = "owns result" → stores only result; intermediate distribution without semantic purpose reveals evolved (not necessary) complexity
- `Distribute-Collect Anti-Pattern`: If operation (1) distributes state, (2) immediately collects it back, (3) operates on total, then distribution is architectural debt; signals either: premature abstraction from earlier design, or misplaced responsibility; refactor: push distribution to where it creates semantic value (after operation, not before)
- `Return Value Redundancy`: Methods returning detailed decomposition (lp_a, lp_b, native_a, etc.) when callers only use aggregates or fetch details from source objects; if returned fields duplicate state already stored elsewhere, return only what callers actually need; pattern: function modifies state + returns copy of modifications = redundancy; solution: return aggregates, let callers query state directly
- `Single Source of Truth Principle`: Information should exist in exactly one authoritative location; bucket.lp_tokens is truth, not result.lp_a; duplication creates synchronization burden and illusion of independent data; test: can fields drift apart? if yes, eliminate duplication; distributed state must have semantic independence (different owners, different lifecycles), not just structural decomposition

### 6.10 Meta-Methodology Patterns

`Framework Evaluation Insights`:

- `Morphological-First Evaluation`: Map solution space before improving—identify dimensions, extremes, vectors, intersections | `RADB as Documentation`: Tests communicate design clearer than prose | `Three-Layer Validation`: Simulator → Testnet → Mainnet (no skipping); each addresses different constraints | `Phase Transition Modeling`: Document thresholds, procedures, monitoring | `Intersection Innovation Recognition`: Identify trade-off resolutions at dimensional intersections | `Complexity Defense Framework`: Constraint → solution traceability; simpler alternatives fail | `Empirical Gap Analysis`: Plan testnet explicitly; avoid correctness=readiness fallacy | `Evolution Vector Planning`: Dimensional positioning not feature lists

`Application Template`: Morphological Analysis (dimensions, extremes, intersections) → Recursive Decomposition (goal, constraint → solution) → Validation Hierarchy (simulator → testnet → mainnet) → Strategic Evolution (vectors, tuning, defense)

---

## 7. Pre-Task Preparation Protocol

Step 1: Load `/docs/README.md` for documentation architecture
Step 2: Integrate entity-specific documentation for task context
Step 3: Verify alignment with architectural decisions and conventions
Step 4: Document knowledge gaps for future enhancement
Step 5: Review emergent properties implications for current task

---

## 8. Task Completion Protocol

Step 1: Verify architectural consistency (sections 3-5)
Step 2: Execute quality validation: `deno ./simulator/tests.js` (55 tests, hierarchical X.Y codes) and verify `./simulator/tests.md` mirror (section counts and names) is synchronized
Step 3: Update `/docs/README.md` guides for affected entities
Step 4: Mandatory Context Evolution:

- Analyze architectural impact
- Update sections 1-6 for currency

Step 5: Add substantive entry to `CHANGELOG.md` documenting changes, insights, and methodology evolution
Step 6: Garbage Collection Evaluation Workflow

---

## 8. Change History

- `[Current]`: Phase 12 SIMULATOR VALIDATION - Floor Formula Verification. `Problem`: Documentation claimed theoretical floor limits ($P=k/(R+S)^2$ and $1/(1+s/a)^2$ ratio) but no automated tests verified these specific mathematical properties. `Solution`: Implemented `Floor Formula & Scenario Verification` (Test 41) in simulator. Validated spot price matches theoretical floor after full user dump. Validated Floor/Ceiling ratio aligns with derived approximation ($44.4\%$ for $a=66.7\%$). `Integration`: Added test to `simulator/tests.js` and updated mirror `simulator/tests.md`. `Status`: 56/56 tests passing. Theory matches Simulation.
- `[Previous]`: Phase 11 EQUILIBRIUM ANALYSIS - Dimensional Correction. `Problem`: $P_{eq}$ formula was dimensionally incorrect (resulting in $\sqrt{Price}$) and lacked clear definition. `Solution`: Corrected to $P_{eq} \approx \sqrt{R_{foreign} \cdot m}$ (yielding correct Price units) and defined it as the "Backing Equilibrium" where Market Cap = Reserves. `Integration`: Updated `tmctol.en.md` and `tmctol.ru.md`. `Status`: Dimensions valid, definitions rigorous.
- `[Legacy-1]`: Phase 10 BURN MECHANICS - Ratchet Formalization. `Problem`: "Ratchet Effect" documentation relied on vague heuristics and confusing notation (`ΔS/(R-ΔS)²`) rather than physical derivation. `Solution`: Formalized the burn mechanic: Supply Contraction ($S \downarrow$) $\rightarrow$ Max Potential Pool Balance Contraction ($R+S \downarrow$) $\rightarrow$ Floor Elevation ($P = k/(R+S)^2 \uparrow$). Explicitly defined "Bidirectional Compression" (Ceiling drops via curve, Floor rises via burn). `Integration`: Updated `tmctol.en.md` and `tmctol.ru.md`. `Status`: Mathematical causality established.
