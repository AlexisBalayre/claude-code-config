#!/bin/bash
# Create an isolated worktree for a feature, on its own branch, with deps installed.
#   scripts/worktree-create.sh <name>   ->  .worktrees/<name> on branch <prefix>/<name>
# Branch prefix and install command come from .claude/project.env.
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

[ -f .claude/project.env ] && . .claude/project.env
PREFIX="${WORKTREE_BRANCH_PREFIX:-feature}"

NAME="${1:?Usage: scripts/worktree-create.sh <name>}"
WORKTREE_DIR=".worktrees/$NAME"
BRANCH="$PREFIX/$NAME"

if [ -d "$WORKTREE_DIR" ]; then
  echo "Error: Worktree '$WORKTREE_DIR' already exists." >&2
  exit 1
fi

mkdir -p .worktrees
git worktree add "$WORKTREE_DIR" -b "$BRANCH"

# Fresh worktrees start without installed dependencies.
if [ -n "${INSTALL_CMD:-}" ]; then
  (cd "$WORKTREE_DIR" && bash -c "$INSTALL_CMD")
fi

# Worktree-local CodeGraph index: without one, codegraph answers from the main
# tree's index, missing symbols changed on this branch. Only when the main
# checkout opted in, and non-fatal so an indexer hiccup never blocks creation.
if [ -d .codegraph ]; then
  (cd "$WORKTREE_DIR" && npx -y @colbymchenry/codegraph@1.6.0 init --yes) ||
    echo "Warning: codegraph init failed; run it manually in $WORKTREE_DIR" >&2
fi

echo ""
echo "Worktree created:"
echo "  Directory: $WORKTREE_DIR"
echo "  Branch:    $BRANCH"
echo ""
echo "cd $WORKTREE_DIR"
