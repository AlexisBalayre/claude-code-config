---
name: handoff
description: Write or update this session's handoff so a fresh agent can resume the work. Use when a worker or lead writes or updates its handoff or STATUS lines, or before a session ends.
argument-hint: "What will the next session be used for?"
---

Write the handoff to `$AGENT_WORK/handoffs/<YYYY-MM-DD>-<session-name>.md`, dated by `date` (local time). A worker's file uses `<SESSION_PREFIX><handle>` (both set in `setup-agent-env`'s params) in place of its session name, the name `board.sh` reads. That folder is machine-local, outside the repo; if `AGENT_WORK` is unset or the folder does not exist, ask the user where handoffs go. If today's file exists, update it in place: rewrite the sections, keep the dated lines under `## Status`.

In a multi-session run, the lead is the session that hands out work and relays to the user; the `agent-workflow` skill holds its rules and the STATUS line's. A solo session leaves the lead out.

Use this shape:

```markdown
# <session-name> handoff, <YYYY-MM-DD HH:MM>

For the next <session-name>. Trust `git`/`gh` over any line here. <Who the lead is and how to reach them, if any.>

## Clean point

<What is running, uncommitted, or held (a lock) right now. "Idle" if nothing.>

## Done

- <Ticket or PR: outcome, head sha or merge sha, worktree kept or not.>

## Open threads

- <Each in-flight item: where it stands, the exact next action, and whose go it waits on.>

## Rules learned

- <A correction the user or lead gave this session that no repo doc, skill or memory file holds yet.>

## Next step

<One line.>

## Status

<STATUS lines, `agent-workflow` WORKER.md §5>
```

- Name the shas, PR numbers, worktree paths and draft paths; point at plans, ADRs, tickets and commits rather than copying them.
- Name the skills the next session should load, in `## Next step`.
- Redact secrets and personal data.
- If the user passed arguments, treat them as the next session's focus and tailor `## Open threads` and `## Next step` to it.

Done when today's file is at the path above with every section that has content filled (Clean point `Idle` when nothing runs), `## Next step` names the skills to load, and the header time came from `date`.
