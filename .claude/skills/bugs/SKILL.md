---
name: bugs
disable-model-invocation: true
allowed-tools: Read, Grep, Glob, Agent, Bash(git ls-files:*), Bash(git rev-parse:*), Bash(git status:*), Bash(wc -l:*), Bash(mktemp:*), Bash(date:*), mcp__linear-server__list_issues, mcp__linear-server__get_issue, mcp__linear-server__save_issue
description: Hunt for bugs in an area's existing code with the review agents, or diagnose one known bug.
---

# Bugs

> **Setup:** the `mcp__linear-server__*` tool names are placeholders for your issue tracker's MCP tools: swap them.

One entry point for bug work. The first word of `$ARGUMENTS` picks the mode; the rest is that mode's input. With no mode, or one not in the table, list the modes and ask which one.

| Mode | Use it to | Read |
| -- | -- | -- |
| `hunt <area>` | review an area's code as it stands for defects no one has reported, and file what holds | [HUNT.md](HUNT.md) |
| `diagnose <symptom>` | find the cause of one known bug, test or slowdown | run `diagnose` |

Read the mode's file now and follow it, or for `diagnose` load the `diagnose` skill with the symptom; the run is done when that file's last step is.
