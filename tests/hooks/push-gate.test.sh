#!/usr/bin/env bash
# Hand-run cases for push-gate.sh: `bash tests/hooks/push-gate.test.sh`.
# Each case feeds a command to the hook as a PreToolUse payload and checks that
# it either prints nothing (defers) or asks. The commands are never run; the
# hook's ls-remote reads throwaway bare remotes only.

set -uo pipefail

HOOK="$(cd "$(dirname "$0")/../../.claude/hooks" && pwd)/push-gate.sh"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0

export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t

# expect none|ask <cwd> <command> [text the ask reason must contain]
expect() {
  local want=$1 cwd=$2 command=$3 reason=${4:-} out rc got
  out=$(jq -n --arg c "$command" --arg d "$cwd" '{tool_input: {command: $c}, cwd: $d}' | bash "$HOOK")
  rc=$?
  if [ "$rc" -ne 0 ]; then
    got="exit $rc"
  elif [ -z "$out" ]; then
    got=none
  elif [ "$(jq -r '.hookSpecificOutput.permissionDecision' <<<"$out")" = ask ]; then
    got=ask
  else
    got="other: $out"
  fi
  if [ "$got" != "$want" ] || { [ -n "$reason" ] && [[ "$out" != *"$reason"* ]]; }; then
    printf 'FAIL want=%s got=%s  %s\n' "$want" "$got" "$command"
    failures=$((failures + 1))
  fi
}

git init -q --bare "$T/remote.git"
git init -q -b feature/x "$T/w"
git -C "$T/w" commit -q --allow-empty -m one
git -C "$T/w" remote add origin "$T/remote.git"
git -C "$T/w" push -q origin feature/x feature/x:feature/merged
git -C "$T/w" push -q origin --delete feature/merged
git init -q -b feature/x "$T/w2"
git -C "$T/w2" remote add origin "$T/unreachable.git"
git init -q -b feature/x "$T/slow"
git -C "$T/slow" remote add origin git@example.invalid:slow.git
git init -q -b feature/x "$T/ev"
git -C "$T/ev" remote add origin git@example.invalid:ev.git
git -C "$T/ev" config core.sshCommand "touch $T/pwned-ssh #"
git init -q -b feature/x "$T/opt"
git -C "$T/opt" remote add origin x
git -C "$T/opt" config remote.origin.url "--upload-pack=touch $T/pwned-opt;git-upload-pack $T/remote.git"
git init -q -b feature/x "$T/hp"
git -C "$T/hp" remote add origin "pwn::$T/remote.git"
mkdir "$T/helper"
for h in pwn ssh; do
  printf '#!/bin/sh\ntouch "%s"\n' "$T/pwned-helper" >"$T/helper/git-remote-$h"
  chmod +x "$T/helper/git-remote-$h"
done
git init -q --bare "$T/fork.git"
git init -q -b feature/x "$T/w3"
git -C "$T/w3" remote add origin "$T/remote.git"
git -C "$T/w3" remote set-url --push origin "$T/fork.git"
git init -q -b feature/x "$T/tty"
git -C "$T/tty" remote add origin git@example.invalid:tty.git
ln -s "$T/w" "$T/w{,}"
mkdir "$T/lit"
ln -s "$T/w" "$T/lit/\$HOME"
ln -s "$T" "$T/lit/~"
git init -q -b feature/x "$T/w4"
git -C "$T/w4" remote add origin "$T/remote.git"
git -C "$T/w4" remote set-url --add --push origin "$T/remote.git"
git -C "$T/w4" remote set-url --add --push origin "$T/fork.git"
mkdir "$T/nojq"
printf '#!/bin/sh\nexit 127\n' >"$T/nojq/jq"
chmod +x "$T/nojq/jq"
printf '%s\n' "export GIT_SSH_COMMAND=\"sh -c 'exec 3</dev/tty && touch $T/hook-had-tty' #\"" >"$T/tty.env"

expect none "$T/w" 'git status'
expect none "$T/w" 'git commit -m "fix the gate"'
expect none "$T/w" 'docker push registry/image'

# A later push to a branch on origin defers to the settings rules.
expect none "$T/w" 'git push origin feature/x'
expect none "$T/w" 'git push -u origin feature/x'
expect none "$T/w" 'git push --set-upstream origin feature/x'
expect none / "git -C $T/w push origin feature/x"
expect none "$T" 'git -C w push origin feature/x'

# The branch -C names decides, not the payload's cwd.
expect none "$T/w2" "git -C $T/w push origin feature/x"
expect ask "$T/w" "git -C $T/w2 push origin feature/x"

expect ask "$T/w" 'git push origin feature/new' feature/new
expect ask "$T/w" 'git push -u origin feature/new' feature/new
expect ask "$T/w" 'git push origin feature/merged' feature/merged
expect ask "$T/w2" 'git push origin feature/x' feature/x
expect ask /nonexistent 'git push origin feature/x'

# The push goes to the push URL, so a branch only on the fetch URL asks.
expect ask "$T/w3" 'git push origin feature/x' feature/x
# It goes to every push URL, and the branch is on the first only.
expect ask "$T/w4" 'git push origin feature/x' feature/x

# A branch in the file AGENT_APPROVED_PLANS names (the lead writes `<branch> <push
# url> ...`) pushes without the prompt to that URL, even before it is on origin; every
# other branch, and that branch aimed at another URL, asks.
printf 'feature/new %s KEY-1 12:00\n' "$T/remote.git" >"$T/plans"
export AGENT_APPROVED_PLANS=$T/plans
expect none "$T/w" 'git push origin feature/new'
expect ask "$T/w2" 'git push -u origin feature/new' feature/new
expect ask "$T/w3" 'git push origin feature/new' feature/new
expect ask "$T/w" 'git push origin feature/ne' feature/ne
expect ask "$T/w" 'git push origin feature/merged' feature/merged
expect ask "$T/w" 'git push origin feature/new && echo done'
# The lead's last line may lack its newline; awk matched it, so the read loop must too.
printf 'feature/new %s' "$T/remote.git" >"$T/plans-nonl"
AGENT_APPROVED_PLANS=$T/plans-nonl expect none "$T/w" 'git push origin feature/new'

# A config that sends the push to another ref asks for a listed branch; the reason tells it
# from the other asks.
for setting in 'remote.origin.push refs/heads/feature/x:refs/heads/main' 'remote.origin.mirror true' \
  'push.default upstream' 'push.default tracking'; do
  read -r key value <<<"$setting"
  git -C "$T/w" config "$key" "$value"
  expect ask "$T/w" 'git push origin feature/new' 'another ref'
  git -C "$T/w" config --unset "$key"
done
# The check runs before the listed and on-origin paths split, so it gates a branch already on origin too.
git -C "$T/w" config remote.origin.mirror true
expect ask "$T/w" 'git push origin feature/x' 'another ref'
git -C "$T/w" config --unset remote.origin.mirror
# Another remote's mapping or mirror does not touch a push to origin.
git -C "$T/w" config remote.other.push refs/heads/feature/x:refs/heads/main
git -C "$T/w" config remote.other.mirror true
expect none "$T/w" 'git push origin feature/new'
git -C "$T/w" config --remove-section remote.other
# The last push.default wins, as in git, and a value holding a newline is no key.
git -C "$T/w" config push.default upstream
git -C "$T/w" config --add push.default simple
expect none "$T/w" 'git push origin feature/new'
git -C "$T/w" config --unset-all push.default
git -C "$T/w" config push.default upstream
git -C "$T/w" config alias.x $'a\npush.default=simple'
expect ask "$T/w" 'git push origin feature/new' 'another ref'
git -C "$T/w" config --unset push.default
git -C "$T/w" config --unset alias.x
# A legacy remotes/origin or branches/origin file holds push lines git config never shows. A
# linked worktree, where agents push from, reads them from the common dir.
mkdir "$T/w/.git/remotes" "$T/w/.git/branches"
printf 'URL: %s\nPush: refs/heads/feature/x:refs/heads/main\n' "$T/remote.git" >"$T/w/.git/remotes/origin"
expect ask "$T/w" 'git push origin feature/new' 'another ref'
rm "$T/w/.git/remotes/origin"
printf '%s#main\n' "$T/remote.git" >"$T/w/.git/branches/origin"
expect ask "$T/w" 'git push origin feature/new' 'another ref'
git -C "$T/w" worktree add -q -b feature/wt "$T/wt"
expect ask "$T/wt" 'git push origin feature/new' 'another ref'
rm "$T/w/.git/branches/origin"
expect none "$T/wt" 'git push origin feature/new'
expect none "$T/w" 'git push origin feature/new'
# Only that file counts: not one in the repo, and not an unset or missing one.
printf 'feature/new KEY-1 12:00\n' >"$T/w/.git/approved-plans"
unset AGENT_APPROVED_PLANS
expect ask "$T/w" 'git push origin feature/new' feature/new
AGENT_APPROVED_PLANS=$T/missing expect ask "$T/w" 'git push origin feature/new' feature/new

# Without jq the hook reads the raw payload: a push or any gh call asks, `git status` gets nothing.
for command in 'git push -u origin feature/new' $'cd x\ngit push -u origin feature/new' $'cd x;\tgit push' $'git -C x \\\npush' \
  'gh pr view 5' 'g\h pr merge 5' 'g""h pr merge 5' $'g\\\nh pr merge 5' 'bash -c "true; gh pr view 5"' \
  $'true #x\\\ngh pr view 5' $'true #x\\\ngh api -X PUT repos/o/r/pulls/5/merge' \
  $'git -C . -c alias.m=$\'!true\\ngh pr merge 5\' m' $'bash -c \'#x\\\ngh pr merge 5\'' \
  $'echo a\\\\\ngit push' 'git status'; do
  out=$(jq -n --arg c "$command" --arg d "$T/w" '{tool_input: {command: $c}, cwd: $d}' | PATH="$T/nojq:$PATH" bash "$HOOK")
  case "$command:$out" in
    'git status:') ok=1 ;;
    'git status:'*) ok= ;;
    *'"permissionDecision":"ask"'*) ok=1 ;;
    *) ok= ;;
  esac
  if [[ -z $ok ]]; then
    printf 'FAIL without jq  %s -> %s\n' "$command" "$out"
    failures=$((failures + 1))
  fi
done

# A call naming neither push nor send-pack starts no process: no jq, cat or grep runs.
mkdir "$T/spy"
for b in jq cat grep; do
  printf '#!/bin/sh\necho %s >>"%s"\nexit 127\n' "$b" "$T/spy.log" >"$T/spy/$b"
  chmod +x "$T/spy/$b"
done
for command in 'git status' 'git commit -m "fix the gate"'; do
  # A fixed cwd: mktemp's random names can spell a word the early test looks for.
  jq -n --arg c "$command" '{tool_input: {command: $c}, cwd: "/w"}' >"$T/spy.json"
  out=$(PATH="$T/spy:$PATH" bash "$HOOK" <"$T/spy.json")
  if [ -n "$out" ] || [ -s "$T/spy.log" ]; then
    printf 'FAIL no process  %s -> %s, ran: %s\n' "$command" "$out" "$(tr '\n' ' ' <"$T/spy.log" 2>/dev/null)"
    failures=$((failures + 1))
  fi
  rm -f "$T/spy.log"
done
# A payload naming push only outside the command passes the early test and still defers.
mkdir "$T/push"
expect none "$T/push" 'git status'

# An ls-remote that outlives the hook's timeout would block the push (an older
# client lets it through), so a slow origin must ask inside it: the hook's
# alarm kills ls-remote before the ssh ends, and stays below the timeout. An
# order, not a time, which load would stretch.
GIT_SSH_COMMAND="sleep 30; touch '$T/ssh-ended' #" expect ask "$T/slow" 'git push origin feature/x' feature/x
if [ -e "$T/ssh-ended" ]; then
  echo "FAIL slow origin: the hook waited for the ssh to end"
  failures=$((failures + 1))
fi
# ls-remote gets what is left of the budget after the hook's own work: a late start
# gets less, and under 1 s it is not started and asks.
# shellcheck disable=SC2016 # sed must see the hook's text literally
budget=$(sed -n 's/.*\$((\([0-9]*\) - SECONDS)).*/\1/p' "$HOOK")
timeout=$(jq '[.. | objects | select(.command? | type == "string" and endswith("/push-gate.sh")) | .timeout] | first' "${HOOK%/*}/../settings.json")
if ! [ "$budget" -lt "$timeout" ] 2>/dev/null; then
  echo "FAIL push-gate.sh's budget (${budget:-none} s) is not below its settings timeout ($timeout s)"
  failures=$((failures + 1))
fi
SECONDS=$budget expect ask "$T/w" 'git push origin feature/x' feature/x
mkdir "$T/perl"
# shellcheck disable=SC2016 # the stub's own $4, the alarm perl is given
printf '#!/bin/sh\necho "$4" >>"%s"\nexit 1\n' "$T/perl.log" >"$T/perl/perl"
chmod +x "$T/perl/perl"
# The hook's own time before ls-remote only lowers the alarm, so each bound holds at any load.
for elapsed in 0 4; do
  rm -f "$T/perl.log"
  PATH="$T/perl:$PATH" SECONDS=$elapsed expect ask "$T/w" 'git push origin feature/x' feature/x
  alarm=$(cat "$T/perl.log" 2>/dev/null || echo)
  if { [ "$elapsed" = 0 ] && [ -z "$alarm" ]; } || { [ -n "$alarm" ] && ! [ "$alarm" -le "$((budget - elapsed))" ]; }; then
    echo "FAIL alarm after $elapsed s: ${alarm:-none}, want at most $((budget - elapsed))"
    failures=$((failures + 1))
  fi
done

# ls-remote runs without a controlling terminal, so an ssh prompt can't read
# the user's keys. Checked only where `script` gives this run a terminal.
if script -q /dev/null sh -c "exec 3</dev/tty && touch $T/run-had-tty" </dev/null >/dev/null 2>&1 && [ -e "$T/run-had-tty" ]; then
  jq -n --arg c 'git push origin feature/x' --arg d "$T/tty" '{tool_input: {command: $c}, cwd: $d}' >"$T/tty.json"
  script -q /dev/null sh -c ". '$T/tty.env'; bash '$HOOK' <'$T/tty.json'" </dev/null >/dev/null 2>&1
  if [ -e "$T/hook-had-tty" ]; then
    echo "FAIL ls-remote reached the controlling terminal"
    failures=$((failures + 1))
  fi
else
  echo "SKIP the terminal check: no terminal from script"
fi

# Any other shape asks, even for a branch already on origin.
expect ask "$T/w" 'git push'
expect ask "$T/w" 'git push origin'
expect ask "$T/w" 'git push origin refs/tags/v1'
expect ask "$T/w" 'git push origin feature/x && gh pr edit 1 --body-file b.md'
expect ask "$T/w" "cd $T/w && git push origin feature/x"
expect ask "$T/w" 'git ls-remote origin feature/x; git push origin feature/x'
expect ask "$T/w" 'git push upstream feature/x'
expect ask "$T/w" "git -c core.hooksPath=/dev/null push origin feature/x"
# Literal `$HOME` and `~` dirs exist here, so only the -C path check can ask.
expect ask "$T/lit" "git -C \$HOME push origin feature/x"
expect ask "$T/w" $'git push\norigin feature/x'
expect ask "$T/w" $'git push origin feature/x\necho done'
# The command may hold the separator the one jq call splits on; the whole command still decides.
expect ask "$T/w" $'git push origin feature/x\x1f; echo done' 'alone in its own call'

# A push the detector missed would run unprompted under the settings' allow rules.
expect ask "$T/w" $'git -C '"$T/w"$' \\\npush origin feature/new'
expect ask "$T/w" "(cd $T/w && git push)"
expect ask "$T/w" 'git push; gh pr create'
expect ask "$T/w" "git -C $T/w --work-tree $T/w push origin feature/new"
expect ask "$T/w" "git -C $T/w --git-dir=$T/w/.git push origin feature/new"
expect ask "$T/w" 'git -C "my dir" push origin feature/x'
expect ask "$T/w" "git -c 'user.name=a b' push origin feature/x"
# shellcheck disable=SC2016 # the hook must see $ROOT unexpanded, as the agent wrote it
expect ask "$T/w" 'git -C "$ROOT"/.worktrees/x push -u origin feature/new'
expect ask "$T/w" 'git -c core.sshCommand="ssh -i k" push origin feature/x'
expect ask "$T/w" 'git -C x --git-dir="/a b/.git" push origin feature/new'
expect ask "$T/w" 'git -C my\ dir push origin feature/x'
expect ask "$T/w" 'git -C . "push" origin feature/x'
expect ask "$T/w" 'git -C . -c alias.p=push p origin feature/x'
expect ask "$T/w" 'git -C . subtree push --prefix=d origin feature/x'
expect ask "$T/w" 'git -C . send-pack origin feature/x'
expect ask "$T/lit" 'git -C ~/w push origin feature/x'
expect ask "$T/w" "git -C $T/w{,} push origin feature/x"

# A false match costs only a prompt.
expect ask "$T/w" 'git commit -m "push the fix"'

# The hook runs outside the sandbox, and a worker can point its worktree's .git
# file at a repo whose config it wrote; ls-remote must not run that config.
expect ask "$T/ev" 'git push origin feature/x' feature/x
if [ -e "$T/pwned-ssh" ]; then
  echo "FAIL ls-remote ran the repo's core.sshCommand"
  failures=$((failures + 1))
fi
expect ask "$T/opt" 'git push origin feature/x' feature/x
if [ -e "$T/pwned-opt" ]; then
  echo "FAIL ls-remote read origin's URL as an option"
  failures=$((failures + 1))
fi
# A <transport>::<address> URL, or a scheme with no built-in transport, runs
# git-remote-<transport> from PATH, ssh:: included.
for u in "pwn::$T/remote.git" "pwn://x" "ssh::$T/remote.git"; do
  git -C "$T/hp" remote set-url origin "$u"
  PATH="$T/helper:$PATH" expect ask "$T/hp" 'git push origin feature/x' feature/x
  if [ -e "$T/pwned-helper" ]; then
    echo "FAIL ls-remote ran the remote helper $u names"
    failures=$((failures + 1))
    rm "$T/pwned-helper"
  fi
done

# A gh call that can merge asks, which beats the settings' `gh pr *` and `gh api*` allow rules.
while IFS= read -r command; do
  expect ask "$T/w" "$command" 'can merge'
done <<'CASES'
gh pr merge 5
gh pr -R o/r merge 5
gh pr --repo=o/r merge 5
gh -R o/r pr merge 5
/opt/homebrew/bin/gh pr merge 5
env GH_REPO=o/r gh pr merge 5
cd x && gh pr merge 5
gh pr view 5; gh pr merge 5
echo $(gh pr merge 5)
x=`gh pr merge 5`
(gh pr merge 5)
bash -c 'gh pr merge 5'
echo 5 | xargs gh pr merge
echo pr merge 5 | xargs gh
gh pr "merge" 5
gh pr 'mer'ge 5
gh pr me\rge 5
g''h pr merge 5
g\h pr merge 5
"gh" pr merge 5
gh pr ${m} 5
gh pr mer${x}ge 5
gh pr $'merge' 5
gh pr $(echo merge) 5
gh pr {merge,5}
gh {pr,merge} 5
gh $cmd merge 5
gh pr update-branch 5
gh pr
gh api -X PUT repos/o/r/pulls/5/merge
gh api repos/o/r/pulls/5/merge --method PUT
gh api "repos/o/r/pulls/5/merge" -XPUT
gh api -X PUT repos/o/r/pulls/5/me''rge
gh api -X PUT repos/o/r/pulls/5/%6Derge
gh api repos/o/r/merges -f base=main -f head=x
gh api repos/o/r/merge-upstream -f branch=main
gh api graphql -f query='mutation{mergePullRequest(input:{pullRequestId:"x"}){clientMutationId}}'
gh api graphql -f query='mutation{mergeBranch(input:{base:"main",head:"x"}){clientMutationId}}'
gh api graphql -f query='mutation{enablePullRequestAutoMerge(input:{pullRequestId:"x"}){clientMutationId}}'
gh api graphql -f query='mutation{enqueuePullRequest(input:{pullRequestId:"x"}){clientMutationId}}'
gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"x"}){thread{isResolved}} m:mergePullRequest(input:{pullRequestId:"y"}){clientMutationId}}'
gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"x"}){thread{isResolved}} m:'$'\x6d''ergePullRequest(input:{pullRequestId:"y"}){clientMutationId}}'
gh api graphql --input q.json
gh api graphql --input=q.json
gh api graphql -F query=@q.graphql
gh api graphql -F 'query=@q.graphql'
gh api graphql -F query=@- <<'EOF'
gh api graphql -f query="$(cat q.graphql)"
gh api graphql -f query=$q
gh api -X PUT repos/o/r/pulls/5/${m}
gh api -X PUT repos/o/r/pulls/5/$m
gh api -X PUT "repos/o/r/pulls/5/$m"
gh api -X PUT repos/o/r/pulls/5/$'merge'
gh api -X PUT repos/o/r/pulls/5/$(cat m)
gh api -X PUT repos/o/r/pulls/5/`cat m`
gh api -X PUT repos/o/r/pulls/5/{merge,}
gh api -X PUT repos/o/r/pulls/5/m{e..e}rge
gh api -X PUT repos/o/r/pulls/5/m*
gh api -X PUT repos/o/r/pulls/5/me\rge
gh api >/dev/null -X PUT repos/o/r/pulls/5/$m
gh api &>/dev/null -X PUT repos/o/r/pulls/5/$m
gh api -X PUT >(cat) repos/o/r/pulls/5/$m
gh api -f t="a | b" -X PUT repos/o/r/pulls/5/$m
gh api -X PUT repos/o/r/pulls/5/$m # it's
x="$(gh api -X PUT repos/o/r/pulls/5/$m)"
bash -c 'gh api -X PUT repos/o/r/pulls/5/$m'
gh "api" -X PUT repos/o/r/pulls/5/$m
curl -X PUT -H "Authorization: token $(gh auth token)" https://api.github.com/repos/o/r/pulls/5/merge
gh auth token
gh alias set m 'pr merge'
gh m 5
gh extension exec x
gh auth status --show-token
gh auth status -t
gh repo sync o/r
gh repo "sync" o/r
GH pr merge 5
git commit -m "a gh pr merge is the user's"
echo gh pr merge 5
gh pr view(:s/view/merge/) 5
gh pr (merge) 5
gh run(:s/run/pr/) merge 5
gh api -X PUT repos/o/r/pulls/5/(merge)
gh api -X PUT repos/o/r/pulls/5/@(merge)
gh api -X PUT "repos/o/r/pulls/5/`cat m`"
gh api graphql -f query="`cat q.graphql`"
gh api -X PUT repos/o/r/pulls/5/m?rge
gh api -X PUT repos/o/r/pulls/5/m[e]rge
gh api -X PUT repos/o/r/pulls/5/Merge
gh api -X PUT repos/{owner}/{repo}/pulls/5/{branch}
gh api -X PUT repos/o/r/pulls/5/:branch
GH_REPO=o/merge gh api -X PUT repos/o/r/pulls/5/{repo}
echo repos/o/r/pulls/5/merge | xargs gh api -X PUT
xargs gh api -X PUT <<< repos/o/r/pulls/5/merge
find . -exec gh api -X PUT {} \;
parallel gh api -X PUT ::: repos/o/r/pulls/5/merge
echo -t | xargs gh auth status
gh auth status {-t,}
gh auth status $'-t'
gh auth status ${x:--t}
gh auth status -?
gh auth status x(:s/x/-t/)
gh auth status 2>&1 -t
gh auth status ";" -t
gh auth status &>/dev/null -t
gh repo $'sync' o/r
gh repo {sync,} o/r
gh repo `echo sync` o/r
gh repo $(echo sync) o/r
gh repo>/dev/null sync o/r
gh repo sy`echo nc` o/r
echo repos/o/r/pulls/5/merge | parallel gh api -X PUT
echo merge | xargs -I view gh pr view 5
echo sync | xargs -I view gh repo view
echo pr | xargs -I issue gh issue merge 5
echo merge | x''args -I@ gh api -X PUT repos/o/r/pulls/5/@
x\args gh api -X PUT <<< repos/o/r/pulls/5/merge
x$''args gh api -X PUT <<< repos/o/r/pulls/5/merge
par""allel gh api -X PUT repos/o/r/pulls/5/{} ::: merge
zargs -- merge -- gh api -X PUT repos/o/r/pulls/5/
find merge '-exec' gh api -X PUT repos/o/r/pulls/5/{} \;
find repos -type f -exec"" gh api -X PUT {} \;
fd merge -x gh api -X PUT repos/o/r/pulls/5/{}
eval 'gh api -X PUT repos/o/r/pulls/5/'\$x
bash -c 'x=merge; gh api -X PUT repos/o/r/pulls/5/'\$x
bash -c "x=merge; gh api -X PUT repos/o/r/pulls/5/\$x;"
zsh -o pipefail -ic 'gh api -X PUT repos/o/r/pulls/5/'\?rge
g$'h' pr merge 5
gh api -X PUT repos/o/r/pulls/5/{owner}
gh api -X PUT repos/o/r/pulls/5/:owner
gh api -X PUT repos/o/r/pulls/5/:repo
gh api repos/{owner}/{repo}-x/pulls/5
echo repos/o/r/pulls/5/merge | "$u"xargs gh api -X PUT
echo repos/o/r/pulls/5/merge | $u''xargs gh api -X PUT
"$u"parallel gh api -X PUT ::: repos/o/r/pulls/5/merge
$SHELL -c 'gh api -X PUT repos/o/r/pulls/5/'\$x
$SHELL -c 'true; gh api -X PUT repos/o/r/pulls/5/'\$x
bash <<< 'true; gh api -X PUT repos/o/r/pulls/5/'\$x
echo 'gh api -X PUT repos/o/r/pulls/5/'\$x | sh
. <(echo 'true; gh api -X PUT repos/o/r/pulls/5/'\$x)
env -S 'gh api -X PUT repos/o/r/pulls/5/${x}'
echo merge | gxargs -I{} gh api -X PUT repos/o/r/pulls/5/{}
fdfind merge -x gh api -X PUT repos/o/r/pulls/5/{}
echo merge | x?rgs gh api -X PUT repos/o/r/pulls/5/
find . -execdir gh api -X PUT {} \;
find . -okdir gh api -X PUT {} \;
echo merge | xargs -I view sh -c 'true; gh pr view 5'
echo "x; gh pr view 5"
echo 'x; gh pr view 5'
echo x\; gh pr view 5
echo then gh pr view 5
"$u"gh pr view 5
echo "see (gh pr view 5)"
x=1;xargs gh pr view 5
bash -c $'true \'\'; gh api -X PUT repos/o/r/pulls/5/\x6derge'
git -C . -c alias.m=$'!: \'\'; gh api -X PUT repos/o/r/pulls/5/\x6derge' m
x=merge bash -c "$(echo "true; gh api -X PUT repos/o/r/pulls/5/\$x")"
eval "${u:-"true; gh api -X PUT repos/o/r/pulls/5/\$x"}"
echo merge | xargs -I view `` gh pr view 5
x=merge; eval `true` gh api -X PUT repos/o/r/pulls/5/\$x
x=`: '`; bash -c 'true; gh pr view 5'
x=`: "`; bash -c "true; gh pr view 5"
x=merge; eval ${u:-true; gh api -X PUT repos/o/r/pulls/5/'$x'}
x=merge; echo `gh api -X PUT repos/o/r/pulls/5/\$x; true`
echo "$(bash -c $'$(gh api -X PUT repos/o/r/pulls/5/\x6derge)')"
eval "$(printf %b '$(gh api -X PUT repos/o/r/pulls/5/\x6derge)')"
x=`gh pr view 5 --json title`
x=`true` gh pr view 5
CASES
expect ask "$T/w" $'# it\'s\nx=merge bash -c \'true; gh api -X PUT repos/o/r/pulls/5/\'\\$x' 'can merge'
expect ask "$T/w" $'true;# it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'echo \\\\\n\'x; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'true #x\\\ngh api -X PUT repos/o/r/pulls/5/merge' 'can merge'
expect ask "$T/w" $'true #x\\\ngh pr view 5' 'can merge'
expect ask "$T/w" $'echo a&# it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'echo a|# it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'(# it\'s\nbash -c \'true; gh pr view 5\')' 'can merge'
expect ask "$T/w" $'echo a)# it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'cat <# it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'echo a ># it\'s\nbash -c \'true; gh pr view 5\'' 'can merge'
expect ask "$T/w" $'sh <<EOF\ngh api -X PUT repos/o/r/pulls/5/\\$x\nEOF' 'can merge'
# The "$( rewrite keeps what a heredoc, a comment or a quote before it hides, and a zsh `&!` starts a comment.
expect ask "$T/w" $'eval "$(sed s/view/merge/ <<\'E\'\n$(gh pr view 5)\nE\n)"' 'can merge'
expect ask "$T/w" $'eval "$(true #\'\necho \'$(gh pr view 5)\' | sed s/view/merge/)"' 'can merge'
expect ask "$T/w" $'true &!# it\'s\nx=merge bash -c \'true; gh api -X PUT repos/o/r/pulls/5/\'\\$x' 'can merge'
# A backslash-newline the shell keeps (a comment, a quote, a quoted heredoc delimiter) must not glue the next call.
expect ask "$T/w" $'bash -c \'#x\\\ngh pr merge 5\'' 'can merge'
expect ask "$T/w" $'eval \'#x\\\ngh pr merge 5\'' 'can merge'
expect ask "$T/w" $'bash -c $\'#x\\\ngh pr merge 5\'' 'can merge'
expect ask "$T/w" $'git -c alias.m=\'!#x\\\ngh pr merge 5\' m' 'can merge'
expect ask "$T/w" $'true &!#x\\\ngh pr merge 5' 'can merge'
expect ask "$T/w" $'cat <<\'x\\\'\nx\\\ngh api -X PUT repos/o/r/pulls/5/merge' 'can merge'
for sep in ';' '&' '|' '(' ')' '<' '>'; do
  expect ask "$T/w" "true$sep#x\\"$'\ngh pr view 5' 'can merge'
done
# An escape that ends in a letter before `gh` reads as a word there.
expect ask "$T/w" $'git -C . -c alias.m=$\'!true\\ngh pr merge 5\' m' 'can merge'
expect ask "$T/w" $'printf \'true\\ngh api -X PUT repos/o/r/pulls/5/merge\\n\' | sh' 'can merge'
expect ask "$T/w" $'bash -c $\'x=1;\\tgh pr merge 5\'' 'can merge'
expect ask "$T/w" 'echo \ng"h" pr view 5' 'can merge'
expect ask "$T/w" 'echo \nGH pr view 5' 'can merge'
# A string holding one of these may not be plain text, so the `$(` in it does not start a command.
for s in 'a$b' 'a`' 'a<' 'a#' 'a(' 'a)'; do
  expect ask "$T/w" "\"$s\$(gh pr view 5)\"" 'can merge'
done
# An empty substitution or a `<->` glob ends no word in zsh, so a `(` qualifier after it rewrites the call.
expect ask "$T/w" 'gh pr view``(:s/view/merge/) 5' 'can merge'
expect ask "$T/w" 'gh run``(:s/run/pr/) merge 5' 'can merge'
expect ask "$T/w" 'gh pr view<->(:s/view1/merge/) 5' 'can merge'
expect ask "$T/w" $'cat <<-EOF | sh\ngh pr view 5\nEOF' 'can merge'
expect ask "$T/w" $'gh pr \\\nmerge 5' 'can merge'
expect ask "$T/w" $'g\\\nh pr merge 5' 'can merge'
expect ask "$T/w" $'gh api -X PUT repos/o/r/pulls/5/mer\\\nge' 'can merge'
expect ask "$T/w" $'echo "it\'s"\ngh api -X PUT repos/o/r/pulls/5/$m \'x\'' 'can merge'
expect ask "$T/w" $'cat >b.md <<EOF\nnever run gh pr merge here\nEOF' 'can merge'

# Reads, the other gh commands a session runs, and the review-thread resolve defer to the settings.
while IFS= read -r command; do
  expect none "$T/w" "$command"
done <<'CASES'
gh pr view 5 --json mergeable
gh pr view 5 --json mergeStateStatus --jq .mergeStateStatus
gh pr view $n --json title
gh pr list --state merged
gh pr diff 5 | head
gh pr checks 5
gh pr status
gh pr create --draft --base main --title "fix: merge order" --body-file b.md
gh pr create --draft --base main --title "feat(tooling): ask on a merge call" --body-file "$TMPDIR/b.md"
gh pr comment 5 --body "@claude review"
gh pr edit 5 --body-file b.md
gh pr ready 5
gh pr checkout 5
sha=$(gh pr view 5 --json headRefOid --jq .headRefOid)
gh run view 1 --log-failed
gh issue list
gh repo view --json nameWithOwner
gh auth status
gh api repos/o/r/pulls/5 --jq .mergeable_state
gh api repos/{owner}/{repo}/pulls/5/comments --paginate
gh api repos/{owner}/{repo}
gh api repos/o/r/branches/merge-queue
gh api search/issues -f q="is:pr [bot] *"
gh api repos/o/r/pulls/5 --jq '.[] | "\(.id) $x"' | head -5
gh api repos/o/r/pulls/5 > "$TMPDIR/pr.json" && echo $?
gh api repos/o/r/pulls/5 2>&1 | tail -n 3
gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"PRRT_x"}){thread{isResolved}}}'
gh api graphql -f query='query($n:Int!){repository(owner:"o",name:"r"){pullRequest(number:$n){title}}}' -F n=5
git log --oneline origin/main -3
ls .github/workflows
echo $GH_TOKEN_SET high
gh auth status 2>&1 | head -3
x="$(gh auth status 2>&1)"
(gh pr view 5)
gh pr "view" 5
x="$(gh api repos/o/r/pulls/5)"
echo "pr: $(gh pr view 5 --json title)"
gh pr list --search "(draft)"
gh api repos/o/r/pulls/5 && ls | xargs echo
gh auth status; ls
gh auth status && ls
bash -n x.sh; wc -c b.md; gh pr view 5
bash x.sh | wc -c && gh pr view 5
bash x.sh && wc -c b.md && gh pr view 5
ls apps/bench && gh pr view 5
gh api /repos/{owner}/{repo}/pulls/5
gh api "repos/{owner}/{repo}?per_page=1"
gh api search/issues -f q=label:branching
echo y | gh pr comment 5 --body-file -
GH_PAGER=cat GH_REPO=o/r gh pr view 5
GH_HOST=a.b:1,x@y%z+w-v gh pr view 5
A='b c' gh pr view 5
if gh pr view 5; then gh pr diff 5; elif gh pr checks 5; then gh pr list; else gh pr status; fi
while ! gh pr checks 5; do gh pr view 5; done
until gh pr checks 5; do sleep 1; done
{ gh pr view 5; }
echo 'a b' && gh pr view 5
echo "a \" b" && gh pr view 5
echo it\'s && gh pr view 5
cat <<<x; gh pr view 5
echo "$HOME a$" '# cost $' a#b && gh pr view 5
echo $# && gh pr view 5
CASES
expect none "$T/w" $'gh auth status\nls'
expect none "$T/w" $'gh pr view 5 # a note\nls'
expect none "$T/w" $'gh pr view 5 \\\n  --json mergeable'
expect none "$T/w" $'gh pr \\\nview 5'
expect none "$T/w" $'echo \\\\\n gh pr view 5'
expect none "$T/w" $'gh pr view 5\\\n  --json mergeable'
expect none "$T/w" 'echo \ngh.exe pr view 5'
expect none "$T/w" $'bash x.sh\nwc -c b.md\ngh pr view 5'

# A perl that fails asks, since the gh check runs on it.
mkdir "$T/noperl"
printf '#!/bin/sh\nexit 127\n' >"$T/noperl/perl"
chmod +x "$T/noperl/perl"
PATH="$T/noperl:$PATH" expect ask "$T/w" 'gh pr view 5' 'can merge'

if [ "$failures" -eq 0 ]; then
  echo "push-gate: all cases pass"
else
  echo "push-gate: $failures case(s) failed"
  exit 1
fi
