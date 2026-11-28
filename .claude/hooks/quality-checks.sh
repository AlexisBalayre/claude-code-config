#!/bin/bash
# Quality checks: format and lint the session's dirty source files.
# Runs on Claude Code Stop event, after every response that modifies files.
# Commands come from .claude/project.env; an empty command is skipped, so an
# unadapted template no-ops. Typecheck and tests are NOT run here: they run in
# scripts/verify.sh before a push, and whole-repo lint is deliberately avoided so
# unrelated red on the trunk can't block an unrelated session.

set -o pipefail

# The session's tree is its cwd when that is a worktree of this repository, so a session
# working in .worktrees/<name> is checked there; any other cwd falls back to the project dir.
# git before 2.31 prints --path-format back instead of a path, hence the shape test.
CWD=$(jq -r '.cwd // empty' <<<"$(cat 2>/dev/null)" 2>/dev/null)
COMMON=$(git -C "$CLAUDE_PROJECT_DIR" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)
[[ $COMMON == /* && $COMMON != *$'\n'* ]] || COMMON=""
TOP=""
if [ -n "$CWD" ] && [ -n "$COMMON" ] && [ "$(git -C "$CWD" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" = "$COMMON" ]; then
  TOP=$(git -C "$CWD" rev-parse --show-toplevel 2>/dev/null)
fi
cd "${TOP:-$CLAUDE_PROJECT_DIR}" || exit 1
[ -f .claude/project.env ] && . .claude/project.env

[ -z "${FORMAT_FIX_CMD}${LINT_CMD}" ] && exit 0

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

if [ -n "${FORMAT_FIX_CMD:-}" ]; then
  printf '%s\n' "$DIRTY" | tr '\n' '\0' | xargs -0 sh -c "$FORMAT_FIX_CMD \"\$@\"" _ >/dev/null 2>&1
fi

if [ -n "${LINT_CMD:-}" ]; then
  if ! OUT=$(printf '%s\n' "$DIRTY" | tr '\n' '\0' | xargs -0 sh -c "$LINT_CMD \"\$@\"" _ 2>&1); then
    # The tail only: this output lands in the model's context on every failing turn.
    printf '%s\n' "$OUT" | tail -n 40 >&2
    echo "Lint failed on the files this session touched ($LINT_CMD). Fix the remaining issues above." >&2
    exit 2
  fi
fi

exit 0
