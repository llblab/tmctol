#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

# Stop running processes
stop_processes() {
    log_info "Stopping running processes..."

    local stopped_processes=()

    if pgrep -f "zombienet" &>/dev/null; then
        log_info "Stopping zombienet processes..."
        pkill -f "zombienet" || true
        stopped_processes+=("zombienet")
    fi

    if pgrep -f "polkadot" &>/dev/null; then
        log_info "Stopping polkadot processes..."
        pkill -f "polkadot" || true
        stopped_processes+=("polkadot")
    fi

    if [[ ${#stopped_processes[@]} -gt 0 ]]; then
        log_success "Stopped processes: ${stopped_processes[*]}"
    else
        log_info "No processes to stop"
    fi
}

# Clean zombienet temp directories
clean_zombienet_temp() {
    log_info "Removing zombienet temp directories..."

    local removed_count=0
    while IFS= read -r -d '' dir; do
        if rm -rf "$dir" 2>/dev/null; then
            ((removed_count++))
        fi
    done < <(find /tmp -maxdepth 1 -type d -name "zombie-*" -print0 2>/dev/null || true)

    if [[ $removed_count -gt 0 ]]; then
        log_success "Removed $removed_count zombienet temp directories"
    else
        log_info "No temp directories to remove"
    fi
}

# Clean generated chain spec
clean_chain_spec() {
    if [[ -f "$TEMPLATE_DIR/chain_spec.json" ]]; then
        log_info "Removing generated chain spec..."
        rm -f "$TEMPLATE_DIR/chain_spec.json"
        log_success "Chain spec removed"
    else
        log_info "No chain spec to remove"
    fi
}

# Clean build artifacts
clean_build_artifacts() {
    log_info "Removing build artifacts..."
    if [[ -d "$TEMPLATE_DIR/target" ]]; then
        rm -rf "$TEMPLATE_DIR/target"
        log_success "Build artifacts removed"
    else
        log_info "No build artifacts to remove"
    fi
}

# Clean downloaded binaries
clean_binaries() {
    log_info "Removing downloaded binaries..."
    if [[ -d "$BIN_DIR" ]]; then
        rm -rf "$BIN_DIR"
        log_success "Downloaded binaries removed"
    else
        log_info "No binaries to remove"
    fi
}

main() {
    log_info "Starting cleanup process"
    echo ""

    stop_processes
    clean_zombienet_temp
    clean_chain_spec

    if [[ "${CLEAN_BUILD:-}" == "1" ]]; then
        clean_build_artifacts
    fi

    if [[ "${CLEAN_BINARIES:-}" == "1" ]]; then
        clean_binaries
    fi

    log_success "Cleanup completed successfully"

    if [[ -z "${CLEAN_BUILD:-}" ]] || [[ -z "${CLEAN_BINARIES:-}" ]]; then
        echo ""
        log_info "Optional clean options:"
        echo "  CLEAN_BUILD=1 $0      # Also remove build artifacts"
        echo "  CLEAN_BINARIES=1 $0   # Also remove downloaded binaries"
    fi
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
