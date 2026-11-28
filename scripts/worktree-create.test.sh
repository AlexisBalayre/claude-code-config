#!/usr/bin/env bash
# Hand-run cases for scripts/worktree-create.sh: `bash scripts/worktree-create.test.sh`.
# The script runs in a throwaway clone whose HEAD is behind its origin, so nothing here touches this checkout or its remotes.

set -uo pipefail

SCRIPTS=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/worktree-create.XXXXXX") || exit 1
trap 'rm -rf "$T"' EXIT
failures=0

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

git init -q --bare -b main "$T/origin.git"
git clone -q "$T/origin.git" "$T/seed" 2>/dev/null
(cd "$T/seed" && git checkout -q -b main && echo a >f && git add f && git commit -qm a && git push -q origin main)
git clone -q "$T/origin.git" "$T/clone"
(cd "$T/seed" && echo b >f && git commit -qam b && git push -q origin main)
new_tip=$(git -C "$T/seed" rev-parse HEAD)

cd "$T/clone" || exit 1
stale_head=$(git rev-parse HEAD)
# A sandboxed worker can create this branch, and the lead and the user run this script.
git branch origin/main "$stale_head" 2>/dev/null
bash "$SCRIPTS/worktree-create.sh" x >"$T/out" 2>&1 || {
  echo "FAIL worktree-create exited non-zero"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
}

got=$(git rev-parse feature/x 2>/dev/null)
if [ "$stale_head" = "$new_tip" ]; then
  echo "FAIL fixture: clone HEAD is not behind origin/main"
  failures=$((failures + 1))
elif [ "$got" != "$new_tip" ]; then
  printf 'FAIL feature/x is %s, want origin/main tip %s (stale HEAD %s)\n' "$got" "$new_tip" "$stale_head"
  failures=$((failures + 1))
fi

if git rev-parse --abbrev-ref feature/x@{upstream} >/dev/null 2>&1; then
  echo "FAIL feature/x tracks an upstream; a bare git push would target it"
  failures=$((failures + 1))
fi

if [ "$failures" -gt 0 ]; then
  echo "$failures case(s) failed"
  exit 1
fi
echo "worktree-create: all cases pass"
