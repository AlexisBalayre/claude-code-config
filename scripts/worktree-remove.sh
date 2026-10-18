#!/bin/bash
# Remove worktrees made by worktree-create.sh, and their branches when no work is lost.
#   scripts/worktree-remove.sh <name>...   ->  removes .worktrees/<name> and its branch
#   scripts/worktree-remove.sh --merged    ->  the same for every worktree whose tip a merged PR shipped
# A worktree with uncommitted or untracked files, a detached HEAD, or a branch
# outside <prefix>/ is kept; ignored files (.env, node_modules) go with it. Its
# branch is deleted once a PR merged into the trunk shipped its tip (and
# origin's too, while still at that tip), or once origin's copy holds every
# commit; otherwise it is kept.
# Branch prefix comes from WORKTREE_BRANCH_PREFIX in .claude/project.env (default: feature).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ -f .claude/project.env ] && . .claude/project.env
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"

USAGE="Usage: scripts/worktree-remove.sh <name>... | --merged"
[ $# -gt 0 ] || { echo "$USAGE" >&2; exit 1; }
MERGED_ONLY=0
if [ "$1" = "--merged" ]; then
  [ $# -eq 1 ] || { echo "$USAGE" >&2; exit 1; }
  MERGED_ONLY=1
fi

# A stale origin ref could vouch for commits the remote no longer has, so
# without a fresh fetch every branch is kept.
FETCHED=1
git fetch --prune --quiet origin || { FETCHED=0; echo "Fetch failed: keeping every branch" >&2; }
[ "$FETCHED" = 1 ] || [ "$MERGED_ONLY" = 0 ] || exit 1

# A squash merge leaves the tip off the trunk, so only GitHub knows it shipped.
shipped() {
  gh pr list --head "$1" --base "${GIT_TRUNK:-main}" --state merged --json headRefOid --jq '.[].headRefOid' 2>/dev/null |
    grep -Fx "$2" >/dev/null
}

if [ "$MERGED_ONLY" = 1 ]; then
  set --
  for DIR in .worktrees/*/; do
    [ -d "$DIR" ] && set -- "$@" "$(basename "$DIR")"
  done
fi

for NAME in "$@"; do
  case "$NAME" in
    ''|-*|*/*|*..*|*[!A-Za-z0-9._-]*)
      echo "Skipped '$NAME': invalid worktree name (letters, digits, . _ - only; no leading dash)" >&2
      continue ;;
  esac
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
  # Before the removal: .worktrees/ can also hold worktrees other tools own, on
  # branches of their own.
  case "$BRANCH" in
    "$PREFIX"/*) ;;
    *) echo "Kept $WORKTREE_DIR: $BRANCH is not a $PREFIX/ branch" >&2; continue ;;
  esac
  TIP=$(git rev-parse "refs/heads/$BRANCH")
  SHIPPED=0
  [ "$FETCHED" = 1 ] && shipped "$BRANCH" "$TIP" && SHIPPED=1
  if [ "$MERGED_ONLY" = 1 ] && [ "$SHIPPED" = 0 ]; then
    echo "Kept $WORKTREE_DIR: no PR merged into the trunk shipped its tip" >&2
    continue
  fi
  # No --force: git refuses a worktree with uncommitted or untracked files.
  if ! git worktree remove "$WORKTREE_DIR"; then
    echo "Kept $WORKTREE_DIR" >&2
    continue
  fi
  echo "Removed $WORKTREE_DIR"
  [ "$FETCHED" = 1 ] || continue

  # A commit made in the worktree since the check above is the branch's too.
  NOW=$(git rev-parse "refs/heads/$BRANCH")
  [ "$NOW" = "$TIP" ] || { TIP=$NOW; SHIPPED=0; }
  REMOTE_TIP=$(git rev-parse -q --verify "refs/remotes/origin/$BRANCH" || true)
  if [ "$SHIPPED" = 1 ]; then
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
