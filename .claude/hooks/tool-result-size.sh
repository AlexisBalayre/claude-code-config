#!/usr/bin/env bash
# PostToolUse hook: warns the model when one tool result is large enough to
# crowd its context, with a hint for the next call. It only warns: the result
# is already in the context, and cutting it would hide a failing test's
# output. A Bash verify log warns from 10k characters, since several of those
# add up in one session; anything else from 20k. A command that only names
# one, like `cat scripts/verify.sh`, can get the 10k bar too: an early warning
# costs one line, a missed one a whole log.
# A sandboxed worker session needs `scripts/verify.sh`, `git push` and the
# lead tools' `worker-git.sh` to run bare. When a command starts with one of
# those, or one follows a newline, `;`, `&`, `|` or `(`, the hint says to keep
# it bare instead of piping. Quoted spans are blanked first, so a grep pattern
# such as 'scripts/verify.sh|git push' does not count as a command.
# It measures the text the model sees, not the payload: Bash stdout and
# stderr, a text Read with its line-number prefix, a WebFetch result, and the
# text of any other result that carries it (Grep content, Grep and Glob file
# lists, a sync Agent report, MCP text blocks). Edit and Write echo their
# input, and an image (a Read, or Bash output shown as one) carries base64,
# so those count as 0.
# Background Bash output reaches the model through a Read of its output
# file (TaskOutput was removed in Claude Code v2.1.277), so the Read branch
# measures it.
set -uo pipefail

jq -c '
  def text: if type == "array" then [.[] | .text? // ""] | join("")
            elif type == "string" then .
            else "" end;
  (.tool_name // "") as $tool
  | .tool_response as $r
  | (try (if $tool == "Bash" then (if $r.isImage == true then 0 else (($r.stdout // "") + ($r.stderr // "")) | length end)
          elif $tool == "Read" then ($r.file.content // "" | length) + 7 * ($r.file.numLines // 0)
          elif $tool == "WebFetch" then $r.result // "" | length
          elif $tool == "Write" then 0
          elif ($r | type) == "object" then ($r.content | text) + ($r.filenames // [] | join("\n")) | length
          else $r | text | length end) catch 0) as $n
  | ($tool == "Bash" and ((.tool_input.command // "")
      | test("scripts/verify\\.sh"))) as $test_log
  | ($tool == "Bash" and ((.tool_input.command // "")
      | gsub("\"[^\"]*\"|\u0027[^\u0027]*\u0027"; "\"\"")
      | test("(^|[;&|(\\n])\\s*((bash\\s+)?([^\\s;&|()]*/)?scripts/verify\\.sh|(bash\\s+)?([^\\s;&|()]*/)?worker-git\\.sh(\\s|$)|git(\\s+-C\\s+\\S+)?\\s+push\\b)"))) as $bare_only
  | select($n > 20000 or ($test_log and $n > 10000))
  | (if $bare_only then "keep that command bare (a pipe breaks it in the worker sandbox) and read only the failing lines of its result"
     elif $tool == "Bash" then "pipe the output through `tail -n 40` or a summary `grep` (pass/fail counts, failing names)"
     elif $tool == "Read" then "read a part with `offset` and `limit`"
     else "narrow the query or ask for fewer fields"
     end) as $hint
  | {hookSpecificOutput: {hookEventName: "PostToolUse",
      additionalContext: "That \($tool) result was \($n) characters. Next time, \($hint)."}}
' 2>/dev/null
exit 0
