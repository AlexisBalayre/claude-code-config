#!/usr/bin/env bash
# Remove worktrees whose remote branch no longer exists (e.g. after the PR merged).
#   scripts/worktree-clean.sh
# Branch prefix comes from WORKTREE_BRANCH_PREFIX in .claude/project.env (default: feature).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ -f .claude/project.env ] && . .claude/project.env
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"

git worktree prune
git fetch --prune origin >/dev/null 2>&1 || true

git worktree list --porcelain \
  | awk '/^worktree /{p=$2} /^branch /{print p" "$2}' \
  | while read -r path ref; do
      [ "$path" = "$ROOT" ] && continue
      branch=${ref#refs/heads/}
      case "$branch" in "$PREFIX"/*) ;; *) continue ;; esac
      if ! git ls-remote --exit-code --heads origin "$branch" >/dev/null 2>&1; then
        echo "Removing $path (remote $branch gone)"
        git worktree remove "$path" --force
      fi
    done
