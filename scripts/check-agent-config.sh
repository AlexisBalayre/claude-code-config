#!/bin/bash
# Fail when a commit range breaks a shell file or the shell harnesses: `bash -n` on every
# changed shell file, then, when any file under scripts/, tests/ or .github/workflows/, shell under
# .claude/hooks/ or any *.test.sh changed, every tests/scripts/*.test.sh and tests/hooks/*.test.sh. A change to the lead
# tools' repo copy or its params runs their *.test.sh too, and only such a change does: they take
# minutes, too long for every scripts/ change, so a change elsewhere that one of them reads goes
# untested until the next lead-tools change. A conventions doc or
# rule file change runs tests/scripts/conventions-rules.test.sh alone, sparing a doc edit the full set.
# --affected keeps, of those, only the harnesses the range can affect (affects() below); CI runs
# without it, so a harness that reaches a changed file only through another file still runs there.
# It reads the files on disk, so run it from a checkout of <head>.
#   scripts/check-agent-config.sh HEAD^1 HEAD                    ->  a PR merge commit's diff (CI)
#   scripts/check-agent-config.sh --affected <merge-base> HEAD   ->  scripts/verify.sh
set -euo pipefail

affected=false
[ "${1:-}" = --affected ] && { affected=true; shift; }
cd "$(git rev-parse --show-toplevel)"
changed=$(git diff --name-only --no-renames "${1:?usage: check-agent-config.sh [--affected] <base> <head>}" "${2:?}")
SHEBANG='^#!.*[/ ](ba)?sh([[:space:]]|$)'
failures=0
harnesses=false
globs=
rules=
SETUP=.claude/skills/setup-agent-env

while IFS= read -r file; do
  case $file in "$SETUP"/lead-tools/* | "$SETUP"/params/*) globs="$SETUP/lead-tools/*.test.sh" ;; esac
  case $file in docs/conventions/* | .claude/rules/*) rules=tests/scripts/conventions-rules.test.sh ;; esac
  case $file in scripts/* | tests/scripts/* | tests/hooks/* | .github/workflows/*) harnesses=true ;; esac
  case $file in
    *.sh) ;;
    # An extensionless file here (pre-commit, pre-push) is shell when its shebang says so. A
    # deleted one counts by name alone: nothing is left to read, and a harness may still call it.
    scripts/* | .claude/hooks/*)
      case ${file##*/} in *.*) continue ;; esac
      if [ -f "$file" ]; then
        first=
        IFS= read -r first <"$file" || true
        [[ $first =~ $SHEBANG ]] || continue
      fi
      ;;
    *) continue ;;
  esac
  case $file in .claude/hooks/* | *.test.sh) harnesses=true ;; esac
  if [ -f "$file" ] && ! bash -n "$file"; then
    echo "check-agent-config: bash -n failed on $file" >&2
    failures=$((failures + 1))
  fi
done <<<"$changed"

# A harness X.test.sh is affected when it changed, its subject (X.sh, or an extensionless X, in
# scripts/ for tests/scripts/, in .claude/hooks/ for tests/hooks/, else beside it) changed, or it
# or a helper beside it (X.test.*) names a changed file's basename as a word. Two
# reads go through no name: every conventions doc and rule file is conventions-rules.test.sh's
# input, and the lead-tools params reach their harnesses through lib.sh.
names=$(sed 's|.*/||' <<<"$changed")
affects() {
  local stem=${1%.test.sh} subject file
  subject=$stem
  case $stem in
    tests/scripts/*) subject=scripts/${stem##*/} ;;
    tests/hooks/*) subject=.claude/hooks/${stem##*/} ;;
  esac
  while IFS= read -r file; do
    case $file in
      "$1" | "$subject".sh | "$subject") return 0 ;;
      docs/conventions/* | .claude/rules/*) [ "$1" = "$rules" ] && return 0 ;;
      "$SETUP"/params/*) case $1 in "$SETUP"/lead-tools/*) return 0 ;; esac ;;
    esac
  done <<<"$changed"
  grep -qwFf <(echo "$names") "$stem".test.*
}

# tests/scripts/*.test.sh already holds the rules harness, so it joins only when the full set is off.
if $harnesses; then globs="tests/scripts/*.test.sh tests/hooks/*.test.sh $globs"; else globs="$globs $rules"; fi
runs=$(mktemp -d "${TMPDIR:-/tmp}/check-agent-config.XXXXXX")
trap 'rm -rf "$runs"' EXIT
list=()
for harness in $globs; do
  [ -f "$harness" ] || continue
  $affected && ! affects "$harness" && continue
  list+=("$harness")
done
# Longest first, by each harness's last time in this clone (an unknown one counts as longest), so the
# longest never starts last behind three others. Results still print in list order.
times="$(git rev-parse --git-common-dir)/check-agent-config.times"
mkdir -p "$times"
order=$(for i in "${!list[@]}"; do
  echo "$(cat "$times/$(tr / _ <<<"${list[$i]}")" 2>/dev/null || echo 999999) $i"
done | sort -rn | cut -d' ' -f2)
# Four at a time: run one by one, a lead-tools change takes many minutes. Each harness keeps its own
# output file, printed whole and in order below. A caller under a held gate lock must not pass its
# holder pid to the harnesses, whose gate-lock cases build and hold their own. AGENT_REPO and
# AGENT_GH_REPO would point a lead-tools suite's scratch REPO at the real one (lib.sh). GNU xargs runs its command once on empty input, hence the guard.
[ "${#list[@]}" -eq 0 ] ||
  for i in $order; do printf '%s\n%s\n' "$runs/$i" "${list[$i]}"; done |
  xargs -P 4 -n 2 bash -c \
    'env -u GATE_LOCK_HELD -u AGENT_REPO -u AGENT_GH_REPO bash "$2" >"$1.out" 2>&1 </dev/null; echo $? $SECONDS >"$1.rc"' _
for i in "${!list[@]}"; do
  rc= secs=
  read -r rc secs 2>/dev/null <"$runs/$i.rc" || true
  [ -z "$secs" ] || echo "$secs" >"$times/$(tr / _ <<<"${list[$i]}")"
  if [ "$rc" = 0 ]; then
    echo "check-agent-config: ${list[$i]} passed (${secs}s)"
  else
    echo "check-agent-config: ${list[$i]} failed (${secs}s):" >&2
    sed 's/^/    /' "$runs/$i.out" >&2
    failures=$((failures + 1))
  fi
done

[ "$failures" -eq 0 ] || { echo "check-agent-config: $failures failure(s)" >&2; exit 1; }
