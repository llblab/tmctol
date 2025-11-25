#!/bin/bash

# Minimal Context Validator
# Simple validation for Documentation RAG system

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTEXT_FILE="$PROJECT_ROOT/AGENTS.md"

ERRORS=0
WARNINGS=0

# Logging
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; ((WARNINGS++)); }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; ((ERRORS++)); }

echo "Context Validator - Protocol Evolution Based"
echo "============================================="

# Check 1: Context file exists
if [[ -f "$CONTEXT_FILE" ]]; then
    log_pass "AGENTS.md exists"
else
    log_fail "AGENTS.md not found"
fi

# Check 2: Has required sections
if grep -q "## 1\. Concept" "$CONTEXT_FILE" 2>/dev/null; then
    log_pass "Core structure sections present"
else
    log_warn "Core structure sections missing"
fi

# Check 3: Has Change History
if grep -q "Change History" "$CONTEXT_FILE" 2>/dev/null; then
    log_pass "Change History section found"
else
    log_warn "Change History section missing"
fi

# Check 4: Link validation with specific reporting
echo "Checking documentation links..."
HAS_BROKEN_LINKS=false

# Check links in key documentation files
for file in "$PROJECT_ROOT"/*.md "$PROJECT_ROOT"/docs/*.md "$PROJECT_ROOT"/template/pallets/*/README.md "$PROJECT_ROOT"/template/pallets/README.md; do
    [[ -f "$file" ]] || continue

    # Find relative links
    while IFS= read -r line; do
        if [[ "$line" == *"](."* && "$line" != *"http"* ]]; then
            # Extract link path
            link_path="${line#*](}"
            link_path="${link_path%%)*}"

            # Skip external links
            [[ "$link_path" == http* ]] && continue

            # Resolve relative path
            if [[ "$link_path" == /* ]]; then
                target="$PROJECT_ROOT$link_path"
            else
                target="$(dirname "$file")/$link_path"
            fi

            # Check if target exists
            if [[ ! -e "$target" ]]; then
                log_fail "Broken link: $link_path in $(basename "$file")"
                HAS_BROKEN_LINKS=true
            fi
        fi
    done < "$file"
done

if [[ "$HAS_BROKEN_LINKS" == "false" ]]; then
    log_pass "Documentation links validated"
fi

# Check 5: Has Meta-Protocol Principles
if grep -q "Meta-Protocol Principles" "$CONTEXT_FILE" 2>/dev/null; then
    log_pass "Meta-Protocol Principles found"
else
    log_warn "Meta-Protocol Principles missing"
fi

# Check 6: Recent activity (freshness)
if [[ -f "$CONTEXT_FILE" ]]; then
    AGE_DAYS=$(( ($(date +%s) - $(stat -c %Y "$CONTEXT_FILE" 2>/dev/null || stat -f %m "$CONTEXT_FILE" 2>/dev/null || echo "0")) / 86400 ))
    if [[ $AGE_DAYS -lt 30 ]]; then
        log_pass "Context is fresh ($AGE_DAYS days old)"
    else
        log_warn "Context is stale ($AGE_DAYS days old)"
    fi
fi

# Check 7: Documentation directory exists
if [[ -d "$PROJECT_ROOT/docs" ]]; then
    log_pass "Documentation directory exists"
else
    log_warn "Documentation directory missing"
fi

# Summary
echo
echo "=================================="
echo "   VALIDATION SUMMARY"
echo "=================================="
echo "Warnings: $WARNINGS"
echo "Errors: $ERRORS"
echo

if [[ $ERRORS -eq 0 ]]; then
    echo -e "${GREEN}✓ Context validation PASSED${NC}"
    echo "Ready for Task Completion Protocol"
    exit 0
else
    echo -e "${RED}✗ Context validation FAILED${NC}"
    echo "Manual intervention required"
    exit 1
fi
