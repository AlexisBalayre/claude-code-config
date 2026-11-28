# Skill catalog

Each skill is a `<name>/SKILL.md`. Claude sees only the one-line description at session start
and loads the full skill when it is relevant. This catalog says **when** each fires and **how**
to invoke it.

**Invoke legend**
- **Auto or `/name`**: Claude triggers on the cue described; you can also run `/name` yourself.
- **Manual only**: you invoke it; Claude never auto-triggers (`disable-model-invocation: true`).

A **mode router** (`ask`, `audit`, `bugs`, `notes`, `tickets`, `tooling`, `wiz`) is one entry point whose first argument picks a mode (`/tickets slice <id>`); run it bare to list its modes.

## Setup: fit the template to a project

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `adapt-to-project` | Right after copying the template into a repo, or when the project grows a new area. Surveys the codebase, fills `.claude/project.env`, `AGENTS.md`, the conventions docs and their rule loaders, the architecture/security/glossary docs, then prunes skills and agents the project can't use. Re-runnable. | Manual only (`/adapt-to-project`) |

## Planning & tickets: from idea to tickets

`tickets`, `file-ticket` and `wayfinder` write to your issue tracker when an issue-tracker MCP server and the `TRACKER_*` IDs in `.env` are configured; otherwise they write local Markdown under `docs/plans/<slug>/` (local-only: `scripts/pre-commit` refuses to commit it). The tracker wiring lives once, in `file-ticket`'s Tracker metadata.

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `tickets` | All ticket work from one entry point. Modes: `file` (one-off tickets and follow-ups), `triage` (groom a Backlog on a pick sheet), `design` (turn a settled plan into an Epic), `slice` (break an Epic into tracer-bullet child tickets with blocking edges), `backfill` (file Done tickets for merged PRs that have none). | Manual only (`/tickets <mode>`) |
| `file-ticket` | Before filing any ticket, follow-up or deferred review finding: searches for duplicates, groups splinters into the ticket of the PR that fixes them, writes from the type templates, and reports a digest. | Auto or `/file-ticket` |
| `wayfinder` | Plan work too big for one agent session as a shared map of decision tickets, then resolve them one at a time. | Manual only (`/wayfinder`) |
| `implement` | Implement a spec or set of tickets: TDD, the project's typecheck/test commands, a self-review against the acceptance criteria, then commit on a feature branch. | Manual only (`/implement`) |
| `implement-spec` | Implement a whole Epic's tickets in parallel: a subagent and worktree per frontier ticket, merged into one integration branch behind a draft PR, then the CI review once the PR is marked ready. | Manual only (`/implement-spec`) |
| `implement-ticket` | Pick up a ticket whose type and size need no Design: check it against the trunk, agree the approach, then implement it. | Auto or `/implement-ticket` |
| `to-questionnaire` | A decision you can't answer alone: turn it into a questionnaire for the one person who can. | Manual only (`/to-questionnaire`) |

## Engineering: build, fix, and clean up

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `tdd` | Building a feature or fixing a bug test-first; "red-green-refactor"; want integration tests. | Auto or `/tdd` |
| `diagnose` | A hard bug or performance regression; "diagnose/debug this"; something broken, throwing, failing, or slow. Builds a feedback loop first. | Auto or `/diagnose` |
| `bugs` | Bug work by mode: `hunt` (review an area's code for defects no one has reported, and file what holds), `diagnose` (find the cause of one known bug; runs `diagnose`). | Manual only (`/bugs <mode>`) |
| `audit` | Codebase audits by mode: `arch` (deepening opportunities, then a grilling on the one you pick), `dead` (ranked dead-code candidates with a verification checklist; it never deletes), `deps` (import cycles, orphans and the exceptions the dependency rules list). | Manual only (`/audit <mode>`) |
| `resolve-merge-conflicts` | A merge, rebase or cherry-pick stopped on conflicts. | Auto or `/resolve-merge-conflicts` |
| `ci-and-agent-config` | Editing `.github/workflows/`, `.claude/hooks/`, `.claude/settings.json` or a shell script: the parser harness cases, the macOS against ubuntu gotchas and the live-run rule. | Auto or `/ci-and-agent-config` |
| `wizard` | A step only a human can do (dashboard clicks, credentials, CI secrets, a one-off cutover): generates an interactive bash wizard that walks them through it. | Auto or `/wizard` |
| `research` | Research a question against primary sources and save the cited findings to `docs/research/<slug>.md`. | Auto or `/research` |

## Thinking & design: get to clarity before coding

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `ask` | Questions by mode: `pr <n>` and `branch` (what a PR or branch changes and why, in reading order), `tech` (dig into a new tech and give a verdict: adopt, try or skip). | Manual only (`/ask <mode>`) |
| `prototype` | Sanity-check a state model or logic, or explore a UI, before committing. Logic prototypes are one shareable HTML file; the prototype is kept on a throwaway branch as evidence. | Auto or `/prototype` |
| `grilling` | Rounds of numbered questions, each with a recommended answer, until no open question remains. Facts come from the codebase, decisions from you. | Auto or `/grilling` |
| `codebase-design` | Shared vocabulary for deep modules: interfaces, seams, testability, deepening. Other skills call it. | Auto or `/codebase-design` |
| `domain-modeling` | Keep the documented language (`docs/glossary.md`, conventions, explanation docs, ADRs) current as design decisions land. | Auto or `/domain-modeling` |
| `html-artifact` | Render a pick sheet or a board as one self-contained HTML page: when you ask for HTML, or the output is rows you pick from or a board snapshot. | Auto or `/html-artifact` |
| `zoom-out` | You're unfamiliar with an area and want a higher-level map of the relevant modules and callers. | Manual only (`/zoom-out`) |

## PR & review: from branch to merged

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `pr-description` | Draft or rewrite a PR title and body (Goal, What changed, Done with its proof, Merge danger), then create or update the draft PR via `gh`. Other skills call it when they open PRs. | Auto or `/pr-description` |
| `pr-ci-review` | Cost-optimal multi-agent code review of local changes or a PR: relevance-gated `review-*` subagents, a validation pass, and inline posting. | Manual only (`/pr-ci-review`) |
| `address-review-comments` | Triage, decide, challenge, and implement a PR's open review threads end to end, replying as you go, with a human in the loop. | Auto or `/address-review-comments` |

## Meta & workflow

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `writing-for-agents` | Create or edit a skill, an agent, `AGENTS.md`, or `CLAUDE.md` well: context pointers, information hierarchy, leading words, invocation mechanics, and this repo's house rules. | Auto or `/writing-for-agents` |
| `tooling` | Improve the agent setup by mode: `audit` (skills, agents, hooks and settings against the workflow), `retro` (mine past automated-review runs for recurring failures; needs CI-produced run history), `session` (read a session transcript and suggest fixes to the agent's environment). | Manual only (`/tooling <mode>`) |
| `handoff` | Write or update this session's handoff file so a fresh agent can resume: clean point, done, open threads, rules learned, next step. | Auto or `/handoff` |
| `agent-workflow` | The rules of a lead or worker session in a multi-session run: design before code, push gates, STATUS lines, review rounds. | Auto or `/agent-workflow` |
| `lead` | Start or resume the lead session of a multi-session run. | Manual only (`/lead`) |
| `setup-agent-env` | Install or check the machine-local lead/worker setup. | Manual only (`/setup-agent-env`) |
| `wait-what` | A message didn't land: the agent re-pitches it with context, plain language, and the glossary's terms. | Manual only (`/wait-what`) |
| `caveman` | Ultra-compressed replies (~75% fewer tokens) with full technical accuracy; "caveman mode", "be brief". | Auto or `/caveman` |

Many of the planning, engineering, thinking and meta skills track [mattpocock/skills](https://github.com/mattpocock/skills), adapted to this template's docs layout and `.claude/project.env`.

## Personal integrations: configure via `.env`

These touch your own tools, so set their values in `.env` (see `.env.example`). `notes daily`
also needs an issue-tracker MCP server enabled in `.claude/settings.local.json`; `fix-sonar`
needs a SonarQube MCP server, and `wiz` needs the Wiz MCP server.

| Skill | When to use | Invoke |
| :---- | :---------- | :----- |
| `notes` | Your Obsidian vault by mode: `daily` (logbook notes synced with the tracker and Git), `study` (a researched study note), `search` (find and link notes). Needs `OBSIDIAN_VAULT` + `OBSIDIAN_DAILY_DIR`; `daily` also needs the `TRACKER_*` IDs. | Manual only (`/notes <mode>`) |
| `fix-sonar` | Burn down open SonarQube issues with per-rule fix strategies and parallel worktree agents. Needs `SONAR_PROJECT_KEY`. | Manual only (`/fix-sonar`) |
| `wiz` | Cloud security through the Wiz MCP by mode: `audit` (read-only investigation scoped to your resources), `fix` (triage open findings through a confidence gate, fix the mechanical ones in one worktree PR, escalate the rest). Needs `WIZ_*` IDs. | Manual only (`/wiz <mode>`) |
