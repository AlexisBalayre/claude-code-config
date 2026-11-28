#!/usr/bin/env bash
# Hand-run cases for tool-result-size.sh: `bash .claude/hooks/tool-result-size.test.sh`.

set -uo pipefail

HOOK="$(dirname "$0")/tool-result-size.sh"
failures=0

# expect <want: silent | text the warning must contain> <tool> <command> <tool_response as jq, $n = size>
expect() {
  local want=$1 tool=$2 command=$3 response=$4 size=$5 out code
  out=$(jq -n --arg t "$tool" --arg c "$command" --argjson n "$size" \
    "{tool_name: \$t, tool_input: {command: \$c}, tool_response: ($response)}" | bash "$HOOK")
  code=$?
  if [ "$code" -ne 0 ]; then
    printf 'FAIL exit=%s  %s %s %s\n' "$code" "$tool" "$size" "$command"
    failures=$((failures + 1))
  elif [ "$want" = silent ] && [ -n "$out" ]; then
    printf 'FAIL want silent, got %s  %s %s %s\n' "$out" "$tool" "$size" "$command"
    failures=$((failures + 1))
  elif [ "$want" != silent ] && ! jq -e --arg w "$want" \
    '.hookSpecificOutput.hookEventName == "PostToolUse" and (.hookSpecificOutput.additionalContext | contains($w))' \
    <<<"$out" >/dev/null 2>&1; then
    printf 'FAIL want "%s", got %s  %s %s %s\n' "$want" "${out:-nothing}" "$tool" "$size" "$command"
    failures=$((failures + 1))
  fi
}

BASH_OUT='{stdout: ("x" * $n), stderr: "", interrupted: false, isImage: false}'
READ_TEXT='{type: "text", file: {filePath: "/f", content: ("x" * $n), numLines: 1}}'

expect silent Read '' "$READ_TEXT" 19993
expect 'offset' Read '' "$READ_TEXT" 19994
# Edit sends no `content`, so the generic object branch counts it 0; only Write needs its own.
expect silent Edit '' '{filePath: "/f", originalFile: ("x" * $n), oldString: "a", newString: "b", structuredPatch: []}' 50000
expect silent Bash 'bash scripts/verify.sh' "$BASH_OUT" 10000
expect silent Bash 'make test' "$BASH_OUT" 12000
expect 'tail -n 40' Bash 'make test' "$BASH_OUT" 25000
expect 'bare' Bash 'bash scripts/verify.sh' "$BASH_OUT" 12000
expect 'tail -n 40' Bash 'scripts/worktree-create.sh x' "$BASH_OUT" 25000
expect 'bare' Bash 'bash /u/work/lead-tools/worker-git.sh merge x' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'cat /u/work/lead-tools/worker-git.sh.bak' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'bash /u/work/lead-tools/worker-git.sh.bak merge x' "$BASH_OUT" 25000
expect 'bare' Bash 'git push origin feature/x' "$BASH_OUT" 25000
expect 'bare' Bash 'cd x && git -C x push origin feature/x' "$BASH_OUT" 25000
expect 'bare' Bash 'scripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash 'cd .worktrees/x && scripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash './scripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash 'bash /abs/path/scripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash 'cd x; git push origin feature/x' "$BASH_OUT" 25000
expect 'bare' Bash '  git push origin feature/x' "$BASH_OUT" 25000
expect 'tail -n 40' Bash "grep -rn 'git push' docs" "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'cat scripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash 'git status || git push origin x' "$BASH_OUT" 25000
expect 'bare' Bash '(git push origin x)' "$BASH_OUT" 25000
expect 'bare' Bash $'cd x\nscripts/verify.sh' "$BASH_OUT" 12000
expect 'bare' Bash $'git add . \\\n  && git push' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'echo "git push"' "$BASH_OUT" 25000
expect 'tail -n 40' Bash "grep -rnE 'scripts/verify.sh|git push' docs" "$BASH_OUT" 25000
expect 'tail -n 40' Bash "rg -n '(git push|worktree-create)' docs" "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'cat $(pwd)/scripts/verify.sh' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'grep -rnE "scripts/verify.sh|git push" docs' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'cat ./scripts/verify.sh' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'head -n 80 /abs/scripts/verify.sh' "$BASH_OUT" 25000
expect 'bare' Bash "git commit -qm \"don't\" && git push origin x && echo 'ok'" "$BASH_OUT" 25000
expect 'bare' Bash 'cat x | git push' "$BASH_OUT" 25000
expect 'bare' Bash 'git -C "$WT" push origin feature/x' "$BASH_OUT" 25000
expect 'tail -n 40' Bash 'git log -p -- scripts/verify.sh' "$BASH_OUT" 25000

if [ "$failures" -gt 0 ]; then
  echo "$failures case(s) failed"
  exit 1
fi
echo "tool-result-size: all cases pass"
