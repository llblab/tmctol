# Governance Operations Guide

## Purpose

Operational checklist for executing governance actions across TMCTOL pallets, with parameter inventory and tuning guidance. Focus: safe changes, pre-checks, and post-change validation.

## Core Principles

- AdminOrigin-only for economic levers; avoid ad-hoc origins.
- Prefer parameter updates over code changes; keep logic immutable where possible.
- Apply changes in maintenance windows; announce before execution.
- Validate on a staging network with production-like params before mainnet.

## Standard Operating Procedure (SOP)

1. `Pre-checks`
   - Ensure node is healthy (finality progressing, peers ≥ required).
   - Confirm signer authority for AdminOrigin/Root.
   - Snapshot current parameters and relevant storage.
   - Identify block height window; ensure block weights leave margin for `on_idle` hooks.

2. `Dry-run (staging)`
   - Run the intended extrinsic set on staging with identical parameters.
   - Execute regression tests relevant to the change (see Validation Mapping below).

3. `Execution (production)`
   - Submit extrinsics in a minimal bundle; avoid unrelated changes.
   - Wait for inclusion/finality; monitor events and errors live.

4. `Post-change validation`
   - Re-read parameters; verify deltas match intent.
   - Run lightweight state checks (balances, mappings).
   - For routing/zap/burning: perform a small-value canary action.
   - Record change in CHANGELOG with block hash and parameters.

## Parameter Inventory (by pallet)

### Axial Router

- `router_fee: Permill`
- `tracked_assets: Vec<AssetId>` (oracle updates)
- `max_hops: u32` (from primitives)
- Validation: small swap path, oracle update event, fee collection event.

### Token Minting Curve (TMC)

- `curve_slope: Balance` (PRECISION-scaled)
- `minting_paused: bool`
- Allocation splits: `TMC_USER_ALLOCATION`, `TMC_ZAP_ALLOCATION` (Permill)
- Validation: mint via router path (ceiling route), ensure pause state respected.

### Burning Manager

- `min_burn_native: Balance`
- `dust_threshold: Balance`
- `slippage_tolerance: Permill`
- Validation: process native/foreign fees; ensure dust is skipped and min burn enforced.

### Treasury-Owned Liquidity (TOL)

- Bucket allocations: `A/B/C/D Permill`
- `max_price_deviation: Permill`
- `min_swap_foreign: Balance`
- Validation: rebalance trigger, bucket sums == 1_000_000, swap guard respects deviation.

### Zap Manager

- `enabled_assets: Vec<AssetKind>`
- `min_swap_foreign: Balance`
- `dust_threshold: Balance`
- `retry_cooldown: u32`
- Validation: on_initialize scan processes enabled assets; cooldown respected after failure.

### Asset Registry

- `register_foreign_asset`, `register_foreign_asset_with_id`
- `link_existing_asset`
- `migrate_location_key`
- Invariants: foreign IDs must have 0xF… mask; mapping must be unique.
- Validation: mapping exists and matches assets pallet; events emitted.

### Token Onboarding Contract (runtime glue)

- Default rule (non-LP): `tol_id = token_asset_id`
- Registration and curve-activation hooks must be idempotent and deterministic
- Governance override is explicit: `bind_token_to_tol(token_asset, custom_tol_id)`
- Override precondition: target domain must already exist
- Fail-fast rule: if domain ensure cannot complete (e.g., domain cap), triggering extrinsic reverts atomically

### Runtime / XCM Config (selected)

- `LocationToAssetId` mapping correctness.
- Trust filters: `ForeignAssetsFromSibling`, `ReserveAssetsFrom`, `XcmReserveTransferFilter`.
- Validation: reserve-transfer relay→para, sibling→para; ensure credited asset matches mapping.

## Tuning Notes

- `Router fee`: default 50 bps (5_000 ppm). Increase cautiously; each +10 bps reduces UX and volume elasticity. Validate slippage + fee vs. oracle deviation guard.
- `TMC slope`: higher slope = stronger ceiling; adjust only after simulator validation. Keep minting paused during slope changes; unpause after validation.
- `Burning thresholds`: raise `min_burn_native` to reduce on-chain churn; ensure accumulated buffers remain below dust attack vectors.
- `Zap thresholds`: increase `dust_threshold` to reduce micro work; watch for stranded small balances. `retry_cooldown` > expected oracle recovery time.
- `TOL buckets`: maintain sum = 100%; adjust in small steps to avoid large liquidity shifts in one block.

## Validation Mapping (minimal smoke tests)

- Router: single-hop swap (native↔std), fee event observed, oracle update succeeds.
- TMC: mint via router when market > curve; pause blocks mint when enabled.
- Burning: enqueue fees (native + foreign); process burns; check slippage guard.
- Zap: deposit foreign + native; on_initialize adds liquidity; foreign surplus swapped; native surplus held.
- Asset Registry: register + link + migrate; events confirm; pallet-assets metadata intact.
- XCM inbound: relay→para reserve transfer; sibling→para if configured; balance credited; mapping resolves to 0xF… ID.

## Runbooks (quick refs)

### Update Router Fee

1. Read current fee.
2. Dry-run on staging with intended fee; run small swap.
3. Submit `update_router_fee(new_fee)`.
4. Validate fee event + oracle update + successful swap.

### Pause/Unpause TMC

1. If changing slope, pause first.
2. Adjust slope (if needed).
3. Unpause; run canary mint (via router path).
4. Verify allocation splits and events.

### Register Foreign Asset

1. Decide deterministic vs. manual ID; ensure 0xF… mask.
2. Set metadata (symbol/decimals/ED, optional sufficient).
3. Register (`register_foreign_asset` or `_with_id`) or `link_existing_asset`.
4. Validate mapping, metadata, and event; test XCM inbound with tiny amount.

### Migrate Location Key

1. Confirm new key is free.
2. Run `migrate_location_key(old, new)`.
3. Verify `MigrationApplied` event; old key removed, new present.
4. Optionally run inbound XCM to confirm resolution.

## Record-Keeping

- Log: block hash, extrinsic hash, parameters before/after, operator account, staging proof (height + result), and any canary tx hashes.
- Update: ROADMAP/CHANGELOG when significant governance ops complete.
