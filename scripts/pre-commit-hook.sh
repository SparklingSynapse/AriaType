#!/usr/bin/env bash
set -euo pipefail

# ── Color helpers ──────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

info()  { echo -e "${GREEN}[pre-commit]${NC} $*"; }
warn()  { echo -e "${YELLOW}[pre-commit]${NC} $*"; }
error() { echo -e "${RED}[pre-commit]${NC} $*"; }

# ── Skip gate ─────────────────────────────────────────────────
if [ "${SKIP_PRE_COMMIT:-}" = "1" ]; then
    warn "SKIP_PRE_COMMIT=1 — skipping all checks."
    exit 0
fi

EXIT_CODE=0

# ── 1. Detect ak-* secret leaks in staged content ─────────────
info "Checking for ak-* secret leaks in staged files..."

AK_MATCHES=$(git diff --cached --diff-filter=ACMR -U0 \
    | grep -n 'ak-[A-Za-z0-9_-]\{10,\}' \
    || true)

if [ -n "$AK_MATCHES" ]; then
    error "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    error "  SECRET LEAK DETECTED — found ak-* key in staged diff"
    error "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "$AK_MATCHES" | while IFS= read -r line; do
        echo "  $line"
    done
    echo ""
    error "Remove the secret before committing."
    error "To bypass: SKIP_PRE_COMMIT=1 git commit ..."
    EXIT_CODE=1
fi

# ── 2. Markdown link check ────────────────────────────────────
info "Running markdown link check..."
if ! pnpm check:md-links; then
    error "Markdown link check failed."
    EXIT_CODE=1
fi

# ── 3. Unit tests ─────────────────────────────────────────────
info "Running unit tests (vitest)..."
if ! pnpm --filter @ariatype/desktop test; then
    error "Unit tests failed."
    EXIT_CODE=1
fi

# ── 4. E2E tests ──────────────────────────────────────────────
if [ "${SKIP_E2E:-}" = "1" ]; then
    warn "SKIP_E2E=1 — skipping e2e tests."
else
    info "Running e2e tests..."
    if ! pnpm --filter @ariatype/desktop test:e2e; then
        error "E2E tests failed."
        EXIT_CODE=1
    fi
fi

# ── Summary ───────────────────────────────────────────────────
if [ "$EXIT_CODE" -ne 0 ]; then
    echo ""
    error "Pre-commit checks FAILED. Fix the issues above and try again."
    exit 1
fi

info "All pre-commit checks passed."
