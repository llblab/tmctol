# Foreign Asset Migration & Bitmask Invariants

## Scope

Operational guide for migrating XCM `Location -> AssetId` keys, validating the 0xF… foreign namespace, and performing migration and E2E XCM validation steps.

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

1. Identify the old and new XCM `Location` (e.g., XCM version bump or junction change).
2. Ensure the target mapping slot is free (`ForeignAssetMapping` has no entry for the new key).
3. Invoke `assetRegistry.migrateLocationKey(old, new)`.
4. Verify:
   - Old key removed, new key present.
   - Event `MigrationApplied { asset_id, old_location, new_location }` emitted.

## Migration execution (manual)

The previous migration harness script has been removed. Execute the migration manually via Polkadot.js Apps or your own client.

Required inputs:

- WS endpoint to the parachain node.
- Sudo or governance origin authorized to call `assetRegistry.migrateLocationKey`.
- `OLD_LOCATION`, `NEW_LOCATION` as `VersionedLocation` (JSON or SCALE hex).

Manual flow:

1. Open Polkadot.js Apps → Extrinsics.
2. Select `assetRegistry` → `migrateLocationKey(old, new)`.
3. Submit with authorized origin.
4. Verify:
   - Old key removed, new key present.
   - Event `MigrationApplied { asset_id, old_location, new_location }` emitted.

## E2E XCM validation (manual)

The previous E2E harness script has been removed. Run the flow via Polkadot.js Apps or your own client.

Suggested manual flow:

1. On relay (or sibling), submit a reserve transfer to the parachain with:
   - `dest` pointing to the parachain.
   - `beneficiary` as the target account on the para.
   - `assets` containing the intended asset and fee asset.
2. Observe relay/sibling events for the reserve transfer.
3. On the parachain, verify the beneficiary balance increased and XCM events were emitted.

Validation checklist:

- Relay/sibling events show the transfer.
- Parachain balance credited.
- XCM events present on the parachain.

## Chain Spec (raw)

Generate chain specs using the available script:

- Script: `scripts/04-generate-chain-spec.sh`
- Output: `template/chain-specs/*.json` (see script output)
- Prereq: `polkadot-omni-node` (auto-downloaded if missing)

Usage:

```bash
./scripts/04-generate-chain-spec.sh
```

## Operational Checklist

- ✔ Mask invariant holds (0xF…).
- ✔ New mapping slot empty before migration.
- ✔ `migrate_location_key` executed via authorized extrinsic; event observed.
- ✔ E2E relay→para (and sibling→para if configured) transfers succeed via manual submission.
- ✔ For zombienet/pop: have the generated raw chain spec available if preset is absent.
- ✔ Update ROADMAP/CHANGELOG after successful run (note migrated keys and harness results).

## Common Pitfalls

- Using non-foreign IDs (`TYPE_STD`, etc.) with foreign APIs → rejected.
- Target mapping already occupied → `AssetAlreadyRegistered`.
- Missing metadata on linked assets: set metadata before `link_existing_asset` if events are expected to carry symbols.
- No coretime on relay: ensure core allocation or on-demand purchase before testing block production.
