#!/bin/bash
# Remove worktrees made by worktree-create.sh, and their branches when no work is lost.
#   scripts/worktree-remove.sh <name>...   ->  removes .worktrees/<name> and its branch
# A worktree with uncommitted or untracked files is kept. Its branch is deleted
# once a merged PR shipped its tip (the remote branch too), or once the remote
# branch holds every commit; otherwise it is kept.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ $# -gt 0 ] || { echo "Usage: scripts/worktree-remove.sh <name>..." >&2; exit 1; }

git fetch --prune origin >/dev/null 2>&1 || true

for NAME in "$@"; do
  WORKTREE_DIR=".worktrees/$NAME"
  if [ ! -d "$WORKTREE_DIR" ]; then
    echo "Skipped $NAME: no worktree at $WORKTREE_DIR" >&2
    continue
  fi

  BRANCH=$(git -C "$WORKTREE_DIR" branch --show-current)
  # No --force: git refuses a worktree with uncommitted or untracked files.
  if ! git worktree remove "$WORKTREE_DIR"; then
    echo "Kept $WORKTREE_DIR" >&2
    continue
  fi
  echo "Removed $WORKTREE_DIR"
  [ -n "$BRANCH" ] || continue

  TIP=$(git rev-parse "$BRANCH")
  REMOTE_TIP=$(git rev-parse -q --verify "refs/remotes/origin/$BRANCH" || true)
  # A squash merge leaves the tip off the trunk, so only GitHub knows it shipped.
  if gh pr list --head "$BRANCH" --state merged --json headRefOid --jq '.[].headRefOid' 2>/dev/null |
    grep -Fx "$TIP" >/dev/null; then
    git branch -D "$BRANCH"
    # A remote tip past the merged one holds commits nothing else has.
    if [ "$REMOTE_TIP" = "$TIP" ]; then
      git push origin --delete "$BRANCH"
    fi
  elif [ -n "$REMOTE_TIP" ] && git merge-base --is-ancestor "$TIP" "$REMOTE_TIP"; then
    git branch -D "$BRANCH"
  else
    echo "Kept branch $BRANCH: no merged PR shipped it, and it has commits no remote holds" >&2
  fi
done
