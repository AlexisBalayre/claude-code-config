#!/usr/bin/env bash
# Remove worktrees whose remote feature branch no longer exists (e.g. after the PR merged).
#   pnpm worktree:clean
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

git worktree prune
git fetch --prune origin >/dev/null 2>&1 || true

git worktree list --porcelain \
  | awk '/^worktree /{p=$2} /^branch /{print p" "$2}' \
  | while read -r path ref; do
      [ "$path" = "$ROOT" ] && continue
      branch=${ref#refs/heads/}
      case "$branch" in feature/*) ;; *) continue ;; esac
      if ! git ls-remote --exit-code --heads origin "$branch" >/dev/null 2>&1; then
        echo "Removing $path (remote $branch gone)"
        git worktree remove "$path" --force
      fi
    done
