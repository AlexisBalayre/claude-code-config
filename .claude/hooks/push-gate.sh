#!/usr/bin/env bash
# PreToolUse(Bash) hook: asks the user before a push that can create a branch
# on origin, since the first push of a PR needs the user's go unless its branch
# is in approved-plans (a ruled-on plan covers it), and before a `gh` call that
# can merge, since a merge is the user's. Prints nothing for anything else, so
# the settings rules still decide; an `allow` here would override them.
# scripts/pre-push checks the verify stamp.
# Only `git [-C <path>] push [-u|--set-upstream] origin <branch>`, alone in its
# call, can skip the prompt: every other shape is asked, never parsed.

set -uo pipefail

IFS= read -r -d '' INPUT || :
# This runs on every Bash call, so a payload that cannot be a candidate below
# exits before any process starts. The payload's JSON keeps ASCII letters
# literal, as the no-jq read below relies on too.
# `gh` however quoting or a line continuation splits it, in a command or in its JSON.
GH_SPELT=$'[gG][\\\\"\'$n\n]*[hH]'
[[ $INPUT == *push* || $INPUT == *send-pack* || $INPUT =~ $GH_SPELT ]] || exit 0
# Without jq, the raw payload still carries the command's words, so the checks
# below run on it and a candidate asks instead of passing unseen.
# One jq reads both fields, split at a unit separator; cwd comes first so a command
# holding the separator still splits right, and `$(...)` strips the command's
# trailing newlines.
NOJQ=
if OUT=$(jq -r '(.cwd // "") + "\u001f" + (.tool_input.command // "")' <<<"$INPUT" 2>/dev/null); then
  CWD=${OUT%%$'\x1f'*}
  COMMAND=${OUT#*$'\x1f'}
else
  # JSON escapes, so a word split by a backslash reads whole: a continuation and a
  # backslash go, `\"` is a quote, any other pair (`\n` before `git`) a blank. Two
  # literal backslashes go first, or the newline after them reads as continued.
  q='"'
  COMMAND=${INPUT//\\\\\\\\/ }
  COMMAND=${COMMAND//\\\\\\n/}
  COMMAND=${COMMAND//\\\\/}
  COMMAND=${COMMAND//\\\"/$q}
  COMMAND=${COMMAND//\\?/ }
  CWD=
  NOJQ=1
fi

# printf, not jq: no reason holds a `"` or `\`, and jq may be missing.
ask() {
  printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"ask","permissionDecisionReason":"push-gate: %s"}}\n' "$1"
  exit 0
}

# A `gh` call can merge in forms the settings rules do not match (flags before the
# subcommand, quotes, expansions, `gh api`), so each `gh` in the command's text asks
# unless it has a shape listed below; a mention in a message or heredoc asks too. A
# `gh api` call is read word by word to the end of its command: a merge path or mutation,
# a body from a file and anything the shell or `gh` would rewrite ask. So does a `(`, which
# zsh globs rewrite a word with. Another word can rewrite a call, add words to it or parse
# it again (`xargs`, `eval`, `bash -c`, a heredoc fed to a shell), so a call also asks
# unless it starts a command: outside quotes, before any heredoc, and after only shell
# keywords and plain `NAME=value` words. Quotes are paired only as far as that needs, so
# text the shell may pair another way asks too, and so does a call after a backtick or a
# `${`, either of which can change how the shell reads it, and so does a backslash-newline
# between two non-blank characters or an escape such as `\n` right before `gh`. Without jq
# every call asks: the payload's own quotes cannot be told from the command's. A merge the
# text does not show (a script file, `gh` held in a variable, another API client) passes:
# branch protection is the backstop. A perl that fails asks.
# shellcheck disable=SC2016 # perl's variables
GH_SCAN='
sub api_ok {
  local $_ = shift;
  my ($word, $bare, $rewritten, $target);
  my $take = sub {
    my $ok = 1;
    if (defined $word) {
      (my $path = $word) =~ s!^/?repos/\{owner\}/\{repo\}(?=[/?]|\z)!!;
      $ok = !$rewritten && $bare !~ /[*?\[]|\{.*(?:,|\.\.).*\}/s
        && $path !~ /\{(?:owner|repo|branch)\}|:(?:owner|repo|branch)(?!\w)/
        && $word !~ /^--input(?:=|\z)|=\@|%|\/merge(?:s|-upstream)?(?![\w-])|mergePullRequest|mergeBranch|enablePullRequestAutoMerge|enqueuePullRequest/i
        unless $target;
      $target = 0;
    }
    ($word, $bare, $rewritten) = (undef, "", 0);
    return $ok;
  };
  $take->();
  while (!/\G(?=\z|[;|\n()]|&(?!>))/gc) {
    if (/\G[ \t]+/gc) { $take->() or return 0 }
    elsif (/\G(?:&>>?|>>?[&|]?|<<?<?[&>]?)/gc) { $take->() or return 0; $target = 1 }
    elsif (/\G\x27([^\x27]*)\x27?/gc) { $word .= $1 }
    elsif (/\G"((?:\\.|[^"\\])*)"?/gcs) { $word .= $1; $rewritten ||= $1 =~ /[\$`]/ }
    elsif (/\G\\(.)/gcs) { $word .= $1 }
    elsif (/\G([\$`])/gc) { $rewritten = 1; $word .= $1 }
    elsif (/\G(.)/gcs) { $word .= $1; $bare .= $1 }
  }
  return 0 if /\G\(/;
  return $take->();
}
local $/;
my $command = <STDIN>;
exit 1 if $ARGV[0];
# A backslash-newline between two non-blank characters glues them in the join below, but a
# comment, a quote or a quoted heredoc delimiter keeps the line break, so the call after it
# would go unseen (`#x\<LF>gh`).
exit 1 if $command =~ /\S\\\n\S/;
# Line continuations go, pairing escapes left to right: the newline after `\\` stays.
$command =~ s/\\(?:\n|(.))/defined $1 ? "\\$1" : ""/ge;
# An escape (`$\x27..\ngh\x27`, `printf`, `echo -e`) can end in a letter right before a `gh` the shell
# reads as a command, but the call regex below sees a word there.
exit 1 if $command =~ /\\\w+g[\\"\x27\$]*h(?![\w.\/:=\@-])/i;
while ($command =~ /(?<![\w.-])g[\\"\x27\$]*h[\\"\x27]*+(?![\w.\/:=\@-])/gi) {
  my $rest = substr($command, pos($command));
  (my $flat = $rest) =~ tr/"\x27\\//d;
  # The text before this call, each closed quote and escape read as Q. A quote the shell may
  # pair another way stays: one opened by `$\x27`, or a `"` holding `$(`.
  my $raw = substr($command, 0, $-[0]);
  (my $before = $raw) =~ s/(?<!\$)\x27[^\x27]*\x27|"(?:\\.|\$(?!\()|[^"\\\$])*"|\\./Q/g;
  # "$( opens a command; the string before it must be plain text, or the rewrite would drop a
  # heredoc, comment or quote that the checks below need.
  $before =~ s/"[^"\x27\$<#()]*\$\(\z/(/;
  my ($group, $sub, $next) = $flat =~ /^[ \t]*([^\s;&|()<>`]*)[ \t]*([^\s;&|()<>`]*)(.?)/;
  exit 1 if $next =~ /[(`<]/
    # A backtick strips escapes from the call it holds, and a `;` inside `${` ends no command.
    || $raw =~ /`|\$\{/
    # A quote left over, a comment or a heredoc.
    || $before =~ /["\x27]|(?<![^\s;&|()<>!])#|(?<!<)<<(?!<)/
    || $before !~ /(?:\A|[;&|\n(])[ \t]*(?:(?:if|then|elif|else|do|while|until|!|\{)[ \t]+)*(?:[A-Za-z_]\w*=[\w.\/:,\@%+-]*[ \t]+)*\z/;
  exit 1 unless
      $group eq "pr" ? $sub =~ /^(?:view|list|diff|checks|status|create|comment|edit|ready|checkout)\z/
    : $group eq "api" ? api_ok($rest)
    : $group eq "auth" ? $rest =~ /^[ \t]*auth[ \t]+status(?:[ \t]+2>&1)*[ \t]*(?=\z|[;|\n)]|&(?!>))/
    : $group eq "repo" ? $sub =~ /^[a-z][a-z-]*\z/ && $sub ne "sync"
    : $group =~ /^(?:run|issue|workflow|search|label|release|browse)\z/;
}
'
if [[ $COMMAND =~ $GH_SPELT ]] && ! perl -e "$GH_SCAN" "$NOJQ" <<<"$COMMAND"; then
  ask "a gh call that can merge or print the token needs the user's go, and so does a gh shape the gate does not list, a mention in text included."
fi

# A push this misses runs unprompted under the settings' git allow rules, and
# aliases (`-c alias.p=push`), quoting and `send-pack` defeat any shape
# match. So any command naming git and push or send-pack is a candidate; a
# false match only costs a prompt.
GIT_WORD='(^|[^[:alnum:]_.-])git([^[:alnum:]_-]|$)'
PUSH_WORD='(^|[^[:alnum:]_-])(push|send-pack)([^[:alnum:]_-]|$)'
[[ $COMMAND =~ $GIT_WORD && $COMMAND =~ $PUSH_WORD ]] || exit 0

CANONICAL='^[[:blank:]]*git([[:blank:]]+-C[[:blank:]]+([[:alnum:]_./-]+))?[[:blank:]]+push([[:blank:]]+(-u|--set-upstream))?[[:blank:]]+origin[[:blank:]]+([[:alnum:]._/][[:alnum:]._/-]*)[[:blank:]]*$'
[[ "$COMMAND" =~ $CANONICAL ]] || ask "only 'git [-C <path>] push [-u|--set-upstream] origin <branch>', alone in its own call, pushes without this prompt."
path=${BASH_REMATCH[2]:-.}
branch=${BASH_REMATCH[5]}

# git sends `git push origin <branch>` to another ref (`main`, say) under a remote.origin.push
# mapping or mirror, or push.default upstream (old name tracking) with the branch tracking
# another one, and a legacy remotes/origin or branches/origin file holds push lines git config
# never shows (builtin/push.c refspec_append_mapped, remote.c). Each asks, even for a branch
# already on origin. `config -l -z` ends each `key\nvalue` entry with a NUL, so a value holding
# a newline cannot pass for a key; the last push.default wins, as in git.
remapped() (
  cd "${CWD:-.}" 2>/dev/null || return 0
  local entry default= remote_file branch_file
  while IFS= read -r -d '' entry; do
    case ${entry%%$'\n'*} in
      remote.origin.push | remote.origin.mirror) return 0 ;;
      push.default) default=${entry#*$'\n'} ;;
    esac
  done < <(git -C "$path" config -l -z 2>/dev/null)
  [[ $default == upstream || $default == tracking ]] && return 0
  { IFS= read -r remote_file; IFS= read -r branch_file; } < <(
    git -C "$path" rev-parse --path-format=absolute --git-path remotes/origin --git-path branches/origin 2>/dev/null)
  [ -e "$remote_file" ] || [ -e "$branch_file" ]
)
remapped && ask "origin's config (remote.origin.push or mirror, push.default upstream or tracking, or a legacy remotes/ or branches/ file) can send '$branch' to another ref, so it needs the user's go."

# The lead lists `<branch> <push url> ...` for a branch whose Design or Approach the
# user approved. The file sits beside frozen-allow.txt, named by the
# user's settings `env` like AGENT_FROZEN_ALLOW: a path the command can't choose and
# the worker can't write. The push skips the prompt only to that URL, since `-C` or a
# redirected .git file can aim origin anywhere.
trusted=
if [[ -n ${AGENT_APPROVED_PLANS:-} ]]; then
  while read -r plan_branch plan_url _ || [[ -n $plan_branch ]]; do
    [[ $plan_branch == "$branch" ]] && { trusted=$plan_url; break; }
  done 2>/dev/null <"$AGENT_APPROVED_PLANS"
fi

# The push goes to every push URL of origin (pushurl, pushInsteadOf), which
# can differ from the fetch URL a plain `ls-remote origin` reads, so more than
# one asks. A hook that outlives its 15 s settings timeout blocks the push
# (onFailure: block; a client older than 2.1.295 lets it through), so ls-remote
# gets what is left of 8 s after the work above (`$SECONDS`; macOS has no
# `timeout`), none at under 1 s, and a killed or skipped one asks. Its own
# session has no controlling terminal, so an ssh passphrase or host-key prompt
# fails at once instead of outliving git and reading the user's keystrokes;
# GIT_TERMINAL_PROMPT=0 does the same for an https credential prompt.
# ls-remote runs from / with the URL, outside the repo: the hook runs outside the
# sandbox, and a worker can point its worktree's .git file at a config whose
# core.sshCommand or protocol.ext.allow would then run. The `--` keeps a URL such as
# --upload-pack=<cmd> from reading as an option. A <transport>::<address> URL, or a
# scheme git has no built-in transport for, runs git-remote-<transport> from PATH:
# the `::` test refuses the first form, GIT_PROTOCOL_FROM_USER=0 the second (an
# always-allowed ssh:: gets past it), and a local path stays allowed.
if ! (cd "${CWD:-.}" && url=$(git -C "$path" remote get-url --push --all origin) && [[ $url != *$'\n'* && $url != *::* ]] \
  && { [[ -n $trusted && $url == "$trusted" ]] || { left=$((8 - SECONDS)) && [[ $left -ge 1 ]] && GIT_TERMINAL_PROMPT=0 GIT_PROTOCOL_FROM_USER=0 perl -MPOSIX -e 'POSIX::setsid(); alarm shift; exec @ARGV' "$left" \
    git -C / -c protocol.file.allow=always ls-remote --exit-code --heads -- "$url" "refs/heads/$branch"; }; }) >/dev/null 2>&1 </dev/null; then
  ask "'$branch' is not on origin's push URL (a first push, a push after merge, several push URLs, a listing for another push URL, or origin unreachable or slow), so it needs the user's go."
fi
exit 0
