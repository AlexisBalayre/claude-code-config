---
name: to-epic
description: Synthesize the current conversation and codebase understanding into an Epic, then file it as an issue. No interview; works from what has already been discussed.
disable-model-invocation: true
---

# To Epic

Take what you already know from the current conversation and codebase exploration, produce an Epic, and file it as an issue. Do NOT interview the user from scratch; synthesize what's already on the table.

> **Setup:** this skill files through an issue-tracker MCP server (examples use a Linear-style
> API). Read your tracker IDs from `.env` (see `.env.example`) and enable the server in
> `.claude/settings.local.json`. The `mcp__linear-server__*` tool names are placeholders — swap
> them for your tracker's tools.

## Tracker metadata

Read these from `.env` (never hardcode real account IDs in the repo):

- **Assignee:** `${TRACKER_ASSIGNEE_ID}`
- **Team:** `${TRACKER_TEAM_ID}`
- **Project:** `${TRACKER_PROJECT_ID}`
- **Label:** `${TRACKER_EPIC_LABEL_ID}` (Epic) — always apply

## Process

### 1. Ground truth

If the relevant area has not been explored yet, do so now. Anchor the Epic in:

- `docs/README.md` Glossary and `docs/conventions/naming.md` for vocabulary. The Epic body MUST use these terms (e.g., `Organization`, `Session`, `Message`); flag and resolve any drift before drafting.
- `docs/reference/` for current service boundaries and wiring.
- `docs/explanation/` for the rationale of the surrounding subsystem.
- `docs/adr/` for hard-to-reverse decisions already locked in. The Epic must not silently re-litigate an existing ADR; if it does, call it out explicitly and decide whether to open a new ADR that supersedes it.
- `.claude/rules/*.md` for the area you'll touch.

For *why*/*how* questions across services, delegate to the `architecture-explainer` subagent.

### 2. Module sketch

Sketch the major modules to build or modify. Run the `/codebase-design` skill for the vocabulary, and actively look for **deep modules**: a simple, testable interface that hides complex implementation and rarely changes. Shallow modules with thin interfaces over large surface area are a smell; surface them and consolidate.

Confirm with the user before drafting:

- Does this module breakdown match your expectations?
- Which modules need test coverage first?

### 3. Draft

Write the Epic using the template below. Show it in chat for review. Never file without explicit user approval of the draft.

### 4. File

Once approved, call `mcp__linear-server__save_issue` with the Epic body as markdown and the Epic label applied.

## Body template

<epic-template>

## Problem statement

The problem, from the user's perspective. Stated in repo vocabulary.

## Solution

The solution, from the user's perspective.

## User stories

A long numbered list. Each in the form: *As an `<actor>`, I want `<feature>`, so that `<benefit>`.* Cover all aspects.

Example: *As a session host, I want an in-flight Message to be silently superseded when a newer revision arrives, so that Participants only ever see the latest version without a flicker.*

## Implementation decisions

Modules to build or modify, interface shapes, architectural decisions, schema changes, API contracts, provider interactions, transport choices (gRPC / WebSocket).

Call out any decision that supersedes or refines an existing ADR, and note whether a new ADR is warranted.

Do NOT pin specific file paths or line numbers; they rot. Inline code snippets only when a prototype produced a state machine, reducer, schema, or type shape that encodes a decision more precisely than prose can. Trim to the decision-rich parts (not a working demo).

## Testing decisions

- Definition of a good test: behavior-only, not implementation.
- Which modules to cover, and at which layer (unit / integration / e2e).
- Prior art: similar tests elsewhere in the codebase (cite by module, not file path).

## Out of scope

What this Epic deliberately does not cover. Explicit no's are as valuable as yes's.

## Further notes

Links to ADRs, prior epics, related issues, open questions.

</epic-template>

## Rules

- Write in the repo's vocabulary. If the conversation used a term that conflicts with the Glossary or `naming.md`, resolve it (or flag it) before filing.
- No file paths or line numbers in the body; they rot.
- No code snippets except the prototype-derived exception above (state machine / reducer / schema / type shape).
- One Epic per issue. For multi-feature work, file the Epic, then have the user run `/to-issues` to break it into independently grabbable tickets as children of this Epic.
- Never file without explicit user approval of the draft.
- If the Epic touches a hard-to-reverse decision with non-obvious rejected alternatives, open the companion ADR before or alongside filing. See [docs/adr/README.md](../../../docs/adr/README.md) for the bar.
