#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/_common.sh"

WEIGHTS_DIR="$TEMPLATE_DIR/runtime/src/weights"

# Default configuration
STEPS=50
REPEAT=20
HEAP_PAGES=4096
CHAIN="dev"

# Custom pallets to benchmark
PALLETS=(
  "pallet_axial_router"
  "pallet_token_minting_curve"
  "pallet_burning_manager"
  "pallet_zap_manager"
  "pallet_treasury_owned_liquidity"
  "pallet_asset_registry"
)

usage() {
  cat << EOF
Usage: $(basename "$0") [OPTIONS] [PALLET_NAME]

Run benchmarks and generate weight files for TMCTOL pallets.

Options:
  --steps N       Number of steps per benchmark (default: $STEPS)
  --repeat N      Number of repetitions per benchmark (default: $REPEAT)
  --all           Benchmark all custom pallets
  --list          List available pallets
  --check         Only verify benchmarks compile (no execution)
  -h, --help      Show this help message

Arguments:
  PALLET_NAME     Specific pallet to benchmark (e.g., pallet_axial_router)
                  If omitted and --all not set, prompts for selection.

Examples:
  $(basename "$0") --all                      # Benchmark all pallets
  $(basename "$0") pallet_axial_router        # Benchmark one pallet
  $(basename "$0") --check                    # Verify compilation only
  $(basename "$0") --steps 100 --repeat 50 --all  # Production-quality run
EOF
  exit 0
}

check_dependencies() {
  if ! command -v cargo &> /dev/null; then
    log_error "Cargo not found. Install Rust toolchain and try again."
    exit 1
  fi

  if ! command -v frame-omni-bencher &> /dev/null; then
    log_warning "frame-omni-bencher not found. Install with:"
    echo "  cargo install --locked frame-omni-bencher --tag polkadot-stable2512-1"
    echo ""
    log_info "Falling back to 'cargo test --features runtime-benchmarks' mode"
    BENCHER_MODE="cargo"
  else
    BENCHER_MODE="omni"
    log_success "frame-omni-bencher found"
  fi
}

build_benchmarks() {
  log_info "Building runtime with benchmarks feature..."
  cd "$TEMPLATE_DIR"

  local start_time=$(date +%s)
  cargo build --release --features runtime-benchmarks -p tmctol-runtime 2>&1
  local end_time=$(date +%s)
  local build_duration=$((end_time - start_time))

  log_success "Benchmark build completed in ${build_duration}s"
}

check_only() {
  log_info "Verifying benchmark compilation..."
  cd "$TEMPLATE_DIR"
  cargo check --features runtime-benchmarks 2>&1
  log_success "All benchmarks compile successfully"
}

# frame-omni-bencher generates files with bare `frame_system`/`frame_support` imports
# and `WeightInfo<T>` struct names. Normalize to project conventions.
normalize_weight_file() {
  local file="$1"
  sed -i 's/use frame_support::/use polkadot_sdk::frame_support::/g' "$file"
  sed -i 's/pub struct WeightInfo/pub struct SubstrateWeight/' "$file"
  sed -i 's/impl<T: frame_system::Config>/impl<T: polkadot_sdk::frame_system::Config>/' "$file"
  sed -i 's/for WeightInfo<T>/for SubstrateWeight<T>/' "$file"
  log_info "  Normalized imports and struct name"
}

run_pallet_benchmark() {
  local pallet_name="$1"
  local output_file="$WEIGHTS_DIR/${pallet_name}.rs"

  log_info "Benchmarking: $pallet_name (steps=$STEPS, repeat=$REPEAT)"

  if [[ "$BENCHER_MODE" == "omni" ]]; then
    frame-omni-bencher v1 benchmark pallet \
      --runtime "$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.compact.compressed.wasm" \
      --pallet "$pallet_name" \
      --extrinsic "*" \
      --steps "$STEPS" \
      --repeat "$REPEAT" \
      --heap-pages "$HEAP_PAGES" \
      --output "$output_file" \
      --template "$TEMPLATE_DIR/.maintain/frame-weight-template.hbs" 2>&1 || {
        # Fallback without template if template doesn't exist
        frame-omni-bencher v1 benchmark pallet \
          --runtime "$TEMPLATE_DIR/target/release/wbuild/tmctol-runtime/tmctol_runtime.compact.compressed.wasm" \
          --pallet "$pallet_name" \
          --extrinsic "*" \
          --steps "$STEPS" \
          --repeat "$REPEAT" \
          --heap-pages "$HEAP_PAGES" \
          --output "$output_file" 2>&1
      }
  else
    log_warning "Running benchmark tests (dry run without weight generation)"
    cd "$TEMPLATE_DIR"
    cargo test --release --features runtime-benchmarks -p tmctol-runtime -- "benchmark" --nocapture 2>&1 || true
    log_warning "Weight files NOT updated (frame-omni-bencher required for weight generation)"
    return 0
  fi

  if [[ -f "$output_file" ]]; then
    normalize_weight_file "$output_file"
    log_success "$pallet_name → $output_file"
  else
    log_error "Weight file not generated for $pallet_name"
    return 1
  fi
}

run_all_benchmarks() {
  log_info "Running benchmarks for ${#PALLETS[@]} pallets..."
  echo ""

  local failed=0
  local succeeded=0
  local start_time=$(date +%s)

  for pallet in "${PALLETS[@]}"; do
    if run_pallet_benchmark "$pallet"; then
      ((succeeded++))
    else
      ((failed++))
      log_error "Failed: $pallet"
    fi
    echo ""
  done

  local end_time=$(date +%s)
  local total_duration=$((end_time - start_time))

  echo "════════════════════════════════════════"
  log_info "Benchmark Summary"
  echo "  Succeeded: $succeeded / ${#PALLETS[@]}"
  echo "  Failed:    $failed / ${#PALLETS[@]}"
  echo "  Duration:  ${total_duration}s"
  echo "  Steps:     $STEPS"
  echo "  Repeat:    $REPEAT"
  echo "════════════════════════════════════════"

  if [[ $failed -gt 0 ]]; then
    log_error "Some benchmarks failed"
    exit 1
  fi

  log_success "All benchmarks completed successfully"
}

list_pallets() {
  echo "Available pallets for benchmarking:"
  for pallet in "${PALLETS[@]}"; do
    local weight_file="$WEIGHTS_DIR/${pallet}.rs"
    if [[ -f "$weight_file" ]]; then
      echo "  ✓ $pallet (weights: $(wc -l < "$weight_file") lines)"
    else
      echo "  ✗ $pallet (no weight file)"
    fi
  done
}

main() {
  local action=""
  local target_pallet=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --steps)    STEPS="$2"; shift 2 ;;
      --repeat)   REPEAT="$2"; shift 2 ;;
      --all)      action="all"; shift ;;
      --list)     action="list"; shift ;;
      --check)    action="check"; shift ;;
      -h|--help)  usage ;;
      *)          target_pallet="$1"; shift ;;
    esac
  done

  log_info "TMCTOL Benchmark Runner"
  echo ""

  if [[ "$action" == "list" ]]; then
    list_pallets
    exit 0
  fi

  if [[ "$action" == "check" ]]; then
    check_only
    exit 0
  fi

  check_dependencies

  if [[ "$BENCHER_MODE" == "omni" ]]; then
    build_benchmarks
  fi

  if [[ -n "$target_pallet" ]]; then
    run_pallet_benchmark "$target_pallet"
  elif [[ "$action" == "all" ]]; then
    run_all_benchmarks
  else
    log_error "Specify a pallet name or use --all"
    echo ""
    usage
  fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
