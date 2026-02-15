#!/bin/bash

# Local Release Workflow Test Script for Parachain Template
# This script mimics the GitHub Actions release workflow to test locally

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RELEASE_DIR="./target/production/wbuild/tmctol-runtime"
WASM_FILE="tmctol_runtime.compact.compressed.wasm"
OUTPUT_DIR="./release-test-output"

echo -e "${GREEN}Starting local release workflow test...${NC}"

# Step 1: Prerequisites check
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

echo -e "${YELLOW}Step 2: Checking Rust compilation prerequisites...${NC}"

# Check if protobuf-compiler is installed
if ! command -v protoc &> /dev/null; then
    echo -e "${RED}Error: protobuf-compiler not found. Install with: sudo apt install -y protobuf-compiler${NC}"
    exit 1
fi
echo "✓ protobuf-compiler found"

# Add wasm32-unknown-unknown target if not present
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "Adding wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
else
    echo "✓ wasm32-unknown-unknown target already installed"
fi

# Check rust-src component
if ! rustup component list --installed | grep -q "rust-src"; then
    echo "Adding rust-src component..."
    rustup component add rust-src
else
    echo "✓ rust-src component already installed"
fi

# Step 3: Build environment preparation
echo -e "${YELLOW}Step 3: Preparing build environment...${NC}"

# Step 4: Build the runtime Wasm
echo -e "${YELLOW}Step 4: Building the runtime WASM (this may take several minutes)...${NC}"
echo -e "${BLUE}Command: cargo build --workspace --locked --profile production${NC}"

# Start timing
start_time=$(date +%s)

cargo build --workspace --locked --profile production

# End timing
end_time=$(date +%s)
build_duration=$((end_time - start_time))
echo -e "${GREEN}✓ Runtime build completed in ${build_duration} seconds${NC}"

# Step 5: Verify Wasm runtime exists
echo -e "${YELLOW}Step 5: Verifying WASM runtime exists...${NC}"

if [ ! -d "$RELEASE_DIR" ]; then
    echo -e "${RED}Error: Release directory not found: $RELEASE_DIR${NC}"
    exit 1
fi

if [ ! -f "$RELEASE_DIR/$WASM_FILE" ]; then
    echo -e "${RED}Error: WASM file not found: $RELEASE_DIR/$WASM_FILE${NC}"
    exit 1
fi

echo "✓ Release directory found: $RELEASE_DIR"
echo "✓ WASM file found: $WASM_FILE"

# Display directory contents
echo -e "${BLUE}Release directory contents:${NC}"
ls -la "$RELEASE_DIR"

# Display file information
echo -e "${BLUE}WASM file information:${NC}"
file "$RELEASE_DIR/$WASM_FILE"

# Display file size in human readable format
wasm_size=$(stat -c%s "$RELEASE_DIR/$WASM_FILE")
wasm_size_mb=$(echo "scale=2; $wasm_size / 1024 / 1024" | bc)
echo "File size: $wasm_size bytes (~${wasm_size_mb} MB)"

# Step 6: Simulate upload preparation
echo -e "${YELLOW}Step 6: Simulating release artifact preparation...${NC}"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Copy the WASM file to output directory (simulating upload)
cp "$RELEASE_DIR/$WASM_FILE" "$OUTPUT_DIR/"

echo "✓ WASM runtime copied to: $OUTPUT_DIR/$WASM_FILE"

# Step 7: Generate release information
echo -e "${YELLOW}Step 7: Generating release information...${NC}"

# Create a release info file
cat > "$OUTPUT_DIR/release-info.md" << EOF
# Parachain Runtime Release

This release contains the optimized WASM runtime for deployment with Omni Node.

## Runtime Details
- **File**: $WASM_FILE
- **Size**: $wasm_size bytes (~${wasm_size_mb} MB)
- **Profile**: Production build with LTO enabled
- **Target**: Built with standard wasm32-unknown-unknown target
- **Build Time**: ${build_duration} seconds

## Usage with Omni Node
\`\`\`bash
# Use with polkadot-omni-node
polkadot-omni-node --chain=your-chain-spec.json
\`\`\`

## Verification
- File type: $(file "$RELEASE_DIR/$WASM_FILE" | cut -d: -f2)
- Build timestamp: $(date)
- Git commit: $(git rev-parse HEAD 2>/dev/null || echo "N/A")
EOF

echo "✓ Release information generated: $OUTPUT_DIR/release-info.md"

# Step 8: Final verification
echo -e "${YELLOW}Step 8: Final verification...${NC}"

# Verify the copied file
if [ -f "$OUTPUT_DIR/$WASM_FILE" ]; then
    copied_size=$(stat -c%s "$OUTPUT_DIR/$WASM_FILE")
    if [ "$wasm_size" -eq "$copied_size" ]; then
        echo "✓ File integrity verified (sizes match)"
    else
        echo -e "${RED}Error: File size mismatch during copy${NC}"
        exit 1
    fi
else
    echo -e "${RED}Error: Copied file not found${NC}"
    exit 1
fi

# Display final results
echo -e "${GREEN}"
echo "========================================="
echo "🎉 LOCAL RELEASE WORKFLOW TEST COMPLETE!"
echo "========================================="
echo -e "${NC}"

echo -e "${BLUE}Release artifacts available at:${NC}"
echo "  📁 Directory: $OUTPUT_DIR"
echo "  🗜️  WASM Runtime: $OUTPUT_DIR/$WASM_FILE"
echo "  📋 Release Info: $OUTPUT_DIR/release-info.md"

echo -e "${BLUE}Next steps:${NC}"
echo "  1. Review the generated release information"
echo "  2. Test the WASM runtime with polkadot-omni-node"
echo "  3. The GitHub Actions release workflow should work identically"

echo -e "${GREEN}✅ Release workflow ready for GitHub Actions!${NC}"
