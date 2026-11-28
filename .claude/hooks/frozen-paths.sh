#!/usr/bin/env bash
# PreToolUse(Edit|Write|NotebookEdit) hook: blocks an edit to a path FROZEN_PATHS_REGEX
# (.claude/project.env, an ERE on the repo-relative path) matches, unless the allow-list
# in $AGENT_FROZEN_ALLOW holds `<branch> <path>` for it. With either unset the freeze is
# off, so only sessions that opt in are frozen.
# Denies rather than asks: a click would leave no allow-list line on record.
# Edits made through Bash pass here and no hook catches them; the user's PR
# review is the only check.

set -uo pipefail

[ -n "${AGENT_FROZEN_ALLOW:-}" ] || exit 0
# The session's own profile, never the edited file's tree: sourcing runs it, and the file can sit
# in a repository nobody vetted.
[ -f "${CLAUDE_PROJECT_DIR:-.}/.claude/project.env" ] && . "${CLAUDE_PROJECT_DIR:-.}/.claude/project.env"
[ -n "${FROZEN_PATHS_REGEX:-}" ] || exit 0

FILE=$(jq -r '(if .tool_name == "NotebookEdit" then .tool_input.notebook_path else .tool_input.file_path end) // empty') || {
  echo "BLOCKED: the hook could not read the edit's path. Stop and ask the user; don't edit it another way." >&2
  exit 2
}
[ -n "$FILE" ] || exit 0
# grep -F reads each line of its pattern as a pattern of its own, so a newline
# in the path could match any allow-list line.
case "$FILE" in *$'\n'*)
  echo "BLOCKED: the file path holds a newline." >&2
  exit 2
  ;;
esac

# realpath resolves `..` and symlinks even through directories a Write has
# yet to create; the branch comes from the file's tree, not the session's.
real=$(python3 -I -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$FILE") || exit 0
dir=$(dirname "$real")
while [ ! -d "$dir" ]; do dir=$(dirname "$dir"); done

# git prints the prefix in its on-disk case; stripping the toplevel from the
# typed path would miss a wrong-case parent directory.
prefix=$(git -C "$dir" rev-parse --show-prefix 2>/dev/null) || exit 0
rel=$prefix${real#"$dir"/}
# A part not yet on disk keeps its typed case, and on a case-insensitive filesystem a Write
# there still lands in the frozen tree. Only the match ignores case; the allow-list line must
# still be exact, so it denies.
shopt -s nocasematch
[[ $rel =~ $FROZEN_PATHS_REGEX ]] || exit 0
shopt -u nocasematch

branch=$(git -C "$dir" branch --show-current)
if [ -r "$AGENT_FROZEN_ALLOW" ] && grep -qxF "$branch $rel" "$AGENT_FROZEN_ALLOW"; then
  exit 0
fi

echo "BLOCKED: $rel is frozen, and $AGENT_FROZEN_ALLOW has no '$branch $rel' line. Stop and ask the user for that line; don't edit it another way." >&2
exit 2
