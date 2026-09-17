# claude-code-power-config

A complete, opinionated **[Claude Code](https://code.claude.com/docs)** configuration template: rules,
skills, subagents, and deterministic hooks, wired together and documented end to end. It is
**stack-agnostic**: nothing assumes a language, package manager, or layout. Copy it into any
project, run **`/adapt-to-project`**, and the template fills in that project's commands,
conventions, architecture docs, and prunes what the project can't use.

![The bundled statusline: working directory, git branch, model, effort level, session name, a context-usage bar, 5-hour and 7-day rate limits, token counts, and live session cost](assets/statusline.png)

<sub>The bundled <a href=".claude/statusline.sh"><code>statusline.sh</code></a>: directory, branch (red on <code>main</code>), model, effort, session name, a context-usage bar, 5h/7d rate limits with reset countdowns, token counts, and live session cost.</sub>

> **TL;DR** — Copy the template into your repo, run `/adapt-to-project`, and you have path-scoped
> conventions, a quality gate on every turn, a multi-agent PR review, and a worktree-first
> workflow that never lets an agent commit to `main`.

---

## What's inside

| Layer | Count | What it does |
| :---- | :---- | :----------- |
| **`AGENTS.md`** + **`CLAUDE.md`** | 2 | Always-on project memory, layered: `AGENTS.md` is the tool-agnostic base (role, conventions map, comments/altitude discipline, git workflow) any coding agent can read; `CLAUDE.md` just imports it and adds Claude Code-only notes. Kept tiny on purpose. |
| **`project.env`** | 1 | The project profile: format/lint/typecheck/test/install commands, generated paths, file-naming pattern, trunk. Every hook and script reads it; an empty key turns its check off. |
| **`rules/`** | 2+ | Path-scoped **pure loaders** (core, testing, plus one per project area once adapted) that auto-load `docs/conventions/*.md` only when you touch matching files. |
| **`skills/`** | 34 | Auto-discoverable workflows: setup (`adapt-to-project`), planning (`to-spec`, `to-tickets`, `wayfinder`, `implement`…), engineering (`tdd`, `diagnosing-bugs`, `resolving-merge-conflicts`, `wizard`, `research`…), thinking & design (`grilling`, `grill-me`, `codebase-design`, `domain-modeling`, `prototype`…), PR & review (`pr-description`, `pr-ci-review`, `address-review-comments`, `review-retro`), meta (`writing-for-agents`, `handoff`, `wait-what`…), and personal integrations (`obsidian-vault`, `daily-note`, `fix-sonar`, `wiz`… — configured via `.env`). Engineering and thinking skills track [mattpocock/skills](https://github.com/mattpocock/skills). |
| **`agents/`** | 12 | Isolated subagents: 4 proactive (`convention-checker`, `migration-reviewer`, `security-reviewer`, `architecture-explainer`), 7 `review-*` reviewers + validator dispatched by `pr-ci-review`, and `comment-pruner` dispatched by its Stop hook. |
| **`hooks/`** | 7 | Zero-LLM shell scripts on lifecycle events: quality gate, convention spot-check, comment-pruner dispatch, git safety, generated-file protection, file-naming validation, compaction preservation. All stack-specific values come from `project.env`. |

The full architecture — what loads when, the context budget, and how to extend each layer — is
documented in **[`.claude/README.md`](.claude/README.md)**. For when and how to use each skill,
see the **[skill catalog](.claude/skills/README.md)**.

```
┌──────────────────────────────────────────────┐
│  Always-On     AGENTS.md (via CLAUDE.md)       │
│                · skill descriptions            │
│  On-Demand     path-scoped rules · skill bodies│
│  Isolated      subagents (own context window)  │
│  External      hooks (deterministic, 0 tokens) │
└──────────────────────────────────────────────┘
```

## The CI review pipeline

The multi-agent PR review runs end to end in GitHub Actions:
**[`claude-code-review.yml`](.github/workflows/claude-code-review.yml)** wires a deterministic
preflight (PR-head checkout, full-vs-incremental mode), the `/pr-ci-review` orchestrator (which
spawns the `review-*` agents and emits a structured record), and a poster script that renders
the record to the PR — inline comments for blocking findings, collapsed sections for the rest,
and a commit status so an unreviewed diff is never mistaken for a clean one. The model itself
has **no write channel** to the PR: its tool allowlist is read-only, which is both the
prompt-injection defense and the delivery guarantee (a dead run still posts "not reviewed").
The deterministic half lives in **[`tools/review/`](tools/review/)** (preflight, schema,
poster, metrics — with the design and threat model in its README); each run's record is
appended to a `ci/review-metrics` orphan branch that the `/review-retro` skill mines to
improve the pipeline itself.

## The worktree-first workflow

The spine of this setup: **never work on `main`, one git worktree per task.** The `git-safety`
hook blocks `checkout -b`/pushes on `main`, `scripts/worktree-create.sh <name>` spins up an isolated
checkout under `.worktrees/`, the quality hook gates every response, and subagents can run in
their own worktree. Read the full loop in **[`docs/workflow.md`](docs/workflow.md)**.

## Quickstart

1. **Copy the template into your repo.**
   ```bash
   git clone https://github.com/AlexisBalayre/claude-code-power-config.git
   cd claude-code-power-config
   cp -R .claude AGENTS.md CLAUDE.md docs scripts .mcp.json ../your-repo/
   # optional: the CI review pipeline
   cp -R .github tools ../your-repo/
   chmod +x ../your-repo/.claude/hooks/*.sh ../your-repo/.claude/statusline.sh ../your-repo/scripts/*
   ```
   Merge `.gitignore` entries by hand. If the repo already has a `CLAUDE.md` or `AGENTS.md`, keep
   it aside: the next step merges its content.
2. **Adapt it.** In a worktree of your repo (`scripts/worktree-create.sh adapt-claude-config`), run
   `/adapt-to-project`. It surveys the codebase, confirms the detected stack with you, then fills
   every `TODO(adapt)` slot: `.claude/project.env`, `AGENTS.md`, `docs/conventions/` plus one rule
   loader per area, the architecture/security/glossary docs, and `CODEOWNERS`. It ends by pruning
   the skills and agents the project can't use (no database, no GitHub PR review, no Obsidian...).
   Re-run it whenever the project grows a new area.
3. **Opt into your tools.** Copy `.claude/settings.local.json.example` to
   `.claude/settings.local.json` (gitignored) and add your personal permissions / MCP servers.
   The shipped [`.mcp.json`](.mcp.json) declares [CodeGraph](https://github.com/colbymchenry/codegraph),
   a local symbol graph the agent queries instead of grepping; build its index once with
   `npx @colbymchenry/codegraph@1.6.0 init` (new worktrees get their own automatically).
4. **Personalize.** Copy `.env.example` to `.env` (gitignored) and fill in the values used by
   the personal-integration skills: your Obsidian vault path, issue-tracker IDs, SonarQube and Wiz IDs.

Until `.claude/project.env` names a command, every hook no-ops, so nothing breaks before the
project is adapted.

## Fine-tuning per project

Everything project-specific lives in a few well-known places, so tuning is editing, not rewiring:

| To change... | Edit |
| :----------- | :--- |
| Commands the quality gate, pre-commit and worktree scripts run | `.claude/project.env` |
| Coding rules for an area | `docs/conventions/<area>.md` (loaded by `.claude/rules/<area>-conventions.md`) |
| Cheap structural checks on every turn | `.claude/spot-checks.tsv` |
| What every agent knows up front | `AGENTS.md` (Claude-only notes in `CLAUDE.md`) |
| System shape, security model, vocabulary, decisions | `docs/reference/`, `docs/explanation/`, `docs/glossary.md`, `docs/adr/` |
| Which skills and agents exist | delete the directory or file, then update its catalog README |

## Repository structure

```
.
├── AGENTS.md                  # tool-agnostic always-on memory (any coding agent)
├── CLAUDE.md                  # thin Claude Code layer: @AGENTS.md + Claude-only notes
├── .env.example               # personal-integration settings (vault path, tracker/Sonar/Wiz IDs)
├── .mcp.json                  # CodeGraph MCP server (pinned, telemetry off)
├── .github/
│   ├── CODEOWNERS             # default reviewer for every path
│   └── workflows/
│       └── claude-code-review.yml # the CI review pipeline (preflight → model → poster)
├── tools/review/              # deterministic review tooling (schema, poster, metrics)
├── scripts/
│   ├── worktree-create.sh     # scripts/worktree-create.sh <name>
│   ├── worktree-clean.sh      # remove worktrees whose remote branch is gone
│   └── pre-commit             # lint/typecheck/test gate for human/CLI commits
├── .claude/
│   ├── README.md              # architecture deep-dive (start here)
│   ├── settings.json          # permissions + hook wiring
│   ├── project.env            # project profile read by hooks and scripts
│   ├── spot-checks.tsv        # convention spot-checks
│   ├── settings.local.json.example
│   ├── statusline.sh
│   ├── rules/  skills/  agents/  hooks/
└── docs/
    ├── README.md              # Diátaxis index
    ├── glossary.md            # shared vocabulary
    ├── workflow.md            # the worktree-first loop
    ├── conventions/           # source of truth imported by rules/ (core, testing, + areas)
    ├── reference/             # architecture.md
    ├── explanation/           # security-model.md
    └── adr/                   # decision records (none shipped)
```

## Make it yours

- **New area or stack change?** Re-run `/adapt-to-project` with a focus, e.g.
  `/adapt-to-project the new mobile app`.
- **Don't want a rule/skill/agent?** Delete the file. Each piece is independent.
- **Add your own?** [`.claude/README.md`](.claude/README.md) has an "Adding new extensions"
  recipe for every layer, and the `writing-for-agents` skill covers writing new skills.

## Acknowledgments & sources

This config stands on ideas and patterns from:

- **[mattpocock/skills](https://github.com/mattpocock/skills)** — Matt Pocock's library of Claude Code skills and the progressive-disclosure approach to skill design.
- **[shanraisshan/claude-code-best-practice](https://github.com/shanraisshan/claude-code-best-practice)** — a practical catalog of Claude Code best practices.
- **[Claude Code documentation](https://code.claude.com/docs)** — the official reference for settings, hooks, skills, subagents, and slash commands.

## License

[MIT](LICENSE).
