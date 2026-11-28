---
name: ci-and-agent-config
description: Checklist for a change to `.github/workflows/`, `.claude/hooks/`, `.claude/settings.json` or a shell script: the parser harness cases, the macOS against ubuntu gotchas and the live-run rule that the shell check cannot see. Use when editing one of them.
---

# CI and agent-config changes

Follow-up fixes on CI, hook and shell PRs fall into three classes: a command shape the parser missed, a macOS against ubuntu difference, and runner state only a live run shows. `scripts/check-agent-config.sh` runs `bash -n` and the shell harnesses in CI (all of them) and in `scripts/verify.sh` (the ones the change can affect). This skill covers the three classes that check cannot see. Work through each section the change touches.

## 1. Harness case first

A hook that reads a command line (`push-gate.sh`) gets a red harness case for each shape the change can meet, before the fix:

- quoting: the word bare, single-quoted, double-quoted, nested, and backslash-escaped;
- chains: `&&`, `||`, `;`, a pipe, a newline, a backslash continuation, a subshell;
- prefixes: `git -C <dir>` and `git -c key=value`, repeated, before the subcommand;
- expansions: `$VAR`, `${VAR}`, `$(...)`, backticks, `{a,b}` braces;
- option forms: combined short flags (`-fu`), abbreviated long ones (`--forc`), `--opt=value` against `--opt value`;
- text that only mentions the command: a heredoc, a commit message, an `echo` argument.

`scripts/pre-push` reads git's ref lines on stdin instead: it gets a case for each of several refs in one push, a branch delete, a tag and a new branch.

A new harness is a `*.test.sh` under `scripts/` or `.claude/hooks/`, which the check finds by that name. Copy the fixture header of `scripts/pre-push.test.sh`: one `mktemp -d` root removed on exit, `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, and a fixed git identity, so fixture commits never reach the user's signing or hooks.

Done when every shape on the list the change can meet has a case that was red before the fix.

## 2. Portability

Workers run macOS: `/bin/bash` 3.2 and BSD tools. CI runs ubuntu: bash 5 and GNU tools, as the non-root `runner` user. Write each line for both, using the repo's existing answers:

- A `set -u` error inside `$((...))` exits 0 under bash 3.2: `scripts/gate-lock.sh`'s fail-closed `trap 'exit 1' EXIT` is the guard.
- File times: `scripts/gate-lock.sh`'s `mtime` tries GNU `stat -c` first, then BSD `stat -f`.
- macOS has no `timeout` and no `flock`: the repo's lock is `scripts/gate-lock.sh`'s `mkdir`.
- `jq` may be missing on a worker machine. A hook that parses its JSON payload falls back without it, as `push-gate.sh` does.
- Root ignores permission bits, so a case that makes a path unwritable fails in a root container.

Done when every tool and builtin the change calls is on the safe side of this list, or the harness passes on macOS and in the CI job.

## 3. Workflows

- A `${{ }}` expression holding PR, issue or comment text reaches `run:` only through `env:`, read as `"$VAR"`.
- `uses:` pins a full commit sha with the version as a trailing comment.
- A new job joins the `needs` of the job branch protection requires, when the workflow funnels its jobs into one; otherwise its failure never reaches the required check.
- A step that depends on runner state (an installed tool, a cache, an artifact id, a token's permissions, a fetched ref) gets a live run before review: the PR's own run, opened as a draft, or `workflow_dispatch`.

Done when the job log shows each changed step doing its work, rather than only a green tick.

## 4. Hooks and settings

Edit hook scripts in your worktree. `.claude/settings*.json` is the user's: the Edit and Write tools are denied on it, so hand the user the exact change (`agent-workflow` §8). The user reviews the diff on the PR before merge. Change the harnesses beside the hooks with them, and the row of the hook table in `.claude/README.md` whose behaviour the change moves.

Done when the harnesses pass and the README row matches the new behaviour.
