#!/usr/bin/env bash
# PreToolUse(Bash) safety hook — blocks dangerous git + shell operations.
# Exit 0 = allow, Exit 2 = block with message.

set -euo pipefail

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

# If there's no command, don't block.
if [ -z "$COMMAND" ]; then
  exit 0
fi

# Trunk branch is configurable via .env (defaults to main).
if [ -f "${CLAUDE_PROJECT_DIR:-.}/.env" ]; then set -a; . "${CLAUDE_PROJECT_DIR:-.}/.env"; set +a; fi
TRUNK="${GIT_TRUNK:-main}"

# --- Destructive shell / SQL patterns -----------------------------------------

if echo "$COMMAND" | grep -qE 'rm[[:space:]]+-rf[[:space:]]|DROP[[:space:]]+TABLE'; then
  echo "BLOCKED: destructive command (rm -rf / DROP TABLE) not allowed." >&2
  exit 2
fi

# --- Destructive / irreversible git operations --------------------------------

if echo "$COMMAND" | grep -qE 'git[[:space:]]+push[[:space:]]+.*(--force\b|-f\b)'; then
  echo "BLOCKED: 'git push --force' is not allowed. Use a regular push or ask the user." >&2
  exit 2
fi

if echo "$COMMAND" | grep -qE 'git[[:space:]]+reset[[:space:]]+--hard\b'; then
  echo "BLOCKED: 'git reset --hard' is destructive. Stash or revert instead." >&2
  exit 2
fi

# --- Branch protection: never create branches on trunk, never push to trunk ---

if echo "$COMMAND" | grep -qE 'git[[:space:]]+checkout[[:space:]]+-b\b'; then
  CURRENT_BRANCH=$(git branch --show-current 2>/dev/null || echo "")
  if [ "$CURRENT_BRANCH" = "$TRUNK" ]; then
    echo "BLOCKED: do not use 'git checkout -b' on $TRUNK. Use 'pnpm worktree:create <name>' instead." >&2
    exit 2
  fi
fi

if echo "$COMMAND" | grep -qE "git[[:space:]]+push[[:space:]]+.*\b${TRUNK}\b"; then
  echo "BLOCKED: never push directly to $TRUNK. Create a PR from a feature branch." >&2
  exit 2
fi

exit 0
