#!/bin/bash
# PreToolUse(Edit|Write) hook — blocks manual edits to generated files
# (GENERATED_PATHS_REGEX in .claude/project.env; unset = no check).

set -o pipefail

[ -f "${CLAUDE_PROJECT_DIR:-.}/.claude/project.env" ] && . "${CLAUDE_PROJECT_DIR:-.}/.claude/project.env"
[ -z "${GENERATED_PATHS_REGEX:-}" ] && exit 0

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[ -z "$FILE_PATH" ] && exit 0

if echo "$FILE_PATH" | grep -qE "$GENERATED_PATHS_REGEX"; then
  echo "BLOCKED: $FILE_PATH is generated. Edit its source and rerun the generator instead." >&2
  exit 2
fi

exit 0
