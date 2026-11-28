---
name: file-ticket
description: Files tracker tickets with no duplicate and no splinter, then reports a digest. Use before filing any ticket, follow-up or deferred review finding, and before acting on ticket candidates a peer or a handoff passes on.
allowed-tools: mcp__linear-server__list_issues, mcp__linear-server__get_issue, mcp__linear-server__save_issue, mcp__linear-server__save_comment, Read, Grep, Bash(git grep:*), Bash(git show:*), Bash(git rev-parse:*)
---

# File a ticket

Two failures waste a ticket: a **duplicate** (the problem is already filed, often minutes earlier by a peer session) and a **splinter** (an item too small to stand alone, which belongs with the fix it rides on). File each problem once, in the ticket of the PR that will fix it.

## Tracker metadata

The one copy of the tracker wiring; every other skill points here. Tickets go through the issue-tracker MCP server. The `mcp__linear-server__*` tool names, and the field and state names used with them (`Backlog`, `Done`, `Canceled`, `relatedTo`, `blockedBy`, `duplicateOf`), are placeholders: swap them for your tracker's. Read the IDs from `.env` (see `.env.example`), never hardcode them:

- **Assignee:** `${TRACKER_ASSIGNEE_ID}`
- **Team:** `${TRACKER_TEAM_ID}`
- **Project:** `${TRACKER_PROJECT_ID}`
- **Epic label:** `${TRACKER_EPIC_LABEL_ID}`
- **Issue prefix:** `${TRACKER_ISSUE_PREFIX}`, the `<KEY>` of every `<KEY>-NNN` below

No tracker configured (no MCP server, or the IDs are blank): a ticket is one local file, `docs/plans/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01`, with the same title and body. Step 2 searches `docs/plans/` with Grep, a relation is a line naming the other file, and a state is a `Status:` line. `docs/plans/` is local-only; `scripts/pre-commit` refuses to commit it.

## 1. List the candidates

Write every ticket you mean to file this round as one line: the problem as an observable symptom, its **anchors** (a symbol, file stem, route, error code or config key), and its source (PR, review item, measurement). Collect the whole round before searching, since grouping needs the full set.

Done when every candidate has its line and at least one anchor.

## 2. Search

Treat a handoff's or a peer's "not yet filed" as a claim to check: peers file in parallel, and a list handed over an hour ago may already be in the tracker.

- Per candidate: `list_issues` on the Team with `query:` set to each anchor in turn. Search by anchor, since two writers title one problem differently.
- Once per round: the team's tickets from the last 7 days (`createdAt: "-P7D"`, `fields: ["title", "status", "parentId"]`), and the open Backlog and To Do tickets of the area.
- `get_issue` every close hit and read its description; a title hides its scope.

Done when every candidate has a verdict:

- **Duplicate of `<KEY>-NNN`**: an open ticket has the same problem and fix. File nothing; propose a comment only for facts it lacks (current `path:line`, a measured figure).
- **Fold into `<KEY>-NNN`**: an open ticket in the same area will be fixed by the same PR. Extend it with an appended section, which files freely and shows in the digest (`agent-workflow` §2); never rewrite its existing text.
- **Canceled `<KEY>-NNN`**: a Canceled ticket has the same problem. Read why it was canceled. A "Merged into `<KEY>-NNN`" line (step 5) points to the survivor: give the candidate the survivor's verdict. Any other cancel is a ruling that stands: file nothing, or file new only with a fact the cancel did not have, with `relatedTo` the Canceled one.
- **New**. A Done ticket for the same problem marks a regression: file new, with `relatedTo` the Done one.

## 3. Group

One ticket per fix, where a fix is the PR an owner would open. Group candidates, with each other and with the open tickets step 2 surfaced, when they share an area (same package or service and module) and one reviewable PR fixes them all. A splinter (a test-only pin, a comment, a one-line guard, a doc line) always rides in its area's open ticket or its group's ticket. Layers fixed by different PRs (a frontend, an API, a background service) get separate tickets, even when one review found them.

Done when no two tickets in the plan, new or folded, would land in the same PR.

## 4. Plan

Do not ask for a yes: file freely. Keep the plan as one table, each candidate with its step 2 verdict, its step 3 group and its target title; step 5 reports it as the digest once the tickets exist. A write to an existing ticket is not filing: it needs the user's yes unless `agent-workflow` §2's standing yeses list it (a cancel, a merge and a `duplicateOf` mark always need it).

Done when the table holds every candidate with its verdict, group and title.

## 5. Write

- **Title**: the area prefix, then the problem as a symptom a reader can verify: "the export routes render an upstream timeout as 503 instead of 504", rather than "Fix the 503 mapping". The prefixes: <!-- TODO(adapt): list the repo's area prefixes, e.g. `API:`, `Web:`, `CI:`, `Tooling:`, `Docs:` -->.
- **Body**: [templates.md](templates.md): the shared core (`## Goal`, `## Why` with one sourced figure, `## Done when`), plus the extras its type calls for. Set the type label (one of the 7 in templates.md) and the points (1, 2, 4, 8 or 16), both required. A parent is an Epic, and a ticket of 8 or more points is split unless it is an Epic. A grouped or folded ticket carries one `## <heading> (was <KEY>-NNN)` section per item, keeping every fact, figure and path.
- **References**: write a PR as "PR NNNN", with the repo's name before it when the tracker serves more than one repo. A tracker may autolink a bare `#` number to whichever repo it guesses, so grep each draft for `#[0-9]` before saving.
- **State**: `Backlog`, unless the user names a cycle. A cycle and Backlog exclude each other: pass the cycle's id and read back `cycleId`.
- **Merge an existing ticket**, only after the user's yes, into a wider survivor: on the merged one, `state: "Canceled"`, `relatedTo: [<survivor>]`, and a prepended "Merged into `<KEY>-NNN` on <date>" line.
- **Mark a true duplicate** only after the user's yes, with `duplicateOf` alone, and let the tracker move the state. Marking X a duplicate can re-point X's own duplicates and reopen them: list them first, then re-read each after the write and re-point any that reopened.
- **Read back** the ticket after every write with `get_issue`, and check what it returns against templates.md: its type label, its estimate, its title prefix, and the sections its type requires. A missing label or estimate is fixed with `save_issue` and read back again.
- **Status**: check `status` in the same read-back. A save can succeed and change nothing, or return a stale description while still applying, so re-read before any retry.

Done when every candidate maps to exactly one ticket id, existing or new, each write was read back and matches templates.md, and the digest of ids goes to the user.
