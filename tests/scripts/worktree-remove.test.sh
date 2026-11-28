#!/usr/bin/env bash
# Cases for scripts/worktree-remove.sh, run by scripts/check-agent-config.sh (verify):
# `bash tests/scripts/worktree-remove.test.sh`. The script runs in a throwaway clone with gh
# stubbed, under a scratch HOME, so nothing here touches this checkout or its remotes. It checks
# that deleting a branch drops that branch's line from the lead's approved-plans file,
# and only that line: a line of the same branch with another repo's push URL stays.

set -uo pipefail

SCRIPTS=$(cd "$(dirname "$0")/../../scripts" && pwd)
T=$(mktemp -d "${TMPDIR:-/tmp}/worktree-remove.XXXXXX") || exit 1
trap 'rm -rf "$T"' EXIT
failures=0

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

mkdir "$T/bin"
printf '#!/bin/sh\nexit 0\n' >"$T/bin/gh"
chmod +x "$T/bin/gh"
export PATH="$T/bin:$PATH"

git init -q --bare -b main "$T/origin.git"
git clone -q "$T/origin.git" "$T/clone" 2>/dev/null
cd "$T/clone" || exit 1
git checkout -q -b main
echo a >f && git add f && git commit -qm a && git push -q origin main
mkdir -p .worktrees scripts
cp "$SCRIPTS/worktree-remove.sh" scripts/
git worktree add -q -b feature/x .worktrees/x
git push -q origin feature/x

URL=$(git remote get-url --push origin)
OTHER=git@github.com:other/repo.git
printf 'feature/x %s KEY-1 12:00\nfeature/x %s KEY-9 12:00\nfeature/y %s KEY-2 12:01\n' "$URL" "$OTHER" "$URL" >"$T/plans"
mkdir "$T/home"
# From a subdirectory: the script finds the main checkout itself.
mkdir sub
(cd sub && HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash ../scripts/worktree-remove.sh x) >"$T/out" 2>&1 || {
  echo "FAIL worktree-remove exited non-zero"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
}

if git rev-parse -q --verify refs/heads/feature/x >/dev/null; then
  echo "FAIL fixture: feature/x was not deleted"
  failures=$((failures + 1))
fi
if grep -qxF "feature/x $URL KEY-1 12:00" "$T/plans"; then
  echo "FAIL feature/x is still listed in approved-plans after its branch went"
  failures=$((failures + 1))
fi
if ! grep -qxF "feature/x $OTHER KEY-9 12:00" "$T/plans"; then
  echo "FAIL feature/x's line with another repo's push URL was dropped"
  failures=$((failures + 1))
fi
if ! grep -q '^feature/y ' "$T/plans"; then
  echo "FAIL feature/y was dropped from approved-plans with feature/x"
  failures=$((failures + 1))
fi

# A branch a merged PR shipped goes through the other deletion path.
git worktree add -q -b feature/z .worktrees/z
git push -q origin feature/z
printf '#!/bin/sh\necho %s\n' "$(git rev-parse refs/heads/feature/z)" >"$T/bin/gh"
printf 'feature/z %s KEY-3 12:02\nfeature/y %s KEY-2 12:01\n' "$URL" "$URL" >"$T/plans"
HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash scripts/worktree-remove.sh z >"$T/out" 2>&1 || {
  echo "FAIL worktree-remove exited non-zero for the merged branch"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
}
if grep -q '^feature/z ' "$T/plans"; then
  echo "FAIL feature/z is still listed after a merged PR's branch went"
  failures=$((failures + 1))
fi
if ! grep -q '^feature/y ' "$T/plans"; then
  echo "FAIL feature/y was dropped with feature/z"
  failures=$((failures + 1))
fi

# The lead lists origin's push URL, which a pushurl splits from the fetch URL.
git worktree add -q -b feature/v .worktrees/v
git push -q origin feature/v
git init -q --bare "$T/push.git"
git remote set-url --push origin "$T/push.git"
printf 'feature/v %s KEY-5 12:03\nfeature/v %s KEY-6 12:04\n' "$T/push.git" "$URL" >"$T/plans"
HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash scripts/worktree-remove.sh v >"$T/out" 2>&1
if grep -qxF "feature/v $T/push.git KEY-5 12:03" "$T/plans" || ! grep -qxF "feature/v $URL KEY-6 12:04" "$T/plans"; then
  echo "FAIL unlist_plan did not match on origin's push URL"
  failures=$((failures + 1))
fi
git remote set-url --push origin "$URL"

# When origin's push URL cannot be read, every line stays and the script says so.
REAL_GIT=$(command -v git)
mkdir "$T/nourl"
printf '#!/bin/sh\n[ "$1 $2 $3" = "remote get-url --push" ] && exit 2\nexec "%s" "$@"\n' "$REAL_GIT" >"$T/nourl/git"
chmod +x "$T/nourl/git"
git worktree add -q -b feature/u .worktrees/u
git push -q origin feature/u
printf 'feature/u %s KEY-7 12:05\n' "$URL" >"$T/plans"
PATH="$T/nourl:$PATH" HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash scripts/worktree-remove.sh u >"$T/out" 2>&1
if git rev-parse -q --verify refs/heads/feature/u >/dev/null; then
  echo "FAIL fixture: feature/u was not deleted"
  failures=$((failures + 1))
fi
if ! grep -qxF "feature/u $URL KEY-7 12:05" "$T/plans" || ! grep -q 'origin has no push URL' "$T/out"; then
  echo "FAIL a failed get-url dropped the line or said nothing"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
fi

# Without AGENT_APPROVED_PLANS the branch still goes, and the script says its line stayed.
git worktree add -q -b feature/w .worktrees/w
git push -q origin feature/w
printf '#!/bin/sh\necho %s\n' "$(git rev-parse refs/heads/feature/w)" >"$T/bin/gh"
HOME=$T/home env -u AGENT_APPROVED_PLANS bash scripts/worktree-remove.sh w >"$T/out" 2>&1
if ! grep -q "Kept feature/w's line in approved-plans" "$T/out"; then
  echo "FAIL no stderr line for an unset AGENT_APPROVED_PLANS"
  failures=$((failures + 1))
fi

# A merged worker's open session recreates an empty .claude/.cc-writes/ after the removal: a
# directory git lists as no worktree goes while it holds no file, and stays once it holds one.
mkdir -p .worktrees/gone/.claude/.cc-writes .worktrees/held/.claude .worktrees/heldempty/.claude
echo x >.worktrees/held/.claude/f
: >.worktrees/heldempty/.claude/f
HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash scripts/worktree-remove.sh --merged >"$T/out" 2>&1 || {
  echo "FAIL worktree-remove --merged exited non-zero with leftover directories"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
}
if [ -e .worktrees/gone ] || grep -q 'Kept .worktrees/gone' "$T/out"; then
  echo "FAIL an empty leftover directory was kept or got a Kept line"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
fi
if [ ! -f .worktrees/held/.claude/f ] || ! grep -q 'Kept .worktrees/held: not a registered worktree' "$T/out"; then
  echo "FAIL a leftover directory holding a file was not kept with a not-a-worktree line"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
fi
if [ ! -e .worktrees/heldempty/.claude/f ] || ! grep -q 'Kept .worktrees/heldempty: not a registered worktree' "$T/out"; then
  echo "FAIL a leftover directory holding an empty file lost it or was not kept"
  sed 's/^/    /' "$T/out"
  failures=$((failures + 1))
fi

# On a case-insensitive filesystem (default APFS), a worktree named in the wrong case is still
# the registered worktree: it is removed, not taken for a leftover.
git worktree add -q -b feature/c .worktrees/c
git push -q origin feature/c
if [ -d .worktrees/C ]; then
  printf '#!/bin/sh\necho %s\n' "$(git rev-parse refs/heads/feature/c)" >"$T/bin/gh"
  HOME=$T/home AGENT_APPROVED_PLANS=$T/plans bash scripts/worktree-remove.sh C >"$T/out" 2>&1
  if [ -e .worktrees/c ] || grep -q 'not a registered worktree' "$T/out"; then
    echo "FAIL a worktree named in the wrong case was taken for a leftover"
    sed 's/^/    /' "$T/out"
    failures=$((failures + 1))
  fi
fi

if [ "$failures" -gt 0 ]; then
  echo "$failures case(s) failed"
  exit 1
fi
echo "worktree-remove: all cases pass"
