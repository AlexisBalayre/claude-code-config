#!/usr/bin/env bash
# Hand-run cases for git-safety.sh: `bash tests/hooks/git-safety.test.sh`.
# Each case feeds a Bash payload from a throwaway repo and checks the exit code
# (0 = allowed, 2 = blocked). The same cases run against the Rust port when its
# binary is built (`cargo build --release` in tools/hooks-rs), so the two cannot
# drift apart.
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
HOOK="${HOOK:-$ROOT/.claude/hooks/git-safety.sh}"
BIN="${BIN:-$ROOT/tools/hooks-rs/target/release/git-safety}"
TMP=$(mktemp -d)
trap 'rm -r "$TMP"' EXIT
fail=0

for branch in main feature/x; do
  repo="$TMP/${branch##*/}"
  git init -q -b "$branch" "$repo"
  mkdir "$repo/.claude"
  echo "GIT_TRUNK=main" >"$repo/.claude/project.env"
done

run() { # <impl: sh|rs> <branch dir> <payload>
  if [ "$1" = sh ]; then
    (cd "$TMP/$2" && CLAUDE_PROJECT_DIR="$TMP/$2" bash "$HOOK" <<<"$3" 2>/dev/null)
  else
    (cd "$TMP/$2" && CLAUDE_PROJECT_DIR="$TMP/$2" "$BIN" <<<"$3" 2>/dev/null)
  fi
}

check() { # <name> <want exit> <command> [branch dir, default x]
  local name=$1 want=$2 dir=${4:-x} payload got impl
  payload=$(jq -n --arg c "$3" '{tool_name: "Bash", tool_input: {command: $c}}')
  for impl in $IMPLS; do
    run "$impl" "$dir" "$payload"
    got=$?
    if [ "$got" = "$want" ]; then echo "ok   [$impl] $name"; else echo "FAIL [$impl] $name: want $want, got $got"; fail=1; fi
  done
}

IMPLS=sh
if [ -x "$BIN" ]; then IMPLS="sh rs"; else echo "skip Rust port: $BIN not built"; fi

check "plain command" 0 "ls -la src"
check "rm -rf" 2 "rm -rf build"
check "rm -r without force" 0 "rm -r build"
check "DROP TABLE" 2 "psql -c 'DROP TABLE users'"
check "force push" 2 "git push --force origin feature/x"
check "force push, short flag" 2 "git push -f origin feature/x"
check "force-with-lease" 2 "git push --force-with-lease origin feature/x"
check "reset --hard" 2 "git reset --hard HEAD~1"
check "reset --soft" 0 "git reset --soft HEAD~1"
check "checkout -b on trunk" 2 "git checkout -b feature/y" main
check "checkout -b off trunk" 0 "git checkout -b feature/y"
check "push to trunk" 2 "git push origin main"
check "push to trunk, refspec" 2 "git push origin HEAD:main"
check "push to trunk, full ref" 2 "git push origin HEAD:refs/heads/main"
check "delete trunk" 2 "git push origin :main"
check "push to trunk, then a chained command" 2 "git push origin main && echo done"
check "branch name that contains the trunk" 0 "git push origin feature/fix-main-red"
check "push, then a PR against trunk" 0 "git push -u origin feature/x && gh pr create --base main"
check "bare push on trunk" 2 "git push" main
check "bare push off trunk" 0 "git push"
check "bare push on a later line, on trunk" 2 "git add -A
git push" main
check "rule words split across lines" 0 "git push origin
main"
check "no command" 0 ""

for impl in $IMPLS; do
  run "$impl" x "not json"
  if [ $? = 2 ]; then echo "ok   [$impl] payload that is not JSON"; else echo "FAIL [$impl] payload that is not JSON"; fail=1; fi
  echo "GIT_TRUNK=develop" >"$TMP/x/.claude/project.env"
  run "$impl" x '{"tool_input":{"command":"git push origin develop"}}'
  if [ $? = 2 ]; then echo "ok   [$impl] trunk read from project.env"; else echo "FAIL [$impl] trunk read from project.env"; fail=1; fi
  echo "GIT_TRUNK=main" >"$TMP/x/.claude/project.env"
done

[ "$fail" = 0 ] && echo "All cases passed" || exit 1
