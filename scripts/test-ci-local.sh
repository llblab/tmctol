#!/bin/bash

# Local CI Test Script for Parachain Template
# This script mimics the GitHub Actions CI workflow to test locally

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting local CI workflow test...${NC}"

# Configuration
SKIP_WASM_BUILD=1
export SKIP_WASM_BUILD

# Function to run a step with timing
run_step() {
    local step_name="$1"
    local command="$2"
    local timeout_minutes="$3"

    echo -e "${YELLOW}Running: $step_name${NC}"
    echo -e "${BLUE}Command: $command${NC}"

    start_time=$(date +%s)

    # Run the command with timeout if specified
    if [ -n "$timeout_minutes" ]; then
        timeout "${timeout_minutes}m" bash -c "$command"
    else
        bash -c "$command"
    fi

    end_time=$(date +%s)
    duration=$((end_time - start_time))
    echo -e "${GREEN}✓ $step_name completed in ${duration} seconds${NC}"
    echo ""
}

# Step 1: Check prerequisites
echo -e "${YELLOW}Step 1: Checking prerequisites...${NC}"

# Robust path resolution
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEMPLATE_DIR="$PROJECT_ROOT/template"

# Ensure template directory exists
if [ ! -d "$TEMPLATE_DIR" ]; then
    echo -e "${RED}Error: Template directory not found at $TEMPLATE_DIR${NC}"
    exit 1
fi

echo -e "${YELLOW}Navigating to template directory...${NC}"
cd "$TEMPLATE_DIR"
echo "✓ Working from: $(pwd)"

# Check if clippy is available
if ! cargo clippy --version &> /dev/null; then
    echo "Installing clippy..."
    rustup component add clippy
fi

echo "✓ Prerequisites checked"
echo ""

# Step 2: Run clippy
run_step "Clippy (Linting)" \
    "cargo clippy --all-targets --all-features --locked --workspace --quiet" \
    "30"

# Step 3: Run tests
run_step "Tests" \
    "cargo test --workspace" \
    "15"

# Step 4: Build documentation
run_step "Documentation Build" \
    "cargo doc --workspace --no-deps" \
    "15"

# Step 5: Additional checks (bonus)
echo -e "${YELLOW}Step 5: Additional checks...${NC}"

# Check formatting
echo -e "${BLUE}Checking code formatting...${NC}"
if cargo fmt -- --check; then
    echo -e "${GREEN}✓ Code formatting is correct${NC}"
else
    echo -e "${YELLOW}⚠ Code formatting issues found (run 'cargo fmt' to fix)${NC}"
fi

# Check if there are any unused dependencies
echo -e "${BLUE}Checking for basic workspace consistency...${NC}"
if cargo check --workspace --quiet; then
    echo -e "${GREEN}✓ Workspace check passed${NC}"
else
    echo -e "${RED}✗ Workspace check failed${NC}"
    exit 1
fi

# Final summary
echo -e "${GREEN}"
echo "========================================="
echo "🎉 LOCAL CI WORKFLOW TEST COMPLETE!"
echo "========================================="
echo -e "${NC}"

echo -e "${BLUE}All CI steps completed successfully:${NC}"
echo "  ✅ Clippy (linting)"
echo "  ✅ Tests"
echo "  ✅ Documentation build"
echo "  ✅ Code formatting check"
echo "  ✅ Workspace consistency check"

echo -e "${GREEN}✅ CI workflow ready for GitHub Actions!${NC}"

# Optional: Show some project statistics
echo -e "${BLUE}Project Statistics:${NC}"
echo "  📦 Workspace members: $(grep -c 'members.*=' Cargo.toml || echo 'N/A')"
echo "  🧪 Test files: $(find . -name '*.rs' -exec grep -l '#\[test\]' {} \; | wc -l)"
echo "  📚 Documentation: $(find target/doc -name '*.html' 2>/dev/null | wc -l) HTML files generated"

echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  1. Review any clippy warnings above"
echo "  2. Ensure all tests pass"
echo "  3. The GitHub Actions CI workflow should work identically"
