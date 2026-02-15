#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

# Dependency check
check_dependencies() {
    if ! command -v cargo &> /dev/null; then
        log_error "cargo is required but not found. Install Rust toolchain and try again."
        exit 1
    fi
    log_info "Dependencies check passed"
}

# Install function with better error handling
install_if_missing() {
    local cmd="$1"
    local pkg="${2:-$1}"

    if command -v "$cmd" &>/dev/null; then
        log_warning "$cmd already installed at: $(command -v "$cmd")"
    else
        log_info "Installing $pkg..."
        if cargo install "$pkg"; then
            log_success "$cmd installed successfully"
        else
            log_error "Failed to install $pkg"
            exit 1
        fi
    fi
}

main() {
    log_info "Starting cargo tools installation"
    echo ""

    check_dependencies

    install_if_missing "zombienet" "zombienet"
    install_if_missing "chain-spec-builder" "staging-chain-spec-builder"

    log_success "All cargo tools installation complete"
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
