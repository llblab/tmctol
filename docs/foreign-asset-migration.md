# Foreign Asset Migration & Bitmask Invariants

## Scope
Operational guide for migrating XCM `Location -> AssetId` keys, validating the 0xF… foreign namespace, and running the provided harnesses for migrations and E2E XCM flows.

## Prerequisites
- Running node(s) with WS access (relay, para, optional sibling).
- `node` ≥ 18 with `@polkadot/api` installed.
- `polkadot-omni-node` binary available (for chain spec generation).
- Signer seed (default examples use `//Alice`).

## Bitmask Invariants (Authoritative)
- Foreign assets MUST use 0xF… mask (`TYPE_FOREIGN = 0xF000_0000`).
- Valid foreign IDs: `AssetKind::Foreign(u32)` OR `AssetKind::Local(id)` where `(id & MASK_TYPE) == TYPE_FOREIGN`.
- `register_foreign_asset_with_id` and `link_existing_asset` reject non-foreign masks.
- `migrate_location_key` moves the mapping only; it MUST NOT collide with an existing mapping.

## Migration Flow (Location Key Upgrade)
1) Identify the old and new XCM `Location` (e.g., XCM version bump or junction change).
2) Ensure the target mapping slot is free (`ForeignAssetMapping` has no entry for the new key).
3) Invoke `assetRegistry.migrateLocationKey(old, new)`.
4) Verify:
   - Old key removed, new key present.
   - Event `MigrationApplied { asset_id, old_location, new_location }` emitted.

## Harness: migrate_location_key
Script: `scripts/xcm-migration-harness.sh`

Inputs (env):
- `WS_ENDPOINT` (default `ws://127.0.0.1:9944`)
- `SURI` (default `//Alice`)
- `OLD_LOCATION`, `NEW_LOCATION` (JSON or hex SCALE `VersionedLocation`)
- `DRY_RUN=true` to inspect weight/fee only

JSON example:
- `OLD_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2000}]}}'`
- `NEW_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2001}]}}'`

Usage:
```bash
OLD_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2000}]}}' \
NEW_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2001}]}}' \
WS_ENDPOINT=ws://127.0.0.1:9944 \
SURI=//Alice \
./scripts/xcm-migration-harness.sh
```

Dry-run:
```bash
DRY_RUN=true \
OLD_LOCATION=0x030101020000 \
NEW_LOCATION=0x030101020001 \
./scripts/xcm-migration-harness.sh
```

Expected:
- On success: `MigrationApplied` event with the same `asset_id` and updated `Location`.

## Harness: E2E XCM (relay → para, sibling → para)
Script: `scripts/xcm-e2e-harness.sh`

Inputs (env):
- `RELAY_WS` (default `ws://127.0.0.1:9944`)
- `PARA_WS` (default `ws://127.0.0.1:9188`)
- `SIBLING_WS` (optional; skips sibling test if unset)
- `PARA_ID` (default `2000`)
- `BENEFICIARY_SURI` (default `//Alice`)
- `AMOUNT` (default `1000000000000` = 1 unit @ 12dp)
- `ASSET_ID`, `FEE_ASSET_ID` (default `0` for relay native)
- `DRY_RUN=true` to inspect fee/weight

Usage (relay→para only):
```bash
RELAY_WS=ws://127.0.0.1:9944 \
PARA_WS=ws://127.0.0.1:9188 \
PARA_ID=2000 \
BENEFICIARY_SURI=//Alice \
./scripts/xcm-e2e-harness.sh
```

Add sibling:
```bash
SIBLING_WS=ws://127.0.0.1:9990 \
./scripts/xcm-e2e-harness.sh
```

Validations:
- Events on relay/sibling for reserve transfer.
- Balance credited on para for the beneficiary.
- Use `DRY_RUN=true` to check fees before live send.

## Chain Spec: paseo-local raw
If the binary lacks `paseo-local`, generate raw spec:
- Script: `scripts/generate-paseo-local-raw.sh`
- Output: `template/chain-specs/paseo-local-raw.json` (override via `OUT`)
- Prereq: `polkadot-omni-node` (auto-downloaded if missing)

Usage:
```bash
./scripts/generate-paseo-local-raw.sh
```

## Operational Checklist
- ✔ Mask invariant holds (0xF…).
- ✔ New mapping slot empty before migration.
- ✔ `migrate_location_key` executed; event observed.
- ✔ E2E relay→para (and sibling→para if configured) transfers succeed.
- ✔ For zombienet/pop: have `paseo-local-raw.json` available if preset is absent.
- ✔ Update ROADMAP/CHANGELOG after successful run (note migrated keys and harness results).

## Common Pitfalls
- Using non-foreign IDs (`TYPE_STD`, etc.) with foreign APIs → rejected.
- Target mapping already occupied → `AssetAlreadyRegistered`.
- Missing metadata on linked assets: set metadata before `link_existing_asset` if events are expected to carry symbols.
- No coretime on relay: ensure core allocation or on-demand purchase before testing block production.
