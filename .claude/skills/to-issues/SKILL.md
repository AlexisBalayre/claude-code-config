---
name: to-issues
description: Break a plan, spec, or Epic into independently-grabbable issues as tracer-bullet vertical slices (end-to-end through every layer), each filed as a child of the parent Epic.
disable-model-invocation: true
---

# To Issues

Break a plan into small, end-to-end vertical slices. Each slice crosses every layer the feature touches (schema, service, transport, UI, tests) for one narrow behavior, rather than covering all of one layer for many behaviors.

> **Setup:** this skill files tickets through an issue-tracker MCP server (the examples use a
> Linear-style API). Fill in your own IDs below and enable the server in
> `.claude/settings.local.json`. The tool names (`mcp__linear-server__*`) are placeholders —
> swap them for your tracker's tools.

## Tracker metadata

Read these from `.env` (never hardcode real account IDs in the repo):

- **Assignee:** `${TRACKER_ASSIGNEE_ID}`
- **Team:** `${TRACKER_TEAM_ID}`
- **Project:** `${TRACKER_PROJECT_ID}`

## Process

### 1. Source

If the user passes an issue ID or URL, fetch it via `mcp__linear-server__get_issue` and read the full body + comments. Otherwise work from the current conversation context.

### 2. Ground truth

Issue titles and bodies MUST use this repo's vocabulary. Before slicing:

- Skim `docs/README.md` Glossary and `docs/conventions/naming.md` for canonical terms (`Organization`, `Session`, `Participant`, `Message`, etc.).
- Check `docs/adr/` for hard-to-reverse decisions in the area. If any proposed slice would silently re-litigate an ADR, flag it to the user before drafting; do not bury the contradiction inside an issue body.
- Skim the relevant `docs/explanation/<topic>.md` for current rationale.

For *why*/*how* questions across services, delegate to the `architecture-explainer` subagent.

### 3. Slice

Break the plan into vertical slices. A **wide refactor** is the exception to the vertical-slice rule — slice it by **expand–contract** instead (see **Wide refactors** below). For each slice:

- **Title:** imperative, action-oriented, in repo vocabulary.
- **Type:** HITL (needs human input: architectural decision, design review, manual verification) or AFK (can be implemented and merged unattended). Prefer AFK; treat HITL as a flag that the slice still has an unresolved question.
- **Blocked by:** which other slices must complete first.
- **User stories covered:** which stories from the parent Epic (if any).

Rules:

- Each slice is a complete narrow path end-to-end. A schema-only or UI-only slice is a smell.
- A completed slice is demoable or verifiable on its own.
- Prefer many thin slices over few thick ones.

### 4. Review

Present the breakdown as a numbered list. Ask:

- Does granularity feel right (too coarse / too fine)?
- Are blocker relationships correct? Anything that could be parallelised?
- Should any slice be merged or split?
- Are HITL and AFK labels correct?

Iterate until the user approves.

### 5. File

Create issues via `mcp__linear-server__save_issue` in dependency order, so real issue IDs can populate the blocking edges of later slices. Each child issue gets `parentId` set to the Epic issue (if any), and each blocker is wired as a native relation via `save_issue`'s `blockedBy` field — the tracker renders the frontier visually, so the human sees what's takeable without opening every issue. The body's "Blocked by" section stays as the human-readable summary.

## Wide refactors

A **wide refactor** is one mechanical change — rename a column, retype a shared symbol — whose **blast radius** fans across the whole monorepo, so a single edit breaks thousands of call sites at once and no vertical slice can land green. Don't force it into a tracer bullet; sequence it as **expand–contract**. First expand: add the new form beside the old so nothing breaks. Then migrate the call sites over in batches sized by blast radius (per workspace, per directory), each batch its own issue blocked by the expand, keeping CI green batch to batch because the old form still exists. Finally contract: delete the old form once no caller remains, in an issue blocked by every migrate batch. When even the batches can't stay green alone, keep the sequence but let them share an integration branch that all block a final integrate-and-verify issue — green is promised only there.

## Issue body template

<issue-template>

## Parent

${TRACKER_ISSUE_PREFIX}-XXXX (or omit if standalone).

## What to build

End-to-end behavior, not layer-by-layer implementation. Stated in repo vocabulary.

Avoid specific file paths or code snippets; they rot. Exception: if a prototype produced a snippet that encodes a decision more precisely than prose can (state machine, reducer, schema, type shape), inline it here and note briefly that it came from a prototype. Trim to the decision-rich parts, not a working demo.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Blocked by

${TRACKER_ISSUE_PREFIX}-YYYY, or "None, can start immediately".

## Notes

Relevant ADRs (`docs/adr/NNNN-*.md`), conventions (`docs/conventions/*.md`), prior art (cite by module, not file path), open questions.

</issue-template>

## Rules

- Do NOT modify or close the parent issue; just set `parentId` on children.
- Do NOT file without explicit user approval of the breakdown.
- Slice by behavior, never by file type or module boundary. One issue = one thin end-to-end capability. (Exception: a wide refactor slices by expand–contract, per the section above.)
- Maximize parallelism: mark slices "None, can start immediately" whenever they are genuinely independent.
- If a slice surfaces a hard-to-reverse decision with non-obvious rejected alternatives, open the companion ADR before filing the slice. See [docs/adr/README.md](../../../docs/adr/README.md).
- Use repo vocabulary throughout titles and bodies. If a conversation term conflicts with the Glossary or `naming.md`, resolve it during step 2 (not in the issue body).
