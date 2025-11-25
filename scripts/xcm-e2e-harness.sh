#!/usr/bin/env bash
# XCM e2e harness: relay→para and sibling→para reserve transfers
# Requirements: node >=18 with @polkadot/api installed; running relay, target para, optional sibling.

set -euo pipefail

# Config (override via env)
RELAY_WS="${RELAY_WS:-ws://127.0.0.1:9944}"
PARA_WS="${PARA_WS:-ws://127.0.0.1:9188}"
SIBLING_WS="${SIBLING_WS:-}"          # if empty, sibling→para test is skipped
PARA_ID="${PARA_ID:-2000}"
BENEFICIARY_SURI="${BENEFICIARY_SURI:-//Alice}"
AMOUNT="${AMOUNT:-1000000000000}"      # 1 unit at 12 decimals
FEE_ASSET_ID="${FEE_ASSET_ID:-0}"     # relay native asset ID
ASSET_ID="${ASSET_ID:-0}"             # asset to transfer (relay native)
DRY_RUN="${DRY_RUN:-false}"

node --input-type=module - <<'EOF'
import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';

const env = process.env;
const RELAY_WS = env.RELAY_WS;
const PARA_WS = env.PARA_WS;
const SIBLING_WS = env.SIBLING_WS;
const PARA_ID = Number(env.PARA_ID);
const BENEFICIARY_SURI = env.BENEFICIARY_SURI;
const AMOUNT = BigInt(env.AMOUNT);
const ASSET_ID = Number(env.ASSET_ID);
const FEE_ASSET_ID = Number(env.FEE_ASSET_ID);
const DRY_RUN = env.DRY_RUN === 'true';

const keyring = new Keyring({ type: 'sr25519' });
const beneficiary = keyring.addFromUri(BENEFICIARY_SURI);

const relayAsset = { id: { Concrete: { parents: 0, interior: 'Here' } }, fun: { Fungible: AMOUNT } };
const feeAsset = { id: { Concrete: { parents: 0, interior: 'Here' } }, fun: { Fungible: AMOUNT / 10n } }; // fee buffer

function mkBeneficiary(api) {
  return {
    parents: 0,
    interior: {
      X1: [
        {
          AccountId32: {
            network: 'Any',
            id: beneficiary.publicKey,
          },
        },
      ],
    },
  };
}

async function sendReserveTransfer(api, originLabel, dest, assets) {
  const tx = api.tx.polkadotXcm.reserveTransferAssets(dest, mkBeneficiary(api), assets, 0);
  if (DRY_RUN) {
    const info = await tx.paymentInfo(beneficiary);
    console.log(`[${originLabel}] DRY RUN fee:`, info.partialFee.toHuman(), 'weight:', info.weight.toHuman());
    return;
  }
  return new Promise((resolve, reject) => {
    tx.signAndSend(beneficiary, ({ status, dispatchError, events }) => {
      if (dispatchError) {
        if (dispatchError.isModule) {
          const decoded = api.registry.findMetaError(dispatchError.asModule);
          console.error(`[${originLabel}] Error: ${decoded.section}.${decoded.name} ${decoded.docs.join(' ')}`);
        } else {
          console.error(`[${originLabel}] Error: ${dispatchError.toString()}`);
        }
        reject(dispatchError);
        return;
      }
      console.log(`[${originLabel}] Status: ${status.toString()}`);
      if (status.isInBlock || status.isFinalized) {
        events.forEach(({ event }) => {
          console.log(`[${originLabel}] Event ${event.section}.${event.method}`, event.data.toHuman());
        });
        if (status.isFinalized) {
          resolve();
        }
      }
    });
  });
}

async function main() {
  // Relay → Para
  console.log(`Connecting relay: ${RELAY_WS}`);
  const relayApi = await ApiPromise.create({ provider: new WsProvider(RELAY_WS) });
  const destPara = { parents: 0, interior: { X1: [{ Parachain: PARA_ID }] } };
  console.log(`[relay→para] Transfer asset ${ASSET_ID} amount ${AMOUNT} to para ${PARA_ID}`);
  await sendReserveTransfer(relayApi, 'relay→para', destPara, [relayAsset, feeAsset]);
  await relayApi.disconnect();

  // Sibling → Para (optional)
  if (SIBLING_WS) {
    console.log(`Connecting sibling: ${SIBLING_WS}`);
    const siblingApi = await ApiPromise.create({ provider: new WsProvider(SIBLING_WS) });
    const destParaFromSibling = { parents: 1, interior: { X1: [{ Parachain: PARA_ID }] } };
    console.log(`[sibling→para] Transfer asset ${ASSET_ID} amount ${AMOUNT} to para ${PARA_ID}`);
    await sendReserveTransfer(siblingApi, 'sibling→para', destParaFromSibling, [relayAsset, feeAsset]);
    await siblingApi.disconnect();
  } else {
    console.log('SIBLING_WS not set; skipping sibling→para test.');
  }

  // Optionally fetch para balance after transfer
  if (PARA_WS) {
    const paraApi = await ApiPromise.create({ provider: new WsProvider(PARA_WS) });
    const balance = await paraApi.query.system.account(beneficiary.address);
    console.log(`[para] Beneficiary free balance:`, balance.data.free.toHuman());
    await paraApi.disconnect();
  }
}

main().catch((err) => {
  console.error('Harness failed:', err);
  process.exit(1);
});
EOF
