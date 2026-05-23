#!/bin/bash
# Quality checks: lint, format, typecheck, and tests
# Runs on Claude Code Stop event — after every response that modifies files
# ALL checks are BLOCKING — no issues allowed.

set -o pipefail

cd "$CLAUDE_PROJECT_DIR" || exit 1

# Skip unless TS/TSX files are dirty — lint, typecheck, and tests only target
# those. Markdown/YAML/JSON edits don't need the full suite.
DIRTY_TS=$(
  {
    git diff --name-only 2>/dev/null
    git diff --cached --name-only 2>/dev/null
  } | grep -E '\.(ts|tsx)$' | sort -u
)

if [ -z "$DIRTY_TS" ]; then
  exit 0
fi

echo "Running quality checks..." >&2

# 1. Lint & format auto-fix (applies fixes via Biome)
echo "-> Lint & format (auto-fix)..." >&2
pnpm lint:fix 1>&2 2>&1

# 2. Verify no lint/format issues remain after auto-fix
echo "-> Verifying lint & format..." >&2
if ! pnpm lint 1>&2; then
  echo "Lint/format check failed. Fix the remaining issues above." >&2
  exit 2
fi

# 3. Typecheck
echo "-> Typecheck..." >&2
if ! pnpm typecheck 1>&2; then
  echo "Typecheck failed. Fix the type errors above." >&2
  exit 2
fi

# 4. Tests — only run for packages affected by changes
echo "-> Tests (affected packages only)..." >&2

# Get changed files (staged + unstaged) relative to repo root
CHANGED_FILES=$(git diff --name-only 2>/dev/null; git diff --cached --name-only 2>/dev/null)

# Build turbo filter args from changed file paths
FILTERS=""
for dir in $(echo "$CHANGED_FILES" | sed -E 's|^([^/]+/[^/]+)/.*|\1|' | sort -u); do
  # Only add filter if the directory has a package.json (is a workspace package)
  if [ -f "$dir/package.json" ]; then
    PKG_NAME=$(node -e "console.log(require('./$dir/package.json').name || '')" 2>/dev/null)
    if [ -n "$PKG_NAME" ]; then
      FILTERS="$FILTERS --filter=$PKG_NAME"
    fi
  fi
done

if [ -z "$FILTERS" ]; then
  echo "   No testable packages affected, skipping tests." >&2
else
  echo "   Running tests for:$FILTERS" >&2
  if ! pnpm turbo run test $FILTERS 1>&2; then
    echo "Tests failed. Fix the failing tests above." >&2
    exit 2
  fi
fi

echo "All quality checks passed!" >&2
exit 0
