# Skill catalog

Each skill is a `<name>/SKILL.md`. Claude sees only the one-line description at session start
and loads the full skill when it is relevant. This catalog says **when** each fires and **how**
to invoke it.

**Invoke legend**
- **Auto or `/name`** — Claude triggers on the cue described; you can also run `/name` yourself.
- **Auto (Claude-only)** — Claude triggers it; hidden from the `/` menu (`user-invocable: false`).
- **Manual only** — you invoke it; Claude never auto-triggers (`disable-model-invocation: true`).

## Scaffolding — generate files that follow the conventions

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `new-api-endpoint` | You ask to "add an endpoint" / "add an API for X". Scaffolds route + schema + service + serializer + test per `docs/conventions/api.md`. | Auto (Claude-only) |
| `new-frontend-route` | "add a page" / "create a route" / "add a frontend view". Scaffolds a TanStack Router route + query hooks + feature components. | Auto (Claude-only) |
| `new-provider` | "add a provider" / "integrate a new email/SMS/push provider". Scaffolds a delivery Provider via the factory + YAML registry. | Auto (Claude-only) |

## Engineering — build, fix, and clean up

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `tdd` | Building a feature or fixing a bug test-first; "red-green-refactor"; want integration tests. Drives the red → green → refactor loop. | Auto or `/tdd` |
| `diagnose` | A hard bug or performance regression; "diagnose/debug this"; something broken/throwing/failing. Runs reproduce → minimise → hypothesise → instrument → fix → regression-test. | Auto or `/diagnose` |
| `find-dead-code` | "find dead code" / "unused exports" / "prune the codebase". Returns a ranked candidate list with a per-item verification checklist — it never deletes. | Auto or `/find-dead-code` |
| `improve-codebase-architecture` | "improve architecture" / "find refactors" / "make it more testable". Finds deep-module and consolidation opportunities, grounded in the glossary + ADRs. | Auto or `/improve-codebase-architecture` |

## Thinking & design — get to clarity before coding

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `prototype` | Sanity-check a data model / state machine, or mock up UI, before committing; "prototype this", "try a few designs". Builds a throwaway runnable prototype. | Auto or `/prototype` |
| `grill-me` | Stress-test your own plan via relentless one-question-at-a-time interviewing; "grill me". | Auto or `/grill-me` |
| `grill-with-docs` | Pressure-test a design against the repo's glossary / conventions / ADRs, updating those docs inline as decisions settle. | Auto or `/grill-with-docs` |
| `zoom-out` | You're unfamiliar with an area and want a higher-level map of the relevant modules and callers. | Manual only (`/zoom-out`) |

## Meta & workflow

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `write-a-skill` | Create or edit a skill with proper structure and progressive disclosure; "write a skill". | Auto or `/write-a-skill` |
| `handoff` | Compact the current conversation into a handoff document for another agent or a fresh session. | Auto or `/handoff` |
| `caveman` | Ultra-compressed replies (~75% fewer tokens) with full technical accuracy; "caveman mode", "be brief". | Auto or `/caveman` |

## Personal integrations — configure via `.env`

These touch your own tools, so set their values in `.env` (see `.env.example`). The two tracker
skills also need an issue-tracker MCP server enabled in `.claude/settings.local.json`.

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `obsidian-vault` | Find, create, or organize notes in your Obsidian vault. Needs `OBSIDIAN_VAULT`. | Auto or `/obsidian-vault` |
| `to-epic` | Turn the current discussion into an Epic and file it. Needs `TRACKER_*` IDs + a tracker MCP. | Auto or `/to-epic` |
| `to-issues` | Slice a plan or Epic into independent, end-to-end vertical-slice issues. Needs `TRACKER_*` IDs + a tracker MCP. | Auto or `/to-issues` |
