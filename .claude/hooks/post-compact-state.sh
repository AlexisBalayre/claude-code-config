#!/usr/bin/env bash
# SessionStart(compact) hook — re-injects the live state a compaction summary
# tends to lose or leave stale: the session's branch and changes, every
# worktree, the open PRs and whether the CI review covers their head.
# Read fresh here, after compaction, because SessionStart stdout is added to
# Claude's context while PreCompact output never reaches the summary.
# Every probe is best-effort: a missing tool or a failed call drops its line.
set -uo pipefail

INPUT=$(cat)
CWD=$(echo "$INPUT" | jq -r '.cwd // empty' 2>/dev/null)
cd "${CWD:-${CLAUDE_PROJECT_DIR:-.}}" 2>/dev/null || exit 0
git rev-parse --git-dir >/dev/null 2>&1 || exit 0

BRANCH=$(git branch --show-current 2>/dev/null)
echo "Live state after compaction (read now; trust it over the summary):"
echo "- cwd: $PWD on ${BRANCH:-a detached HEAD}"
CHANGES=$(git status --short 2>/dev/null | head -20)
[ -n "$CHANGES" ] && printf -- '- uncommitted here:\n%s\n' "$(printf '%s\n' "$CHANGES" | sed 's/^/    /')"

echo "- worktrees (path, branch, uncommitted files):"
git worktree list --porcelain 2>/dev/null |
  awk '/^worktree /{p=substr($0,10)} /^branch /{print p"\t"substr($0,19)} /^detached/{print p"\t(detached)"}' |
  while IFS=$'\t' read -r path branch; do
    printf '    %s  %s  %s\n' "$path" "$branch" "$(git -C "$path" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
  done

# The claude-review commit status sits on the PR's head commit, so "none" means
# the CI review has not covered the current head.
if command -v gh >/dev/null 2>&1; then
  PRS=$(gh pr list --state open --limit 20 --json number,headRefName,headRefOid,statusCheckRollup \
    --jq '.[] | "    #\(.number) \(.headRefName) @\(.headRefOid[:7]) review: \(([.statusCheckRollup[] | select(.context == "claude-review") | .state][0]) // "none")"' 2>/dev/null)
  [ -n "$PRS" ] && printf -- '- open PRs:\n%s\n' "$PRS"
fi
exit 0
