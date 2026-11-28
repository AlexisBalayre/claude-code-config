#!/bin/bash
# Hand-run cases for check-dashes.sh: `bash tests/scripts/check-dashes.test.sh`.
# Each case builds a change in a throwaway repo (staged, or committed for the
# range case) and checks the exit code (0 = passes, 1 = blocked).
set -uo pipefail

CHECK="$(cd "$(dirname "$0")/../../scripts" && pwd)/check-dashes.sh"
EM=$(printf '\342\200\224')
EN=$(printf '\342\200\223')
failures=0

# Fresh repo with one commit: a.md holds a dashed line and a plain one.
new_repo() {
  # An empty $repo would aim every git call below at the caller's own checkout.
  repo=$(mktemp -d) || exit 1
  git -C "$repo" init -q
  git -C "$repo" config user.email t@example.com
  git -C "$repo" config user.name t
  git -C "$repo" config commit.gpgsign false
  printf 'old %s line\nplain line\n' "$EM" > "$repo/a.md"
  git -C "$repo" add a.md
  git -C "$repo" commit -qm init
}

expect() {
  local want=$1 name=$2 got
  shift 2
  (cd "$repo" && bash "$CHECK" "$@") >/dev/null 2>&1
  got=$?
  if [ "$got" -ne "$want" ]; then
    printf 'FAIL want=%s got=%s  %s\n' "$want" "$got" "$name"
    failures=$((failures + 1))
  fi
  rm -rf "$repo"
}

new_repo; printf 'new %s line\n' "$EM" >> "$repo/a.md"; git -C "$repo" add a.md
expect 1 'added line with an em dash' --cached

new_repo; printf 'pages 2%s5\n' "$EN" > "$repo/b.ts"; git -C "$repo" add b.ts
expect 1 'added line with an en dash, in a new file' --cached

new_repo; printf 'old %s line\nplain line, edited\n' "$EM" > "$repo/a.md"; git -C "$repo" add a.md
expect 0 'dash only on an unchanged neighbouring line' --cached

new_repo; printf 'plain line\n' > "$repo/a.md"; git -C "$repo" add a.md
expect 0 'dash only on a removed line' --cached

new_repo; printf '++ counter %s\n' "$EM" >> "$repo/a.md"; git -C "$repo" add a.md
expect 1 'added line starting with ++, which must not read as a file header' --cached

new_repo; printf 'new %s line\n' "$EN" >> "$repo/a.md"; git -C "$repo" commit -qam next
expect 1 'committed range, as CI passes it' HEAD^1 HEAD

if [ "$failures" -gt 0 ]; then
  echo "$failures case(s) failed"
  exit 1
fi
echo "All cases passed"
