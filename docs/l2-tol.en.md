# `L2 TOL` Specification

This specification defines `L2 TOL` systems that issue custom L2 tokens (L2TOLT - L2 TOL Tokens) while establishing L2 Treasury-Owned Liquidity paired with the L1 Native token. L2 TOLs may use various emission mechanisms including TMC (Token Minting Curve) or custom minting strategies. Core innovations include declining voting power, progressive participation rewards, and per-block micro-streaming for MEV elimination.

---

## Table of Contents

1. [Core Definitions](#1-core-definitions)
2. [System Invariants](#2-system-invariants)
3. [Governance Mechanics](#3-governance-mechanics)
4. [Treasury Integration](#4-treasury-integration)
5. [L2 TOL Lifecycle](#5-l2-toldao-lifecycle)
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
- `DripVault` — Per-block streaming contract for continuous treasury operations
- `Declining Power` — Voting weight decreasing from 10x to 1x over the voting period (except L1 TOL: constant 10x)
- `Progressive Rewards` — Redistribution creating 1:5 ratio between passive and active participants
- `Team Shares` — Locked allocations with governance rights but no transfer ability
- `Invoice Voting` — BLDR pattern using AMPLIFY/APPROVE/REDUCE/VETO mechanics for contributor payments

---

## 2. System Invariants

Every certified L2 TOL MUST enforce these on-chain invariants:

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

```
L2_TOL_native ≥ configured_minimum_native
```

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

### 2.7 Treasury TOL LP Management

TOL LP tokens from TMCTOL minting are held in Treasury for governance-controlled strategic deployment. This pattern applies to both Native TOLDAO (L1) and L2 TOLs with different governance scopes:

```rust
struct TreasuryLpManager {
    tol_lp_balance: Balance,
    total_lp_accumulated: Balance,
}

impl TreasuryLpManager {
    fn store_tol_lp(lp_tokens: Balance) {
        Treasury::receive_lp_tokens(lp_tokens);
        Self::increment_accumulated(lp_tokens);
    }
}
```

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
- No admin keys or unilateral control at any level
- Total TOL LP tracked on-chain for transparency across all deployments
- Floor guarantee maintained regardless of LP location or governance level
- Cross-level coordination: L2 TOL strategic decisions may trigger L1 review if significant

`Critical Invariants (Universal)`:

- Treasury ownership = permanent protocol ownership (no individual withdrawal at any level)
- All strategic deployment decisions require governance approval (L1 or L2 as appropriate)
- Floor calculation remains valid across all deployment configurations and governance levels
- LP tokens never leave protocol control, only change location within protocol-controlled accounts
- Transparency requirement: all LP movements tracked on-chain regardless of originating governance level

---

## 3. Governance Mechanics

### 3.1 Declining Voting Power

Voting weight decays linearly from 10x to 1x over different periods, incentivizing early participation:

```rust
fn calculate_voting_power(
    voter: AccountId,
    vote_time: Timestamp,
    vote_start: Timestamp,
    vote_end: Timestamp,
    base_weight: Balance,
    l1_tol_l2tolt_balance: Balance
) -> Balance {
    // L1 TOL holdings maintain constant 10x throughout voting period
    if voter == L1_TOL_ACCOUNT {
        return l1_tol_l2tolt_balance * 10;
    }

    // Regular holders experience declining power
    let progress = (vote_time - vote_start) / (vote_end - vote_start);
    let multiplier = 10.0 - (9.0 * progress);

    // Apply GovXP multiplier for competent participants
    let govxp_multiplier = calculate_govxp_multiplier(voter);

    base_weight * multiplier * govxp_multiplier
}
```

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

```rust
fn calculate_govxp_multiplier(govxp: u64) -> FixedU128 {
    let max_bonus = FixedU128::from(4.0);  // Maximum bonus 4.0 (resulting multiplier 5.0x)
    let k = FixedU128::from(0.0005);       // Curve steepness coefficient

    // Logistic curve (S-curve) for diminishing returns effect
    let exponent = -k * FixedU128::from(govxp);
    let sigmoid = FixedU128::from(2) / (FixedU128::from(1) + exponent.exp());

    FixedU128::from(1) + (max_bonus * (sigmoid - FixedU128::from(1)))
}
```

`Curve Properties`:

- At GovXP = 0, multiplier ≈ 1.0x
- Growth most intensive in mid-range GovXP
- Multiplier asymptotically approaches 5.0x, preventing infinite influence growth
- Diminishing returns effect encourages continuous participation, not accumulation

Active governance participants receive enhanced rewards with GovXP weighting:

```rust
fn distribute_rewards(epoch: EpochId) -> Result<(), Error> {
    let base_reward = calculate_base_staking_reward();
    let active_voters = count_active_participants(epoch);
    let passive_stakers = total_stakers - active_voters;

    // Non-participants receive 20% of base
    let passive_reward = base_reward * 0.2;
    let total_passive_payout = passive_reward * passive_stakers;

    // Redistribution pool from penalties
    let penalty_pool = (base_reward * passive_stakers) - total_passive_payout;

    // Active voters share base + redistribution with GovXP weighting
    let total_weighted_stake = calculate_total_weighted_stake(active_voters);
    for active_voter in active_voters {
        let govxp_multiplier = calculate_govxp_multiplier(govxp);
        let weighted_stake = get_staked_balance(voter) * govxp_multiplier;
        let voter_reward = base_reward + (penalty_pool * weighted_stake / total_weighted_stake);
        distribute_to_voter(voter, voter_reward);
    }

    // Result: up to 5x rewards for competent participation
    distribute(passive_stakers, passive_reward);
}
```

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

```
Final_Vote_Weight = Token_Balance × Temporal_Multiplier × GovXP_Multiplier
```

- `Token_Balance`: Amount of staked tokens (L2TOLT)
- `Temporal_Multiplier`: Time-based multiplier (10x → 1x in Declining Power)
- `GovXP_Multiplier`: Multiplier from 1.0x to 5.0x based on GovXP

`Fee Distribution Formula`:

```
Share_of_Fees = Total_Fees_Pool × (Staked_Balance × GovXP_Multiplier) / Global_Weighted_Stake
```

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

```
Timing: Start 25% 50% 75% End
Direct holder: 10x 7.75x 5.5x 3.25x 1x
L1 TOL: 10x 10x 10x 10x 10x ← No decay
GovXP Min: 1x 1x 1x 1x 1x ← Base multiplier
GovXP Max: 5x 5x 5x 5x 5x ← Maximum bonus (configurable via L1 governance)
```

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

Eliminates MEV through continuous micro-transactions:

```rust
struct DripVault {
    total_allocation: Balance,
    blocks_remaining: BlockNumber,
    amount_per_block: Balance,
}

impl DripVault {
    fn initialize(allocation: Balance, duration_blocks: BlockNumber) -> Self {
        Self {
            total_allocation: allocation,
            blocks_remaining: duration_blocks,
            amount_per_block: allocation / duration_blocks,
        }
    }

    fn execute_block(&mut self) -> Balance {
        if self.blocks_remaining > 0 {
            self.blocks_remaining -= 1;
            self.amount_per_block  // Amount too small for profitable MEV
        } else {
            0
        }
    }
}
```

`Configuration Example`:

```
2-year stream: 2 * 365 * 7200 blocks = 5,256,000 blocks
Per-block amount: 10,000 NATIVE / 5,256,000 = 0.0019 NATIVE
MEV profit: price_impact(0.0019) - gas_cost < 0 ✓
```

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

BLDR serves as the canonical L2 TMCTOL DAO implementation (using TMC) for transparent payroll:

```rust
struct BldrConfig {
    team_share: Permill,        // 0% for pure treasury model
    treasury_share: Permill,    // 100% if no team
    invoice_cooldown: BlockNumber,
    max_invoice_size: Balance,
}
```

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

   ```
   Multiplier = (AMPLIFY_votes × 2.0 + APPROVE_votes × 1.0 + REDUCE_votes × 0.5)
                / (AMPLIFY_votes + APPROVE_votes + REDUCE_votes)
   ```

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

```
Invoice: 1000 USDC requested

Vote distribution:
- AMPLIFY: 35% (350 tokens) → 700 weighted
- APPROVE: 45% (450 tokens) → 450 weighted
- REDUCE: 15% (150 tokens) → 75 weighted
- VETO: 5% (50 tokens) → ignored (< 50%)

Multiplier = (700 + 450 + 75) / (350 + 450 + 150) = 1.29
Final payment: 1000 × 1.29 = 1,290 USDC
Runtime update: Voting period = 7 days × (1 / 1.29) ≈ 5.4 days (accelerated deployment)
```

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

```
Treasury seeds BLDR → Contributors earn → Some sell for Native →
XYK price dips → Treasury buyback → Relock with multiplier →
Stronger governance → Better decisions → More value → Repeat
```

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

```
Attack_cost = min(
    l1_tol_l2tolt × 10,            // L1 TOL constant 10x advantage
    circulating_l2tolt × price × 5, // Need >50% with 1x vs early 10x voters
    native_market_cap × 0.1,        // L1 Native takeover
    opportunity_cost(10x_lockup)    // Time-weighted capital
)
```

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

```rust
const GOVXP_CONFIG: GovXpConfig = GovXpConfig {
    max_bonus: FixedU128::from(4.0),      // Maximum bonus 4.0 (resulting multiplier 5.0x)
    k_coefficient: FixedU128::from(0.0005), // Logistic curve steepness coefficient
    oracle_success_reward: 100,           // Reward for successful prediction
    oracle_failure_penalty: 50,           // Penalty for failed prediction
    winning_vote_base: 10,                // Base reward for correct vote
    proposal_success_reward: 500,         // Reward for successful proposal
    invoice_approve_reward: 250,          // Reward for APPROVE invoice
    invoice_amplify_reward: 1000,          // Reward for AMPLIFY invoice
    invoice_veto_penalty: 500,            // Penalty for VETO invoice
    delegation_multiplier: FixedU128::from_rational(1, 5), // 20% of delegate's earned
    inactivity_decay: FixedU128::from_rational(99, 100),   // 1% decay per inactive epoch
    inactivity_epochs: 91,                // Inactivity period for decay (91 days)
};
```

```rust
const DEFAULT_CONFIG: L2TolDaoConfig = L2TolDaoConfig {
    // Shares (fully configurable at L2 TOL creation, modifiable via governance unless set immutable)
    l2_tol_share: Permill::from_percent(33),
    treasury_share: Permill::from_percent(33),
    user_share: Permill::from_percent(34),
    team_share: Permill::from_percent(0),

    // Governance
    // Note: L1 TOL receives constant 10x multiplier (no decay) - not configurable
    declining_power_start: 10,  // Direct holders start at 10x
    declining_power_end: 1,     // Direct holders end at 1x
    voting_period: 7 * DAYS,              // Standard voting period (configurable via multiplier)
    native_voting_period: 14 * DAYS,      // Native token voting period
    veto_override_threshold: 14 * DAYS,

    // Economics
    min_native_floor: 1000 * UNITS,
    min_mint_amount: 100 * UNITS,
    buyback_threshold: Permill::from_percent(95),

    // Streaming
    default_drip_duration: 2 * YEARS,
    blocks_per_drip: 1,
};
```

### 8.2 Governance Schemas and Phased Rollout

The L2 TOL framework serves as a progressive factory for creating L2 DAOs with extensive configurability. All parameters are fully configurable at creation and can be modified via governance unless explicitly set as immutable. The framework evolves progressively - basic team governance models are available at launch, with advanced features (LP-based voting, custom veto mechanics, etc.) added through protocol upgrades as the ecosystem matures.

`Phase 1: Ecosystem L2 TOLs Only`

- Launch Timeline: Initial protocol deployment
- Authorization: L1 DAO referendum required
- Initial Native: Voted amount from L1 Treasury
- Governance: L1 TOL constant 10x (no decay), three-path resolution
- Direct holders: Declining power (10x → 1x)
- Examples: BLDR (payroll), core ecosystem utilities
- Security: Maximum (L1-vetted, L1-funded)

`Phase 2: User-Created L2 TOLs`

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

### 9.1 Core Pallets

```rust
trait L2TolDaoPallet {
    // Lifecycle - Framework serves as progressive L2 TOL factory with extensive configurability
    fn register_dao(config: L2TolDaoConfig, origin: Origin) -> Result<DaoId, Error>;
    fn activate_dao(dao_id: DaoId, initial_native: Balance) -> Result<(), Error>;

    // Minting (implementation varies by emission mechanism)
    fn mint(dao_id: DaoId, amount: Balance) -> Result<Balance, Error>;
    fn custom_emission(dao_id: DaoId, params: EmissionParams) -> Result<Balance, Error>;

    // Governance
    fn cast_vote(proposal: ProposalId, weight: Balance, conviction: u8);
    fn calculate_voting_power(
        voter: AccountId,
        time: Timestamp,
        vote_start: Timestamp,
        vote_end: Timestamp,
        base_weight: Balance,
        l1_tol_l2tolt_balance: Balance
    ) -> Balance;
    fn resolve_proposal(proposal: ProposalId) -> ResolutionPath;

    // L2 TOL Management
    fn add_liquidity_to_l2_tol(dao_id: DaoId, native: Balance, l2pdt: Balance);
    fn get_l2_tol_reserves(dao_id: DaoId) -> (Balance, Balance);
}
```

### 9.2 Critical Hooks

```rust
trait Hooks {
    fn on_mint(dao_id: DaoId, amount: Balance);
    fn on_vote_cast(voter: AccountId, power: Balance);
    fn on_proposal_executed(proposal: ProposalId);
    fn on_participation_reward(voter: AccountId, reward: Balance);
}
```

### 9.3 Storage

#### 9.3.1 GovXP Storage

```rust
/// Main GovXP score storage
#[pallet::storage]
pub(super) type GovXPStorage<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    T::AccountId,
    u64,
    ValueQuery,
>;

/// GovXP delegation registry
#[pallet::storage]
pub(super) type DelegateRegistry<T: Config> = StorageMap<
    _,
    Blake2_128Concat,
    T::AccountId,  // Delegator
    T::AccountId,  // Delegate
    OptionQuery,
>;

/// Voting data for GovXP reward calculations
#[pallet::storage]
pub(super) type ProposalVoters<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    DaoId,
    Blake2_128Concat,
    ProposalId,
    BoundedVec<(T::AccountId, VoteData), T::MaxVotersPerProposal>,
    ValueQuery,
>;

/// GovXP parameter configuration storage
#[pallet::storage]
pub(super) type GovXpConfigStorage<T: Config> = StorageValue<
    _,
    GovXpConfig,
    ValueQuery,
>;
```

- L2 TOL reserves and LP token tracking
- Voting power decay schedules
- Participation history for rewards
- Treasury and team vesting schedules
- DripVault streaming states

---

## Conclusion

L2 TOL architecture enables second-level DAOs with mathematical security guarantees through:

1. `Declining voting power` preventing last-minute governance attacks
2. `Per-block streaming` eliminating MEV entirely
3. `Progressive rewards` incentivizing active participation
4. `L2 TOL immutability` ensuring permanent second-level liquidity
5. `Dual-track governance` with execution delay for potential L1 veto intervention
6. `Unified 10x multiplier` for L1 Treasury holdings in any L2 TOL
7. `Evolution by Design` enabling continuous protocol improvement

The system creates sustainable token economies where L2TOLT tokens carry governance power with mandatory L2 Treasury-Owned Liquidity support, while treasury participation provides alignment without traditional team allocations, maintaining security through economic incentives rather than bureaucratic structures.

---

- `Version`: 1.0.0
- `Date`: November 2025
- `Author`: LLB Lab
- `License`: MIT
