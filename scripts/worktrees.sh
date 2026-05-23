#!/usr/bin/env bash
# Worktree helpers. One worktree per feature; never work on main.
#
#   scripts/worktrees.sh create <name>   ->  .worktrees/<name> on branch feature/<name>
#   scripts/worktrees.sh clean           ->  remove worktrees whose remote branch is gone
#
# Wired into package.json as `pnpm worktree:create` / `pnpm worktree:clean`.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

create() {
  local name=${1:-}
  [ -z "$name" ] && { echo "usage: worktrees.sh create <name>" >&2; exit 1; }
  local branch="feature/$name"
  local path=".worktrees/$name"
  [ -e "$path" ] && { echo "worktree already exists: $path" >&2; exit 1; }

  # Reuse the branch if it already exists, otherwise create it.
  if git show-ref --verify --quiet "refs/heads/$branch"; then
    git worktree add "$path" "$branch"
  else
    git worktree add "$path" -b "$branch"
  fi
  echo "Created $path on $branch"
}

clean() {
  git worktree prune
  git fetch --prune origin >/dev/null 2>&1 || true

  # Remove worktrees whose upstream feature branch no longer exists on the remote.
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
}

case "${1:-}" in
  create) shift; create "$@" ;;
  clean)  clean ;;
  *) echo "usage: worktrees.sh {create <name>|clean}" >&2; exit 1 ;;
esac
