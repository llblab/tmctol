#!/bin/bash

# Download script for polkadot-omni-node
# This script downloads the latest polkadot-omni-node binary for local development

set -e

# Robust path resolution
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default Configuration
REPO="paritytech/polkadot-sdk"
BINARY_NAME="polkadot-omni-node"
INSTALL_DIR="$PROJECT_ROOT/template"
TARGET_VERSION="stable2509-2"
FORCE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_dependencies() {
    if ! command -v curl &> /dev/null; then
        print_error "curl is required but not installed"
        exit 1
    fi

    if ! command -v jq &> /dev/null; then
        print_error "jq is required for reliable release parsing"
        exit 1
    fi
}

get_release_url() {
    local version="$1"
    local api_url

    if [ "$version" == "latest" ]; then
        api_url="https://api.github.com/repos/${REPO}/releases/latest"
    else
        api_url="https://api.github.com/repos/${REPO}/releases/tags/polkadot-$version"
        # Fallback: try without 'polkadot-' prefix if failed (some repos differ)
        if ! curl -f -s -I "$api_url" > /dev/null; then
             api_url="https://api.github.com/repos/${REPO}/releases/tags/$version"
        fi
    fi

    local response=$(curl -s "$api_url")

    # Check if release exists
    if echo "$response" | jq -e '.message == "Not Found"' > /dev/null; then
        print_error "Release version '$version' not found in $REPO"
        exit 1
    fi

    local download_url=$(echo "$response" | jq -r ".assets[] | select(.name == \"$BINARY_NAME\") | .browser_download_url")

    if [ -z "$download_url" ] || [ "$download_url" = "null" ]; then
        print_error "Could not find download URL for $BINARY_NAME in version $version"
        print_info "Available assets:"
        echo "$response" | jq -r '.assets[].name' | head -10
        exit 1
    fi

    echo "$download_url"
}

download_binary() {
    local url="$1"
    local output_path="$INSTALL_DIR/$BINARY_NAME"

    print_info "Downloading $BINARY_NAME"
    print_info "URL: $url"

    if curl -L -f --progress-bar -o "$output_path" "$url"; then
        print_info "Download completed, checking file..."
        if [ -f "$output_path" ]; then
            chmod +x "$output_path"
            print_info "Downloaded and made executable: $output_path"
            print_info "File size: $(du -h "$output_path" | cut -f1)"
        else
            print_error "Download completed but file not found at: $output_path"
            exit 1
        fi
    else
        print_error "Failed to download $BINARY_NAME"
        exit 1
    fi
}

verify_binary() {
    local binary_path="$INSTALL_DIR/$BINARY_NAME"

    if [ -x "$binary_path" ]; then
        print_info "Verifying binary..."
        if "$binary_path" --version > /dev/null 2>&1; then
            local version=$("$binary_path" --version 2>/dev/null | head -n1)
            print_info "Successfully verified: $version"
        else
            print_warning "Binary downloaded but version check failed"
        fi
    else
        print_error "Binary is not executable"
        exit 1
    fi
}

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --version|-v)
                TARGET_VERSION="$2"
                shift 2
                ;;
            --output|-o)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --force|-f)
                FORCE=true
                shift
                ;;
            --help|-h)
                echo "Usage: $0 [OPTIONS]"
                echo "Options:"
                echo "  --version, -v <ver>  Specify version (default: latest)"
                echo "  --output, -o <dir>   Specify output directory (default: template/)"
                echo "  --force, -f          Force overwrite existing binary"
                echo "  --help, -h           Show this help"
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    print_info "Polkadot Omni Node Download Script"
    print_info "Target Version: $TARGET_VERSION"
    print_info "Install Directory: $INSTALL_DIR"

    # Create directory if missing
    mkdir -p "$INSTALL_DIR"

    # Check if binary already exists
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ] && [ "$FORCE" = false ]; then
        print_warning "Binary already exists at $INSTALL_DIR/$BINARY_NAME"
        print_info "Use --force to overwrite"
        exit 0
    fi

    check_dependencies

    local download_url=$(get_release_url "$TARGET_VERSION")
    print_info "Downloading from: $download_url"

    download_binary "$download_url"
    verify_binary

    print_info "✅ $BINARY_NAME successfully downloaded to $INSTALL_DIR"
}

main "$@"
