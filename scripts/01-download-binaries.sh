#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

POLKADOT_VERSION="${POLKADOT_VERSION:-polkadot-stable2512-1}"
RELEASE_URL="https://github.com/paritytech/polkadot-sdk/releases/download/${POLKADOT_VERSION}"

BINARIES=(
    "polkadot"
    "polkadot-execute-worker"
    "polkadot-prepare-worker"
    "polkadot-omni-node"
)

# Dependency check
check_dependencies() {
    if ! command -v curl &> /dev/null; then
        log_error "curl is required but not found. Install curl and try again."
        exit 1
    fi
    log_info "Dependencies check passed"
}

main() {
    log_info "Starting binary download process"
    echo "  Version: $POLKADOT_VERSION"
    echo "  Target dir: $BIN_DIR"
    echo ""

    check_dependencies

    mkdir -p "$BIN_DIR"

    for binary in "${BINARIES[@]}"; do
        local binary_path="$BIN_DIR/$binary"
        if [[ -x "$binary_path" ]]; then
            log_warning "$binary already exists, skipping download"
            continue
        fi

        log_info "Downloading $binary..."
        if curl -fsSL "${RELEASE_URL}/${binary}" -o "$binary_path"; then
            chmod +x "$binary_path"
            log_success "$binary downloaded and made executable"
        else
            log_error "Failed to download $binary"
            exit 1
        fi
    done

    log_success "All binaries downloaded successfully"
    echo ""
    log_info "To add binaries to PATH:"
    echo "  export PATH=\"\$PATH:$BIN_DIR\""
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
