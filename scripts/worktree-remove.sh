#!/bin/bash
# Remove worktrees made by worktree-create.sh, and their branches when no work is lost.
#   scripts/worktree-remove.sh <name>...   ->  removes .worktrees/<name> and its branch
# A worktree with uncommitted or untracked files, or a detached HEAD, is kept;
# ignored files (.env, node_modules) go with it. Its <prefix>/ branch is deleted
# once a merged PR shipped its tip (and origin's too, while still at that tip),
# or once origin's copy holds every commit; otherwise it is kept.
# Branch prefix comes from WORKTREE_BRANCH_PREFIX in .claude/project.env (default: feature).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ -f .claude/project.env ] && . .claude/project.env
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"

[ $# -gt 0 ] || { echo "Usage: scripts/worktree-remove.sh <name>..." >&2; exit 1; }

# A stale origin ref could vouch for commits the remote no longer has, so
# without a fresh fetch every branch is kept.
FETCHED=1
git fetch --prune --quiet origin || { FETCHED=0; echo "Fetch failed: keeping every branch" >&2; }

for NAME in "$@"; do
  WORKTREE_DIR=".worktrees/$NAME"
  if [ ! -d "$WORKTREE_DIR" ]; then
    echo "Skipped $NAME: no worktree at $WORKTREE_DIR" >&2
    continue
  fi

  BRANCH=$(git -C "$WORKTREE_DIR" branch --show-current)
  if [ -z "$BRANCH" ]; then
    echo "Kept $WORKTREE_DIR: detached HEAD, whose commits no branch may hold" >&2
    continue
  fi
  # No --force: git refuses a worktree with uncommitted or untracked files.
  if ! git worktree remove "$WORKTREE_DIR"; then
    echo "Kept $WORKTREE_DIR" >&2
    continue
  fi
  echo "Removed $WORKTREE_DIR"
  case "$BRANCH" in
    "$PREFIX"/*) ;;
    *) echo "Kept branch $BRANCH: not a $PREFIX/ branch" >&2; continue ;;
  esac
  [ "$FETCHED" = 1 ] || continue

  TIP=$(git rev-parse "refs/heads/$BRANCH")
  REMOTE_TIP=$(git rev-parse -q --verify "refs/remotes/origin/$BRANCH" || true)
  # A squash merge leaves the tip off the trunk, so only GitHub knows it shipped.
  if gh pr list --head "$BRANCH" --state merged --json headRefOid --jq '.[].headRefOid' 2>/dev/null |
    grep -Fx "$TIP" >/dev/null; then
    git branch -D "$BRANCH"
    if [ "$REMOTE_TIP" = "$TIP" ]; then
      # The lease has origin refuse the delete if the branch moved since the fetch.
      git push --quiet --force-with-lease="refs/heads/$BRANCH:$TIP" origin --delete "$BRANCH" ||
        echo "Kept origin/$BRANCH: delete refused (see git's error above)" >&2
    elif [ -n "$REMOTE_TIP" ]; then
      echo "Kept origin/$BRANCH: it moved past the merged tip" >&2
    fi
  elif [ -n "$REMOTE_TIP" ] && git merge-base --is-ancestor "$TIP" "$REMOTE_TIP"; then
    git branch -D "$BRANCH"
  else
    echo "Kept branch $BRANCH: no merged PR shipped it, and it has commits no remote holds" >&2
  fi
done
