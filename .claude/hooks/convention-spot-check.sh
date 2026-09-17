#!/usr/bin/env bash
# Stop hook: structural convention spot-check on the files this session changed,
# driven by the checks in .claude/spot-checks.tsv. Findings reach the model once
# per Stop cycle (exit 2); on the stop_hook_active re-run the hook is silent so a
# heuristic the model judged a false positive cannot loop. Exit-0 output never
# reaches the model, so "advisory" here means "shown once, then dropped".
set -uo pipefail

cd "${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}" 2>/dev/null || exit 0

CHECKS=.claude/spot-checks.tsv
grep -qvE '^[[:space:]]*(#|$)' "$CHECKS" 2>/dev/null || exit 0

if [ ! -t 0 ]; then
  INPUT=$(cat 2>/dev/null || true)
  printf '%s' "$INPUT" | grep -q '"stop_hook_active"[[:space:]]*:[[:space:]]*true' && exit 0
fi

# Untracked files included: a freshly written file is where a violation most often lands.
ALL_FILES=$(
  {
    git diff --name-only HEAD 2>/dev/null
    git diff --cached --name-only 2>/dev/null
    git ls-files --others --exclude-standard 2>/dev/null
  } | sort -u
)
[ -z "$ALL_FILES" ] && exit 0

WARNINGS=""

while IFS= read -r file; do
  [ -f "$file" ] || continue
  while IFS=$'\t' read -r path_re content_re message; do
    case "$path_re" in ''|'#'*) continue ;; esac
    [[ "$file" =~ $path_re ]] || continue
    if grep -qE -- "$content_re" "$file" 2>/dev/null; then
      WARNINGS+="  $file: $message\n"
    fi
  done < "$CHECKS"
done <<< "$ALL_FILES"

[ -z "$WARNINGS" ] && exit 0

{
  echo
  echo "Convention spot-check on files changed this session:"
  echo -e "$WARNINGS"
  echo "Fix each real finding; if one is a heuristic misfire, say so and stop again (this check stays silent on the re-run)."
} >&2
exit 2
