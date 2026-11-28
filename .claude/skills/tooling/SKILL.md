---
name: tooling
disable-model-invocation: true
allowed-tools: Bash(git fetch origin ci/review-metrics), Bash(git ls-tree --name-only origin/ci/review-metrics records/), Bash(git cat-file -p origin/ci/review-metrics:records/*), Read, Edit, Write, Glob, Grep, Agent, Skill
model: opus
description: Audit the agent tooling (skills, agents, hooks, settings) against the trunk, run the review retrospective, or run a retrospective on one coding session.
---

# Tooling

`/tooling <mode> [args]`. Read the mode's file in this folder, then follow it.

| Mode | File | Use |
| -- | -- | -- |
| `audit` | [AUDIT.md](AUDIT.md) | find where skills, agents, hooks and settings disagree with the trunk or with each other, rank the fixes |
| `retro` | [RETRO.md](RETRO.md) | mine past review runs and propose fixes to the review setup |
| `session` | [SESSION.md](SESSION.md) | mine one session's transcript for improvements to the agent's environment |

With no mode, or one not in the table, list the modes and ask which one.
