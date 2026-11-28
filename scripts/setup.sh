#!/bin/bash
# Install the git hooks: scripts/setup.sh, once per clone (every worktree shares them).
# pre-push calls added-lines.sh from beside itself, so both are copied.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
HOOKS="$(git rev-parse --git-common-dir)/hooks"
mkdir -p "$HOOKS"
cp scripts/pre-commit scripts/pre-push scripts/added-lines.sh "$HOOKS/"
chmod +x "$HOOKS/pre-commit" "$HOOKS/pre-push" "$HOOKS/added-lines.sh"
echo "Installed pre-commit, pre-push and added-lines.sh into $HOOKS"
