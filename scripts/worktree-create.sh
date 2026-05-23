#!/bin/bash
# Create an isolated worktree for a feature, on its own branch, with deps installed.
#   pnpm worktree:create <name>   ->  .worktrees/<name> on branch <prefix>/<name>
# Branch prefix is configurable via WORKTREE_BRANCH_PREFIX in .env (default: feature).
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

# Load personalization (.env) if present.
[ -f .env ] && { set -a; . ./.env; set +a; }
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"

NAME="${1:?Usage: pnpm worktree:create <name>}"
WORKTREE_DIR=".worktrees/$NAME"
BRANCH="$PREFIX/$NAME"

if [ -d "$WORKTREE_DIR" ]; then
  echo "Error: Worktree '$WORKTREE_DIR' already exists." >&2
  exit 1
fi

mkdir -p .worktrees
git worktree add "$WORKTREE_DIR" -b "$BRANCH"

# Install dependencies in the new worktree (fresh worktrees start without node_modules)
(cd "$WORKTREE_DIR" && pnpm install)

echo ""
echo "Worktree created:"
echo "  Directory: $WORKTREE_DIR"
echo "  Branch:    $BRANCH"
echo ""
echo "cd $WORKTREE_DIR"
