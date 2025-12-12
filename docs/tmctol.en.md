# `TMCTOL` Framework Specification

## Abstract

TMCTOL (Token Minting Curve + Treasury-Owned Liquidity) is a tokenomic framework establishing mathematically defined price boundaries through treasury-controlled liquidity accumulation. The system combines unidirectional token emission with automated XYK reserve allocation to create calculable downside protection ranging from 11% to 25% of equilibrium price, contingent on governance maintaining specified system parameters.

`Key Properties`:

- Linear price ceiling via minting curve: `P_ceiling = P₀ + m·S/PRECISION`
- Hyperbolic price floor via XYK mechanics: `P_floor = R_foreign/(R_native + S_sold)²`
- Supply compression through fee burning (0.5% router fee)
- Multi-bucket TOL architecture enabling governance flexibility

---

## 1. Framework Foundation

### 1.1 Core Innovation

TMCTOL addresses the fundamental problem of unlimited downside risk in token economics by establishing `mathematically bounded risk` through the interaction of three mechanisms:

1. `Unidirectional Minting Curve (TMC)`: Creates deterministic price ceiling without redemption option
2. `Treasury-Owned Liquidity (TOL)`: Automatically allocates 66.6% of mints to protocol-controlled XYK reserves
3. `Fee Burning Router`: Directs 0.5% of trading fees to supply compression

The framework transforms unbounded downside into a calculable range where floor protection depends explicitly on governance maintaining system conditions.

### 1.2 Governance Dependencies

All price protection properties are `conditional guarantees` requiring:

`Conditional Requirements`:

- TOL liquidity remains allocated to XYK pools (governance must not withdraw reserves)
- Distribution ratios maintained per protocol specification (66.6% TOL, 33.3% user allocation)
- Fee burning mechanism operates continuously (router fee collection and burn execution)
- Multi-bucket parameters enforced (Bucket_A dedicated to floor protection)

`Governance Risks`:

- Treasury withdrawal of TOL reduces effective floor proportionally
- Allocation ratio changes alter boundary calculations
- Emergency mechanism activation may bypass constraints
- Strategic bucket deployment affects realized floor level (11-25% range)

`Critical Distinction`: The framework provides "mathematically defined risk boundaries" rather than "absolute safety." Floor preservation represents a governance-dependent system property requiring continuous parameter enforcement. Traditional tokens exhibit unbounded downside regardless of governance quality; TMCTOL bounds this risk through transparent mathematical relationships contingent on maintaining specified conditions.

---

## 2. Technical Architecture

### 2.1 Core Type System

`Dimensional Types with PRECISION Scaling (10¹²)`:

- `Balance`: Native token quantities
- `Price [Foreign/Native]`: Exchange rates scaled by PRECISION
- `Slope [Foreign/Native²]`: Linear emission rate parameter
- `Permill`: Parts-per-million ratio (10⁶) for dimensionless quantities

`Purpose`: Type system encodes physical units preventing categorical errors. Price operations preserve dimensional correctness; ratio operations maintain scale separation.

### 2.2 Minting Curve Mathematics

`Linear Emission Model`:

```
spot_price(S) = P₀ + m·S/PRECISION

where:
  P₀ = initial_price
  m = slope parameter [Foreign/Native²]
  S = current supply
```

`Quadratic Integration for Minting`:

```
Payment calculation integrates price curve:
F_required = P₀·ΔS + m·(S₀·ΔS + ΔS²/2)/PRECISION

where:
  F_required = foreign payment needed
  S₀ = supply before mint
  ΔS = tokens to mint
```

`Property`: Deterministic pricing eliminates front-running opportunities. Each mint amount has exactly one price; no price manipulation possible through transaction ordering.

### 2.3 Token Distribution

`Two-Way Split`:

- User allocation: 33.3% (immediate liquidity)
- TOL allocation: 66.6% (protocol reserves)

`Conservation Invariant`:

```
user_amount + tol_amount ≡ total_minted
```

Distribution occurs atomically within minting transaction; no tokens exist outside these two allocations. This eliminates traditional tokenomics complexity (team vesting, treasury separate from liquidity) by consolidating into governance-controlled TOL structure.

### 2.4 Multi-Bucket TOL Architecture

`Bucket Structure (66.6% total allocation)`:

- `Bucket_A (33.3% total supply)`: Base liquidity — primary floor protection mechanism. The floor protection is guaranteed by maintaining 33.3% of circulating supply in liquidity. If Bucket_A's share exceeds this threshold (due to token burning reducing total supply or strategic accumulation), excess liquidity can be migrated to other DEX ecosystems to stimulate arbitrage and ecosystem expansion
- `Bucket_B (11.1% total supply)`: Development buyback — strategic token acquisition. Token burning increases this bucket's relative share of circulating supply
- `Bucket_C (11.1% total supply)`: Operations capital — liquid deployment for needs to obtain Native/Foreign. Token burning increases this bucket's relative share of circulating supply
- `Bucket_D (11.1% total supply)`: Strategic reserve — discretionary governance allocation. Token burning increases this bucket's relative share of circulating supply

`Allocation vs. Circulating Share`: Initial bucket allocations represent fixed percentages of total supply at minting. However, circulating shares (percentage of current supply held by each bucket) evolve dynamically through token burning, buybacks, and strategic reallocations. This distinction enables the system to maintain floor protection guarantees while allowing strategic liquidity expansion.

`Capital Efficiency`: Four independent XYK pools achieve ~100% capital utilization through continuous deployment cycles (with temporary buffers recycled into subsequent mints) versus 0% for traditional treasuries holding idle unbacked tokens. Each bucket maintains separate LP positions enabling granular governance control.

`Floor Protection Range`: Effective floor varies based on bucket utilization:

- `Maximum (25%)`: All buckets providing floor support, no deployment
- `Minimum (11%)`: Only Bucket_A providing floor support, others deployed
- `Typical`: Governance balances between floor protection and ecosystem development
- `Excess Liquidity Migration`: Token burning increases all buckets' relative shares of circulating supply. For Bucket_A, floor protection is guaranteed by maintaining 33.3% of circulating supply in liquidity. When Bucket_A's share exceeds this threshold due to supply compression, governance can authorize migration of excess liquidity to other DEX ecosystems to stimulate arbitrage and ecosystem expansion

### 2.5 Axial Router Mechanism

`Price Discovery Gateway`:

- Compares TMC spot price against XYK pool pricing
- Routes trades to mechanism offering better execution
- Collects 0.5% fee (default) directed to burning
- Emits route type (TMC/XYK) for transparency

`Fee Structure`:

- Router fee: 0.5% → 100% burned (supply compression)
- XYK fee: 0.0% (default) → maximizes spread tightness
- Governance may activate XYK fees for additional deflation

`Critical Function`: Router ensures consistent price discovery while creating deflationary pressure through mandatory fee burning. Without router, arbitrage opportunities would exist between TMC and XYK pricing.

### 2.6 Zap Liquidity Mechanism

`Purpose`: Intelligent liquidity addition handling price imbalances between native/foreign reserves.

`Strategy`: When pool price diverges from fair value, Zap mechanism:

1. Calculates optimal split between native/foreign contributions
2. Swaps excess portion of imbalanced asset
3. Adds liquidity with balanced ratios
4. Maximizes LP tokens received per contribution

`Protection`: Slippage tolerance (default 5%) prevents value extraction during pool initialization or large imbalances. Mechanism ensures TOL allocations achieve maximum liquidity depth.

### 2.7 Fee Burning System

`Accumulation Phase`:

- Router collects fees in foreign asset
- Fees accumulate in buffer until minimum threshold
- Prevents dust burns wasting gas

`Execution Phase`:

- Swaps foreign fees for native tokens via XYK
- Burns native tokens (removes from total supply)
- Updates total_burned metric for transparency

`Supply Dynamics`: Burning creates bidirectional compression—ceiling decreases (fewer tokens at given slope), floor increases (fixed reserves divided by smaller supply). This accelerates convergence toward equilibrium.

---

## 3. Mathematical Foundations

### 3.1 XYK Constant Product Necessity

`Mathematical Basis`:

```
XYK Invariant: k = R_native × R_foreign (constant)

After selling ΔS native tokens:
R_native' = R_native + ΔS
R_foreign' = k / R_native'

Price = R_foreign' / R_native' > 0 for all finite ΔS
```

`Critical Property`: Foreign reserves approach zero asymptotically but never reach zero. This mathematical guarantee underlies floor protection—price can deteriorate indefinitely but never reaches zero value.

`Comparison with Concentrated Liquidity`:

Concentrated liquidity depletes at specific thresholds:

```
Depletion point: θ = 1/(1 + A^(1/3))

A = 10:  reserves depleted at 24% price drop
A = 50:  reserves depleted at 15.7% price drop
A = 100: reserves depleted at 9.1% price drop
```

`Analysis`: Constant product maintains non-zero reserves under all price deterioration scenarios. Concentrated liquidity exhibits discontinuous reserve behavior where floor protection completely fails beyond depletion threshold. XYK's "inefficiency" (wider spreads) is precisely its strength for floor protection mechanisms.

### 3.2 Price Boundaries

`Ceiling Definition`:

```
P_ceiling(S) = P₀ + m·S/PRECISION

Properties:
- Monotonically increasing with supply
- Deterministic (no market dependency)
- Governance-invariant (only changes via parameter modification)
```

`Floor Calculation`:

```
k = R_TOL_native × R_TOL_foreign (constant product invariant)
R_native' = R_TOL_native + S_sold
P_floor = k / (R_native')²

Derived approximation:
P_floor / P_ceiling ≈ 1 / (1 + s/a)²
where s = S_sold/S_total and a = floor support fraction (e.g. a ≈ 1/3 for Bucket_A)
```

`Scenario Analysis` (assuming `a = 33.3%` Base Support):

| Scenario      | Sellable Source                | Sold Fraction ($s$) | Floor/Ceiling Ratio | Volatility |
| :------------ | :----------------------------- | :------------------ | :------------------ | :--------- |
| `User Exit`   | Public Allocation (33%) sold   | $0.333$             | `25%`               | $4\times$  |
| `System Exit` | Public + Treasury (B/C/D) sold | $0.667$             | `11%`               | $9\times$  |

`Key Dependency`:

- `User Exit`: Represents total selling of initial public supply. With Bucket_A supporting ($a=33\%$), the floor holds at 25%.
- `System Exit`: Represents a catastrophic scenario where Treasury buckets (B, C, D) enter circulation and are also sold. Even then, Bucket_A guarantees an 11% hard floor.

### 3.3 Ratchet Effect Analysis

`Mechanism`: Floor elevation occurs through asymmetric component interaction:

`Component Dependencies`:

1. `TMC Pricing`: Linear supply curve `P(S) = P₀ + m·S/PRECISION`
2. `TOL Allocation`: Fixed reserve ratio maintained by governance
3. `Supply Burning`: Fee-driven compression `dS/dt = -f·V_trade`

`System Interaction`:

When burning reduces circulating supply ($S_{circ}$) by $\Delta S$:

1.  `Supply Contraction`: $S'_{circ} = S_{circ} - \Delta S$
2.  `Floor Elevation`: The maximum potential pool balance decreases, raising the floor.
    $$P'_{floor} = \frac{k}{(R_{native} + S'_{circ})^2} > \frac{k}{(R_{native} + S_{circ})^2}$$
3.  `Ceiling Depression`: The minting price lowers as supply retracts.
    $$P'_{ceiling} = P(S'_{circ}) < P(S_{circ})$$
4.  `Result`: The price corridor compresses from both sides (Bidirectional Compression).

`Floor Elevation Velocity`:

```
dP_floor/dt ∝ (burn_rate × R_foreign) / (R_native)⁴

Properties:
- Acceleration increases as R_native decreases
- Velocity proportional to burn rate (governance controls via fees)
- Quadratic denominator creates superlinear growth
```

`Phase Evolution`:

`Phase 1 — Early Accumulation` (low TOL/supply ratio):

- TMC pricing dominates, XYK spreads wide
- Floor elevation rate: low
- System fragile, high volatility

`Phase 2 — Transition` (moderate TOL/supply ratio):

- Both mechanisms contribute comparably
- Floor elevation accelerates
- Bootstrap gravity well forms (~15% TOL/market-cap)

`Phase 3 — Maturation` (high TOL/supply ratio):

- XYK mechanism dominates
- Convergence to equilibrium
- System exhibits stability

`Governance Contingency`: Ratchet operates only when governance maintains:

1. TOL reserve allocations (prevents drainage)
2. Fee burning mechanism (enables supply compression)
3. Distribution ratios (ensures TOL accumulation)

Reversal requires governance decisions: reserve withdrawal, fee deactivation, or allocation changes. Floor elevation represents state-dependent dynamics, not irreversible progression.

### 3.4 Bidirectional Compression

`Supply Burning Effects`:

```
TMC pricing exhibits explicit supply dependence:
P_TMC(S) = P₀ + m·S/PRECISION

When supply decreases by ΔS:
ΔP_ceiling = -m·ΔS/PRECISION (ceiling compression)
P_floor = const(R_TOL) (floor unchanged if reserves constant)

Net effect: spread compression with floor-ceiling convergence
```

`Progression Example` (R_TOL = 666,667 Foreign, m = 1,500,000):

| Supply | P_ceiling | P_floor (min) | Spread |
| ------ | --------- | ------------- | ------ |
| 1M     | 1.501     | 0.11          | 13.6×  |
| 500k   | 0.751     | 0.22          | 3.4×   |
| 200k   | 0.301     | 0.56          | 0.54×  |

`Critical Point`: When P_floor > P_ceiling, arbitrage incentives reverse. Minting becomes more attractive than market selling, creating natural equilibrium.

### 3.5 Equilibrium Analysis

`Backing Equilibrium`:

The price point where the Market Cap implied by the Curve ($P \cdot S$) is fully backed by the Foreign Reserve ($R_{foreign}$).

$$P_{eq} \approx \sqrt{R_{foreign} \cdot m_{slope}}$$

`Dimensional Validation`:
$$\sqrt{[Foreign] \cdot \left[\frac{Foreign}{Native^2}\right]} = \sqrt{\left[\frac{Foreign^2}{Native^2}\right]} = \left[\frac{Foreign}{Native}\right] = [Price]$$

`Significance`:

- `Gravity Well`: Price oscillates around this value as volatility stabilizes.
- `Router Behavior`: Below $P_{eq}$, supply is "oversold" (heavy floor support). Above $P_{eq}$, supply is "premium" (utility driven).

`Numerical Example`:

```
R_TOL = 1,000,000 native tokens
m = 1,000,000,000 (slope in PRECISION units)
PRECISION = 10¹²

P_eq ≈ √(1,000,000 × 1,000,000,000 / 10¹²)
    = √1,000
    ≈ 31.62 Foreign per Native
```

`Interpretation`: Equilibrium represents price level where TMC and XYK mechanisms yield equivalent pricing. System converges to this point through supply burning. Convergence velocity depends on burn rate; higher fees accelerate approach.

`Governance Dependency`: Equilibrium explicitly depends on R_TOL and m parameters. Changes to TOL allocation or slope directly alter convergence target. This enables governance to adjust long-term price targets through parameter modification.

---

## 4. Economic Model

### 4.1 Supply Dynamics

`Emission`: Unidirectional minting creates monotonically increasing supply ceiling. No redemption mechanism prevents reserve drainage.

`Compression`: Fee burning creates deflationary pressure. Net supply trajectory depends on mint rate versus burn rate:

```
dS/dt = mint_rate - burn_rate
where burn_rate = f_router × V_trade
```

`Capital Efficiency`: Multi-bucket TOL achieves ~100% capital utilization through deployment cycles:

- `Traditional treasury`: 0% (holds unbacked tokens in vaults)
- `Single pool TOL`: ~50% (capital locked in single pool)
- `Four-bucket TOL`: ~100% (continuous XYK liquidity deployment with temporary buffer recycling, varied governance thresholds)

`Flexibility`: Bucket independence enables:

- `Bucket_A`: Permanent floor support (no withdrawal)
- `Buckets 2-4`: Strategic deployment per governance decisions
- Effective floor ranges 11% minimum (only Bucket_A) to 25% maximum (all buckets) based on deployment choices

### 4.2 Infrastructure Premium

`Theorem`: For equal liquidity depth, protocol-owned liquidity provides better execution than mercenary LP capital.

`Proof Sketch`:

1. Mercenary LPs extract fees (dilute reserves over time)
2. Protocol TOL grows from mints (accumulates over time)
3. For equal starting liquidity, TOL provides tighter spreads long-term

`Implication`: Zero XYK fees (default) optimal when TOL provides all liquidity. No need to compensate external LPs; protocol benefits from tight spreads and user convenience.

### 4.3 Value Flows

`Minting Flow`:

```
Foreign payment → TMC calculation → Token emission
                                       ↓
                          User (33.3%) + TOL (66.6%)
                                            ↓
                          TOL → Multi-bucket distribution → XYK pools
```

`Trading Flow`:

```
Trade request → Router price comparison → Route selection
                                            ↓
                            TMC (if better) or XYK (if better)
                                            ↓
                            Fee collection (0.5%) → Foreign buffer → Burn execution
```

`Burning Flow`:

```
Foreign fees accumulate → Threshold reached → Swap to native → Burn
                                            ↓
                        Supply decreases → Ceiling compresses + Floor elevates
```

---

## 5. System Dynamics

### 5.1 Virtuous Cycle

```
Adoption → Mints → Higher ceiling + More TOL
    ↓                             ↓
Activity ← Trading ← Higher floor (stronger support)
    ↓
Burning ← Fees ← Volume
    ↓
Narrower range → Reduced volatility → Increased confidence → Adoption
```

`Feedback Mechanisms`:

- Positive: Adoption drives minting → TOL accumulation → stronger floor → confidence → adoption
- Negative: High volatility → reduced confidence → lower adoption → slower TOL growth
- Stabilizing: Floor elevation → reduced downside → risk-adjusted returns improve

### 5.2 Systemic Behavior

`Equilibrium Regions`:

- `Floor proximity`: Reduced arbitrage opportunity; awaiting catalyst
- `Equilibrium band`: Balanced forces; stable trading reflects fair value
- `Ceiling proximity`: Minting incentivized; emission accelerates

`Volatility Dynamics`: Spread compression (P_ceiling - P_floor) decreases monotonically over time given sustained burning. This represents mathematical consequence of supply compression with fixed reserve ratios, not an economic promise but a deterministic outcome contingent on governance maintaining conditions.

### 5.3 Evolution Path

`High-Volatility Development Phase`:

- Wide price range characteristic of early systems
- Floor building via TOL accumulation
- Price discovery through market mechanisms

`Maturing Growth Phase`:

- Governance deploys TOL strategically (parachain expansion, development funding)
- Framework flexibility enables growth without sacrificing floor protection
- Burn effects create deflationary pressure

`Matured Ecosystem Phase`:

- Stability emerges from accumulated TOL depth
- Range compression approaches equilibrium
- Governance maintains deployment flexibility

`Advanced Stability Phase`:

- Narrow range achieved through bidirectional compression
- Price converges to √(R_TOL × m / PRECISION)
- System exhibits "rising stability asset" properties

---

## 6. Implementation Requirements

### 6.1 Technical Implementation

`TOL Reserve Management`:

- Treasury controls allocation via governance
- Withdrawal requires consensus (no unilateral admin keys)
- Multi-bucket structure with independent LP positions
- Share-based accounting prevents edge cases from pool state changes

`XYK Mechanism`:

- Constant product formula necessary for floor properties
- Pool initialization via Zap mechanism handling imbalances
- Slippage protection prevents value extraction
- Reserves transparent and verifiable on-chain

`Fee Routing`:

- Router collects 0.5% fee (configurable via governance)
- Foreign fees accumulate until minimum threshold
- Burn execution swaps to native, removes from supply
- Total burned tracked for transparency

`Precision Requirements`:

- PRECISION = 10¹² for Price and Slope types
- PPM = 10⁶ for dimensionless ratios
- All arithmetic checked for overflow
- Dimensional correctness enforced by type system

### 6.2 Critical Invariants

`Conservation`:

```
user_amount + tol_amount ≡ total_minted
```

Violation indicates distribution calculation error.

`Constant Product`:

```
k = R_native × R_foreign (before) ≈ k' (after fees)
```

XYK trades preserve k within fee tolerance.

`Non-Negative Reserves`:

```
R_native > 0 and R_foreign > 0 always
```

Reserve depletion would break floor guarantee.

`Monotonic Ceiling`:

```
P_ceiling(S₂) ≥ P_ceiling(S₁) for S₂ ≥ S₁
```

Price ceiling never decreases except via burning.

### 6.3 Economic Conditions

`Utility Requirement`: Token demand must derive from genuine use cases. Without utility, downside protection mechanisms operate in isolation without recovery catalyst.

`Continuous Development`: Protocol improvements maintain competitive positioning. Floor provides time for development; recovery requires utility delivery.

`Market Dynamics`: Long-term holders implicitly accept volatility during maturation. Floor protection bounds downside; upside depends on utility and adoption.

`Transparent Communication`: Fee structures and allocation formulas disclosed. Governance decisions visible to participants. Floor protection explicitly marked as governance-dependent.

### 6.4 Governance Structure

#### 6.4.1 Super-User Governance Model

The system launches with `super-user privileges` assigned to the founding team, providing comprehensive administrative control during the initial bootstrap phase. This centralized governance model enables rapid protocol adjustments and security enhancements during critical early development stages.

`Super-User Capabilities`:

- Full parameter adjustment authority (TOL ratios, fee rates, bucket distributions)
- Emergency mechanism activation for security incidents
- Protocol upgrade and migration execution
- Treasury management and strategic allocation decisions

`Team Veto Authority`: Upon super-user removal, the team retains `veto voting power` representing the Native tokens contained within TOL reserves. This veto mechanism provides ongoing protection against governance attacks while enabling progressive decentralization.

`Veto Power Mechanics`:

- Team votes with TOL-held Native tokens for veto decisions
- Veto requires >50% threshold of total voting power
- Veto power applies exclusively to critical parameter changes affecting:
  - TOL allocation ratios (floor protection mechanisms)
  - Fee burning parameters (supply compression)
  - Emergency mechanism activation
  - Protocol migration or sunset decisions

#### 6.4.2 Progressive DAO Transition

`Vesting Governance Power`: The system supports gradual decentralization through vesting mechanisms applied to veto power. As the protocol matures and community governance demonstrates competence, veto authority can be progressively reduced:

- `Phase 1`: Full team veto with TOL voting power
- `Phase 2`: Reduced veto thresholds (e.g., 75% → 60% → 51%)
- `Phase 3`: Time-locked veto with cooling periods
- `Phase 4`: Community-controlled veto through DAO governance

`Technical Enforcement`: Future implementations may technically restrict team governance to veto-only capabilities, preventing parameter manipulation while preserving security oversight.

#### 6.4.3 Treasury Discipline

`Parameter Maintenance`:

- TOL allocation ratio (66.6% default)
- Bucket distribution (50% Bucket_A, 16.67% each for 2-4)
- Fee rates (0.5% router, 0.0% XYK default)
- Burn thresholds and execution parameters
- Veto power vesting schedules

`Risk Communication`: Documentation explicitly states floor protection depends on governance maintaining conditions. No absolute guarantees provided. Market participants understand recovery requires utility delivery beyond floor mechanism.

---

## 7. Advantages & Trade-offs

### 7.1 Framework Strengths

`Mathematically Defined Boundaries`:

- Floor: 11-25% range depending on buckets utilization
- Ceiling: Deterministic via linear curve
- Both verifiable on-chain through transparent formulas

`Governance Flexibility`:

- Multi-bucket enables balancing protection vs. development
- Fee parameters adjustable for economic optimization
- Deployment strategies adaptable to market conditions

`Capital Efficiency`:

- ~100% capital utilization via independent bucket structure with buffer recycling
- No mercenary capital requiring yield incentives
- Protocol owns liquidity, governance controls strategy

### 7.2 Framework Limitations

`Governance Dependencies`:

- Floor protection requires continuous parameter enforcement
- Reserve withdrawal would reduce effective floor proportionally
- Emergency mechanisms could bypass normal constraints

`Market Dependencies`:

- Recovery from floor requires market confidence in utility
- Floor provides opportunity but not guarantee of appreciation
- Arbitrage creates mechanism but not obligation for recovery

`Complexity Trade-offs`:

- Multi-bucket structure adds governance overhead
- Zap mechanism requires sophisticated liquidity management
- Router requires maintenance of price discovery infrastructure

---

## 8. Summary

TMCTOL establishes a framework with mathematically derived price relationships and governance-controlled parameters:

`Core Mechanisms`:

- Unidirectional minting via linear curve creates deterministic ceiling
- 66.6% TOL allocation to XYK reserves establishes hyperbolic floor
- Fee burning (0.5% router fee) compresses supply driving floor elevation
- Multi-bucket architecture enables governance flexibility between protection and development

`Mathematical Framework`:

```
Ceiling:    P_ceiling = P₀ + m·S/PRECISION
Floor:      P_floor = k / (R_native + S_sold)²
Equilibrium: P_eq ≈ √(R_TOL × m / PRECISION)
Velocity:    dP_floor/dt ∝ burn_rate/(R_native)⁴
```

`Critical Dependencies`:

1. Governance maintains TOL allocation (66.6% of mints)
2. Bucket distribution enforced (Bucket_A for floor support)
3. Fee burning mechanism operates continuously
4. Reserves protected from withdrawal without consensus

`Framework Boundaries`:

Floor protection operates within 11-25% range based on bucket deployment:

- Maximum protection (25%): All buckets providing support, no deployment
- Minimum protection (11%): Bucket_A only providing support, others deployed
- Governance controls trade-off between floor strength and ecosystem development

`Realization Requirements`:

System exhibits predicted dynamics (floor elevation, range compression) only when governance sustains parameters. Downside risk remains bounded by mathematical relationships. Upside recovery depends on protocol delivering utility and maintaining market confidence. Framework provides quantifiable risk parameters rather than absolute guarantees.

---

- `Version`: 1.0.0
- `Date`: November 2025
- `Authors`: LLB Lab
- `License`: MIT
