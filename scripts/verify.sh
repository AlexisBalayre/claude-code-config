#!/bin/bash
# scripts/verify.sh: runs the gates a push needs on the committed tree, then records that tree as
# green in this worktree's git dir, where scripts/pre-push reads it. Lint (LINT_CMD on the source
# files the branch changed) and the shell harnesses run first; TYPECHECK_CMD and TEST_CMD then run
# under the repo's gate lock (scripts/gate-lock.sh). Commands come from .claude/project.env; an
# empty one is skipped. The stamp is keyed on the tree, so a message-only amend stays green. A
# dirty tree is never stamped: the gates would have run on files the commit lacks.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
cd "$(git rev-parse --show-toplevel)"
[ -f .claude/project.env ] && . .claude/project.env

# An explicit mode, since status.showUntrackedFiles=no would hide a new file
# the gates read from disk but the commit lacks.
clean() { [ -z "$(git status --porcelain --untracked-files=normal)" ]; }

if ! clean; then
  echo "verify: commit or drop your changes first; a dirty tree is never stamped." >&2
  exit 1
fi
head=$(git rev-parse HEAD)
# With no merge-base (the trunk was never fetched), diff from the empty tree, so every file counts
# as changed. By its full name: a bare origin/<trunk> resolves to a branch or tag of that name first.
base=$(git merge-base "refs/remotes/origin/${GIT_TRUNK:-main}" HEAD 2>/dev/null || git hash-object -t tree /dev/null)

ext_re=$(printf '%s' "${SOURCE_EXTENSIONS:-}" | tr -s ' ' '|')
sources=
[ -z "$ext_re" ] || sources=$(git diff --name-only --diff-filter=d "$base" HEAD | grep -E "\.($ext_re)$" || true)

# Lint and the shell harnesses run at once, before the lock: they need no machine to themselves,
# so a red one never takes the lock. The harnesses stay in the foreground: a background job here
# ignores SIGINT for good, so Ctrl-C would orphan their tree.
out=$(mktemp "${TMPDIR:-/tmp}/verify.XXXXXX")
trap 'rm -f "$out"' EXIT
lint_pid=
if [ -n "${LINT_CMD:-}" ] && [ -n "$sources" ]; then
  printf '%s\n' "$sources" | tr '\n' '\0' | xargs -0 sh -c "$LINT_CMD \"\$@\"" _ >"$out" 2>&1 &
  lint_pid=$!
fi
red=
bash "$here/check-agent-config.sh" --affected "$base" HEAD || red=check-agent-config
# red is set before the print, so a print that fails (a closed pipe) cannot pass a red job.
if [ -n "$lint_pid" ]; then
  if wait "$lint_pid"; then cat "$out"; else red+="${red:+, }lint"; cat "$out" >&2; fi
fi
# The last stderr line names what failed, since the output above it can run to pages.
[ -z "$red" ] || { echo "verify: red: $red" >&2; exit 1; }

steps=()
[ -z "${TYPECHECK_CMD:-}" ] || steps+=("$TYPECHECK_CMD")
[ -z "${TEST_CMD:-}" ] || steps+=("$TEST_CMD")
if [ "${#steps[@]}" -gt 0 ]; then
  "$here/gate-lock.sh" bash -c \
    'for step in "$@"; do bash -c "$step" || { echo "verify: red: $step" >&2; exit 1; }; done' _ "${steps[@]}"
fi

if [ "$(git rev-parse HEAD)" != "$head" ] || ! clean; then
  echo "verify: HEAD or the tree changed during the run, so the gates tested something else. Run scripts/verify.sh again." >&2
  exit 1
fi

tree=$(git rev-parse "$head^{tree}")
echo "$tree $(date -u +%Y-%m-%dT%H:%M:%SZ) $head" >>"$(git rev-parse --git-dir)/verify-stamp"
echo "verify: tree $tree of $head is green."
