#!/bin/bash
# Quality checks: format/lint the session's dirty source files + repo typecheck.
# Runs on Claude Code Stop event — after every response that modifies files.
# Commands come from .claude/project.env; an empty command is skipped, so an
# unadapted template no-ops. Tests are NOT run here: scripts/pre-commit runs
# TEST_CMD, and whole-repo lint is deliberately avoided so unrelated red on the
# trunk can't block an unrelated session.

set -o pipefail

cd "$CLAUDE_PROJECT_DIR" || exit 1
[ -f .claude/project.env ] && . .claude/project.env

[ -z "${FORMAT_FIX_CMD}${LINT_CMD}${TYPECHECK_CMD}" ] && exit 0

EXT_RE=$(printf '%s' "${SOURCE_EXTENSIONS:-}" | tr -s ' ' '|')
[ -z "$EXT_RE" ] && exit 0

# Includes untracked files (Write-created files aren't staged yet).
DIRTY=$(
  {
    git diff --name-only 2>/dev/null
    git diff --cached --name-only 2>/dev/null
    git ls-files --others --exclude-standard 2>/dev/null
  } | grep -E "\.($EXT_RE)$" | sort -u | while read -r f; do [ -f "$f" ] && echo "$f"; done
)

[ -z "$DIRTY" ] && exit 0

echo "Running quality checks..." >&2

if [ -n "${FORMAT_FIX_CMD:-}" ]; then
  echo "-> Format/fix dirty files: $FORMAT_FIX_CMD" >&2
  printf '%s\n' "$DIRTY" | tr '\n' '\0' | xargs -0 sh -c "$FORMAT_FIX_CMD \"\$@\"" _ 1>&2
fi

if [ -n "${LINT_CMD:-}" ]; then
  echo "-> Lint dirty files: $LINT_CMD" >&2
  if ! printf '%s\n' "$DIRTY" | tr '\n' '\0' | xargs -0 sh -c "$LINT_CMD \"\$@\"" _ 1>&2; then
    echo "Lint failed on the files this session touched. Fix the remaining issues above." >&2
    exit 2
  fi
fi

# Whole repo: type errors cross file boundaries.
if [ -n "${TYPECHECK_CMD:-}" ]; then
  echo "-> Typecheck: $TYPECHECK_CMD" >&2
  if ! bash -c "$TYPECHECK_CMD" 1>&2; then
    echo "Typecheck failed. Fix the type errors above." >&2
    exit 2
  fi
fi

echo "All quality checks passed!" >&2
exit 0
