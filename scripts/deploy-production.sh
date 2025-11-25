#!/bin/bash

# Elegant Production Deployment Automation
# Phase 4: Production Deployment & Governance
# TMCTOL Ecosystem - Production Ready

set -euo pipefail

# Color definitions for elegant output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly MAGENTA='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly NC='\033[0m' # No Color

# Configuration - Elegant and maintainable
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
readonly RUST_ROOT="$PROJECT_ROOT/template"
readonly DEPLOYMENT_DIR="$PROJECT_ROOT/deployment"
readonly CONFIG_DIR="$DEPLOYMENT_DIR/config"
readonly ARTIFACTS_DIR="$DEPLOYMENT_DIR/artifacts"
readonly LOGS_DIR="$DEPLOYMENT_DIR/logs"
readonly BACKUP_DIR="$DEPLOYMENT_DIR/backup"

# Deployment phases for clear progression
readonly DEPLOYMENT_PHASES=(
    "PREREQUISITES"
    "VALIDATION"
    "BUILD"
    "CONFIGURATION"
    "DEPLOYMENT"
    "VERIFICATION"
    "MONITORING"
)

# Elegant logging function
log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    case "$level" in
        "INFO")    echo -e "${GREEN}[INFO]${NC} $timestamp - $message" ;;
        "WARNING") echo -e "${YELLOW}[WARNING]${NC} $timestamp - $message" ;;
        "ERROR")   echo -e "${RED}[ERROR]${NC} $timestamp - $message" ;;
        "DEBUG")   echo -e "${BLUE}[DEBUG]${NC} $timestamp - $message" ;;
        "SUCCESS") echo -e "${MAGENTA}[SUCCESS]${NC} $timestamp - $message" ;;
        *)         echo -e "${CYAN}[$level]${NC} $timestamp - $message" ;;
    esac
}

# Phase separator for elegant output
phase_separator() {
    local phase="$1"
    echo -e "\n${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${MAGENTA}                   PHASE: $phase${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}\n"
}

# Error handling with elegance
error_exit() {
    log "ERROR" "$1"
    exit 1
}

# Validation functions
validate_prerequisites() {
    phase_separator "PREREQUISITES"
    log "INFO" "Validating system prerequisites..."

    # Check Rust toolchain
    if ! command -v rustc &> /dev/null; then
        error_exit "Rust toolchain not found. Install with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi

    # Check required tools
    local required_tools=("git" "jq" "curl" "protoc")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            error_exit "Required tool '$tool' not found"
        fi
    done

    # Check WASM target
    if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
        log "INFO" "Adding WASM target..."
        rustup target add wasm32-unknown-unknown
    fi

    log "SUCCESS" "All prerequisites validated"
}

validate_build_environment() {
    phase_separator "VALIDATION"
    log "INFO" "Validating build environment..."

    # Ensure we're in rust project root
    if [[ -d "$RUST_ROOT" ]]; then
        cd "$RUST_ROOT"
    else
        error_exit "Template directory not found at $RUST_ROOT"
    fi

    if [[ ! -f "Cargo.toml" ]]; then
        error_exit "Cargo.toml not found in $RUST_ROOT"
    fi

    # Check disk space (minimum 10GB)
    local available_space=$(df . | awk 'NR==2 {print $4}')
    if [[ $available_space -lt 10485760 ]]; then
        error_exit "Insufficient disk space (minimum 10GB required)"
    fi

    # Check memory (minimum 8GB)
    local total_memory=$(free -g | awk 'NR==2 {print $2}')
    if [[ $total_memory -lt 8 ]]; then
        log "WARNING" "Low memory detected (8GB recommended)"
    fi

    log "SUCCESS" "Build environment validated"
}

# Build functions
build_production_runtime() {
    phase_separator "BUILD"
    log "INFO" "Building production runtime..."

    # Clean previous builds for consistency
    log "DEBUG" "Cleaning previous builds..."
    cargo clean

    # Build with production profile
    log "INFO" "Compiling runtime with production profile (this may take several minutes)..."

    local start_time=$(date +%s)
    if ! cargo build --workspace --locked --profile production; then
        error_exit "Runtime compilation failed"
    fi
    local end_time=$(date +%s)
    local build_duration=$((end_time - start_time))

    log "SUCCESS" "Runtime built successfully in ${build_duration} seconds"

    # Verify WASM artifact
    local wasm_path="target/production/wbuild/tmctol-runtime/parachain_template_runtime.compact.compressed.wasm"
    if [[ ! -f "$wasm_path" ]]; then
        error_exit "WASM runtime artifact not found"
    fi

    local wasm_size=$(stat -c%s "$wasm_path")
    local wasm_size_mb=$(echo "scale=2; $wasm_size / 1024 / 1024" | bc)
    log "INFO" "WASM runtime size: ${wasm_size_mb} MB"

    # Create artifacts directory
    mkdir -p "$ARTIFACTS_DIR"
    cp "$wasm_path" "$ARTIFACTS_DIR/"

    log "SUCCESS" "Production runtime artifacts prepared"
}

# Configuration functions
generate_production_configs() {
    phase_separator "CONFIGURATION"
    log "INFO" "Generating production configurations..."

    mkdir -p "$CONFIG_DIR"

    # Generate production chain spec template
    cat > "$CONFIG_DIR/production-chain-spec.json" << 'EOF'
{
  "name": "TMCTOL-Production",
  "id": "tmctol_production",
  "chainType": "Live",
  "bootNodes": [],
  "telemetryEndpoints": null,
  "protocolId": "tmctol",
  "properties": {
    "ss58Format": 42,
    "tokenDecimals": 12,
    "tokenSymbol": "TMC"
  },
  "consensusEngine": null,
  "codeSubstitutes": {},
  "genesis": {
    "runtime": {
      "system": {
        "code": "0x"
      },
      "balances": {
        "balances": []
      },
      "parachainInfo": {
        "parachainId": 1000
      },
      "sudo": {
        "key": null
      }
    }
  }
}
EOF

    # Generate production node configuration
    cat > "$CONFIG_DIR/production-node.toml" << 'EOF'
[parachain]
chain = "production-chain-spec.json"

[network]
listen_addr = "/ip4/0.0.0.0/tcp/30333"
public_addr = "/ip4/0.0.0.0/tcp/30333"

[rpc]
listen_addr = "0.0.0.0:9944"

[telemetry]
endpoints = []

[log]
filter = "info,runtime=debug"

[state_pruning]
mode = "archive"
EOF

    log "SUCCESS" "Production configurations generated"
}

# Deployment functions
setup_deployment_infrastructure() {
    phase_separator "DEPLOYMENT"
    log "INFO" "Setting up deployment infrastructure..."

    # Create deployment directories
    mkdir -p "$DEPLOYMENT_DIR" "$CONFIG_DIR" "$ARTIFACTS_DIR" "$LOGS_DIR" "$BACKUP_DIR"

    # Set proper permissions
    chmod 755 "$DEPLOYMENT_DIR"
    chmod 644 "$CONFIG_DIR"/*

    log "SUCCESS" "Deployment infrastructure ready"
}

deploy_validator_node() {
    log "INFO" "Deploying validator node configuration..."

    # Generate validator keys (in production, these would be securely generated)
    cat > "$CONFIG_DIR/validator-setup.md" << 'EOF'
# Validator Node Setup Guide

## Key Generation
Generate validator keys securely:
```bash
# Generate session keys
subkey generate --scheme sr25519 --network substrate
subkey generate --scheme ed25519 --network substrate

# Keep keys secure and never commit to version control
```

## Node Configuration
1. Update production-node.toml with your public addresses
2. Set proper telemetry endpoints for monitoring
3. Configure state pruning based on storage requirements

## Security Hardening
- Use firewall rules to restrict access
- Set up monitoring and alerting
- Regular security updates
- Secure key management practices
EOF

    log "SUCCESS" "Validator deployment configuration prepared"
}

# Verification functions
verify_deployment() {
    phase_separator "VERIFICATION"
    log "INFO" "Verifying deployment readiness..."

    # Check all required files exist
    local required_files=(
        "$ARTIFACTS_DIR/parachain_template_runtime.compact.compressed.wasm"
        "$CONFIG_DIR/production-chain-spec.json"
        "$CONFIG_DIR/production-node.toml"
        "$CONFIG_DIR/validator-setup.md"
    )

    for file in "${required_files[@]}"; do
        if [[ ! -f "$file" ]]; then
            error_exit "Required file missing: $file"
        fi
    done

    # Verify WASM integrity
    local wasm_file="$ARTIFACTS_DIR/parachain_template_runtime.compact.compressed.wasm"
    if ! file "$wasm_file" | grep -q "WebAssembly"; then
        error_exit "WASM file integrity check failed"
    fi

    log "SUCCESS" "Deployment verification completed"
}

# Monitoring setup
setup_monitoring() {
    phase_separator "MONITORING"
    log "INFO" "Setting up monitoring infrastructure..."

    # Generate monitoring configuration
    cat > "$CONFIG_DIR/monitoring-setup.md" << 'EOF'
# Production Monitoring Setup

## Metrics Collection
Enable Prometheus metrics in node configuration:
```toml
[telemetry]
endpoints = [
    ["wss://telemetry.polkadot.io/submit/", 0]
]

# Or self-hosted Prometheus
prometheus_external = true
prometheus_port = 9615
```

## Key Metrics to Monitor
- Block production rate
- Transaction throughput
- Finality latency
- Memory usage
- Network connectivity
- Economic metrics (TOL utilization, burn rate)

## Alerting Configuration
Set up alerts for:
- Block production stalls
- High memory usage
- Network connectivity issues
- Economic parameter deviations
EOF

    log "SUCCESS" "Monitoring configuration prepared"
}

# Main deployment orchestration
main() {
    echo -e "${MAGENTA}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                TMCTOL PRODUCTION DEPLOYMENT                 ║"
    echo "║                  Elegant Automation v1.0                    ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"

    log "INFO" "Starting elegant production deployment..."

    # Execute deployment phases
    validate_prerequisites
    validate_build_environment
    build_production_runtime
    generate_production_configs
    setup_deployment_infrastructure
    deploy_validator_node
    verify_deployment
    setup_monitoring

    # Final summary
    echo -e "\n${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    DEPLOYMENT COMPLETE                       ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"

    log "SUCCESS" "Production deployment completed successfully!"
    log "INFO" "Artifacts location: $ARTIFACTS_DIR"
    log "INFO" "Configurations location: $CONFIG_DIR"
    log "INFO" "Next steps:"
    log "INFO" "  1. Review generated configurations"
    log "INFO" "  2. Set up validator nodes with secure key management"
    log "INFO" "  3. Configure monitoring and alerting"
    log "INFO" "  4. Perform final security audit before mainnet launch"

    echo -e "\n${CYAN}Deployment Summary:${NC}"
    echo -e "  📁 Artifacts:    $ARTIFACTS_DIR"
    echo -e "  ⚙️  Configs:      $CONFIG_DIR"
    echo -e "  📊 Logs:         $LOGS_DIR"
    echo -e "  💾 Backups:      $BACKUP_DIR"
    echo -e "\n${YELLOW}Remember: Always practice secure key management and regular backups!${NC}"
}

# Elegant script execution with error handling
trap 'log "ERROR" "Deployment interrupted by user"; exit 1' INT TERM

# Check if running directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
