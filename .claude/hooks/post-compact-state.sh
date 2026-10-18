#!/usr/bin/env bash
# SessionStart(compact) hook: re-injects the live state a compaction summary
# tends to lose or leave stale: the session's directory and branch, the other
# worktrees, the open PRs and whether the CI review covers their head.
# Read fresh here, after compaction, because SessionStart stdout is added to
# Claude's context while PreCompact output never reaches the summary.
# Every probe is best-effort: a missing tool or a failed call drops its line.
set -uo pipefail

CWD=$(jq -r '.cwd // empty' 2>/dev/null)
# No git status, in any tree: refreshing the index runs the fsmonitor and clean
# filters config names, and another session can point a worktree's .git file,
# or the shared .git/config, at its own commands. They would run as this
# session, outside any sandbox. The calls below only read refs and worktrees.
cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0
SELF=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0

# Paths and branch names are chosen by whoever made the worktree and land in
# Claude's context: keeping them to path-safe characters stops one faking a
# line or a tag of this output.
safe() { printf '%s' "$1" | LC_ALL=C tr -c 'A-Za-z0-9._/() -' '?'; }

BRANCH=$(git branch --show-current 2>/dev/null)
echo "Live state after compaction (read now; trust it over the summary):"
[ -n "$CWD" ] && echo "- session cwd: $(safe "$CWD")"
echo "- checkout: $(safe "$SELF") on $(safe "${BRANCH:-a detached HEAD}")"

echo "- other worktrees (path, branch):"
git worktree list --porcelain 2>/dev/null |
  awk '/^worktree /{p=substr($0,10)} /^branch /{print p"\t"substr($0,19)} /^detached/{print p"\t(detached)"}' |
  while IFS=$'\t' read -r path branch; do
    [ "$path" = "$SELF" ] || printf '    %s  %s\n' "$(safe "$path")" "$(safe "$branch")"
  done

# The claude-review commit status sits on the PR's head commit, so "none" means
# the CI review has not covered the current head.
if command -v gh >/dev/null 2>&1; then
  PRS=$(gh pr list --state open --limit 20 --json number,headRefName,headRefOid,statusCheckRollup \
    --jq '.[] | "    #\(.number) \(.headRefName | gsub("[^A-Za-z0-9._/-]"; "?") | .[:60]) @\(.headRefOid[:7]) review: \(([.statusCheckRollup[] | select(.context == "claude-review") | .state][0]) // "none")"' 2>/dev/null)
  [ -n "$PRS" ] && printf -- '- open PRs:\n%s\n' "$PRS"
fi
exit 0
