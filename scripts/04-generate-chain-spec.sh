#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

# CHAIN_TYPE controls the generated chain spec profile:
#   Development  (default) — well-known dev accounts visible in Polkadot JS
#   Local                  — local testnet with dev accounts
#   Live                   — production, no dev accounts
CHAIN_TYPE="${CHAIN_TYPE:-Development}"
PARA_ID="${PARA_ID:-2000}"
RELAY_CHAIN="${RELAY_CHAIN:-rococo-local}"

resolve_chain_profile() {
    case "$CHAIN_TYPE" in
        Development)
            PRESET="development"
            CHAIN_NAME="TMCTOL Development"
            CHAIN_ID="tmctol-dev"
            ;;
        Local)
            PRESET="local_testnet"
            CHAIN_NAME="TMCTOL Local Testnet"
            CHAIN_ID="tmctol-local"
            ;;
        Live)
            PRESET="development"
            CHAIN_NAME="TMCTOL"
            CHAIN_ID="tmctol"
            ;;
        *)
            log_error "Unknown CHAIN_TYPE: $CHAIN_TYPE (expected: Development, Local, Live)"
            exit 1
            ;;
    esac
}

# Dependency check
check_dependencies() {
    if ! command -v chain-spec-builder &> /dev/null; then
        log_error "chain-spec-builder not found. Run 02-install-tools.sh first."
        exit 1
    fi
    if ! command -v python3 &> /dev/null; then
        log_error "python3 is required for chain spec patching."
        exit 1
    fi
    log_info "Dependencies check passed"
}

# Generate chain spec
generate_chain_spec() {
    local wasm_path="$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.compact.compressed.wasm"
    local chain_spec_path="$TEMPLATE_DIR/chain_spec.json"

    log_info "Generating chain specification"
    echo "  Chain type: $CHAIN_TYPE"
    echo "  Preset: $PRESET"
    echo "  Para ID: $PARA_ID"
    echo "  Relay chain: $RELAY_CHAIN"
    echo "  WASM: $wasm_path"
    echo ""

    if [[ ! -f "$wasm_path" ]]; then
        log_error "Runtime WASM not found. Run 03-build-runtime.sh first."
        echo "  Expected: $wasm_path"
        exit 1
    fi

    cd "$TEMPLATE_DIR"

    chain-spec-builder create \
        -c "$RELAY_CHAIN" \
        -p "$PARA_ID" \
        -r "$wasm_path" \
        named-preset "$PRESET"

    if [[ -f "$TEMPLATE_DIR/chain_spec.json" ]] && [[ "$TEMPLATE_DIR/chain_spec.json" != "$chain_spec_path" ]]; then
        mv "$TEMPLATE_DIR/chain_spec.json" "$chain_spec_path"
    fi

    patch_chain_spec "$chain_spec_path"

    log_success "Chain specification generated"
}

patch_chain_spec() {
    local spec_path="$1"
    log_info "Patching chain spec metadata (chainType=$CHAIN_TYPE, name=$CHAIN_NAME, id=$CHAIN_ID)"

    python3 -c "
import json, sys
with open(sys.argv[1], 'r') as f:
    spec = json.load(f)
spec['chainType'] = sys.argv[2]
spec['name'] = sys.argv[3]
spec['id'] = sys.argv[4]
with open(sys.argv[1], 'w') as f:
    json.dump(spec, f, indent=2)
    f.write('\n')
" "$spec_path" "$CHAIN_TYPE" "$CHAIN_NAME" "$CHAIN_ID"
}

# Verify output
verify_output() {
    local chain_spec_path="$TEMPLATE_DIR/chain_spec.json"

    if [[ -f "$chain_spec_path" ]]; then
        local size=$(du -h "$chain_spec_path" | cut -f1)
        log_success "Chain spec file verified"
        echo "  Path: $chain_spec_path"
        echo "  Size: $size"
        echo "  Chain type: $CHAIN_TYPE"
        echo "  Name: $CHAIN_NAME"
        echo "  ID: $CHAIN_ID"
        echo "  Para ID: $PARA_ID"
        echo "  Relay chain: $RELAY_CHAIN"
    else
        log_error "Chain specification not generated"
        exit 1
    fi
}

main() {
    log_info "Starting chain spec generation"
    echo ""

    resolve_chain_profile
    check_dependencies
    generate_chain_spec
    verify_output

    log_success "Chain spec generation completed successfully"
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
