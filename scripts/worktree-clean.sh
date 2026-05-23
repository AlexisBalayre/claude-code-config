#!/usr/bin/env bash
# Remove worktrees whose remote branch no longer exists (e.g. after the PR merged).
#   pnpm worktree:clean
# Branch prefix is configurable via WORKTREE_BRANCH_PREFIX in .env (default: feature).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ -f .env ] && { set -a; . ./.env; set +a; }
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
