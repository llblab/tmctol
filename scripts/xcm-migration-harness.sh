#!/usr/bin/env bash
# XCM migration harness for pallet-asset-registry::migrate_location_key
# Uses polkadot-js API to submit the extrinsic and prints the resulting events.

set -euo pipefail

# Inputs (environment variables):
#   WS_ENDPOINT         WebSocket endpoint (default: ws://127.0.0.1:9944)
#   SURI                Signer seed/URI (default: //Alice)
#   OLD_LOCATION        XCM location to migrate FROM (JSON or hex SCALE)
#   NEW_LOCATION        XCM location to migrate TO   (JSON or hex SCALE)
#   DRY_RUN=true        If set, only builds the extrinsic and prints details.
#
# JSON examples:
#   OLD_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2000}]}}'
#   NEW_LOCATION='{"parents":1,"interior":{"X1":[{"Parachain":2001}]}}'
#
# Hex example (already SCALE-encoded VersionedLocation):
#   OLD_LOCATION=0x030101020000
#   NEW_LOCATION=0x030101020001

WS_ENDPOINT="${WS_ENDPOINT:-ws://127.0.0.1:9944}"
SURI="${SURI:-//Alice}"
OLD_LOCATION="${OLD_LOCATION:-}"
NEW_LOCATION="${NEW_LOCATION:-}"
DRY_RUN="${DRY_RUN:-false}"

if [[ -z "$OLD_LOCATION" || -z "$NEW_LOCATION" ]]; then
  echo "Usage: OLD_LOCATION=<json|hex> NEW_LOCATION=<json|hex> [WS_ENDPOINT=...] [SURI=...] [DRY_RUN=true] $0"
  exit 1
fi

node --input-type=module - <<'EOF'
import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';

const env = process.env;
const WS_ENDPOINT = env.WS_ENDPOINT;
const SURI = env.SURI;
const DRY_RUN = env.DRY_RUN === 'true';
const OLD_LOCATION = env.OLD_LOCATION;
const NEW_LOCATION = env.NEW_LOCATION;

function parseLocation(api, value) {
  if (!value) throw new Error('Missing location value');
  const trimmed = value.trim();
  if (trimmed.startsWith('0x')) {
    return api.createType('XcmVersionedLocation', trimmed);
  }
  try {
    const json = JSON.parse(trimmed);
    return api.createType('XcmVersionedLocation', { V4: json });
  } catch (e) {
    throw new Error(`Failed to parse location. Provided: ${trimmed}. Error: ${e.message}`);
  }
}

async function main() {
  console.log(`Connecting to ${WS_ENDPOINT} ...`);
  const api = await ApiPromise.create({ provider: new WsProvider(WS_ENDPOINT) });

  const oldLoc = parseLocation(api, OLD_LOCATION);
  const newLoc = parseLocation(api, NEW_LOCATION);

  console.log('Old Location:', oldLoc.toHuman());
  console.log('New Location:', newLoc.toHuman());

  const keyring = new Keyring({ type: 'sr25519' });
  const signer = keyring.addFromUri(SURI);

  const tx = api.tx.assetRegistry.migrateLocationKey(oldLoc, newLoc);

  if (DRY_RUN) {
    const info = await tx.paymentInfo(signer);
    console.log('--- DRY RUN ---');
    console.log('Call data:', tx.method.toHex());
    console.log('Partial Fee:', info.partialFee.toHuman());
    console.log('Weight:', info.weight.toHuman());
    await api.disconnect();
    return;
  }

  console.log('Submitting extrinsic...');
  const unsub = await tx.signAndSend(signer, ({ status, dispatchError, events }) => {
    if (dispatchError) {
      if (dispatchError.isModule) {
        const decoded = api.registry.findMetaError(dispatchError.asModule);
        console.error(`Error: ${decoded.section}.${decoded.name}: ${decoded.docs.join(' ')}`);
      } else {
        console.error(`Error: ${dispatchError.toString()}`);
      }
      unsub();
      process.exit(1);
    }

    console.log(`Status: ${status.toString()}`);
    if (status.isInBlock || status.isFinalized) {
      console.log('Events:');
      events.forEach(({ event }) => {
        console.log(` - ${event.section}.${event.method}`, event.data.toHuman());
      });
      if (status.isFinalized) {
        console.log(`Finalized at block hash: ${status.asFinalized.toHex()}`);
        unsub();
        process.exit(0);
      }
    }
  });
}

main().catch((err) => {
  console.error('Migration failed:', err);
  process.exit(1);
});
EOF
