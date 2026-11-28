#!/usr/bin/env bash
# Hand-run cases for git-safety.sh's payload reading: `bash tests/hooks/git-safety.test.sh`.
# The rules themselves are covered by tools/hooks-rs/tests/git_safety.rs.

set -uo pipefail

HOOK="$(cd "$(dirname "$0")/../../.claude/hooks" && pwd)/git-safety.sh"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0

# A PATH that holds what the hook calls, minus jq.
mkdir "$T/bin"
for tool in bash cat grep git; do ln -s "$(command -v "$tool")" "$T/bin/$tool"; done

expect() { # <want exit> <name> <PATH> <payload>
  local want=$1 name=$2 path=$3 payload=$4 code
  (cd "$T" && printf '%s' "$payload" | CLAUDE_PROJECT_DIR=$T PATH=$path bash "$HOOK" 2>/dev/null)
  code=$?
  if [ "$code" -ne "$want" ]; then
    printf 'FAIL want=%s got=%s  %s\n' "$want" "$code" "$name"
    failures=$((failures + 1))
  fi
}

FORCE='{"tool_input":{"command":"git push --force origin feature/x"}}'
PLAIN='{"tool_input":{"command":"ls -la"}}'

expect 2 'a payload that is not JSON blocks' "$PATH" 'not json'
expect 2 'a forced push blocks' "$PATH" "$FORCE"
expect 0 'a plain command passes' "$PATH" "$PLAIN"
expect 2 'without jq, a forced push still blocks' "$T/bin" "$FORCE"
expect 0 'without jq, a plain command passes' "$T/bin" "$PLAIN"
expect 2 'without jq, a push to the trunk blocks' "$T/bin" \
  '{"tool_input":{"command":"git push origin main","description":"Push"}}'
expect 2 'without jq, a push to the trunk blocks in spaced JSON' "$T/bin" \
  '{"tool_input": {"command": "git push origin HEAD:main", "description": "Push"}}'
expect 0 'without jq, a push to a branch passes' "$T/bin" \
  '{"tool_input":{"command":"git push origin feature/x","description":"not to main "}}'
expect 2 'without jq, an escaped tab still separates the words' "$T/bin" \
  '{"tool_input":{"command":"git reset\t--hard"}}'
expect 2 'without jq, a quoted word does not end the command' "$T/bin" \
  '{"tool_input":{"command":"git commit -m \"x\" && git push --force"}}'
expect 0 'without jq, the description is not read' "$T/bin" \
  '{"tool_input":{"command":"ls","description":"after git reset --hard"}}'
git init -q -b main "$T/trunk" && (
  cd "$T/trunk" || exit 1
  printf '%s' '{"tool_input":{"command":"git push","description":"Push"}}' |
    CLAUDE_PROJECT_DIR=$T PATH=$T/bin bash "$HOOK" 2>/dev/null
  [ $? -eq 2 ]
) || { echo 'FAIL without jq, a bare push on the trunk blocks'; failures=$((failures + 1)); }

[ "$failures" -eq 0 ] && echo "git-safety: all cases pass" || exit 1
