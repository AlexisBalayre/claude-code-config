#!/usr/bin/env bash
# Hand-run cases for scripts/gate-lock.sh and its callers (scripts/verify.sh takes the lock,
# scripts/pre-commit does not): `bash scripts/gate-lock.test.sh`. Every case runs in a throwaway repo
# against stub commands, so nothing here touches this checkout's lock.

set -uo pipefail

SCRIPTS=$(cd "$(dirname "$0")" && pwd)
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0

# The fixtures must not inherit the user's hooksPath or commit signing.
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

check() {
  local name=$1
  shift
  if ! "$@"; then
    printf 'FAIL  %s\n' "$name"
    sed 's/^/    /' "$T/err" 2>/dev/null
    failures=$((failures + 1))
  fi
}

# macOS has no `timeout`: a call still running after 30 s counts as hung and is killed. Pass the
# script, not gl: a backgrounded function runs in a subshell, and killing that orphans the
# gate-lock.sh child, which keeps the caller's stdout open.
within30() {
  "$@" 2>"$T/err" &
  local pid=$!
  (sleep 30 && kill "$pid") >/dev/null 2>&1 &
  local watchdog=$!
  wait "$pid"
  local rc=$?
  kill "$watchdog" 2>/dev/null
  wait "$watchdog" 2>/dev/null
  return "$rc"
}

# A killed case can leave its lock or a waiter behind: clear the lock and let the waiter finish.
reset() {
  rm -rf "$LOCK" "$LOCK.queue"
  wait
  rm -rf "$LOCK" "$LOCK.queue" "$T/go"
  : >"$T/polls"
  : >"$T/log"
  : >"$T/err"
}

gl() { "$SCRIPTS/gate-lock.sh" "$@"; }
# Waits for a backgrounded gl to hold the lock: its git calls take an unknown time on a slow host.
locked() {
  local i=0
  until [ -s "$LOCK/pid" ] || [ $((i += 1)) -gt 100 ]; do sleep 0.1; done
}
# A holder's script that lets go once the waiter's $T/err shows the text, or after 20 s against a hang: a fixed
# sleep can end before a waiter slowed by load reaches its notice.
until_err() { echo "for _ in \$(seq 200); do grep -qs '$1' '$T/err' && break; sleep 0.1; done"; }
before() { [ "$(grep -n "^$1" "$T/log" | head -1 | cut -d: -f1)" -lt "$(grep -n "^$2" "$T/log" | head -1 | cut -d: -f1)" ]; }

git init -q -b feature/x "$T/w"
cd "$T/w" || exit 1
echo a >f && git add f && git commit -qm a
LOCK="$T/w/.git/gate-lock"
reset

GATE_LOCK_LABEL=first gl bash -c "echo s1 >>'$T/log'; $(until_err 'gate-lock: waiting'); echo e1 >>'$T/log'" &
locked
gl bash -c "echo s2 >>'$T/log'; echo e2 >>'$T/log'" 2>"$T/err"
wait
check 'two holders serialise' test "$(tr '\n' ' ' <"$T/log")" = "s1 e1 s2 e2 "
check 'a waiter prints the holder label' grep -q 'gate-lock: waiting for first (held 0m' "$T/err"
check 'and gives a short hold no rm -rf advice' bash -c "! grep -q 'rm -rf' '$T/err'"
check 'and its queue position' grep -q 'queue position 1)' "$T/err"
reset

# The waiters' sleep logs their label, so a case can see each 2 s poll.
mkdir "$T/poll"
printf '#!/bin/bash\necho "$GATE_LOCK_LABEL" >>"%s/polls"\nexec %s "$@"\n' "$T" "$(command -v sleep)" >"$T/poll/sleep"
chmod +x "$T/poll/sleep"
wait_as() { GATE_LOCK_LABEL=$1 PATH="$T/poll:$PATH" "$SCRIPTS/gate-lock.sh" sh -c "echo $1 >>'$T/log'" 2>/dev/null & }
polled() {
  local i=0
  until [ "$(grep -cx "$1" "$T/polls")" -ge "${2:-1}" ] || [ $((i += 1)) -gt 200 ]; do sleep 0.05; done
}

# The holder lets go just after A's poll, and B and C poll at other points of the 2 s cycle, so they poll first: only
# the queue keeps the arrival order. Each starts over 1 s after the last one's first poll, so in a later second.
GATE_LOCK_LABEL=holder gl bash -c "for _ in \$(seq 200); do [ -e '$T/go' ] && break; sleep 0.1; done
  n=\$(grep -cx A '$T/polls')
  for _ in \$(seq 200); do [ \"\$(grep -cx A '$T/polls')\" -gt \"\$n\" ] && break; sleep 0.05; done" &
locked
wait_as A
polled A
sleep 1.1
wait_as B
polled B
sleep 1.1
wait_as C
polled C
touch "$T/go"
wait
check 'waiters take the lock in their arrival order' test "$(tr '\n' ' ' <"$T/log")" = "A B C "
reset

GATE_LOCK_LABEL=holder gl bash -c "for _ in \$(seq 200); do [ -e '$T/go' ] && break; sleep 0.1; done" &
locked
wait_as A
a=$!
polled A
sleep 1.1
wait_as B
polled B
{ kill -9 "$a" && wait "$a"; } 2>/dev/null
touch "$T/go"
for _ in $(seq 80); do grep -qx B "$T/log" && break; sleep 0.1; done
check 'a waiter killed while queued does not hold up the next' grep -qx B "$T/log"
check 'which drops its entry' test -z "$(ls "$LOCK.queue")"
reset

# pid 1 is alive and not this user's: kill -0 fails with EPERM, as on a waiter in another sandbox.
mkdir "$LOCK.queue" && touch "$LOCK.queue/1000000000-1"
"$SCRIPTS/gate-lock.sh" true 2>"$T/err" &
waiter=$!
sleep 3
check 'a fresh entry whose pid kill -0 may not signal keeps its place' kill -0 "$waiter"
{ kill "$waiter" && wait "$waiter"; } 2>/dev/null
reset

mkdir "$LOCK.queue" && touch -t 202601010000 "$LOCK.queue/1000000000-1"
check 'an entry not touched for a minute is dropped' within30 "$SCRIPTS/gate-lock.sh" true
reset

# Behind a live holder, the waiter's own entry is the only one.
sleep 30 >/dev/null 2>&1 &
old=$!
mkdir "$LOCK" && echo "$old" >"$LOCK/pid"
"$SCRIPTS/gate-lock.sh" true 2>"$T/err" &
waiter=$!
for _ in $(seq 100); do [ -n "$(ls "$LOCK.queue" 2>/dev/null)" ] && break; sleep 0.1; done
touch -t 202601010000 "$LOCK.queue/"*
for _ in $(seq 100); do [ -n "$(find "$LOCK.queue" -type f -mmin -1)" ] && break; sleep 0.1; done
check 'a waiter touches its queue entry on every poll' test -n "$(find "$LOCK.queue" -type f -mmin -1)"
{ kill "$waiter" && wait "$waiter"; } 2>/dev/null
check 'a terminated waiter drops its queue entry' test -z "$(ls "$LOCK.queue")"
kill "$old"
wait "$old" 2>/dev/null
reset

mkdir "$LOCK.queue" && touch "$LOCK.queue/9999999999-1"
check 'a later arrival with a lower pid queues behind' within30 "$SCRIPTS/gate-lock.sh" true
reset

# A 9-digit epoch is older than a 10-digit one: the order is numeric, not by name.
sleep 30 >/dev/null 2>&1 &
old=$!
mkdir "$LOCK" && echo "$old" >"$LOCK/pid"
mkdir "$LOCK.queue" && touch "$LOCK.queue/999999999-1"
"$SCRIPTS/gate-lock.sh" true 2>"$T/err" &
waiter=$!
for _ in $(seq 200); do grep -qs 'queue position 2)' "$T/err" && break; sleep 0.1; done
check 'an earlier arrival keeps its place whatever its digits' kill -0 "$waiter"
check 'and the waiter behind it reports position 2' grep -q 'queue position 2)' "$T/err"
{ kill "$waiter" && wait "$waiter"; } 2>/dev/null
kill "$old"
wait "$old" 2>/dev/null
reset

mkdir "$LOCK" && sh -c 'echo $$' >"$LOCK/pid"
mkdir "$LOCK.queue" && touch "$LOCK.queue/999999999-1"
"$SCRIPTS/gate-lock.sh" true 2>"$T/err" &
waiter=$!
sleep 3
check 'a waiter behind the head leaves a dead holder lock to it' test -s "$LOCK/pid"
{ kill "$waiter" && wait "$waiter"; } 2>/dev/null
reset

gl false 2>"$T/err"
check 'the exit status is the command one' test $? -eq 1
check 'a failing command releases the lock' test ! -e "$LOCK"
reset

mkdir "$LOCK" && sh -c 'echo $$' >"$LOCK/pid"
check 'a dead-pid lock is taken over' within30 "$SCRIPTS/gate-lock.sh" true
reset

mkdir "$LOCK" && touch -t 202601010000 "$LOCK"
check 'an empty lock older than 60 s is cleared' within30 "$SCRIPTS/gate-lock.sh" true
reset

# A holder killed between its label write and its pid write.
mkdir "$LOCK" && echo first >"$LOCK/label" && touch -t 202601010000 "$LOCK"
check 'an old lock holding only a label is cleared' within30 "$SCRIPTS/gate-lock.sh" true
reset

# The holder's lock taken over mid-run (two waiters racing on one dead holder): its exit must
# leave the new holder's lock alone.
gl bash -c "for _ in \$(seq 200); do [ -e '$T/go' ] && break; sleep 0.1; done" &
locked
sleep 5 >/dev/null 2>&1 &
taker=$!
echo "$taker" >"$LOCK/pid"
touch "$T/go"
wait %1
check 'an exiting holder keeps a lock taken over from it' test "$(cat "$LOCK/pid" 2>/dev/null)" = "$taker"
kill "$taker"
wait "$taker" 2>/dev/null
reset

# GNU stat reads -f as --file-system: it prints a report and fails, and accepts -c.
mkdir "$T/gnu"
cat >"$T/gnu/stat" <<'EOF2'
#!/bin/bash
if [ "$1" = -c ]; then echo 0; else echo "  File: \"$3\""; exit 1; fi
EOF2
chmod +x "$T/gnu/stat"
mkdir "$LOCK" && touch -t 202601010000 "$LOCK"
within30 env PATH="$T/gnu:$PATH" "$SCRIPTS/gate-lock.sh" touch "$T/ran"
check 'an old empty lock is cleared under GNU stat' test -e "$T/ran"
reset

# Passes trivially on bash 4+: it guards the fail-closed trap against macOS /bin/bash 3.2.
mkdir "$T/junk"
printf '#!/bin/bash\necho junk\n' >"$T/junk/stat"
chmod +x "$T/junk/stat"
mkdir "$LOCK"
within30 env PATH="$T/junk:$PATH" "$SCRIPTS/gate-lock.sh" touch "$T/ran2"
check 'a run that dies before its command fails' test $? -ne 0 -a ! -e "$T/ran2"
reset

check 'a nested call under a live GATE_LOCK_HELD runs at once' within30 "$SCRIPTS/gate-lock.sh" "$SCRIPTS/gate-lock.sh" true
reset

mkdir "$LOCK" && echo 1 >"$LOCK/pid"
check 'a nested call whose holder kill -0 may not signal runs at once' within30 env GATE_LOCK_HELD=1 "$SCRIPTS/gate-lock.sh" true
check 'and leaves the lock to it' grep -qx 1 "$LOCK/pid"
reset

mkdir "$LOCK" && echo 1 >"$LOCK/pid" && touch -t 200001010000 "$LOCK/pid"
within30 env GATE_LOCK_HELD=1 "$SCRIPTS/gate-lock.sh" true
check 'a GATE_LOCK_HELD whose holder kill -0 may not signal past 2 h queues' grep -q 'taking over the stale lock of pid 1' "$T/err"
reset

GATE_LOCK_LABEL=holder gl bash -c "$(until_err 'gate-lock: waiting')" &
locked
# This shell's pid is alive but does not hold the lock.
GATE_LOCK_HELD=$$ gl true 2>"$T/err"
check 'a stale GATE_LOCK_HELD still queues' grep -q 'waiting for holder' "$T/err"
reset

# pid 1 is alive and not this user's, so kill -0 fails with EPERM, as on a holder outside the
# waiter's sandbox.
mkdir "$LOCK" && echo 1 >"$LOCK/pid"
"$SCRIPTS/gate-lock.sh" true 2>"$T/err" &
waiter=$!
sleep 3
check 'a waiter that may not signal a live holder keeps waiting' kill -0 "$waiter"
check 'and keeps its lock' grep -qx 1 "$LOCK/pid"
{ kill "$waiter" && wait "$waiter"; } 2>/dev/null
reset

# The same holder past 2 h: a pid reused by another user's process.
mkdir "$LOCK" && echo 1 >"$LOCK/pid" && touch -t 200001010000 "$LOCK/pid"
check 'a lock whose holder kill -0 may not signal is taken over past 2 h' within30 "$SCRIPTS/gate-lock.sh" true
reset

bash -c "$(until_err 'holds the lock')" >/dev/null 2>&1 &
old=$!
mkdir "$LOCK" && echo "$old" >"$LOCK/pid" && touch -t 200001010000 "$LOCK/pid"
within30 "$SCRIPTS/gate-lock.sh" true
check 'a waiter on a long-held lock names the holder pid and its user' grep -qF "pid $old (user " "$T/err"
# git resolves the fixture's symlinked tmp path, so match the lock path's tail.
check 'and the rm -rf line' grep -q 'rm -rf .*/w/.git/gate-lock$' "$T/err"
reset

# The holder releases between the waiter's pid read and its notice: this stat clears the lock at the notice's read of
# the pid's time, as that release does.
mkdir "$T/release"
printf '#!/bin/bash\nrm -f "%s/pid" "%s/label"; rmdir "%s" 2>/dev/null; exit 1\n' "$LOCK" "$LOCK" "$LOCK" >"$T/release/stat"
chmod +x "$T/release/stat"
sleep 30 >/dev/null 2>&1 &
old=$!
mkdir "$LOCK" && echo "$old" >"$LOCK/pid" && echo holder >"$LOCK/label"
within30 env PATH="$T/release:$PATH" "$SCRIPTS/gate-lock.sh" touch "$T/ran3"
check 'a waiter whose holder released before its notice prints none' bash -c "! grep -q 'waiting for' '$T/err'"
check 'and runs its command' test -e "$T/ran3"
kill "$old"
wait "$old" 2>/dev/null
reset

# A lock with no pid yet, as a holder's between its mkdir and its writes: a notice would name no holder. This stat
# counts its calls, and 3 take the waiter past its first notice under GNU and BSD stat alike.
mkdir "$T/count"
printf '#!/bin/bash\necho >>"%s/stats"\nexec %s "$@"\n' "$T" "$(command -v stat)" >"$T/count/stat"
chmod +x "$T/count/stat"
mkdir "$LOCK"
env PATH="$T/count:$PATH" "$SCRIPTS/gate-lock.sh" touch "$T/ran4" 2>"$T/err" &
waiter=$!
for _ in $(seq 200); do [ $(($(wc -l 2>/dev/null <"$T/stats" || echo 0))) -ge 3 ] && break; sleep 0.1; done
rmdir "$LOCK"
wait "$waiter"
check 'a waiter on a lock with no pid yet prints no notice' bash -c "! grep -q 'waiting for' '$T/err'"
check 'and takes the lock once it is free' test -e "$T/ran4"
reset

# No case can hold the gap between the acquire's two writes open, so this pins their order: a waiter that finds the
# pid finds the label.
check 'the acquire writes the label before the pid' \
  grep -qF 'mkdir "$LOCK" && echo "$label" >"$LOCK/label" && echo $$ >"$LOCK/pid"' "$SCRIPTS/gate-lock.sh"

git worktree add -q "$T/w2" -b feature/w2
check 'a worktree takes the lock in the git common dir' \
  bash -c "cd '$T/w2' && '$SCRIPTS/gate-lock.sh' test -d '$LOCK'"
reset

mkdir "$T/bin"
printf '#!/bin/bash\necho "$1" >>"%s/log"\necho "$*"\n[ "${STUB_FAIL:-}" != "$1" ]\n' "$T" >"$T/bin/stub"
chmod +x "$T/bin/stub"
# The holder lets go once the log shows every step named (lint by default), or after 20 s, so a
# verify that wrongly waits for the lock before those steps still finishes, and fails the order check.
hold() {
  local all= step
  for step in "${@:-lint}"; do all+="grep -q '^$step' '$T/log' && "; done
  gl bash -c "for _ in \$(seq 200); do ${all}break; sleep 0.1; done
    echo holder-done >>'$T/log'" &
  locked
}
last_line() { [ "$(tail -n 1 "$T/err")" = "$1" ]; }
no_suites() { ! grep -q -e '^typecheck' -e '^test' "$T/log"; }
stamped() { grep -q "^$(git rev-parse 'HEAD^{tree}') " .git/verify-stamp 2>/dev/null; }
unstamped() { ! stamped; }
env_file() {
  mkdir -p .claude
  printf 'SOURCE_EXTENSIONS="ts"\nLINT_CMD="stub lint"\nTYPECHECK_CMD="%s"\nTEST_CMD="%s"\n' "$1" "$2" >.claude/project.env
}

# With no origin/main every file counts as changed, so this harness runs under verify.
mkdir scripts && echo "echo harness >>'$T/log'" >scripts/mark.test.sh
env_file 'stub typecheck' 'stub test'
echo a >a.ts
git add -A && git commit -qm harness
hold harness lint
PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'a held lock lets verify run its harnesses first' before harness holder-done
check 'and its lint' before lint holder-done
check 'and makes its suites wait' before holder-done typecheck
check 'with the typecheck before the tests' before typecheck test
check 'and stamps the tree' stamped
rm -f .git/verify-stamp
reset

STUB_FAIL=lint PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'a red lint stops verify' test $? -ne 0
check 'and names it on the last line' last_line 'verify: red: lint'
check 'before its suites' no_suites
check 'and writes no stamp' unstamped
reset

STUB_FAIL=lint PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>&-
check 'a red lint whose output cannot print stops verify' test $? -ne 0
check 'before its suites' no_suites
check 'and writes no stamp' unstamped
reset

echo 'exit 1' >scripts/fail.test.sh && git add scripts && git commit -qm fail
PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'a failing harness stops verify' test $? -ne 0
check 'and names it on the last line' last_line 'verify: red: check-agent-config'
check 'before its suites' no_suites
check 'and writes no stamp' unstamped
reset

# A red lane step: a clean lint, so the lane runs, and the stub fails the first step alone.
git rm -q scripts/fail.test.sh && git commit -qm unfail
STUB_FAIL=typecheck PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'a red lane step stops verify' test $? -ne 0
check 'names the step on the last line' last_line 'verify: red: stub typecheck'
check 'before the tests' bash -c "! grep -q '^test' '$T/log'"
check 'and writes no stamp' unstamped
reset

echo b >b.ts && git add b.ts
hold
PATH="$T/bin:$PATH" bash "$SCRIPTS/pre-commit" >/dev/null 2>"$T/err"
wait
check 'the pre-commit lints its staged files without waiting for a held lock' before lint holder-done
check 'and runs no typecheck or tests' no_suites
git commit -qm staged
reset

# The cases above run with no origin/main (verify diffs from the empty tree); from here it exists.
git update-ref refs/remotes/origin/main HEAD
echo a >a.md && git add a.md && git commit -qm docs
PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'a branch that changed no source file runs no lint' bash -c "! grep -q '^lint' '$T/log'"
check 'and still runs its suites' before typecheck test
reset

env_file '' ''
git commit -qam 'no suites'
PATH="$T/bin:$PATH" bash "$SCRIPTS/verify.sh" >/dev/null 2>"$T/err"
check 'with no typecheck or test command verify runs no suite' no_suites
check 'and stamps the tree' stamped
reset

chmod a-w "$T/w/.git"
within30 "$SCRIPTS/gate-lock.sh" true
rc=$?
chmod u+w "$T/w/.git"
check 'an uncreatable lock fails at once' test "$rc" -eq 1
reset

if [ "$failures" -eq 0 ]; then
  echo "all cases passed"
else
  echo "$failures case(s) failed"
  exit 1
fi
