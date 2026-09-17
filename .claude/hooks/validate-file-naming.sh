#!/usr/bin/env bash
# PreToolUse(Write) hook — blocks new files whose name breaks the project's naming
# convention (FILE_NAMING_* in .claude/project.env; unset = no check).
# Exit 0 = allow, Exit 2 = block with message.
set -euo pipefail

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')
[ -z "$FILE_PATH" ] && exit 0

# Hooks are per-session, keyed on the directory the session started in, so a session launched here
# and working in a sibling repo would otherwise hold that repo to this convention. Only paths under
# this checkout are ours.
[ -z "${CLAUDE_PROJECT_DIR:-}" ] && exit 0
[[ "$FILE_PATH" != "$CLAUDE_PROJECT_DIR"/* ]] && exit 0

[ -f "$CLAUDE_PROJECT_DIR/.claude/project.env" ] && . "$CLAUDE_PROJECT_DIR/.claude/project.env"
[ -z "${FILE_NAMING_SCOPE_REGEX:-}" ] || [ -z "${FILE_NAMING_REGEX:-}" ] && exit 0

# Overwriting an existing file doesn't choose a name, so legacy names stay writable.
[ -e "$FILE_PATH" ] && exit 0

REL_PATH="${FILE_PATH#"$CLAUDE_PROJECT_DIR"/}"
[[ "$REL_PATH" =~ $FILE_NAMING_SCOPE_REGEX ]] || exit 0

FILENAME=$(basename "$FILE_PATH")
[[ "$FILENAME" =~ $FILE_NAMING_REGEX ]] && exit 0

echo "BLOCKED: file name '$FILENAME' does not match the project naming convention ($FILE_NAMING_REGEX)." >&2
[ -n "${FILE_NAMING_HINT:-}" ] && echo "$FILE_NAMING_HINT" >&2
echo "Full naming rules: docs/conventions/core.md §Naming" >&2
exit 2
