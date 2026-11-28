<!-- House style: the pr-description skill.
     Title: <type>(<scope>): <summary> (<KEY>-NNN)   80 chars max, no trailing period.
     The ticket key is optional: a PR with no ticket has none. -->

## Goal

<!-- One line: what this PR delivers. End it with what the merge does to the ticket
     (it works in the description only, never in a comment):
     Closes <KEY>-NNN. or Closes #N.   the merge finishes the ticket.
     Refs <KEY>-NNN.                   the ticket stays open after the merge.
     Part of <KEY>-NNN                 links an epic without closing it.
     The Why stays on the ticket. -->

## What changed

<!-- Up to 5 bullets or a table, plus the updated mermaid diagram when a flow changed. -->

## Done

<!-- The ticket's Done-when items, each ticked with its evidence after a colon or a spaced hyphen:
     a test name, a screenshot, a link or a figure.
     An item this PR does not finish stays unticked, and the Goal line says Refs. -->

- [ ]

<!-- Proof: <test|check|removal>: <evidence>
     test: a test that fails without the fix. check: a manual or browser check, required for
     any visible change. removal: a pure-removal chore, green tests plus a search. -->

## Merge danger

<!-- One line: the door (one-way or two-way) and the blast radius, or "None".
     A schema or migration change: one line per migration, what it adds and the
     backwards-compat verdict. A destructive migration is a one-way door. -->
