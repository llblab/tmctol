#!/usr/bin/env bash
# Generate paseo-local raw chain spec for zombienet/pop when the binary lacks the preset.
#
# Outputs:
#   template/chain-specs/paseo-local-raw.json (override via OUT)
#
# Env vars:
#   POLKADOT_VERSION   polkadot-omni-node version (default: stable2509-2)
#   OUT                output path (default: $PROJECT_ROOT/template/chain-specs/paseo-local-raw.json)
#
# Usage:
#   ./scripts/generate-paseo-local-raw.sh
#   OUT=/tmp/paseo-raw.json ./scripts/generate-paseo-local-raw.sh
#
# Prereqs:
#   - bash, curl
#   - rust toolchain not required (relay spec only)
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

POLKADOT_VERSION="${POLKADOT_VERSION:-stable2509-2}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMPLATE_DIR="$PROJECT_ROOT/template"
OUT_BASENAME="${OUT_BASENAME:-paseo-local-raw.json}"
OUT="${OUT:-$TEMPLATE_DIR/$OUT_BASENAME}"
CHAIN_SPEC_PLAIN_BASENAME="${CHAIN_SPEC_PLAIN_BASENAME:-paseo-local-plain.json}"
OMNI_NODE="$TEMPLATE_DIR/polkadot-omni-node"

# Remove any stray chain_spec.json created in the project root from prior runs
rm -f "$PROJECT_ROOT/chain_spec.json"

echo -e "${YELLOW}==> Generating paseo-local raw chain spec${NC}"
echo "Output: $OUT"
mkdir -p "$(dirname "$OUT")"

# Ensure binary
if [ ! -x "$OMNI_NODE" ]; then
  echo -e "${YELLOW}Downloading polkadot-omni-node ${POLKADOT_VERSION}...${NC}"
  "$PROJECT_ROOT/scripts/download-omni-node.sh" --version "$POLKADOT_VERSION" --output "$TEMPLATE_DIR"
fi

# Build runtime wasm if missing
RUNTIME_WASM="$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.wasm"
if [ ! -f "$RUNTIME_WASM" ]; then
  echo -e "${YELLOW}==> Building runtime wasm (release)${NC}"
  (cd "$TEMPLATE_DIR" && cargo build --package tmctol-runtime --release)
fi

# Generate raw spec via chain-spec-builder (development preset)
CHAIN_ID="${CHAIN_ID:-tmctol-dev}"
PARA_ID="${PARA_ID:-2000}"
RELAY_CHAIN="${RELAY_CHAIN:-paseo-local}"
TEMPLATE_PRESET="${TEMPLATE_PRESET:-development}"
CHAIN_SPEC_PLAIN="${CHAIN_SPEC_PLAIN:-$TEMPLATE_DIR/$CHAIN_SPEC_PLAIN_BASENAME}"

echo -e "${YELLOW}==> Creating chain spec (preset: ${TEMPLATE_PRESET})${NC}"
(
  cd "$TEMPLATE_DIR"
  "$OMNI_NODE" chain-spec-builder \
    --chain-spec-path "$CHAIN_SPEC_PLAIN_BASENAME" \
    create \
    --runtime "$RUNTIME_WASM" \
    --para-id "$PARA_ID" \
    --chain-id "$CHAIN_ID" \
    -t "$TEMPLATE_PRESET" \
    --relay-chain "$RELAY_CHAIN" \
    named-preset "$TEMPLATE_PRESET"
)

echo -e "${YELLOW}==> Converting to raw${NC}"
(
  cd "$TEMPLATE_DIR"
  "$OMNI_NODE" chain-spec-builder convert-to-raw "$CHAIN_SPEC_PLAIN_BASENAME" > "$OUT"
)

echo -e "${GREEN}✓ Done.${NC}"
echo "Saved to: $OUT"
