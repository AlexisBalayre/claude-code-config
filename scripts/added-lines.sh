#!/bin/bash
# Print the PR cap's verdict from <base> to <head> as one `pass|warn|fail <added>
# <fail-at>` line, then `added<TAB>path` for each production file changed.
# It warns at 400 and fails at 800, or above 1,000 when every changed file is
# *.md. Two callers: scripts/pre-push, which blocks a Claude Code push, and the
# PR Size workflow. Installed by scripts/setup.sh beside the hooks.
set -euo pipefail

# The installed copy lives in <git-common-dir>/hooks, so the profile is found from the checkout.
env="$(git rev-parse --show-toplevel)/.claude/project.env"
[ -f "$env" ] && . "$env"

# Paths the count skips: tests, fixtures, mocks, snapshots, migrations and
# lockfiles, plus the project's generated files (GENERATED_PATHS_REGEX).
skipped='(^|/)migrations/|(^|/)[^/]*(\.lock|-lock\.(json|yaml))$|(^|/)(tests?|__tests__|__mocks__|__snapshots__)/|(^|/)([^/]*-)?fixtures?/|\.(test|spec|mock)\.[a-z]+$|_test\.[a-z]+$|(^|/)test_[^/]*$'
[ -z "${GENERATED_PATHS_REGEX:-}" ] || skipped="$skipped|$GENERATED_PATHS_REGEX"

# A rename prints as `old => new` or `dir/{old => new}/rest`: match the new
# path. --no-renames would count a moved production file in full.
counted=$(git -c core.quotePath=false diff --numstat "$1" "$2" | skip="$skipped" awk -F'\t' '
  $1 == "-" { next }
  {
    p = $3
    if (match(p, /\{[^}]* => [^}]*\}/)) {
      mid = substr(p, RSTART + 1, RLENGTH - 2)
      sub(/.* => /, "", mid)
      p = substr(p, 1, RSTART - 1) mid substr(p, RSTART + RLENGTH)
      gsub(/\/\/+/, "/", p)
      sub(/^\//, "", p)
    } else if (index(p, " => ")) {
      sub(/.* => /, "", p)
    }
    if (p !~ ENVIRON["skip"]) print $1 "\t" p
  }')
added=$(awk -F'\t' '{ s += $1 } END { print s + 0 }' <<<"$counted")
fail_at=800
# awk, not grep: grep exits 1 on no match, which set -e would take for a failure.
if [ -z "$(git -c core.quotePath=false diff --name-only "$1" "$2" | awk '!/\.md$/')" ]; then
  fail_at=1001 # 1,001, not 1,000: the check below is -ge, and 1,000 passes
fi
if [ "$added" -ge "$fail_at" ]; then
  echo "fail $added $fail_at"
elif [ "$added" -ge 400 ]; then
  echo "warn $added $fail_at"
else
  echo "pass $added $fail_at"
fi
[ -z "$counted" ] || printf '%s\n' "$counted"
