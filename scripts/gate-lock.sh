#!/bin/bash
# gate-lock.sh <command...>: runs the command under the repo's one gate lock, so heavy runs
# from every worktree (scripts/verify.sh, any other command) go one at a time.
# Waits on a mkdir lock in the git common dir, which every worktree shares, and releases it on
# exit, failure included. A lock whose holder pid is gone is taken over. Waiters queue in
# gate-lock.queue beside it and take the lock in arrival order. Exit status is the
# command's. GATE_LOCK_LABEL names the holder for waiters (default: the branch).
set -u
# bash 3.2 exits 0 on a set -u error inside $((...)): a run that dies before its command must
# not read as a pass, or verify would stamp a tree whose gates never ran.
trap 'exit 1' EXIT
LOCK="$(git rev-parse --path-format=absolute --git-common-dir)/gate-lock"
# Every mkdir failure below reads as contention, so a lock that can never be made (git older
# than 2.31 prints --path-format back, an unwritable .git) would wait forever.
[ -w "${LOCK%/*}" ] || { echo "gate-lock: cannot create $LOCK" >&2; exit 1; }
# A detached HEAD has no branch: name it by its short sha.
label="${GATE_LOCK_LABEL:-$(git branch --show-current | grep . || git rev-parse --short HEAD)}"

# GNU first: GNU stat reads -f as --file-system and still prints a report, BSD stat rejects -c.
mtime() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null || date +%s; }
# A sandboxed waiter may not signal a holder outside its own sandbox, so kill -0 fails with EPERM
# on a live pid. A dead holder's pid reused by a root or other-user process fails the same way, so
# EPERM counts as alive only for 2 h, well past a verify run on a loaded machine.
max_hold=7200
alive() {
  local err
  err=$(LC_ALL=C kill -0 "$1" 2>&1) && return 0
  [[ $err == *"not permitted"* ]] && [ $(($(date +%s) - $(mtime "$LOCK/pid"))) -lt $max_hold ]
}

# A command run under the lock (verify inside a held lock, any nested gate-lock.sh call) passes
# straight through. A leaked GATE_LOCK_HELD only counts while its pid is alive
# and is the lock's holder, so it cannot skip the queue.
if [ -n "${GATE_LOCK_HELD:-}" ] && [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$GATE_LOCK_HELD" ] &&
  alive "$GATE_LOCK_HELD"; then
  exec "$@"
fi

QUEUE="$LOCK.queue"
mkdir -p "$QUEUE" || exit 1
# Seconds, since BSD date has no %N: arrivals within one second queue by pid.
me="$QUEUE/$(date +%s)-$$"
trap 'rm -f "$me"; exit 1' EXIT
# Prints how many live waiters queued before this one, removing the dead ones' entries. A waiter
# touches its entry on every poll, so a stale entry is dead whatever kill -0 says: EPERM on a
# waiter in another sandbox, or a killed waiter's pid reused.
count_ahead() {
  local e err n=0
  for e in $(ls "$QUEUE" 2>/dev/null | sort -t- -k1,1n -k2,2n); do
    [ "$QUEUE/$e" = "$me" ] && break
    if [ $(($(date +%s) - $(mtime "$QUEUE/$e"))) -le 60 ] &&
      { err=$(LC_ALL=C kill -0 "${e#*-}" 2>&1) || [[ $err == *"not permitted"* ]]; }; then
      n=$((n + 1))
    else
      rm -f "$QUEUE/$e"
    fi
  done
  echo $n
}

last_notice=0
while :; do
  touch "$me"
  ahead=$(count_ahead)
  # The pid write is part of the acquire: it fails when a waiter cleared the fresh lock as empty. The label goes in
  # first, so a waiter that reads the pid finds the label too.
  [ "$ahead" -eq 0 ] && { mkdir "$LOCK" && echo "$label" >"$LOCK/label" && echo $$ >"$LOCK/pid"; } 2>/dev/null && break
  holder=$(cat "$LOCK/pid" 2>/dev/null || true)
  # Only the head clears a lock, so no waiter behind it can clear the lock the head just took. Re-read
  # before clearing: the holder may have released and another run taken the lock.
  if [ "$ahead" -eq 0 ] && [ -n "$holder" ] && ! alive "$holder" && [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$holder" ]; then
    echo "gate-lock: taking over the stale lock of pid $holder ($(cat "$LOCK/label" 2>/dev/null))" >&2
    rm -f "$LOCK/pid" "$LOCK/label"
    rmdir "$LOCK" 2>/dev/null
    continue
  fi
  now=$(date +%s)
  # No pid for over a minute: a holder killed between mkdir and the pid write, or between the
  # release's rm and rmdir.
  if [ "$ahead" -eq 0 ] && [ -z "$holder" ] && [ $((now - $(mtime "$LOCK"))) -gt 60 ]; then
    echo "gate-lock: clearing an empty lock left by a killed holder" >&2
    rm -f "$LOCK/label"
    rmdir "$LOCK" 2>/dev/null
    continue
  fi
  if [ -n "$holder" ] && [ $((now - last_notice)) -ge 60 ]; then
    since=$(mtime "$LOCK/pid")
    name=$(cat "$LOCK/label" 2>/dev/null || echo "pid $holder")
    # A holder that released since its pid was read took its label and pid file with it, which would print its pid
    # and mtime's fallback to now, a negative hold: retry the acquire instead.
    [ "$(cat "$LOCK/pid" 2>/dev/null)" = "$holder" ] || continue
    held=$((now - since))
    echo "gate-lock: waiting for $name (held $((held / 60))m$((held % 60))s, queue position $((ahead + 1)))" >&2
    # Past a normal run's length the holder may be hung, or a pid reused by a live process.
    if [ "$held" -gt 1800 ]; then
      echo "gate-lock: pid $holder (user $(ps -o user= -p "$holder" 2>/dev/null || echo unknown)) holds the lock; if it runs no gate, rm -rf $LOCK" >&2
    fi
    last_notice=$now
  fi
  sleep 2
done
rm -f "$me"
# Release only a lock this process still holds: two waiters taking over one dead holder can race,
# and the loser must not delete the winner's lock on its way out.
trap '[ "$(cat "$LOCK/pid" 2>/dev/null)" = $$ ] && { rm -f "$LOCK/pid" "$LOCK/label"; rmdir "$LOCK" 2>/dev/null; }' EXIT
GATE_LOCK_HELD=$$ "$@"
