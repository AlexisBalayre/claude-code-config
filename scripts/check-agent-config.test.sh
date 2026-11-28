#!/usr/bin/env bash
# Hand-run cases for check-agent-config.sh: `bash scripts/check-agent-config.test.sh`
# (scripts/verify.sh also runs it, as one of the harnesses the script finds).
# Each case commits a change in a throwaway repo whose own harness,
# scripts/mark.test.sh, appends a line to $MARK each time it runs.

set -uo pipefail

CHECK="$(cd "$(dirname "$0")" && pwd)/check-agent-config.sh"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0
n=0
BROKEN='if true; then'

# The fixtures must not inherit the user's hooksPath or commit signing.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
export MARK="$T/mark"

put() { mkdir -p "$repo/$(dirname "$1")" && printf '%s\n' "$2" >"$repo/$1"; }
commit() { git -C "$repo" add -A && git -C "$repo" commit -qm "$1"; }

new_repo() {
  n=$((n + 1))
  repo="$T/r$n"
  : >"$MARK"
  git init -q "$repo"
  put README.md base
  put scripts/mark.test.sh 'echo "held=${GATE_LOCK_HELD:-}${AGENT_REPO:-}${AGENT_GH_REPO:-}" >>"$MARK"'
  commit init
}

check() { (cd "$repo" && bash "$CHECK" HEAD^1 HEAD) >"$T/check" 2>&1; }
affected() { (cd "$repo" && bash "$CHECK" --affected HEAD^1 HEAD) >"$T/check" 2>&1; }
ran() { [ -s "$MARK" ]; }
# A harness that appends its own name to $MARK, with an optional extra line.
harness() { put "$1" "echo ${1##*/} >>\"\$MARK\"${2:+$'\n'$2}"; }
only() { [ "$(sort "$MARK" | tr '\n' ' ')" = "$* " ]; }

expect() {
  local want=$1 name=$2 got
  shift 2
  "$@" >/dev/null 2>&1
  got=$?
  [ "$got" -eq 0 ] || got=1
  if [ "$got" -ne "$want" ]; then
    printf 'FAIL want=%s got=%s  %s\n' "$want" "$got" "$name"
    sed 's/^/    /' "$T/check"
    failures=$((failures + 1))
  fi
}

new_repo; put a.sh "$BROKEN"; commit next
expect 1 'a changed .sh with a syntax error' check

new_repo; put a.sh "$BROKEN"; commit broken; put README.md edited; commit next
expect 0 'a broken .sh the range leaves unchanged' check

new_repo; put scripts/x $'#!/usr/bin/env bash\nif true; then'; commit next
expect 1 'a changed extensionless script with a bash shebang' check

new_repo; put scripts/data $'name\nif true; then'; commit next
expect 0 'an extensionless file with no shell shebang' check
expect 0 'runs the harnesses, as any file under scripts/ does' ran

new_repo; put .claude/hooks/README.md text; commit next
expect 0 'a non-shell file under .claude/hooks/' check
expect 1 'runs no harness' ran

new_repo; put .github/workflows/ci.yml 'name: ci'; commit next
expect 0 'a workflow change' check
expect 0 'runs the harnesses' ran

new_repo; put docs/a.md text; commit next
expect 0 'a Markdown-only change' check
expect 1 'runs no harness' ran

new_repo; put scripts/ok.sh 'echo ok'; commit next
expect 0 'a valid script under scripts/' check
expect 0 'runs the harnesses' ran

new_repo; put scripts/fail.test.sh 'exit 1'; commit harness; put scripts/ok.sh 'echo ok'; commit next
expect 1 'a failing harness' check

# Three harnesses that each wait for the other two to start pass only when they run at once.
new_repo
for h in a b c; do put "scripts/$h.test.sh" "touch '$T/$h'; for _ in \$(seq 50); do [ -e '$T/a' ] && [ -e '$T/b' ] && [ -e '$T/c' ] && exit 0; sleep 0.1; done; exit 1"; done
put scripts/z.test.sh $'echo first\necho second\nexit 1'
# Sorts first and finishes last, so a print in completion order puts it out of place.
put scripts/0.test.sh $'sleep 1\nexit 1'
commit next
expect 1 'harnesses at once, two failing' check
expect 0 'pass together' test "$(grep -c 'test.sh passed' "$T/check")" -eq 4
expect 0 'and print the failing one whole' bash -c "grep -A2 -E 'z.test.sh failed \([0-9]+s\):' '$T/check' | grep -qx '    second'"
expect 0 'and print the seconds of each passing one' test "$(grep -cE 'test\.sh passed \([0-9]+s\)$' "$T/check")" -eq 4
expect 0 'in harness order' test "$(grep -oE '[a-z0-9]+\.test\.sh (passed|failed)' "$T/check" | tr '\n' ' ')" = '0.test.sh failed a.test.sh passed b.test.sh passed c.test.sh passed mark.test.sh passed z.test.sh failed '
rm -f "$T/a" "$T/b" "$T/c"

new_repo; put .claude/hooks/new.test.sh 'echo hooks >>"$MARK"'; commit next
expect 0 'a new harness under .claude/hooks/' check
expect 0 'is found and run' grep -qx hooks "$MARK"

new_repo; put .claude/hooks/h.sh 'echo ok'; commit next
export GATE_LOCK_HELD=4242 AGENT_REPO=/real AGENT_GH_REPO=o/r
expect 0 'a hook change run under a held gate lock and a lead-tools override' check
unset GATE_LOCK_HELD AGENT_REPO AGENT_GH_REPO
expect 0 'keeps GATE_LOCK_HELD and the overrides out of the harnesses' grep -qx 'held=' "$MARK"

new_repo; put scripts/gone.sh 'echo ok'; commit gone; git -C "$repo" rm -q scripts/gone.sh; commit next
expect 0 'a deleted script' check
expect 0 'runs the harnesses' ran

new_repo; put tools/x.test.sh 'echo ok'; commit next
expect 0 'a .test.sh outside scripts/ and .claude/hooks/' check
expect 0 'runs the harnesses' ran

LT=.claude/skills/setup-agent-env/lead-tools
new_repo; put $LT/l.test.sh 'echo lead >>"$MARK"'; commit harness; put $LT/README.md text; commit next
expect 0 'a non-shell change under the lead tools' check
expect 0 'runs the lead-tools harnesses' grep -qx lead "$MARK"
expect 1 'and no other harness' grep -q held= "$MARK"

new_repo; put $LT/l.test.sh 'echo lead >>"$MARK"'; commit harness; put scripts/ok.sh 'echo ok'; commit next
expect 0 'a change under scripts/' check
expect 1 'runs no lead-tools harness' grep -qx lead "$MARK"
expect 0 'runs the scripts/ harnesses' grep -q held= "$MARK"

new_repo; put $LT/l.test.sh 'echo lead >>"$MARK"'; commit harness; put $LT/README.md text; put scripts/ok.sh 'echo ok'
commit next
expect 0 'a change under both scripts/ and the lead tools' check
expect 0 'runs the lead-tools harnesses' grep -qx lead "$MARK"
expect 0 'and the scripts/ harnesses' grep -q held= "$MARK"

new_repo; put $LT/l.test.sh 'echo lead >>"$MARK"'; commit harness; put .claude/skills/setup-agent-env/params/repo.env X=1
commit next
expect 0 'a change to the lead tools params' check
expect 0 'runs the lead-tools harnesses' grep -qx lead "$MARK"

new_repo; put $LT/fail.test.sh 'exit 1'; commit harness; put $LT/README.md text; commit next
expect 1 'a failing lead-tools harness' check

new_repo; put $LT/l.test.sh 'echo lead >>"$MARK"'; commit harness; put .claude/skills/setup-agent-env/SKILL.md text
commit next
expect 0 'a change elsewhere under setup-agent-env' check
expect 1 'runs no lead-tools harness' grep -qx lead "$MARK"

new_repo; put scripts/conventions-rules.test.sh 'echo rules >>"$MARK"'; commit harness; put docs/conventions/core.md text
commit next
expect 0 'a conventions doc change' check
expect 0 'runs the conventions rules harness' grep -qx rules "$MARK"
expect 1 'and no other harness' grep -q held= "$MARK"

new_repo; put scripts/conventions-rules.test.sh 'echo rules >>"$MARK"'; commit harness; put scripts/ok.sh 'echo ok'; put .claude/rules/core.md text
commit next
expect 0 'a rule file change beside a scripts/ change' check
expect 0 'runs the rules harness once, in the full set' test "$(grep -cx rules "$MARK")" -eq 1

new_repo; put a.sh "$BROKEN"; put b.sh "$BROKEN"; commit next
expect 1 'two broken scripts' check
expect 0 'are both reported' test "$(grep -c 'bash -n failed' "$T/check")" -eq 2

# --affected, verify's run, keeps only the harnesses the range can affect; each case commits one change.
change() { put "$1" "${2:-echo ok}"; commit "$1"; : >"$MARK"; }
new_repo
harness scripts/a.test.sh; harness scripts/pre-push.test.sh; harness scripts/b.test.sh ': lib.sh'
harness scripts/c.test.sh; put scripts/c.test.mjs '// scripts/helper.sh'
harness scripts/conventions-rules.test.sh
harness $LT/l.test.sh ': lib.sh'; harness $LT/m.test.sh
commit harnesses
change scripts/a.sh
expect 0 'a script under --affected' affected
expect 0 'runs only its harness' only a.test.sh
change scripts/pre-push $'#!/bin/bash\necho ok'
expect 0 'an extensionless script' affected
expect 0 'runs only its harness' only pre-push.test.sh
change scripts/lib.sh
expect 0 'a script with no harness of its own' affected
expect 0 'runs only the harness that names it' only b.test.sh
change scripts/helper.sh
expect 0 'a script a harness helper names' affected
expect 0 'runs only the harness whose helper names it' only c.test.sh
harness scripts/b.test.sh; commit b; : >"$MARK"
expect 0 'a changed harness' affected
expect 0 'runs only itself' only b.test.sh
change .github/workflows/ci.yml 'name: ci'
expect 0 'a workflow' affected
expect 1 'runs no harness' ran
for doc in docs/conventions/core.md .claude/rules/core.md; do
  change "$doc" text
  expect 0 "$doc" affected
  expect 0 'runs only the rules harness' only conventions-rules.test.sh
done
change .claude/skills/setup-agent-env/params/repo.env X=2
expect 0 'a lead-tools param' affected
expect 0 'runs every lead-tools harness' only l.test.sh m.test.sh
change $LT/lib.sh
expect 0 'a lead-tools file' affected
expect 0 'runs only the lead-tools harness that names it' only l.test.sh
change scripts/other.sh
expect 0 'a script nothing names' affected
expect 1 'runs no harness' ran
change scripts/a.sh 'echo again'
expect 0 'a script without --affected' check
expect 0 'runs every scripts/ harness' grep -qx b.test.sh "$MARK"

# Five harnesses, four at a time: the one with the shortest last time starts alone, after the rest.
# z has no time yet, so it counts as the longest and is the one a run in glob order would start last.
new_repo
for h in a b c mark z; do harness "scripts/$h.test.sh" 'sleep 2'; done
commit harnesses
times="$repo/.git/check-agent-config.times"
mkdir -p "$times"
# No time is 0, so z would start last if an unknown time counted as 0 too.
echo 2 >"$times/scripts_a.test.sh"; echo 4 >"$times/scripts_b.test.sh"; echo 3 >"$times/scripts_c.test.sh"
echo 1 >"$times/scripts_mark.test.sh"
change scripts/ok.sh
expect 0 'harnesses with known times' check
expect 0 'start longest first' test "$(tail -n1 "$MARK")" = mark.test.sh
expect 0 'and record each time' grep -qx '[0-9]*' "$times/scripts_z.test.sh"

if [ "$failures" -eq 0 ]; then
  echo "all cases passed"
else
  echo "$failures case(s) failed"
  exit 1
fi
