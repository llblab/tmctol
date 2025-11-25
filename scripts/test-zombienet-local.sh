#!/usr/bin/env bash
# Local pop-based network launcher: paseo-local relay + TMCTOL para 2000
# Prereqs: pop CLI available in PATH. Downloads polkadot-omni-node if missing.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

POLKADOT_VERSION="${POLKADOT_VERSION:-stable2512}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMPLATE_DIR="$PROJECT_ROOT/template"
NETWORK_FILE="$TEMPLATE_DIR/network.toml"
OMNI_NODE="$TEMPLATE_DIR/polkadot-omni-node"

echo -e "${YELLOW}==> Checking prerequisites${NC}"
if ! command -v pop >/dev/null 2>&1; then
  echo -e "${RED}pop CLI not found in PATH. Install pop CLI and retry.${NC}"
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo -e "${RED}cargo not found in PATH. Install Rust toolchain and retry.${NC}"
  exit 1
fi

echo -e "${YELLOW}==> Building runtime (release)${NC}"
cd "$TEMPLATE_DIR"
cargo build --package tmctol-runtime --release

echo -e "${YELLOW}==> Ensuring polkadot-omni-node binary${NC}"
if [ ! -x "$OMNI_NODE" ]; then
  echo "Downloading polkadot-omni-node $POLKADOT_VERSION ..."
  "$PROJECT_ROOT/scripts/download-omni-node.sh" --version "$POLKADOT_VERSION" --output "$TEMPLATE_DIR"
fi

echo -e "${YELLOW}==> Generating chain spec (plain)${NC}"
CHAIN_SPEC_PATH="$TEMPLATE_DIR/target/release/tmctol-spec.json"
RUNTIME_WASM="$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.wasm"
mkdir -p "$(dirname "$CHAIN_SPEC_PATH")"
"$OMNI_NODE" chain-spec-builder \
  --chain-spec-path "$CHAIN_SPEC_PATH" \
  create \
  --runtime "$RUNTIME_WASM" \
  --para-id 2000 \
  --chain-id tmctol-dev \
  -t development \
  --relay-chain paseo-local \
  named-preset development

echo -e "${YELLOW}==> Launching network via pop (paseo-local relay + TMCTOL para 2000)${NC}"
echo "Config: $NETWORK_FILE"
set +e
pop up "$NETWORK_FILE"
EXIT_CODE=$?
set -e

if [ $EXIT_CODE -ne 0 ]; then
  echo -e "${RED}Network launch failed (exit code $EXIT_CODE). Check pop logs above.${NC}"
  exit $EXIT_CODE
fi

echo -e "${GREEN}Network launched successfully.${NC}"

# Auto-purchase on-demand coretime if RELAY_ENDPOINT is provided
if [ -n "${RELAY_ENDPOINT:-}" ]; then
  echo -e "${YELLOW}==> Purchasing on-demand coretime for para 2000 via $RELAY_ENDPOINT${NC}"
  if ! pop call chain \
    --url "$RELAY_ENDPOINT" \
    --pallet OnDemand \
    --function place_order \
    --args "max_amount=${CORETIME_MAX_AMOUNT:-10000000},para_id=2000" \
    --suri "${CORETIME_SURI:-//Alice}" \
    --skip-confirm; then
    echo -e "${RED}Coretime purchase failed. You may need to retry manually.${NC}"
  else
    echo -e "${GREEN}Coretime order placed (check events for OnDemand::OnDemandOrderPlaced).${NC}"
  fi
else
  echo -e "${YELLOW}Reminder:${NC} set RELAY_ENDPOINT to auto-buy coretime or run manually:"
  echo 'pop call chain --pallet OnDemand --function place_order --args "max_amount=10000000,para_id=2000" --suri //Alice --url <relay_endpoint>'
fi

echo -e "${GREEN}Press Ctrl+C to stop the network when done.${NC}"
