# `L2 TOL` Specification

This specification defines `L2 TOL` systems that issue custom L2 tokens (L2TOLT - L2 TOL Tokens) while establishing L2 Treasury-Owned Liquidity paired with the L1 Native token. L2 TOLs may use various emission mechanisms including TMC (Token Minting Curve) or custom minting strategies.

`Layer map`:

| Layer                 | Core actors                                                 | Governance signal                                                                 |
| :-------------------- | :---------------------------------------------------------- | :-------------------------------------------------------------------------------- |
| `L1 (Native)`         | TMC engine, L1 TOL buckets A/B/C/D, L1 treasury             | L1 treasury can hold L2TOLT and vote with constant `10x`                          |
| `Bridge`              | L1 ↔ L2 token and governance connectivity                   | L1 can participate in critical L2 resolution paths                                |
| `L2 (Child TOL DAOs)` | BLDR and custom L2 TOL instances with configurable emission | Direct holders use declining power (`10x → 1x`) plus GovXP multiplier (`1x → 5x`) |

`Key Properties`:

- L1 Treasury holdings in any L2 TOL receive `constant 10x voting power` (no decay)
- Direct L2TOLT holders experience `declining power` (10x → 1x over voting period)
- `GovXP` (Governance Experience Points) adds 1x-5x multiplier based on proven competence
- `DripVault` per-block streaming reduces MEV extractability and introduces a temporal execution buffer for treasury operations

---

## Table of Contents

1. [Core Definitions](#1-core-definitions)
2. [System Invariants](#2-system-invariants)
3. [Governance Mechanics](#3-governance-mechanics)
4. [Treasury Integration](#4-treasury-integration)
5. [L2 TOL Lifecycle](#5-l2-tol-lifecycle)
6. [BLDR Pattern](#6-bldr-pattern)
7. [Security Model](#7-security-model)
8. [Configuration Parameters](#8-configuration-parameters)
9. [Implementation Requirements](#9-implementation-requirements)

---

## 1. Core Definitions

### Primary Entities

- `Native (L1)` — Parachain base token governed by first-order referenda at L1 level
- `L2 TOL` — Second-level DAO maintaining L2 protocol-owned liquidity paired with L1 Native
- `Ecosystem L2 TOL` — L2 TOL initialized by ecosystem origin, receiving L1 Treasury funding
- `L2TOLT` — L2 TOL DAO Token issued by an L2 TOL with mandatory TOL support
- `L1 TOL` — Treasury-Owned Liquidity at L1 (Native) level
- `L2 TOL` — Second-level Treasury-Owned Liquidity in L2 TOL
- `BLDR` — Canonical L2TOLT builder token for ecosystem payroll and development

### Mechanisms

- `TMC` — Token Minting Curve, one possible emission mechanism for L2TOLT
- `L2 TMCTOL DAO` — A special case of L2 TOL that combines token minting curve and protocol-owned liquidity
- `Custom Emission` — Alternative minting mechanisms configured per L2 TOL requirements
- `DripVault` — Per-block streaming system for continuous treasury operations (future multi-instance model)
- `Declining Power` — Voting weight decreasing from 10x to 1x over the voting period (except L1 TOL: constant 10x)
- `Progressive Rewards` — Redistribution creating 1:5 ratio between passive and active participants
- `Team Shares` — Locked allocations with governance rights but no transfer ability
- `Invoice Voting` — BLDR pattern using AMPLIFY/APPROVE/REDUCE/VETO mechanics for contributor payments

---

## 2. System Invariants

In this specification profile, certified L2 TOLs are expected to enforce these on-chain invariants:

### 2.1 L2 TOL Management

LP tokens marked as L2 TOL can be managed in two modes:

`Locked Mode`:

- LP tokens require for withdrawal:
  - L2 TOL supermajority approval (≥66%)
  - Mandatory timelock (14 days)
  - L1 Native veto window (7 days)

`Treasury Mode`:

- LP tokens held on L2 DAO Treasury balance
- Governed by DAO voting mechanisms
- Flexibility configured at TOLDAO initialization

### 2.2 Native Floor Liquidity

Initial L2 TOL must satisfy minimum requirements configured via L1 TOLDAO pallet parameters (set through L1 referendum):

- `L2_TOL_native ≥ configured_minimum_native`

This ensures XYK mathematics remain valid and prevents dust attacks.

### 2.3 L2 TOL Protection via L1 TOL

When L1 TOL holds L2TOLT tokens of any L2 TOL:

- L1 TOL L2TOLT holdings receive `constant 10x voting multiplier` throughout entire voting period (no decay)
- Direct L2TOLT holders experience declining power (10x → 1x), L1 TOL maintains constant 10x
- L2TOLT balance in L1 TOL determined at vote resolution time
- Applies universally whether L2 TOL was ecosystem-initiated or user-created
- Provides ecosystem-level protection without extreme multipliers
- Native holders vote in Track 2 to direct how L1 TOL votes are deployed

### 2.4 Team Share Requirements

Team allocation is fully configurable per L2 TOL with progressive flexibility:

- `Optional team share`: May be 0% (pure treasury model) or any configured percentage
- `Flexible vesting`: No vesting, or any duration - fully parameterized (0+ years)
- `Governance rights`: Full voting, veto-only, or LP-based voting from treasury tokens
- `Transfer restrictions`: Configurable lock periods or immediate transferability
- `Framework evolution`: New team governance models can be added progressively
- `Initial availability`: Basic configurations at launch, advanced features added via upgrades
- All parameters configurable at L2 TOL creation and modifiable via governance unless set immutable

### 2.5 Telemetry Transparency

Real-time on-chain emission of:

- L2 TOL depth (Native and L2TOLT reserves)
- Treasury balances (including locked team shares)
- Voting power multipliers and decay state
- Participation rates and reward distributions

### 2.6 Evolutionary Design

- System parameters and mechanisms are configurable via governance
- Protocol can evolve when better mechanics are discovered
- Changes require consensus through established governance paths
- Progressive feature rollout: basic team governance at launch, advanced models (LP-based voting, custom veto) added via upgrades
- Framework supports extensive configurability with 0-100% team shares and fully parameterized vesting durations
- Evolution by Design: continuous improvement is a core principle

### 2.7 Protocol LP Management

TOL LP tokens from TMCTOL minting are held under protocol-controlled accounts for governance-directed strategic deployment. This pattern applies to both Native TOLDAO (L1) and L2 TOLs with different governance scopes.

Implementation note: execution wiring can differ by layer. The simulator may model direct logical transitions, while the template runtime uses hook-driven proactive pallet actors and account-based flows. The economic invariant is the same: LP remains under protocol control and moves only through governance-defined mechanisms.

`Strategic Capabilities (Both L1 and L2)`:

1. `Cross-Parachain Deployment`: Via governance referendum, LP can be redeployed to adjacent parachain pools to stimulate arbitrage and tighten cross-chain pricing
2. `XYK Fee Utilization`: XYK fees default to 0.0% for maximum range compression. Governance can activate fees (e.g., 0.3%) to enable Treasury-held LP to collect trading fees for buyback-and-burn programs (Native for L1, L2TOLT for L2)
3. `Liquidity Rebalancing`: Treasury can migrate LP between pools in response to changing market conditions while preserving floor guarantee
4. `Multi-Token Liquidity`: For L2 TOLs, can deploy LP across multiple trading pairs (L2TOLT/Native, L2TOLT/Foreign, etc.)

`Governance Scope Differences`:

| Aspect             | Native TOLDAO (L1)                 | L2 TOL                                    |
| ------------------ | ---------------------------------- | ----------------------------------------- |
| `Voting Body`      | Native token holders               | L2TOLT holders + L1 Treasury proxy        |
| `Deployment Range` | All parachains in ecosystem        | Typically within host parachain ecosystem |
| `Fee Utilization`  | Native buyback-and-burn (optional) | L2TOLT buyback-and-burn (optional)        |
| `Veto Rights`      | N/A (top-level governance)         | L1 super user can veto                    |
| `Voting Period`    | 14 days standard                   | 7 days (14 if L1 involved)                |

`Governance Process`:

- LP deployment requires governance referendum (voting period varies by level)
- Target deployment model: no unilateral operator control. Bootstrap phases may use temporary super-user authority with explicit sunset governance
- Total TOL LP tracked on-chain for transparency across all deployments
- Modeled floor behavior depends on liquidity depth, routing quality, and governance constraints across deployment locations
- Cross-level coordination: L2 TOL strategic decisions may trigger L1 review if significant

`Critical Invariants (Universal)`:

- Treasury ownership is intended as protocol ownership under governed accounts; withdrawal or relocation rights depend on configured governance rules
- All strategic deployment decisions require governance approval (L1 or L2 as appropriate)
- Floor model remains analyzable across deployment configurations, while realized market floor depends on executable liquidity conditions
- LP tokens never leave protocol control, only change location within protocol-controlled accounts
- Transparency requirement: all LP movements tracked on-chain regardless of originating governance level

---

## 3. Governance Mechanics

### 3.1 Declining Voting Power

Voting weight decays linearly from 10x to 1x over different periods, incentivizing early participation.

`Conceptual voting power model`:

- If voter is `L1 TOL` account: `power = l1_tol_l2tolt_balance × 10 × GovXP_multiplier`
- If voter is direct holder:
  - `progress = (vote_time - vote_start) / (vote_end - vote_start)`
  - `temporal_multiplier = 10 - 9 × progress`
  - `power = base_weight × temporal_multiplier × GovXP_multiplier`

`Properties`:

- `Direct L2TOLT holders`: Declining power from 10x (voting start) → 1x (voting end)
- `L1 TOL holdings`: Constant 10x multiplier throughout entire voting period (no decay)
- `GovXP Multiplier`: Additional multiplier from 1.0x to 5.0x based on participant reputation
- L2 TOL votes: 7-day period by default
- L1 Native TOLDAO votes: 14-day period when involved
- Linear decay for direct holders prevents last-minute manipulation
- L1 TOL advantage incentivizes ecosystem participation without extreme multiplier
- GovXP enhances influence of competent participants, creating meritocratic system

### 3.2 Progressive Participation Rewards

#### 3.2.1 GovXP System (Governance Experience Points)

GovXP is a non-transferable (soulbound) numerical attribute tied to wallet address that dynamically changes based on provable on-chain actions.

`Core Principles`:

- `Reward Competence, Not Capital`: GovXP shifts focus from pure token ownership to decision quality
- `Dynamic Reputation`: GovXP is not a static achievement but continuously updated metric
- `Dual Influence`: GovXP simultaneously enhances political influence (voting power) and economic returns
- `Transparency and Verifiability`: All GovXP changes based on publicly verifiable transactions

`GovXP Multiplier Formula (Wisdom Curve)`:

- `GovXP_multiplier = 1 + max_bonus × (sigmoid(govxp) - 1)`
- `max_bonus = 4.0` (therefore max multiplier is `5.0x`)
- `sigmoid` uses configurable steepness coefficient `k = 0.0005`

`Curve Properties`:

- At GovXP = 0, multiplier ≈ 1.0x
- Growth most intensive in mid-range GovXP
- Multiplier asymptotically approaches 5.0x, preventing infinite influence growth
- Diminishing returns effect encourages continuous participation, not accumulation

Active governance participants receive enhanced rewards with GovXP weighting.

`Conceptual reward distribution model`:

1. Compute `base_reward` per participant
2. Apply passive-participation baseline payout (for example, 20% of base)
3. Route the remaining reward pool to active voters
4. Split active pool by weighted stake (`stake × GovXP_multiplier`)
5. Resulting reward ratio can approach `1:5` between passive and highly competent active participants

### 3.3 L1 TOL Voting Advantage

#### 3.3.1 GovXP Accrual and Decay Mechanisms

GovXP is automatically accrued and decayed upon resolution of system events:

| Action     | Progression Path | Condition                             | GovXP Change            | Purpose                         |
| ---------- | ---------------- | ------------------------------------- | ----------------------- | ------------------------------- |
| Prediction | Oracle           | Successful outcome prediction         | +100                    | Stimulates analysis             |
| Prediction | Oracle           | Failed prediction                     | -50                     | Penalizes spam                  |
| Voting     | Strategist       | Vote on winning side                  | + (10 × days_remaining) | Rewards early correct decisions |
| Proposal   | Strategist       | Accepted proposal with KPI completion | +500                    | Encourages quality initiatives  |
| Invoice    | BLDR Builder     | APPROVE (≥1.0 multiplier)             | +250                    | Good work reward                |
| Invoice    | BLDR Builder     | AMPLIFY (≥1.5 multiplier)             | +1000                   | Outstanding work reward         |
| Invoice    | BLDR Builder     | VETO (>50%)                           | -500                    | Quality control penalty         |
| Delegation | Citizen          | Delegate earns GovXP                  | + (Delegate_XP × 0.2)   | Talent discovery incentive      |
| Inactivity | General          | No actions for 90 days                | ×0.99 decay             | Prevents "eternal elite"        |

`Final Voting Power Formula`:

- `Final_Vote_Weight = Token_Balance × Temporal_Multiplier × GovXP_Multiplier`

- `Token_Balance`: Amount of staked tokens (L2TOLT)
- `Temporal_Multiplier`: Time-based multiplier (10x → 1x in Declining Power)
- `GovXP_Multiplier`: Multiplier from 1.0x to 5.0x based on GovXP

`Fee Distribution Formula`:

- `Share_of_Fees = Total_Fees_Pool × (Staked_Balance × GovXP_Multiplier) / Global_Weighted_Stake`

- `Total_Fees_Pool`: Total fees to distribute per epoch
- `Staked_Balance`: Participant's staked token amount
- `Global_Weighted_Stake`: Sum of (Staked_Balance × GovXP_Multiplier) across all stakers

`L1 TOL Holdings (All L2 TOLs)`:

- When L1 TOL holds L2TOLT tokens, they receive `constant 10x multiplier` (no decay)
- Direct L2TOLT holders experience declining power (10x → 1x), L1 TOL maintains 10x
- GovXP multiplier applies to all participants, including L1 TOL (if applicable)
- Applies universally whether L2 TOL was ecosystem-initiated or user-created
- L2TOLT amount in L1 TOL determined at vote resolution time
- Native holders vote in Track 2 to decide how L1 TOL deploys its L2TOLT votes
- Creates economic incentive for L2 TOLs to have L1 Treasury participation

`Voting Power Comparison`:

| Profile       | Start | 25%   | 50%  | 75%   | End | Notes                          |
| :------------ | :---- | :---- | :--- | :---- | :-- | :----------------------------- |
| Direct holder | 10x   | 7.75x | 5.5x | 3.25x | 1x  | Declining power                |
| L1 TOL        | 10x   | 10x   | 10x  | 10x   | 10x | No decay                       |
| GovXP Min     | 1x    | 1x    | 1x   | 1x    | 1x  | Base multiplier                |
| GovXP Max     | 5x    | 5x    | 5x   | 5x    | 5x  | Configurable via L1 governance |

`Strategic Implications`:

- L1 TOL votes carry consistent weight throughout voting period
- Early direct voters (10x) match L1 TOL power initially
- Late direct voters (1x-3x) have significantly less influence than L1 TOL
- GovXP enhances influence of competent participants regardless of voting time
- Incentivizes L2 TOLs to court L1 Treasury participation
- Balanced advantage without extreme multipliers
- Creates meritocratic system where influence depends on proven competence
- All GovXP parameters configurable via L1 governance for system adaptability

### 3.4 Governance Resolution Mechanisms

`Super User Veto Power`:

- Super user can veto any L2 TOL referendum (both user-created and ecosystem-initiated)
- Veto immediately cancels the referendum without further voting
- Provides ultimate safety mechanism for critical security issues

`Two-Track Voting System`:
When L1 TOL holds L2TOLT tokens, L2 TOL uses a dual-track voting mechanism:

`Track 1: Direct L2TOLT Voting`

- L2TOLT holders vote directly with declining power (10x → 1x)
- Standard L2 TOL governance rules apply
- Produces a winning position (FOR or AGAINST) with total vote strength

`Track 2: Native Proxy Voting`

- Native (L1) holders vote on how L1 TOL should deploy its L2TOLT holdings
- Decision determines whether L1 TOL L2TOLT votes for/against the proposal
- `L1 TOL L2TOLT receives constant 10x multiplier throughout voting period`
- L2TOLT balance in L1 TOL calculated at vote resolution time

Both tracks run simultaneously for 7 days, then the system determines the outcome:

`Path 1: Consensus (tracks align) - 7 days total`

- Both tracks vote the same way (both FOR or both AGAINST)
- L1 TOL L2TOLT maintains constant 10x multiplier (no decay)
- Votes sum: Track 1 votes (with decay) + (L1 TOL L2TOLT × 10)
- Immediate execution after 7 days

`Path 2: Divergence with L1 TOL Superiority - 7 days total`

When tracks vote opposite ways AND L1 TOL votes (constant 10x) are stronger:

- Example: Track 1 weighted votes = 8000 AGAINST, L1 TOL = 1000 L2TOLT × 10 = 10000 FOR
- L1 TOL constant multiplier (10000 FOR) exceeds Track 1 declining votes (8000 AGAINST)
- Result: L1 TOL position wins, Track 1 minority votes ignored
- Final: 10000 FOR wins, immediate execution

`Path 3: Divergence with Execution Delay - 14 days total`

When tracks vote opposite ways BUT Track 1 remains stronger than L1 TOL:

- Example: Track 1 weighted votes = 8000 AGAINST, L1 TOL = 500 L2TOLT × 10 = 5000 FOR
- Track 1 winner (8000 AGAINST) exceeds L1 TOL votes (5000 FOR)
- Result: Track 1 position wins but with 7-day execution delay
- Delay provides time for L1 DAO to initiate veto referendum if needed

Critical actions requiring L1 TOLDAO involvement:

- L2 TOL withdrawal or modification
- Bonding curve parameter changes
- Treasury spending above threshold
- Governance rule modifications

---

## 4. Treasury Integration

### 4.1 Per-Block Micro-Streaming

Mitigates a class of MEV opportunities by fragmenting execution into continuous micro-transactions and adds a temporal execution dimension (buffer) for treasury actions.

`Conceptual DripVault model`:

- A policy defines total allocation, duration, and execution cadence in blocks
- Execution is fragmented into small periodic actions rather than one large transaction
- Each action is processed under normal safety policies (permissions, slippage/oracle constraints, accounting)
- Remaining allocation, elapsed schedule, and completion state are tracked on-chain
- Smaller per-step notional can reduce extractable value per transaction under normal market conditions

`Illustrative scale`:

- 2-year stream at ~7200 blocks/day → ~5,256,000 blocks
- 10,000 NATIVE over that horizon → ~0.0019 NATIVE average per block
- Expected MEV edge can become less attractive as per-step value shrinks

### 4.2 Treasury Participation Modes

1. `One-shot seed` — Immediate swap for strategic position
2. `DripVault stream` — Continuous support over months/years
3. `Revenue recycling` — Percentage of income auto-converts to L2TOLT

---

## 5. L2 TOL Lifecycle

### 5.1 Genesis Phase

`Registration requirements vary by phase`:

`Phase 1: Ecosystem L2 TOLs`

- Initiated via L1 DAO referendum
- Initial Native allocation decided through L1 governance vote
- Emission mechanism configuration (TMC parameters, custom minting rules, etc.)
- Share distribution (user, l2_tol, treasury, team) - team share fully configurable (0-100%)
- Team governance model (full voting, veto-only, LP-based voting, etc.)
- Vesting configuration (none, or any duration - fully parameterized)
- Governance configuration with progressive feature availability
- All parameters configurable and modifiable via governance unless set immutable
- Framework serves as progressive L2 TOL factory - basic features at launch, advanced via upgrades

`Phase 2: User-Created L2 TOLs`

- Permissionless registration (no L1 vote required)
- User must provide minimum Native threshold (set via L1 DAO parameters)
- Same emission and distribution requirements as ecosystem TOLDAOs (team share optional)
- Shares sum to exactly 100%
- L2 TOL share ≥ 20%
- Team governance configuration (if team share exists)

`Validation checks (both types)`:

- Shares sum to exactly 100%
- L2 TOL share ≥ 20%
- Initial Native meets minimum requirement:
  - `Ecosystem`: Amount approved in L1 referendum
  - `User-Created`: Meets threshold configured via L1 DAO parameters
- Emission mechanism properly configured
- Team governance configuration compliant (if team share exists) - no vesting duration restrictions

### 5.2 Launch Phase

`Activation mint sequence`:

1. Lock initial Native contribution
2. Execute first mint through configured emission mechanism (TMC or custom)
3. Create XYK pool with L2 TOL allocation
4. Configure LP tokens per TOL management mode (locked or treasury-held)
5. Distribute treasury and team allocations (if configured)
6. Activate governance mechanisms
7. Framework operates as progressive L2 TOL factory - basic features at launch, advanced governance models added via upgrades

### 5.3 Operational Phase

`Continuous operations`:

- Each mint enforces cap check and routes to L2 TOL
- Proposals follow dual approval path for critical actions
- Participation rewards calculate and distribute each epoch
- DripVault executes per-block streaming if configured

### 5.4 Maturation Metrics

L2 TOL considered mature when:

- L2 TOL depth > 100,000 NATIVE equivalent
- 6 months of positive cash flow
- Average participation rate > 30%
- No critical proposals vetoed for 3 months

---

## 6. BLDR Pattern

### 6.1 Architecture

BLDR serves as the canonical L2 TMCTOL DAO implementation (using TMC) for transparent payroll.

`Conceptual BLDR configuration surface`:

| Parameter          | Purpose                                                              |
| :----------------- | :------------------------------------------------------------------- |
| `team_share`       | Team allocation, optionally `0%` for pure treasury model             |
| `treasury_share`   | Treasury allocation, optionally dominant when team share is low/zero |
| `invoice_cooldown` | Minimum delay between invoice actions                                |
| `max_invoice_size` | Hard cap per invoice to limit governance shock                       |

### 6.2 Invoice Flow

Invoice voting combines declining power with nuanced approval mechanics:

`Voting Options`:

- `AMPLIFY (×2.0)` — Exceptional work, increase payment or accelerate timeline (👍👍)
- `APPROVE (×1.0)` — Good work, agree with requested amount/timeline (👍)
- `REDUCE (×0.5)` — Work completed but amount high or timeline long (👎)
- `VETO (block)` — Block invoice/update completely (❌)

`Generalized Vote Options`:

L1 TOL governance can configure custom vote options for L2 TOLs beyond the standard set:

- Standard: `AMPLIFY (×2.0)`, `APPROVE (×1.0)`, `REDUCE (×0.5)`, `VETO (block)`
- Extended: `TRIPLE (×3.0)`, `QUARTER (×0.25)`, `ONE_THIRD (×0.33)`, etc.
- Time-specific: `ACCELERATE (×0.5 timeline)`, `DELAY (×2.0 timeline)`

`Configuration Flexibility`:

- L1 TOL can set vote options and multipliers per L2 TOL
- Options can be specialized for different contexts (payments, runtime updates, parameter changes)
- Multipliers affect different aspects: payment amounts, voting periods, implementation timelines

`Resolution Logic`:

1. Check VETO threshold first:
   - If `VETO > 50%` → Invoice rejected immediately (0 payment)
   - If `VETO ≤ 50%` → VETO votes ignored, proceed to multiplier calculation

2. Calculate weighted multiplier from evaluation votes (AMPLIFY/APPROVE/REDUCE):
   - `Multiplier = (AMPLIFY_votes × 2.0 + APPROVE_votes × 1.0 + REDUCE_votes × 0.5) / (AMPLIFY_votes + APPROVE_votes + REDUCE_votes)`

3. Apply multiplier contextually:
   - For payments: `final_payment = requested_amount × multiplier`
   - For runtime updates: `voting_period = standard_period × (1 / multiplier)` (accelerated deployment)
   - For parameter changes: `time_multiplier = multiplier` (affects implementation timeline)
   - For governance decisions: Multiplier affects urgency and implementation priority

`Process Flow`:

1. `Submission` — Contributor submits invoice with deliverables hash
2. `Voting Period` — Standard 7 days (configurable) with declining power (10x → 1x)
   - L1 TOL BLDR holdings: constant 10x (no decay)
   - Direct BLDR holders: declining from 10x (early) to 1x (late)
3. `VETO Check` — If VETO > 50%, invoice rejected
4. `Multiplier Calculation` — Weighted average of AMPLIFY/APPROVE/REDUCE
5. `Execution` — Automated execution:
   - Payments: `base_amount × multiplier` in BLDR or Native
   - Runtime updates: `voting_period = standard_period × (1 / multiplier)` for accelerated deployment
   - Parameter changes: Multiplier affects implementation urgency
6. `Progressive Rewards` — Active voters earn ~2x vs passive holders
7. `Tracking` — On-chain reference to work completed

`Example`:

- Invoice: `1000 USDC` requested
- Vote distribution:
  - `AMPLIFY`: 35% (350 tokens) → 700 weighted
  - `APPROVE`: 45% (450 tokens) → 450 weighted
  - `REDUCE`: 15% (150 tokens) → 75 weighted
  - `VETO`: 5% (50 tokens) → ignored (`< 50%`)
- Multiplier: `(700 + 450 + 75) / (350 + 450 + 150) = 1.29`
- Final payment: `1000 × 1.29 = 1,290 USDC`
- Runtime update timeline: `7 days × (1 / 1.29) ≈ 5.4 days` (accelerated deployment)

`Key Insights`:

- VETO is binary (block or allow), not part of pricing
- Evaluation votes (AMPLIFY/APPROVE/REDUCE) determine multiplier applied contextually
- For payments: affects amount; for updates: affects timeline; for governance: affects urgency
- Early voters with 10x influence pricing more than late voters
- L1 TOL BLDR holdings maintain consistent 10x influence throughout
- System naturally converges to fair market pricing and optimal timelines through repeated iterations
- L1 TOL can configure L2 TOL parameters, including custom vote options beyond AMPLIFY/REDUCE (e.g., TRIPLE, 1/3)
- Runtime updates benefit from accelerated timelines: high approval (AMPLIFY/TRIPLE) reduces voting period proportionally

### 6.3 Economic Loop

- Treasury seeds BLDR
- Contributors earn BLDR
- Some participants sell for Native
- XYK price dips
- Treasury performs buyback
- Relock with multiplier
- Governance strengthens
- Decision quality improves
- System value compounds
- Cycle repeats

---

## 7. Security Model

### 7.1 Attack Vectors and Mitigations

#### 7.1.1 GovXP System Attacks

`Collusion Farming`:

- `Vector`: Group of participants creates trivial proposals and votes for them to farm GovXP
- `Mitigation`: Proposal authorship reward tied to real KPI completion, not just acceptance. "Ratchet" and other mechanisms make harmful proposals economically unprofitable for entire system

`Sybil Delegation Attack`:

- `Vector`: Attacker creates multiple accounts, delegates to themselves and attempts to farm GovXP
- `Mitigation`: GovXP earned through actions, not delegation alone. To receive GovXP from delegation, main account (delegate) must perform useful actions. Attack becomes economically impractical

`Elite Entrenchment`:

- `Vector`: Veterans with high GovXP can suppress new ideas and create barriers for newcomers
- `Mitigation`: GovXP_Multiplier logistic curve creates "plateau" (maximum 5.0x), preventing veterans from becoming omnipotent. GovXP decay mechanism for inactive participants ensures reputation must be constantly maintained

`Prediction Market Manipulation`:

- `Vector`: Coordination to create false signals in prediction oracle
- `Mitigation`: Penalties for incorrect predictions (-50 GovXP) create economic barrier to spam. Successful predictions require real analysis

| Attack Vector        | Mitigation                           | Cost Formula                            |
| -------------------- | ------------------------------------ | --------------------------------------- |
| `Mint-whale capture` | L1 TOL constant 10x (no decay)       | `cost > l1_tol_holdings × 10`           |
| `Flash governance`   | Declining voting power (10x → 1x)    | `cost = full_position × 10 × lock_time` |
| `Late-stage attack`  | L1 TOL maintains 10x when others 1x  | `cost > l1_tol_holdings × 10`           |
| `L1 Native takeover` | Veto override for critical actions   | `cost > native_market_cap × quorum%`    |
| `MEV exploitation`   | Per-block streaming                  | `profit < 0` (unprofitable)             |
| `Treasury drain`     | Three-path resolution + thresholds   | Requires L1 override or L2 TOL consent  |
| `Spam L2 creation`   | Minimum Native threshold (Phase 2)   | `cost = threshold × native_price`       |
| `Invoice fraud`      | VETO threshold (>50% blocks payment) | Community vigilance + reputation        |

### 7.2 Economic Security

#### 7.2.1 GovXP Manipulation Protection

`Computational Complexity`:

- GovXP_Multiplier calculations performed on-demand (voting, fee distribution) and not stored in state
- Mass accruals (e.g., for voting) designed to avoid gas exhaustion through lazy payouts or batch processing

`Parameter Flexibility`:

- All key GovXP parameters (Max_Bonus, k, accrual/decay values) changeable via L1 governance
- System can adapt and tune over time based on real-world usage experience

`Economic Alignment`:

- GovXP creates economic incentives for quality participation, not just capital accumulation
- System naturally converges to meritocratic model where influence proportional to proven competence
- Slow GovXP decay for inactive participants prevents "eternal elite"

Minimum attack cost for any L2 TOL:

- `Attack_cost = min(l1_tol_l2tolt × 10, circulating_l2tolt × price × 5, native_market_cap × 0.1, opportunity_cost(10x_lockup))`
- `l1_tol_l2tolt × 10`: L1 TOL constant 10x advantage
- `circulating_l2tolt × price × 5`: Need >50% with 1x vs early 10x voters
- `native_market_cap × 0.1`: L1 Native takeover path
- `opportunity_cost(10x_lockup)`: Time-weighted capital lock cost

`Key insights`:

- `Phase 1`: Only ecosystem L2 TOLs launch, vetted through L1 governance
- `Phase 2`: Permissionless L2 TOL creation with minimum Native threshold prevents spam
- `Protection scales dynamically`: User-created L2 TOLs gain ecosystem-level security when L1 TOL acquires their L2TOLT tokens
- `L1 TOL advantage`: Constant 10x (no decay) creates reliable defense layer
- `Economic barrier`: Threshold ensures serious projects only, adjustable via L1 DAO
- `Time-based defense`: Early voters (10x) and L1 TOL (constant 10x) dominate governance

### 7.3 Defense Layers

#### 7.3.1 Multi-layered GovXP Security

`First Layer — Economic Barriers`:

- Penalties for incorrect predictions and vetoed invoices
- Diminishing returns effect in multiplier logistic curve
- Slow reputation decay for inactive participants

`Second Layer — Temporal Constraints`:

- Declining voting power prevents last-minute manipulation
- Lockup periods for LP token withdrawals
- Super user veto as ultimate safety mechanism

`Third Layer — Community Mechanisms`:

- GovXP delegation encourages talent discovery and support
- Reward system for successful proposals incentivizes quality initiatives
- Transparency of all GovXP changes through publicly verifiable transactions

`Fourth Layer — Parameter Adaptability`:

- All key GovXP parameters manageable via L1 referendums
- Ability to adjust system based on real experience
- Protection against "ossification" through evolutionary design

1. `Mathematical` — L1 TOL constant 10x (no decay) provides reliable ecosystem protection
2. `Temporal` — Declining power (10x → 1x) for direct holders creates time-based security
3. `Economic` — Attack cost exceeds potential gain through multiple mechanisms
4. `Resolution` — Three-path voting system provides appropriate response windows
5. `Social` — Transparent operations enable monitoring
6. `Hierarchical` — Super user veto provides ultimate safety mechanism
7. `Meritocratic` — GovXP system rewards competence and quality participation

---

## 8. Configuration Parameters

### 8.1 Recommended Defaults

#### 8.1.1 GovXP Parameters

`Recommended baseline values` (governance-adjustable):

| Parameter                 | Baseline                       |
| :------------------------ | :----------------------------- |
| `max_bonus`               | `4.0` (max multiplier `5.0x`)  |
| `k_coefficient`           | `0.0005`                       |
| `oracle_success_reward`   | `+100`                         |
| `oracle_failure_penalty`  | `-50`                          |
| `winning_vote_base`       | `+10 × days_remaining`         |
| `proposal_success_reward` | `+500`                         |
| `invoice_approve_reward`  | `+250`                         |
| `invoice_amplify_reward`  | `+1000`                        |
| `invoice_veto_penalty`    | `-500`                         |
| `delegation_multiplier`   | `20%` of delegate-earned GovXP |
| `inactivity_decay`        | `1%` decay per inactive epoch  |
| `inactivity_epochs`       | `91 days`                      |

`Recommended L2 TOL default profile` (governance-adjustable unless immutable at launch):

| Category   | Parameter                 | Baseline     |
| :--------- | :------------------------ | :----------- |
| Shares     | `l2_tol_share`            | `33%`        |
| Shares     | `treasury_share`          | `33%`        |
| Shares     | `user_share`              | `34%`        |
| Shares     | `team_share`              | `0%`         |
| Governance | `declining_power_start`   | `10x`        |
| Governance | `declining_power_end`     | `1x`         |
| Governance | `voting_period`           | `7 days`     |
| Governance | `native_voting_period`    | `14 days`    |
| Governance | `veto_override_threshold` | `14 days`    |
| Economics  | `min_native_floor`        | `1000 units` |
| Economics  | `min_mint_amount`         | `100 units`  |
| Economics  | `buyback_threshold`       | `95%`        |
| Streaming  | `default_drip_duration`   | `2 years`    |
| Streaming  | `blocks_per_drip`         | `1 block`    |

`Invariant note`: L1 TOL retains constant `10x` voting multiplier and does not decay over vote time.

### 8.2 Governance Schemas and Phased Rollout

The L2 TOL framework serves as a progressive factory for creating L2 DAOs with extensive configurability. All parameters are fully configurable at creation and can be modified via governance unless explicitly set as immutable. The framework evolves progressively - basic team governance models are available at launch, with advanced features (LP-based voting, custom veto mechanics, etc.) added through protocol upgrades as the ecosystem matures.

`Phase 1: Ecosystem L2 TOLs Only`:

- Launch Timeline: Initial protocol deployment
- Authorization: L1 DAO referendum required
- Initial Native: Voted amount from L1 Treasury
- Governance: L1 TOL constant 10x (no decay), three-path resolution
- Direct holders: Declining power (10x → 1x)
- Examples: BLDR (payroll), core ecosystem utilities
- Security: Maximum (L1-vetted, L1-funded)

`Phase 2: User-Created L2 TOLs`:

- Launch Timeline: After Phase 1 proven stable
- Authorization: Permissionless (threshold-gated)
- Initial Native: User provides minimum threshold (L1 DAO-configurable)
- Governance: Direct holders declining power (10x → 1x), optional L1 TOL participation
- L1 TOL advantage: If L1 TOL holds L2TOLT, constant 10x (no decay)
- Protection Level: Scales with L1 TOL L2TOLT holdings
- Security: Economic barrier (threshold) + optional L1 involvement

`Governance Schema Types`:

1. `Ecosystem Schema` (Phase 1 default)
   - L1 TOL constant 10x (no decay) on L2TOLT holdings
   - Direct holders: declining power (10x → 1x)
   - Three-path resolution with auto-pause
   - Maximum security and L1 backing

2. `User-Created Schema` (Phase 2 default)
   - Direct holders: standard declining voting power (10x → 1x)
   - L1 TOL can acquire L2TOLT: constant 10x (no decay)
   - Security scales with L1 TOL participation
   - Early participation (days 1-2) receives 10x as initial advantage

3. `Pure L1-Proxy Schema` (optional, either phase)
   - All decisions via L1 TOLDAO referendum
   - L2TOLT purely economic
   - Maximum security for critical infrastructure

---

## 9. Implementation Requirements

### 9.1 Core Runtime Responsibilities

A conforming implementation should provide runtime surfaces for:

- `Lifecycle`: register DAO, activate DAO, enforce bootstrap constraints
- `Emission`: mint path per configured emission model (TMC or custom)
- `Governance`: vote casting, deterministic vote-power calculation, proposal resolution
- `Liquidity`: L2 TOL reserve updates, LP accounting, and treasury/lock mode handling

### 9.2 Critical Hook Semantics

A conforming runtime should preserve these hook-level semantics:

- `on_mint`: enforce cap/allocation invariants and accounting updates
- `on_vote_cast`: persist vote metadata needed for deterministic resolution and GovXP logic
- `on_proposal_executed`: apply post-resolution state transitions and timelock/veto outcomes
- `on_participation_reward`: apply reward/penalty deltas and maintain transparent distribution records

### 9.3 Storage Requirements (Conceptual)

A conforming implementation should track at minimum:

- GovXP score per account
- GovXP delegation links
- Proposal voter metadata for reward and resolution logic
- GovXP parameter configuration state
- L2 TOL reserves and LP token tracking
- Voting power decay schedules
- Participation history for rewards
- Treasury and team vesting schedules
- DripVault streaming states

---

## Hard vs Soft Guarantees

### Hard Guarantees (when encoded and enforced on-chain)

- `Deterministic Vote Math`: Voting weight formulas are deterministic for the selected governance schema
- `Governance Gates`: Timelocks, supermajority rules, and veto paths are enforceable when enabled in configuration
- `On-Chain Transparency`: Vote records, LP movements, and treasury state transitions are auditable on-chain
- `Config-Bounded Behavior`: Parameter ranges and rule sets are bounded by runtime configuration and upgrade governance

### Soft Guarantees (market and operational outcomes)

- `MEV Resistance`: Micro-streaming can reduce extractability, but does not remove all MEV vectors
- `Attack-Cost Models`: Economic deterrence depends on liquidity, participation, and adversary capital
- `Floor Realization`: Modeled floor behavior may diverge from observed market prices under stressed liquidity conditions
- `Governance Quality`: Long-term resilience depends on participation quality, parameter stewardship, and operational security

## Conclusion

L2 TOL architecture enables second-level DAOs with formalized security assumptions and bounded mechanisms through:

1. `Declining voting power` preventing last-minute governance attacks
2. `Per-block streaming` reducing MEV extractability and adding a temporal execution buffer in treasury paths
3. `Progressive rewards` incentivizing active participation
4. `L2 TOL governance constraints` protecting second-level liquidity under configured rules
5. `Dual-track governance` with execution delay for potential L1 veto intervention
6. `Unified 10x multiplier` for L1 Treasury holdings in any L2 TOL
7. `Evolution by Design` enabling continuous protocol improvement

The system creates sustainable token economies where L2TOLT tokens carry governance power with mandatory L2 Treasury-Owned Liquidity support, while treasury participation provides alignment without traditional team allocations. Security outcomes are bounded by protocol rules and implementation discipline, and remain dependent on market and governance conditions.

---

- `Version`: 1.1.0
- `Date`: February 2026
- `Author`: LLB Lab
- `License`: MIT
