#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

phase_separator() {
    local phase="$1"
    echo -e "\n${CYAN}────────────────────────────────────────────────────────────────${NC}"
    echo -e "${CYAN}  Phase: $phase${NC}"
    echo -e "${CYAN}────────────────────────────────────────────────────────────────${NC}\n"
}

# Run step with timing
run_step() {
    local step="$1"
    local script="$2"
    local description="$3"

    phase_separator "$step: $description"

    if [[ ! -x "$SCRIPT_DIR/$script" ]]; then
        log_error "Script not found or not executable: $script"
        exit 1
    fi

    log_info "Executing $script..."
    local start_time=$(date +%s)
    "$SCRIPT_DIR/$script"
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    log_success "Step completed in ${duration} seconds"
}

# Display network endpoints
display_endpoints() {
    echo ""
    log_info "Network endpoints (once started):"
    echo "  Relay Chain (Alice): ws://localhost:9944"
    echo "  Relay Chain (Bob):   ws://localhost:9955"
    echo "  Parachain (Charlie): ws://localhost:9988"
    echo ""
    echo "  Polkadot.js Apps:"
    echo "    Relay: https://polkadot.js.org/apps/?rpc=ws://localhost:9944"
    echo "    Para:  https://polkadot.js.org/apps/?rpc=ws://localhost:9988"
}

main() {
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║           TMCTOL Parachain - Local Zombienet Setup           ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    log_info "Starting orchestrated setup and run process"
    echo ""

    # Check skip flags
    local SKIP_DOWNLOAD="${SKIP_DOWNLOAD:-}"
    local SKIP_TOOLS="${SKIP_TOOLS:-}"
    local SKIP_BUILD="${SKIP_BUILD:-}"
    local SKIP_CHAINSPEC="${SKIP_CHAINSPEC:-}"

    export CHAIN_TYPE="${CHAIN_TYPE:-Development}"

    # Step 1: Download binaries
    if [[ -z "$SKIP_DOWNLOAD" ]]; then
        run_step "1/5" "01-download-binaries.sh" "Download Polkadot binaries"
    else
        log_warning "Skipping step 1: Download binaries (SKIP_DOWNLOAD set)"
    fi

    # Step 2: Install tools
    if [[ -z "$SKIP_TOOLS" ]]; then
        run_step "2/5" "02-install-tools.sh" "Install cargo tools"
    else
        log_warning "Skipping step 2: Install tools (SKIP_TOOLS set)"
    fi

    # Step 3: Build runtime
    if [[ -z "$SKIP_BUILD" ]]; then
        run_step "3/5" "03-build-runtime.sh" "Build parachain runtime"
    else
        log_warning "Skipping step 3: Build runtime (SKIP_BUILD set)"
    fi

    # Step 4: Generate chain spec
    if [[ -z "$SKIP_CHAINSPEC" ]]; then
        run_step "4/5" "04-generate-chain-spec.sh" "Generate chain spec"
    else
        log_warning "Skipping step 4: Generate chain spec (SKIP_CHAINSPEC set)"
    fi

    # Check dependencies before spawning
    check_spawn_dependencies

    # Step 5: Spawn Zombienet
    phase_separator "5/5: Spawn Zombienet"
    display_endpoints
    log_info "Starting network (Ctrl+C to stop)..."
    echo ""

    exec "$SCRIPT_DIR/05-spawn-zombienet.sh"
}

# Check dependencies for spawning zombienet
check_spawn_dependencies() {
    if [[ -d "$BIN_DIR" ]]; then
        export PATH="$BIN_DIR:$PATH"
    fi

    local missing_deps=()
    for cmd in polkadot polkadot-omni-node zombienet; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing dependencies for spawning zombienet: ${missing_deps[*]}"
        echo "  Run steps 1 and/or 2 first, or ensure SKIP_DOWNLOAD and SKIP_TOOLS are not set"
        exit 1
    fi
    log_info "Spawn dependencies verified"
}

# Run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
