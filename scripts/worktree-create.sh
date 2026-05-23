#!/bin/bash
# Create an isolated worktree for a feature, on its own branch, with deps installed.
#   pnpm worktree:create <name>   ->  .worktrees/<name> on branch feature/<name>
set -euo pipefail

NAME="${1:?Usage: pnpm worktree:create <name>}"
WORKTREE_DIR=".worktrees/$NAME"
BRANCH="feature/$NAME"

if [ -d "$WORKTREE_DIR" ]; then
  echo "Error: Worktree '$WORKTREE_DIR' already exists." >&2
  exit 1
fi

# Ensure .worktrees/ exists
mkdir -p .worktrees

# Create worktree + branch
git worktree add "$WORKTREE_DIR" -b "$BRANCH"

# Install dependencies in the new worktree (fresh worktrees start without node_modules)
(cd "$WORKTREE_DIR" && pnpm install)

echo ""
echo "Worktree created:"
echo "  Directory: $WORKTREE_DIR"
echo "  Branch:    $BRANCH"
echo ""
echo "cd $WORKTREE_DIR"
