#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

# Dependency check
check_dependencies() {
    if ! command -v rustc &> /dev/null; then
        log_error "Rust toolchain not found. Install Rust and try again."
        exit 1
    fi
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Install Rust toolchain and try again."
        exit 1
    fi
    log_info "Dependencies check passed"
}

# Setup WASM target
setup_wasm_target() {
    log_info "Checking WASM target..."
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        log_info "Installing wasm32-unknown-unknown target..."
        rustup target add wasm32-unknown-unknown
        log_success "WASM target installed"
    else
        log_success "WASM target already installed"
    fi
}

# Build the runtime
build_runtime() {
    log_info "Building parachain runtime (this may take several minutes)..."

    cd "$TEMPLATE_DIR"

    local start_time=$(date +%s)
    cargo build --release -p tmctol-runtime
    local end_time=$(date +%s)
    local build_duration=$((end_time - start_time))

    log_success "Runtime build completed in ${build_duration} seconds"
}

# Verify build output
verify_build() {
    local wasm_path="$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.compact.compressed.wasm"

    if [[ -f "$wasm_path" ]]; then
        local wasm_size=$(du -h "$wasm_path" | cut -f1)
        log_success "Runtime WASM artifact verified"
        echo "  Path: $wasm_path"
        echo "  Size: $wasm_size"
    else
        log_error "Runtime WASM not found at expected path: $wasm_path"
        exit 1
    fi
}

main() {
    log_info "Starting runtime build process"
    echo ""

    check_dependencies
    setup_wasm_target
    build_runtime
    verify_build

    log_success "Runtime build process completed successfully"
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
