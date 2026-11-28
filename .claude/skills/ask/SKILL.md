---
name: ask
disable-model-invocation: true
allowed-tools: Read, Grep, Bash(git rev-parse*), Bash(git merge-base*), WebSearch, mcp__linear-server__get_issue, mcp__linear-server__list_comments
description: Answer a question about a PR, a branch or a new tech: explain the change, or give a verdict on the tech.
---

# Ask

> **Setup:** the `mcp__linear-server__*` tool names are placeholders for your issue tracker's MCP tools: swap them.

One entry point for a question. The first word of `$ARGUMENTS` picks the mode; the rest is that mode's input. With no mode, or one not in the table, list the modes and ask which one.

| Mode | Use it to | Read |
| -- | -- | -- |
| `pr <n>` | explain what a PR changes and why, in reading order | [BRANCH.md](BRANCH.md), its PR number path |
| `branch [<name>]` | explain a branch the same way (the current branch by default) | [BRANCH.md](BRANCH.md), its Branch path |
| `tech <name or question>` | dig into a new tech and give a verdict: adopt, try or skip | [TECH.md](TECH.md) |

Read the mode's file now and follow it; the run is done when that file's last step is.

## Shared rules

- **Answer; the verdict on a PR is the reviewer's.** Findings and suggested fixes belong to `pr-ci-review`, a PR body to `pr-description`, a hunt for bugs to `/bugs`.
- **Every claim has its evidence**: a `path:line`, a commit, a ticket, a doc or a source URL. A claim with none is left out, or marked as not stated anywhere.
- **Read-only.** `/ask` posts nothing, writes nothing to the tracker or GitHub, and changes nothing in the working tree or the branch (a `git fetch` only updates `refs/remotes`); the only other writes are the ones the user picks at the end of `tech`: an Epic through `file-ticket`, or a prototype in the scratchpad.
