#!/usr/bin/env bash
# Hand-run cases for scripts/pre-push and scripts/verify.sh: `bash scripts/pre-push.test.sh`.
# Every push goes from a throwaway repo to a throwaway bare remote, and verify runs against a
# stub command, so nothing here touches this checkout or its remotes.

set -uo pipefail

SCRIPTS=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0

# The fixtures must not inherit the user's hooksPath or commit signing.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
# A Claude Code session sets CLAUDECODE=1, which would subject every fixture push to the size cap.
unset CLAUDECODE

expect() {
  local want=$1 name=$2 got
  shift 2
  "$@" >"$T/out" 2>&1
  got=$?
  [ "$got" -eq 0 ] || got=1
  if [ "$got" -ne "$want" ]; then
    printf 'FAIL want=%s got=%s  %s\n' "$want" "$got" "$name"
    sed 's/^/    /' "$T/out"
    failures=$((failures + 1))
  fi
}

stamp() { echo "$(git rev-parse "$1^{tree}") 2026-01-01T00:00:00Z $(git rev-parse "$1")" >>"$(git rev-parse --git-dir)/verify-stamp"; }
commit() { echo "$1" >f && git add f && git commit -qm "$1"; }

git init -q --bare "$T/remote.git"
git init -q -b feature/x "$T/w"
cd "$T/w" || exit 1
git config core.hooksPath "$T/w/.git/hooks"
cp "$SCRIPTS/pre-push" "$SCRIPTS/added-lines.sh" .git/hooks/
git remote add origin "$T/remote.git"

# A push needs a green stamp for the pushed commit's tree.
commit a
expect 1 'no stamp' git push -q origin feature/x
stamp HEAD
expect 0 'stamp for the tree' git push -q origin feature/x
commit b
expect 1 'stamp only for an older tree' git push -q origin feature/x
git commit -q --amend -m 'b, reworded'
stamp HEAD
git commit -q --amend -m 'b, reworded again'
expect 0 'a message-only amend keeps the tree green' git push -q origin feature/x

# Every ref in one push needs its own stamp.
git branch feature/y HEAD
git checkout -q feature/y
commit c
expect 1 'one of two refs unstamped' git push -q origin feature/x:refs/heads/feature/x3 feature/y
stamp HEAD
expect 0 'both refs stamped' git push -q origin feature/x:refs/heads/feature/x3 feature/y

# A branch delete pushes no tree, and a tag push is exempt.
git checkout -q -b feature/z
commit d
expect 0 'a stamped ref pushed while HEAD is not' git push -q origin feature/x:refs/heads/feature/x2
expect 0 'a branch delete' git push -q origin :feature/y
git tag v1
expect 0 'a tag push' git push -q origin v1

# A worktree reads its own stamp, also when pushed with `git -C`.
git worktree add -q "$T/w2" -b feature/w2 feature/z
cd "$T" || exit 1
expect 1 'another worktree has no stamp' git -C "$T/w2" push -q origin feature/w2
(cd "$T/w2" && stamp HEAD)
expect 0 'git -C reads that worktree stamp' git -C "$T/w2" push -q origin feature/w2
cd "$T/w" || exit 1

# A Claude Code push warns at 400 production lines over origin/main and fails at
# 800, or above 1,000 when every changed file is *.md.
add() { mkdir -p "$(dirname "$2")" && seq "$1" >"$2" && git add "$2"; }
mkdir .claude
cat >.claude/project.env <<'EOF'
SOURCE_EXTENSIONS="ts"
LINT_CMD="stub lint"
TYPECHECK_CMD="stub typecheck"
TEST_CMD="stub test"
GENERATED_PATHS_REGEX='(^|/)generated/'
EOF
git add .claude
add 2000 big && add 2000 src/fixtures/a.json && add 2000 src/lib/a.snap && git commit -qm big
git update-ref refs/remotes/origin/main HEAD
base=$(git rev-parse HEAD)
on() { git checkout -q -B "$1" "$base"; }
claude_push() { CLAUDECODE=1 git push -q origin "HEAD:refs/heads/$1" 2>&1 | tee "$T/push"; }
warned() { grep -q 'production lines' "$T/push"; }
# The PR Size workflow reads the same verdict over the PR's merge-base.
ci_verdict() { [ "$(bash "$SCRIPTS/added-lines.sh" "$base" HEAD | sed -n 1p)" = "$1" ]; }
on size-399 && add 399 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a Claude push adding 399 production lines' claude_push size-399
expect 1 'does not warn' warned
expect 0 'the CI verdict passes 399' ci_verdict 'pass 399 800'
on size-400 && add 400 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a Claude push adding 400 production lines' claude_push size-400
expect 0 'warns' warned
expect 0 'the CI verdict warns at 400' ci_verdict 'warn 400 800'
on size-799 && add 799 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a Claude push adding 799 production lines' claude_push size-799
on size-over && add 800 src/a.ts && git commit -qm a && stamp HEAD
expect 1 'a Claude push adding 800 production lines' claude_push size-over
expect 0 'the CI verdict fails at 800' ci_verdict 'fail 800 800'
expect 0 'the same push outside Claude Code' git push -q origin HEAD:refs/heads/size-over
echo size-waived >>"$(git rev-parse --git-common-dir)/size-waiver"
expect 0 'a waived branch over the cap' claude_push size-waived
on size-docs && add 990 docs/a.md && add 10 README.md && git commit -qm a && stamp HEAD
expect 0 'a *.md-only push adding 1,000 lines' claude_push size-docs
expect 0 'the CI verdict allows 1,000 for *.md only' ci_verdict 'warn 1000 1001'
add 1 docs/c.md && git commit -qm a && stamp HEAD
expect 1 'a *.md-only push adding 1,001 lines' claude_push size-docs
on size-docs-mixed && add 999 docs/a.md && add 1 src/a.ts && git commit -qm a && stamp HEAD
expect 1 'one non-*.md file holds the cap at 800' claude_push size-docs-mixed
on size-docs-utf8 && add 990 'docs/é.md' && add 10 README.md && git commit -qm a
expect 0 'a non-ASCII *.md path stays *.md-only, though git quotes it' ci_verdict 'warn 1000 1001'
on size-prefixtures && add 800 src/prefixtures/a.ts && git commit -qm a
expect 0 'a directory that only ends in fixtures counts' ci_verdict 'fail 800 800'
on size-no-counter && add 10 src/a.ts && git commit -qm a && stamp HEAD
mv .git/hooks/added-lines.sh "$T/added-lines.sh"
expect 1 'a pre-push installed without added-lines.sh refuses a small Claude push' claude_push size-no-counter
printf '#!/bin/bash\nexit 1\n' >.git/hooks/added-lines.sh && chmod +x .git/hooks/added-lines.sh
expect 1 'a failing added-lines.sh refuses a small Claude push' claude_push size-no-counter
printf '#!/bin/bash\nprintf "10\\tsrc/a.ts\\n"\n' >.git/hooks/added-lines.sh
expect 1 'an added-lines.sh that prints no verdict line refuses a small Claude push' claude_push size-no-counter
mv "$T/added-lines.sh" .git/hooks/
on size-tests && add 700 src/a.ts && add 900 src/a.test.ts && add 900 scripts/x.test.sh && add 900 app/test/unit/b.ts \
  && add 900 tools/tests/test_x.py && add 900 pkg/x_test.go && add 900 src/fixtures/c.json \
  && add 900 tooling/rule-fixtures/x/safe.patch && add 900 src/a.mock.ts && add 900 src/__mocks__/m.ts \
  && add 900 db/migrations/0001_x.sql \
  && git commit -qm a && stamp HEAD
expect 0 'tests, fixtures, mocks and migrations do not count' claude_push size-tests
expect 0 'the CI verdict skips them too' ci_verdict 'warn 700 800'
on size-generated && add 5000 src/generated/api.ts && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a path GENERATED_PATHS_REGEX matches does not count' claude_push size-generated
on size-lock && add 3000 services/api/uv.lock && add 3000 web/package-lock.json && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a nested lockfile does not count' claude_push size-lock
on size-renamed && git mv src/fixtures/a.json src/fixtures/b.json && seq 2001 3500 >>src/fixtures/b.json && git add src/fixtures/b.json && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a renamed fixture counts under its new name (dir/{old => new})' claude_push size-renamed
expect 0 'the CI verdict skips it too' ci_verdict 'warn 700 800'
on size-snapshot && mkdir -p src/__snapshots__ && git mv src/lib/a.snap src/__snapshots__/a.snap && seq 2001 3500 >>src/__snapshots__/a.snap && git add src/__snapshots__/a.snap && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a file moved into __snapshots__ counts under its new name' claude_push size-snapshot
on size-moved && mkdir -p tests && git mv big tests/big.txt
seq 2001 3500 >>tests/big.txt && git add tests/big.txt && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'a moved file counts under its new path (old => new)' claude_push size-moved
on size-deleted && git rm -q big && add 700 src/a.ts && git commit -qm a && stamp HEAD
expect 0 'deleted lines do not count' claude_push size-deleted
# A branch or tag named origin/main, here before the 2,000-line commit, resolves before
# refs/remotes/origin/main.
on size-decoy && add 10 src/a.ts && git commit -qm a && stamp HEAD
git branch origin/main "$base^"
expect 0 'a local origin/main branch does not move the size cap base' claude_push size-decoy
expect 1 'so 10 lines do not warn' warned
git branch -q -D origin/main && git tag origin/main "$base^"
expect 0 'nor does a tag named origin/main' claude_push size-decoy-tag
git tag -d origin/main >/dev/null
git checkout -q feature/z

# verify stamps only a clean tree whose gates all pass, unchanged by the run.
mkdir "$T/bin"
cat >"$T/bin/stub" <<'EOF'
#!/bin/bash
echo "$1" >>"$STUB_CALLS"
[ "$1" = "${STUB_TOUCH:-}" ] && echo x >touched
[ "$1" = "${STUB_COMMIT:-}" ] && git commit -q --allow-empty -m mid-run
[ "$1" != "${STUB_FAIL:-}" ]
EOF
chmod +x "$T/bin/stub"
verify() { PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh"; }
green() { grep -q "^$(git rev-parse "HEAD^{tree}") " .git/verify-stamp; }
commit e
echo x >untracked
export STUB_CALLS="$T/calls"
expect 1 'verify on a dirty tree' verify
expect 1 'a dirty tree runs no gate' test -e "$STUB_CALLS"
expect 1 'a dirty tree is not stamped' green
git config status.showUntrackedFiles no
expect 1 'an untracked file counts whatever status.showUntrackedFiles says' verify
git config --unset status.showUntrackedFiles
rm untracked
export STUB_FAIL=test
expect 1 'verify with a red gate' verify
expect 1 'a red gate writes no stamp' green
unset STUB_FAIL
export STUB_TOUCH=test
expect 1 'verify when the tree changes mid-run' verify
expect 1 'a changed tree is not stamped' green
unset STUB_TOUCH
rm touched
export STUB_COMMIT=test
expect 1 'verify when HEAD moves mid-run' verify
expect 1 'a moved HEAD is not stamped' green
unset STUB_COMMIT
expect 0 'verify on a clean tree' verify
expect 0 'a green run stamps the tree' green
expect 0 'and the push passes' git push -q origin feature/z
expect 1 'a branch that changed no source file runs no lint' grep -qx lint "$STUB_CALLS"

# verify's base is refs/remotes/origin/main, whatever a branch named origin/main holds. From
# $base the diff would hold a source file, and so run the lint.
add 1 src/a.ts && git commit -qm source
git update-ref refs/remotes/origin/main HEAD
git branch origin/main "$base"
: >"$STUB_CALLS"
expect 0 'verify with a local origin/main branch' verify
expect 1 'diffs from refs/remotes/origin/main, so runs no lint' grep -qx lint "$STUB_CALLS"
git branch -q -D origin/main

if [ "$failures" -eq 0 ]; then
  echo "all cases passed"
else
  echo "$failures case(s) failed"
  exit 1
fi
