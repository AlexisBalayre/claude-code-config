#!/bin/bash
# Quality checks: lint/format on the session's dirty files + repo typecheck.
# Runs on Claude Code Stop event — after every response that modifies files.
# Tests are NOT run here: the pre-commit hook runs the full turbo suite, and
# whole-repo lint is deliberately avoided so unrelated red on main can't block
# an unrelated session.

set -o pipefail

cd "$CLAUDE_PROJECT_DIR" || exit 1

# Lint/typecheck only target TS/TSX; markdown/YAML/JSON edits skip the suite.
# Includes untracked files (Write-created files aren't staged yet).
DIRTY_TS=$(
  {
    git diff --name-only 2>/dev/null
    git diff --cached --name-only 2>/dev/null
    git ls-files --others --exclude-standard 2>/dev/null
  } | grep -E '\.(ts|tsx)$' | sort -u | while read -r f; do [ -f "$f" ] && echo "$f"; done
)

if [ -z "$DIRTY_TS" ]; then
  exit 0
fi

echo "Running quality checks..." >&2

# 1. Lint & format auto-fix on dirty files only (applies fixes via Biome)
echo "-> Lint & format dirty files (auto-fix)..." >&2
echo "$DIRTY_TS" | xargs pnpm exec biome check --write 1>&2 2>&1

# 2. Verify no lint/format issues remain after auto-fix
echo "-> Verifying lint & format..." >&2
if ! echo "$DIRTY_TS" | xargs pnpm exec biome check 1>&2; then
  echo "Lint/format check failed on the files this session touched. Fix the remaining issues above." >&2
  exit 2
fi

# 3. Typecheck (whole repo — type errors cross file boundaries; turbo caches)
echo "-> Typecheck..." >&2
if ! pnpm typecheck 1>&2; then
  echo "Typecheck failed. Fix the type errors above." >&2
  exit 2
fi

echo "All quality checks passed!" >&2
exit 0
