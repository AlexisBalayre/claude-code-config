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
| `tdd` | Building a feature or fixing a bug test-first; "red-green-refactor"; want integration tests. Drives the red → green loop (refactoring belongs to the review stage, not the loop). | Auto or `/tdd` |
| `diagnose` | A hard bug or performance regression; "diagnose/debug this"; something broken, throwing, failing, or slow. Runs reproduce → minimise → hypothesise → instrument → fix → regression-test. | Auto or `/diagnose` |
| `resolve-merge-conflicts` | A merge, rebase, or cherry-pick stopped on conflicts. Resolves by intent (both sides' goals), and regenerates — never hand-merges — generated files. | Auto or `/resolve-merge-conflicts` |
| `find-dead-code` | "find dead code" / "unused exports" / "prune the codebase". Returns a ranked candidate list with a per-item verification checklist — it never deletes. | Manual only (`/find-dead-code`) |
| `improve-codebase-architecture` | "improve architecture" / "find refactors" / "make it more testable". Scans for deepening opportunities, presents a visual HTML report, then grills through whichever one you pick. | Manual only (`/improve-codebase-architecture`) |

## Thinking & design — get to clarity before coding

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `prototype` | Sanity-check a data model / state machine, or mock up UI, before committing; "prototype this", "try a few designs". Builds a throwaway runnable prototype. | Auto or `/prototype` |
| `grilling` | The shared grilling core: a one-question-at-a-time interview until shared understanding — facts from the codebase, decisions from you. Fires when you want a plan stress-tested; other skills run it too. | Auto or `/grilling` |
| `grill-me` | Thin wrapper: run a plain `/grilling` session on your plan or design. | Manual only (`/grill-me`) |
| `grill-with-docs` | Thin wrapper: run a `/grilling` session with `/domain-modeling` active, so the glossary / conventions / ADRs are updated as decisions crystallise. | Manual only (`/grill-with-docs`) |
| `codebase-design` | Shared vocabulary and principles for designing deep modules — interfaces, seams, testability. Other skills import it. | Auto or `/codebase-design` |
| `domain-modeling` | Keeps the repo's documented language (Glossary, naming, ADRs) current as design decisions land. Other skills lean on it during design sessions. | Auto or `/domain-modeling` |
| `zoom-out` | You're unfamiliar with an area and want a higher-level map of the relevant modules and callers. | Manual only (`/zoom-out`) |

## PR & review — from branch to merged

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `pr-description` | Draft or rewrite a PR title/body in the repo's house style, then create or update the PR via `gh`. Other skills call it when they open PRs. | Auto or `/pr-description` |
| `pr-ci-review` | Cost-optimal multi-agent code review of local changes or a PR: relevance-gated `review-*` subagents, a validation pass, and inline posting. | Manual only (`/pr-ci-review`) |
| `address-review-comments` | Triage, decide, challenge, and implement a PR's open review threads end to end, replying as you go — human in the loop. | Auto or `/address-review-comments` |
| `review-retro` | Mine past automated-review runs for recurring process/judgment failures and propose evidence-cited fixes to the review setup as one PR. Needs CI-produced run history (see the skill's setup note). | Manual only (`/review-retro`) |

## Meta & workflow

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `write-a-skill` | Create or edit a skill well — invocation economics, information hierarchy, leading words, and this repo's house rules. | Auto or `/write-a-skill` |
| `handoff` | Compact the current conversation into a handoff document for another agent or a fresh session. | Manual only (`/handoff`) |
| `caveman` | Ultra-compressed replies (~75% fewer tokens) with full technical accuracy; "caveman mode", "be brief". | Auto or `/caveman` |

## Personal integrations — configure via `.env`

These touch your own tools, so set their values in `.env` (see `.env.example`). The tracker
skills (`to-epic`, `to-issues`, `backfill-issues`, `daily-note`) also need an issue-tracker MCP
server enabled in `.claude/settings.local.json`; `fix-sonar` needs a SonarQube MCP server, and
`wiz` / `fix-wiz` need the Wiz MCP server.

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `obsidian-vault` | Find, create, or organize notes in your Obsidian vault. Needs `OBSIDIAN_VAULT`. | Auto or `/obsidian-vault` |
| `daily-note` | Daily Obsidian logbook notes (`--plan` / `--summary` / `--status` / `--checkin`), synced with the tracker and Git. Needs `OBSIDIAN_VAULT` + `OBSIDIAN_DAILY_DIR` + `TRACKER_*` IDs. | Manual only (`/daily-note`) |
| `to-epic` | Turn the current discussion into an Epic and file it. Needs `TRACKER_*` IDs + a tracker MCP. | Manual only (`/to-epic`) |
| `to-issues` | Slice a plan or Epic into independent, end-to-end vertical-slice issues. Needs `TRACKER_*` IDs + a tracker MCP. | Manual only (`/to-issues`) |
| `backfill-issues` | Backfill tracker issues from merged PRs that lack one, grouped and user-approved before filing. Needs `TRACKER_*` IDs + a tracker MCP. | Manual only (`/backfill-issues`) |
| `fix-sonar` | Burn down open SonarQube issues with per-rule fix strategies and parallel worktree agents. Needs `SONAR_PROJECT_KEY`. | Manual only (`/fix-sonar`) |
| `wiz` | Read-only investigation of your cloud security posture through the Wiz MCP, routed by security domain and scoped to your resources. Needs `WIZ_*` IDs. | Manual only (`/wiz`) |
| `fix-wiz` | Triage open Wiz findings (container CVEs, SAST) through a confidence gate; auto-fix the mechanical ones in one worktree PR, escalate the rest. Needs `WIZ_*` IDs. | Manual only (`/fix-wiz`) |
