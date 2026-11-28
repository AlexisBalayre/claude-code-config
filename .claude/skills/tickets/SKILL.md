---
name: tickets
disable-model-invocation: true
allowed-tools: Read, Grep, Agent, Bash(git grep:*), Bash(git show:*), Bash(git log:*), Bash(git merge-base:*), Bash(git rev-parse:*), Bash(mktemp:*), Bash(gh pr list:*), Bash(gh pr view:*), Bash(date:*), Write, mcp__linear-server__list_issues, mcp__linear-server__get_issue, mcp__linear-server__list_comments, mcp__linear-server__list_cycles, mcp__linear-server__save_issue
description: "Run all tracker ticket work from one entry point: file, triage, design, slice or backfill."
---

# Tickets

One entry point for ticket work. The first word of `$ARGUMENTS` picks the mode; the rest is that mode's input. With no mode, or one not in the table, list the modes and ask which one.

| Mode | Use it to | Read |
| -- | -- | -- |
| `file` | file one-off tickets, follow-ups or review findings | run `file-ticket` |
| `triage` | groom one area's or one Epic's Backlog: Cancel, Merge, Re-point or Keep each ticket | [TRIAGE.md](TRIAGE.md) |
| `design` | turn a settled plan into an Epic | [DESIGN.md](DESIGN.md) |
| `slice` | break an Epic or a plan into child tickets and fill the Epic's Slices table | [SLICE.md](SLICE.md) |
| `backfill` | file Done tickets for this cycle's merged PRs that have none | [BACKFILL.md](BACKFILL.md) |

Read the mode's file now and follow it; the run is done when that file's last step is.

## Shared rules

- **Every write goes through `file-ticket`.** Each new ticket passes its §2 duplicate search, its §3 grouping and its §5 write: the title prefix, the type label and the points, then the read-back. The tracker's tools and IDs live once, in its `Tracker metadata`, with the local-file form `file`, `design` and `slice` take when no tracker is configured; `triage` and `backfill` need one. The mode files name only what they add.
- **Bodies come from `.claude/skills/file-ticket/templates.md`**: the shared core plus the type's extras. A mode adds sections; it never carries a copy of the core.
- **The user's yes comes first** in `triage`, `design`, `slice` and `backfill`: each shows its plan (a pick sheet, a draft, a breakdown, a grouping) and writes only what the user approved. `file` follows `file-ticket`, which files freely and reports a digest. Run from a worker, the plan goes to the lead, and the answer counts only as `agent-workflow` §1 relays it.
- **Repo vocabulary**: grep `docs/glossary.md` for each term and read `docs/conventions/core.md`; resolve a term that conflicts before it reaches a ticket. Check `docs/adr/` for the area: a ticket that re-litigates an ADR says so, or the companion ADR comes first (`docs/adr/README.md` has the bar).
- **Modules, not paths**: a Goal, a story or a decision names modules, since paths rot. A Bug's `## Cause` keeps its `path:line`, and a prototype snippet that encodes a decision (a state machine, a reducer, a schema, a type shape) may go in, marked as from a prototype and trimmed to the decision.
