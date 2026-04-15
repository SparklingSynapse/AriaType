#!/bin/sh

set -e

if [ "$SKIP_SIMPLE_GIT_HOOKS" = "1" ]; then
    echo "[INFO] SKIP_SIMPLE_GIT_HOOKS is set to 1, skipping hook."
    exit 0
fi

node scripts/generate-changelog.mjs

if [ -f "CHANGELOG.md" ]; then
    if git diff --quiet CHANGELOG.md; then
        exit 0
    fi

    if git log -1 --format="%ae" | grep -q "noreply@github.com"; then
        echo "[INFO] GitHub commit, skipping amend"
        exit 0
    fi

    BRANCH=$(git rev-parse --abbrev-ref HEAD)
    UPSTREAM=$(git rev-parse --abbrev-ref @{upstream} 2>/dev/null || echo "")

    if [ -n "$UPSTREAM" ]; then
        LOCAL=$(git rev-parse HEAD)
        REMOTE=$(git rev-parse "$UPSTREAM" 2>/dev/null || echo "")

        if [ "$LOCAL" = "$REMOTE" ]; then
            echo "[WARN] HEAD is pushed to $UPSTREAM. Cannot amend without force push."
            echo "[WARN] CHANGELOG.md updated but not committed. Run:"
            echo "       git add CHANGELOG.md && git commit --amend --no-edit"
            exit 0
        fi
    fi

    if git log -1 --format='%an %ae' | grep -q "$(git config user.name).*$(git config user.email)"; then
        echo "[INFO] Amending commit to include CHANGELOG.md"
        git add CHANGELOG.md
        git commit --amend --no-edit
    else
        echo "[WARN] Last commit not by current user, skipping amend"
        echo "[WARN] CHANGELOG.md updated but not committed."
    fi
fi