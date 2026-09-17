# Skill catalog

Each skill is a `<name>/SKILL.md`. Claude sees only the one-line description at session start
and loads the full skill when it is relevant. This catalog says **when** each fires and **how**
to invoke it.

**Invoke legend**
- **Auto or `/name`** — Claude triggers on the cue described; you can also run `/name` yourself.
- **Auto (Claude-only)** — Claude triggers it; hidden from the `/` menu (`user-invocable: false`).
- **Manual only** — you invoke it; Claude never auto-triggers (`disable-model-invocation: true`).

## Setup — fit the template to a project

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `adapt-to-project` | Right after copying the template into a repo, or when the project grows a new area. Surveys the codebase, fills `.claude/project.env`, `AGENTS.md`, the conventions docs and their rule loaders, the architecture/security/glossary docs, then prunes skills and agents the project can't use. Re-runnable. | Manual only (`/adapt-to-project`) |

## Planning & specs — from idea to tickets

`to-spec`, `to-tickets` and `wayfinder` publish to your issue tracker when an issue-tracker MCP server and the `TRACKER_*` IDs in `.env` are configured; otherwise they write local Markdown under `docs/plans/<slug>/` (local-only: `scripts/pre-commit` refuses to commit it).

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `to-spec` | Turn the current conversation into a spec: no interview, just synthesis of what was already discussed. | Manual only (`/to-spec`) |
| `to-tickets` | Break a plan, spec, or conversation into tracer-bullet tickets, each declaring its blocking edges. | Manual only (`/to-tickets`) |
| `wayfinder` | Plan work too big for one agent session as a shared map of decision tickets, then resolve them one at a time. | Manual only (`/wayfinder`) |
| `implement` | Implement a spec or set of tickets: TDD, the project's typecheck/test commands, a self-review against the acceptance criteria, then commit on a feature branch. | Manual only (`/implement`) |
| `to-questionnaire` | A decision you can't answer alone: turn it into a questionnaire for the one person who can. | Manual only (`/to-questionnaire`) |

## Engineering — build, fix, and clean up

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `tdd` | Building a feature or fixing a bug test-first; "red-green-refactor"; want integration tests. | Auto or `/tdd` |
| `diagnosing-bugs` | A hard bug or performance regression; "diagnose/debug this"; something broken, throwing, failing, or slow. Builds a feedback loop first and redacts secrets from anything it shows. | Auto or `/diagnosing-bugs` |
| `resolving-merge-conflicts` | A merge, rebase, or cherry-pick stopped on conflicts. Resolves by intent and regenerates, never hand-merges, generated files. | Auto or `/resolving-merge-conflicts` |
| `wizard` | A step only a human can do (dashboard clicks, credentials, CI secrets, a one-off cutover): generates an interactive bash wizard that walks them through it. | Auto or `/wizard` |
| `research` | Research a question against primary sources and save the cited findings to `docs/research/<slug>.md`. | Auto or `/research` |
| `find-dead-code` | "find dead code" / "unused exports" / "prune the codebase". Returns a ranked candidate list with a per-item verification checklist; it never deletes. | Manual only (`/find-dead-code`) |
| `improve-codebase-architecture` | "improve architecture" / "find refactors" / "make it more testable". Scans for deepening opportunities, presents a visual HTML report, then grills through whichever one you pick. | Manual only (`/improve-codebase-architecture`) |

## Thinking & design — get to clarity before coding

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `prototype` | Sanity-check a state model or logic, or explore a UI, before committing. Logic prototypes are one shareable HTML file; the prototype is kept on a throwaway branch as evidence. | Auto or `/prototype` |
| `grilling` | The shared grilling core: rounds of numbered questions, each with a recommended answer, until no open question remains. Facts come from the codebase, decisions from you. | Auto or `/grilling` |
| `grill-me` | Thin wrapper: a plain grilling session on your plan or design. | Manual only (`/grill-me`) |
| `grill-with-docs` | Thin wrapper: grilling plus `domain-modeling`, so the glossary and ADRs are updated as decisions land. | Manual only (`/grill-with-docs`) |
| `codebase-design` | Shared vocabulary for deep modules: interfaces, seams, testability, design-it-twice. Other skills call it. | Auto or `/codebase-design` |
| `domain-modeling` | Build and sharpen the domain model in `docs/glossary.md` and `docs/adr/` while discussing terminology or decisions. | Auto or `/domain-modeling` |
| `zoom-out` | You're unfamiliar with an area and want a higher-level map of the relevant modules and callers. | Manual only (`/zoom-out`) |

## PR & review — from branch to merged

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `pr-description` | Draft or rewrite a PR title and body (summary, before/after evidence, merge danger) for fast human review, then create or update the PR via `gh`. Other skills call it when they open PRs. | Auto or `/pr-description` |
| `pr-ci-review` | Cost-optimal multi-agent code review of local changes or a PR: relevance-gated `review-*` subagents, a validation pass, and inline posting. | Manual only (`/pr-ci-review`) |
| `address-review-comments` | Triage, decide, challenge, and implement a PR's open review threads end to end, replying as you go, with a human in the loop. | Auto or `/address-review-comments` |
| `review-retro` | Mine past automated-review runs for recurring process/judgment failures and propose evidence-cited fixes to the review setup as one PR. Needs CI-produced run history (see the skill's setup note). | Manual only (`/review-retro`) |

## Meta & workflow

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `writing-for-agents` | Create or edit a skill, `AGENTS.md`, or `CLAUDE.md` well: context pointers, information hierarchy, leading words, invocation mechanics, and this repo's house rules. | Auto or `/writing-for-agents` |
| `handoff` | Compact the current conversation into a handoff document for another agent or a fresh session. | Manual only (`/handoff`) |
| `wait-what` | A message didn't land: the agent re-pitches it with context, plain language, and the glossary's terms. | Manual only (`/wait-what`) |
| `caveman` | Ultra-compressed replies (~75% fewer tokens) with full technical accuracy; "caveman mode", "be brief". | Auto or `/caveman` |

The planning, engineering, thinking and meta skills track [mattpocock/skills](https://github.com/mattpocock/skills), adapted to this template's docs layout and `.claude/project.env`.

## Personal integrations — configure via `.env`

These touch your own tools, so set their values in `.env` (see `.env.example`). The tracker
skills (`backfill-issues`, `daily-note`) also need an issue-tracker MCP
server enabled in `.claude/settings.local.json`; `fix-sonar` needs a SonarQube MCP server, and
`wiz` / `fix-wiz` need the Wiz MCP server.

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `obsidian-vault` | Find, create, or organize notes in your Obsidian vault. Needs `OBSIDIAN_VAULT`. | Auto or `/obsidian-vault` |
| `daily-note` | Daily Obsidian logbook notes (`--plan` / `--summary` / `--status` / `--checkin`), synced with the tracker and Git. Needs `OBSIDIAN_VAULT` + `OBSIDIAN_DAILY_DIR` + `TRACKER_*` IDs. | Manual only (`/daily-note`) |
| `backfill-issues` | Backfill tracker issues from merged PRs that lack one, grouped and user-approved before filing. Needs `TRACKER_*` IDs + a tracker MCP. | Manual only (`/backfill-issues`) |
| `fix-sonar` | Burn down open SonarQube issues with per-rule fix strategies and parallel worktree agents. Needs `SONAR_PROJECT_KEY`. | Manual only (`/fix-sonar`) |
| `wiz` | Read-only investigation of your cloud security posture through the Wiz MCP, routed by security domain and scoped to your resources. Needs `WIZ_*` IDs. | Manual only (`/wiz`) |
| `fix-wiz` | Triage open Wiz findings (container CVEs, SAST) through a confidence gate; auto-fix the mechanical ones in one worktree PR, escalate the rest. Needs `WIZ_*` IDs. | Manual only (`/fix-wiz`) |
