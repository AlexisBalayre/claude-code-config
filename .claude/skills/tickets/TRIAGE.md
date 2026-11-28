# Triage mode

Groom one slice of the Backlog: check each ticket against the trunk, propose a verdict and a priority, and let the user pick on an `html-artifact` pick sheet. Nothing in the tracker changes until the user ticks.

## 1. Scope

The input is one area (a title prefix from `file-ticket` §5, such as `Tooling:`, optionally narrowed by a package or a label) or one Epic id.

- **An Epic:** `list_issues` with its `parentId` and `state: "Backlog"`.
- **An area:** `query` misses most titles by prefix, so list the team's Backlog (the Team of `file-ticket`'s Tracker metadata, `state: "Backlog"`, `fields: ["title", "status", "priority", "parentId", "updatedAt"]`, `limit: 100`), follow every cursor, and keep the titles that start with the prefix.

Done when the list holds at most 30 tickets. Over 30, propose a narrower scope (a package, a label, an age) and ask.

## 2. Check each ticket against the trunk

Read each ticket with `get_issue` and its `list_comments`. Take its anchors (a `path:line`, a symbol, a config key, a sha, a figure) and check them at the trunk (`GIT_TRUNK` in `.claude/project.env`), `origin/<trunk>`. Use `git grep`, `git show <branch>:<path>` and `git merge-base --is-ancestor <sha> <branch>`. Over 10 tickets, split the check across `Explore` subagents of up to 10 tickets each, giving each the ids and the verdicts below.

For each ticket, also run `file-ticket` §2's per-anchor search for an open ticket outside the scope that the same PR would fix: a hit makes it a Merge.

Give each ticket one verdict, with the evidence that decided it:

- **Cancel**: close it. A merged PR fixed it (name the PR): it closes as `Done`. Its code is gone or a standing ruling rejects it (name the commit or the ruling): it closes as `Canceled`.
- **Merge**: an open ticket has the same fix, so one PR closes both. Name the survivor.
- **Re-point**: the problem stands but an anchor no longer holds. Give the new `path:line` at the current trunk sha, to replace that anchor in place. The Modules rule of `SKILL.md` governs new bodies, and a Re-point adds no path the ticket did not already cite. A ticket whose anchors all hold is a Keep, however old the sha it cites. An Epic is a Re-point when its `## Slices` table misses a child or shows a stale status.
- **Keep**: the ticket stands as written.

And a priority (Urgent, High, Medium, Low), with one line of why.

Note anything outside the scope the checks turned up (a shipped ticket still open elsewhere, a duplicate pair) for the hand-over; it gets no row.

Done when every ticket has a verdict, its evidence, and a priority.

## 3. Pick sheet

Build the page with `html-artifact`'s Pick sheet: one row per ticket, keyed by its id, with the choices `Cancel`, `Merge`, `Re-point` and `Keep`. Beside each row show its title, the evidence, the proposed verdict, the priority and, for Merge, the survivor. A row's note overrides the priority with one of the four priority words, and the survivor with `into <KEY>-NNN`. Hand it over as `html-artifact` says, with the out-of-scope notes from step 2.

Done when the user's picks are back, verbatim.

## 4. Apply the ticks

Read each picked ticket's state again first, and skip one that has left Backlog since step 1 (name it in the digest). Write only the rows the user picked, each through `file-ticket` §5:

- **Cancel**: `state: "Done"` or `"Canceled"` as step 2 decided, and a prepended line "Closed on <date> in triage: <evidence>".
- **Merge**: §5's merge into the survivor, its facts carried into a `(was <KEY>-NNN)` section of the survivor. A Merge with no survivor, proposed or in the note, is skipped and named in the digest.
- **Re-point**: patch the stale anchors in place, the rest of the body intact. For an Epic, rewrite its `## Slices` table and diagram from its children.
- **Keep**: the body unchanged.

Set the priority on every picked row: the note's when it has one, else the proposed one. A row with no choice changes nothing.

Done when every picked row was read back, and the user has the digest: one line per ticket, with its verdict and priority.
