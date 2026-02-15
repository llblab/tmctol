#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

ZOMBIENET_CONFIG="${ZOMBIENET_CONFIG:-$TEMPLATE_DIR/zombienet.toml}"

# Dependency check
check_dependencies() {
    local missing_deps=()

    for cmd in polkadot polkadot-omni-node zombienet; do
        if ! command -v "$cmd" &>/dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        echo "  Run 01-download-binaries.sh and/or 02-install-tools.sh first"
        exit 1
    fi

    log_info "Dependencies check passed"
}

# Verify prerequisites
verify_prerequisites() {
    if [[ ! -f "$ZOMBIENET_CONFIG" ]]; then
        log_error "Zombienet config not found: $ZOMBIENET_CONFIG"
        exit 1
    fi

    if [[ ! -f "$TEMPLATE_DIR/chain_spec.json" ]]; then
        log_error "Chain spec not found. Run 04-generate-chain-spec.sh first"
        echo "  Expected: $TEMPLATE_DIR/chain_spec.json"
        exit 1
    fi

    log_info "Prerequisites verified"
}

# Setup PATH if needed
setup_path() {
    if [[ -d "$BIN_DIR" ]]; then
        export PATH="$BIN_DIR:$PATH"
        log_info "Added $BIN_DIR to PATH"
    fi
}

# Spawn network
spawn_network() {
    log_info "Spawning Zombienet network"
    echo "  Config: $ZOMBIENET_CONFIG"
    echo "  Chain spec: $TEMPLATE_DIR/chain_spec.json"
    echo "  polkadot: $(command -v polkadot)"
    echo "  polkadot-omni-node: $(command -v polkadot-omni-node)"
    echo "  zombienet: $(command -v zombienet)"
    echo ""

    log_info "Starting network (Ctrl+C to stop)..."
    echo ""

    cd "$TEMPLATE_DIR"
    exec zombienet --provider native spawn "$ZOMBIENET_CONFIG"
}

main() {
    log_info "Starting Zombienet spawn process"
    echo ""

    check_dependencies
    verify_prerequisites
    setup_path

    spawn_network
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
