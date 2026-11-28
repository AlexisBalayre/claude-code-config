#!/bin/bash
# Remove worktrees made by worktree-create.sh, and their branches when no work is lost.
#   scripts/worktree-remove.sh <name>...   ->  removes .worktrees/<name> and its branch
#   scripts/worktree-remove.sh --merged    ->  the same for every worktree whose tip a merged PR shipped,
#                                              and every such <prefix>/ branch left with no worktree
# A worktree with uncommitted or untracked files, a detached HEAD, or a branch
# outside <prefix>/ is kept; ignored files (.env, node_modules) go with it. Its
# branch is deleted once a PR merged into the trunk shipped its tip (and
# origin's too, while still at that tip), or once origin's copy holds every
# commit; otherwise it is kept. A directory in .worktrees/ that git reads as
# part of the main checkout goes while it holds no file, else it is kept.
# Branch prefix and trunk come from .claude/project.env (default: feature, main).
set -euo pipefail

# The main checkout, which holds .worktrees/ and outlives any of them: run from a
# worktree this removes, the script's own directory is gone by the end.
ROOT=$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")
cd "$ROOT"
MAIN_TOPLEVEL=$(git rev-parse --show-toplevel)

[ -f .claude/project.env ] && . .claude/project.env
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"
TRUNK="${GIT_TRUNK:-main}"

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
# Prints the merged PR head that holds the tip: the tip itself, or a descendant
# when commits were added on GitHub (a suggestion, Update branch). gh's stderr
# stays visible: a failed lookup (no auth, say) reads as unshipped, and only
# gh's error says it failed.
shipped() {
  local oid
  for oid in $(gh pr list --head "$1" --base "$TRUNK" --state merged --json headRefOid --jq '.[].headRefOid'); do
    git cat-file -e "$oid^{commit}" 2>/dev/null || git fetch --quiet origin "$oid" 2>/dev/null || continue
    if git merge-base --is-ancestor "$2" "$oid"; then
      echo "$oid"
      return 0
    fi
  done
  return 1
}

# Drops a branch's line from the lead's approved-plans file (AGENT_APPROVED_PLANS,
# read by .claude/hooks/push-gate.sh) once the branch is deleted, so a later branch of that name needs a fresh approval.
unlist_plan() {
  [ -f "${AGENT_APPROVED_PLANS:-}" ] || {
    echo "Kept $1's line in approved-plans: AGENT_APPROVED_PLANS names no file here (the lead removes it)" >&2
    return 0
  }
  # A line holds its checkout's push URL, and another repository may name its branches as
  # this one does: only this repo's line of the branch goes.
  local url kept
  url=$(git remote get-url --push origin) || {
    echo "Kept $1's line in approved-plans: origin has no push URL here" >&2
    return 0
  }
  kept=$(B=$1 U=$url awk '!($1 == ENVIRON["B"] && $2 == ENVIRON["U"])' "$AGENT_APPROVED_PLANS") &&
    printf '%s\n' "$kept" >"$AGENT_APPROVED_PLANS"
}

# Deletes a branch once a merged PR shipped its tip, the PR's head given as
# $3 (empty when none did), and origin's copy too while it still sits at that
# head; or once origin's copy holds every commit. Otherwise keeps it.
drop_branch() {
  local branch=$1 tip=$2 merged_head=$3 remote_tip
  remote_tip=$(git rev-parse -q --verify "refs/remotes/origin/$branch" || true)
  if [ -n "$merged_head" ]; then
    git branch -D "$branch"
    unlist_plan "$branch"
    if [ "$remote_tip" = "$merged_head" ]; then
      # The lease has origin refuse the delete if the branch moved since the fetch.
      git push --quiet --force-with-lease="refs/heads/$branch:$merged_head" origin --delete "refs/heads/$branch" ||
        echo "Kept origin/$branch: delete refused (see git's error above)" >&2
    elif [ -n "$remote_tip" ]; then
      echo "Kept origin/$branch: it moved past the merged tip" >&2
    fi
  elif [ -n "$remote_tip" ] && git merge-base --is-ancestor "$tip" "$remote_tip"; then
    git branch -D "$branch"
    unlist_plan "$branch"
  else
    echo "Kept branch $branch: no merged PR shipped it, and it has commits no remote holds" >&2
  fi
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
  # A merged worker's open session recreates its .claude/.cc-writes/, a directory git
  # resolves to the main checkout.
  if [ "$(git -C "$WORKTREE_DIR" rev-parse --show-toplevel 2>/dev/null)" = "$MAIN_TOPLEVEL" ]; then
    find "$WORKTREE_DIR" -depth -type d -empty -delete || true
    if [ -d "$WORKTREE_DIR" ]; then
      echo "Kept $WORKTREE_DIR: not a registered worktree, and it holds files" >&2
    else
      echo "Removed leftover $WORKTREE_DIR: no worktree, no files"
    fi
    continue
  fi

  BRANCH=$(git -C "$WORKTREE_DIR" branch --show-current 2>/dev/null) ||
    { echo "Skipped $NAME: $WORKTREE_DIR is not a working worktree" >&2; continue; }
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
  [ "$FETCHED" = 1 ] && MERGED_HEAD=$(shipped "$BRANCH" "$TIP") && SHIPPED=1
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
  drop_branch "$BRANCH" "$TIP" "$([ "$SHIPPED" = 1 ] && echo "$MERGED_HEAD")"
done

# Before the branch pass: an entry whose directory is gone still reports its
# worktreepath, and git refuses to delete a branch such an entry holds.
git worktree prune

# A branch whose worktree went by hand, or whose removal kept the branch, has
# no worktree left to list it: --merged also drops those a merged PR shipped.
if [ "$MERGED_ONLY" = 1 ]; then
  # fd 3, so gh and git inside the loop cannot read the list.
  while read -r BRANCH WORKTREE <&3; do
    [ -z "$WORKTREE" ] || continue
    TIP=$(git rev-parse "refs/heads/$BRANCH")
    MERGED_HEAD=$(shipped "$BRANCH" "$TIP") || continue
    echo "Removing branch $BRANCH: no worktree, and a merged PR shipped it"
    drop_branch "$BRANCH" "$TIP" "$MERGED_HEAD"
  done 3< <(git for-each-ref --format='%(refname:short) %(worktreepath)' "refs/heads/$PREFIX/")
fi
