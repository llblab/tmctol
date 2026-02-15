# Treasury-Owned Liquidity: Rising Price Floor Architecture

> **On-Chain Account** (PalletId: `tolpalle`)
>
> - SS58: `5EYCAe5jXm4d4X1VBioovFFvqtsD9qayotKmPLFmNAPWtBYQ`
> - Hex: `0x6d6f646c746f6c70616c6c650000000000000000000000000000000000000000`

## Executive Summary

The Treasury-Owned Liquidity (TOL) pallet is a `TolId-Scoped Multi-Bucket Accumulation Engine` for protocol-owned LP accounting across four buckets:

- `A` Anchor
- `B` Building
- `C` Capital
- `D` Dormant

The pallet provides:

- deterministic LP split by PPM
- domain config lifecycle (`create_tol(tol_id, ...)` then domain-level update/manage)
- bounded request queue processing (`on_initialize`)
- periodic bucket purity sweep (`on_idle`) that moves non-LP bucket balances to Burning Manager
- governance-manual unwind for non-anchor buckets (`B/C/D`)
- hard block for Anchor unwind (`A`)
- LP ingress validation (`is_lp`) before distribution
- token-to-domain binding (`AssetId -> TolId`) for LP ingress routing

`Phase note`: runtime now uses token-scoped `tol_id` domains for config, bucket state, queue, and zap buffer. Compatibility domain remains `tol_id = 0`.

`Important current-state note`: in the current runtime wiring, TMC mint flow is routed to Zap Manager account first. TOL queue/distribution extrinsics exist and are callable, but are not auto-invoked by TMC/ZapManager pallet code paths.

## Architecture Overview

### Runtime Wiring (Current)

```mermaid
graph TD
    TMC[TMC mint_with_distribution] -->|Mint/transfer allocations| ZMAcc[ZapManager Account]
    ZMAcc -->|on_initialize + on_idle| ZM[Zap Manager]
    ZM -->|add_liquidity| Pool[Asset Conversion Pool]
    Pool -->|LP minted| ZM
    ZM -->|transfer_lp_tokens_to_tol (resolver)| TOLAcc[Resolved TOL Ingress Account]

    ManualOrAutomation[Integration call signed by ZapManagerAccount] -->|receive_lp_tokens| TOL[TOL Pallet]
    TOL -->|distribute_lp_tokens_to_buckets| BA[Bucket A Account]
    TOL -->|distribute_lp_tokens_to_buckets| BB[Bucket B Account]
    TOL -->|distribute_lp_tokens_to_buckets| BC[Bucket C Account]
    TOL -->|distribute_lp_tokens_to_buckets| BD[Bucket D Account]

    TOL -->|on_idle non-LP sweep| BMAcc[BurningManager Account]

    ZMCall[Optional call signed by ZapManagerAccount] -->|receive_mint_allocation| Queue[PendingRequests]
    Queue -->|on_initialize| Buffers[ZapBufferState]
```

### Design Principles

1. `Rising Price Floor`: LP accumulation strengthens long-term floor support
2. `Segmentation`: Anchor/Building/Capital/Dormant isolate treasury intents
3. `Bounded Processing`: queue and sweep loops are bounded by runtime constants
4. `PPM Determinism`: integer arithmetic, no floating-point drift
5. `Conservation`: bucket D receives remainder (`total - A - B - C`)
6. `Bucket Purity`: non-LP balances in bucket accounts are automatically swept to Burning Manager
7. `LP Namespace`: clean-slate genesis initializes AssetConversion LP IDs in `TYPE_LP` space

### Account Boundaries

- `TOL pallet account`: default LP ingress/source account (`tol_id = 0`) for bucket distribution
- `Token-scoped ingress accounts`: derived by `tol_id` for token-domain LP routing (default domain resolves to pallet account)
- `Bucket accounts (A/B/C/D)`: per-domain LP custody accounts (default constants for `tol_id=0`, deterministic sub-accounts for `tol_id>0`); non-LP residue is swept out in `on_idle`
- `Burning Manager account`: destination for non-LP bucket sweep transfers
- `Treasury destination account`: governance-selected unwind target, which may be distinct from the TOL pallet account

Interpretation: TOL is the protocol liquidity treasury layer, but architecture does not require it to be the only treasury destination in the ecosystem. Governance can route unwind outputs to separate operational treasury accounts when policy requires.

`Current architecture`: token-scoped ingress routing and per-domain liquidity/bucket state isolation are active. Full cross-pallet token-domain lifecycle coupling (TMC/Zap/TOL execution domains) remains planned.

## Core Logic

### 1) LP Distribution (`receive_lp_tokens` → `distribute_lp_tokens_to_buckets`)

When `receive_lp_tokens` is called by `ZapManagerAccount`, LP is split by configured bucket PPM:

```rust
bucket_a_amount = (lp_amount * BucketAAllocation) / 1_000_000;
bucket_b_amount = (lp_amount * BucketBAllocation) / 1_000_000;
bucket_c_amount = (lp_amount * BucketCAllocation) / 1_000_000;
bucket_d_amount = lp_amount - bucket_a_amount - bucket_b_amount - bucket_c_amount;
```

Then LP is transferred from the resolved TOL ingress account (`tol_id` domain) to the four `bucket accounts`.

### 2) Queue Path (`receive_mint_allocation` → `PendingRequests` → `ZapBufferState`)

`receive_mint_allocation(token_asset, ...)` resolves `tol_id` from token binding and enqueues domain-local requests. `on_initialize` drains each active domain queue into domain-local `ZapBufferState[tol_id]`.

`Current runtime note`: this path exists in pallet API, but TMC does not call it directly in the default wiring.

### 3) Token Domain Binding (`bind_token_to_tol`) + Deterministic Auto Bootstrap

Manual governance binding remains available:

```rust
bind_token_to_tol(token_asset, tol_id)
```

Current runtime also provides deterministic default binding for non-LP assets:

- default domain convention: `tol_id = token_asset_id` for `Local/Foreign` assets
- compatibility domain remains `tol_id = 0` (legacy default constants)
- helper path: `ensure_domain_for_token(token_asset, foreign_asset, total_allocation)`
- if governance already bound a token to a non-default domain, ensure logic keeps that override and hardens that bound domain instead of silently overwriting it

Runtime effect:

- `TokenTolBindings[token_asset] = tol_id`
- LP ingress account resolves to `ingress_account_for_tol_id(tol_id)`
- `distribute_lp_tokens_to_buckets` resolves ingress source from LP pair + token binding
- bootstrap can happen idempotently from runtime hooks (Asset Registry and TMC curve activation)

### 4) Manual Unwind (`unwind_bucket_liquidity`)

Governance (`AdminOrigin`) can unwind LP for buckets `B/C/D`:

```rust
unwind_bucket_liquidity(bucket_id, lp_asset, lp_amount, destination)
```

Rules:

- `bucket_id = 0` (`Anchor`) is rejected (`BucketAUnwindDisabled`)
- `bucket_id = 1..3` is allowed
- LP is removed via `AssetConversion::remove_liquidity`
- resulting assets are transferred to `destination`
- governance can set `destination` to a treasury account that is distinct from the TOL pallet account
- unwind is policy-driven by bucket type (`A` blocked, `B/C/D` allowed via `AdminOrigin`) and can process any LP-classified asset

### 5) Bucket Purity Sweep (`on_idle`)

TOL executes a bounded idle sweep that keeps bucket accounts LP-only by design:

- iterates active `tol_id` domains and inspects domain bucket A/B/C/D balances for tracked non-LP assets from each domain config
- transfers non-LP balances from bucket accounts to `BurningManagerAccount`
- emits `NonLpAssetSwept` per successful transfer
- bounded by `MaxNonLpSweepsPerBlock` and remaining block weight

### 6) LP Namespace Initialization (Clean-Slate Runtime)

At genesis build, TOL triggers AssetConversion LP namespace initialization:

- sets `pallet_asset_conversion::NextPoolAssetId` to at least `TYPE_LP | 1`
- ensures newly created pools mint LP IDs inside the LP bitmask namespace (`0x4...`)
- enables strict LP detection via `AssetKind::is_lp()` across runtime LP classification paths

## Multi-Bucket Policy

| Bucket | Name     | Default PPM | % of Total | Current Policy |
| :----- | :------- | :---------- | :--------- | :------------- |
| `A`    | Anchor   | 500,000     | 50.00%     | Auto-unwind disabled; manual unwind disabled |
| `B`    | Building | 166,667     | 16.67%     | Manual unwind allowed; DripVault auto-drip planned |
| `C`    | Capital  | 166,667     | 16.67%     | Manual unwind allowed; DripVault gradual unwind planned |
| `D`    | Dormant  | 166,666     | 16.66%     | No auto policy; manual unwind allowed |

See roadmap direction: [`aaa-specification.md`](./aaa-specification.md) (System AAA guardrails and delivery phases).

## Configuration Surface

| Parameter                | Type      | Runtime Default | Status |
| :----------------------- | :-------- | :-------------- | :----- |
| `BucketAAllocation`      | `u32`     | 500,000         | Active |
| `BucketBAllocation`      | `u32`     | 166,667         | Active |
| `BucketCAllocation`      | `u32`     | 166,667         | Active |
| `BucketDAllocation`      | `u32`     | 166,666         | Active |
| `MaxTolRequestsPerBlock` | `u32`     | 10              | Active |
| `MaxNonLpSweepsPerBlock` | `u32`     | 16              | Active |
| `MaxTolDomains`          | `u32`     | 1024            | Active |
| `MinSwapForeign`         | `u128`    | `1 * PRECISION` | Declared in TOL config |
| `MaxPriceDeviation`      | `Permill` | 20%             | Declared in TOL config; not consumed by TOL logic today |
| `BucketARatio`           | `Permill` | 50.00%          | Declared; currently not enforced in TOL logic |
| `BucketBRatio`           | `Permill` | 16.67%          | Declared; currently not enforced in TOL logic |
| `BucketCRatio`           | `Permill` | 16.67%          | Declared; currently not enforced in TOL logic |

Additional active wiring:

- `BurningManagerAccount` is configured as the sweep destination for non-LP bucket residue
- LP namespace initialization runs in TOL genesis build via `AssetConversion::initialize_lp_asset_namespace()`

## Core Storage Types (Current)

```rust
pub struct TolConfig {
    pub token_asset: AssetKind,
    pub foreign_asset: AssetKind,
    pub total_tol_allocation: u128,
    pub current_tol_supply: u128,
}

pub struct BucketAllocation {
    pub target_allocation_ppm: u32,
    pub native_reserves: u128,
    pub foreign_reserves: u128,
    pub lp_tokens: u128,
}

pub struct ZapBuffer {
    pub pending_native: u128,
    pub pending_foreign: u128,
}

pub struct TolAllocationRequest {
    pub total_native: u128,
    pub total_foreign: u128,
}
```

TolId-scoped storage layout (current runtime):

- `TolConfigurations[tol_id] -> TolConfig` domain config record for token/foreign pair and TOL totals
- `ActiveTolDomains -> BoundedVec<TolId>` bounded domain registry for hook iteration
- `TokenTolBindings[token_asset] -> tol_id` token-domain routing map for LP ingress
- `BucketA/B/C/D[tol_id] -> BucketAllocation` domain bucket states
- `PendingRequests[tol_id] -> BoundedVec<TolAllocationRequest>` bounded domain queue for allocation intake
- `ZapBufferState[tol_id] -> ZapBuffer` domain accumulation state for queued mint allocations

## Events (Exact Runtime Shape)

| Event | Fields |
| :---- | :----- |
| `TolCreated` | `{ tol_id, token_asset, foreign_asset, total_allocation }` |
| `BucketAllocationUpdated` | `{ tol_id, token_asset, bucket_id, new_allocation_ppm }` |
| `TokenTolBound` | `{ token_asset, tol_id }` |
| `TokenDomainEnsured` | `{ token_asset, tol_id, action, previous_foreign_asset, foreign_asset }` where `action ∈ {Created, Rebound, Noop}` |
| `ZapBufferUpdated` | `{ tol_id, token_asset, pending_native, pending_foreign }` |
| `TreasuryWithdraw` | `{ tol_id, asset, amount, destination }` |
| `TolRequestsProcessed` | `{ tol_id, count }` |
| `LPTokensReceived` | `{ tol_id, lp_asset_id, lp_amount, distributed_block }` |
| `LPTokensDistributed` | `{ tol_id, lp_asset_id, bucket_a_amount, bucket_b_amount, bucket_c_amount, bucket_d_amount, total_amount }` |
| `BucketLiquidityUnwound` | `{ tol_id, token_asset, bucket_id, lp_asset, lp_amount, native_out, foreign_out, destination }` |
| `NonLpAssetSwept` | `{ tol_id, bucket_id, asset, amount, destination }` |

Operator flow reference: lifecycle checkpoint runbook in [`core-architecture.md`](./core-architecture.md) (§3.6)

## Integration Notes

### TMC → TOL

Current runtime path is:

- on `create_curve`, runtime glue calls deterministic domain bootstrap for the token (`ensure_domain_for_token`) and enables the token in Zap Manager
- during mint flow, TMC mints native allocation to `ZapManagerAccount`
- during mint flow, TMC transfers foreign input asset to `ZapManagerAccount`

TMC does **not** directly call TOL queue methods in the default runtime wiring.

### Zap Manager → TOL

Zap Manager currently:

- executes liquidity provisioning
- resolves token-scoped TOL ingress account via runtime resolver
- transfers LP tokens to the resolved ingress account
- emits Zap Manager events

Zap Manager does **not** directly dispatch TOL `receive_lp_tokens` in its pallet code path.

### LP Ingress Validation

TOL validates LP classification at ingress:

- `receive_lp_tokens` requires `lp_asset.is_lp()`
- `distribute_lp_tokens_to_buckets` requires `lp_asset.is_lp()`
- ingress source account resolves from LP pair + `TokenTolBindings`
- `unwind_bucket_liquidity` accepts any LP-classified asset for allowed buckets (`B/C/D`)

### Oracle / Slippage Guard

- Active swap guard is in `Zap Manager` price validation path
- TOL `MaxPriceDeviation` config exists but is not directly used in TOL dispatch logic today

## Security Model

- `Origin Gating`: `AdminOrigin` for privileged config/unwind operations
- `Anchor Immutability`: bucket `A` unwind blocked by pallet error
- `LP Ingress Gate`: only LP-classified assets are accepted for LP distribution/unwind paths
- `Bucket Purity`: non-LP bucket residue is moved to Burning Manager in bounded idle sweeps
- `Bounded Queue`: `MaxTolRequestsPerBlock = 10`
- `Bounded Sweep`: `MaxNonLpSweepsPerBlock = 16`
- `Deterministic Accounting`: PPM split + remainder capture in bucket D

### Antifragile Notes

Detailed lifecycle risk handling lives in:

- `core-architecture.md` (§3.5 + §3.6)

## Current Status vs Roadmap

### Implemented now

- Domain-scoped runtime state and custody are active (`tol_id` configs, buckets, queue, buffer, per-domain accounts)
- Deterministic token-domain routing is active (`AssetId -> TolId`, with governance override support)
- LP safety invariants are active (`is_lp` ingress gate, bucket purity sweeps, Anchor unwind disabled)
- Clean-slate LP namespace initialization is active (`TYPE_LP`)

### Planned

- Automated production trigger path for LP bucket distribution (`receive_lp_tokens` integration)
- DripVault automation for `B/C` policy flows
- Advanced per-domain governance/policy surfaces and execution fairness expansion (Zap-first)
- Canonical `$BLDR` policy profile hardening

For checkpoint-level operational flow, use `core-architecture.md` (§3.6).
