# claude-code-power-config

A complete, opinionated **[Claude Code](https://code.claude.com/docs)** configuration for a
strict-convention TypeScript monorepo — rules, skills, slash commands, subagents, and
deterministic hooks, wired together and documented end to end.

It is extracted from real day-to-day use on a production monorepo and fully genericized onto a
fictional demo project (**"Acme"**, a real-time messaging platform), so you can read every
piece in context, then lift what you want into your own repo.

> **TL;DR** — Copy `.claude/` into your repo, adapt `CLAUDE.md` and `.claude/rules/` to your
> stack, make the hooks executable, and you have path-scoped conventions, a quality gate on
> every turn, a multi-agent PR review, and a worktree-first workflow that never lets an agent
> commit to `main`.

---

## What's inside

| Layer | Count | What it does |
| :---- | :---- | :----------- |
| **`CLAUDE.md`** | 1 | Always-on project memory: role, workspaces, git workflow, key commands. Kept tiny on purpose. |
| **`rules/`** | 9 | Path-scoped convention rules that auto-load `docs/conventions/*.md` only when you touch matching files (the "split pattern"). |
| **`skills/`** | 14 | Auto-discoverable workflows: scaffolding (`new-api-endpoint`, `new-provider`…), engineering (`tdd`, `diagnose`, `find-dead-code`…), thinking (`grill-me`, `grill-with-docs`, `zoom-out`, `prototype`), and meta (`write-a-skill`, `handoff`, `caveman`). |
| **`commands/`** | 1 | `/code-review` — a 6-agent PR review with a validation pass and inline comments. |
| **`agents/`** | 4 | Isolated subagents: `convention-checker` (Haiku), `migration-reviewer` (Sonnet), `security-reviewer` (Opus), `architecture-explainer` (Sonnet). |
| **`hooks/`** | 6 | Zero-LLM shell scripts on lifecycle events: quality gate, convention spot-check, git safety, generated-file protection, file-naming validation, compaction preservation. |

The full architecture — what loads when, the context budget, and how to extend each layer — is
documented in **[`.claude/README.md`](.claude/README.md)**.

```
┌──────────────────────────────────────────────┐
│  Always-On     CLAUDE.md · unconditional rules │
│                · skill descriptions            │
│  On-Demand     path-scoped rules · skill bodies│
│  Isolated      subagents (own context window)  │
│  External      hooks (deterministic, 0 tokens) │
└──────────────────────────────────────────────┘
```

## The worktree-first workflow

The spine of this setup: **never work on `main`, one git worktree per task.** The `git-safety`
hook blocks `checkout -b`/pushes on `main`, `pnpm worktree:create <name>` spins up an isolated
checkout under `.worktrees/`, the quality hook gates every response, and subagents can run in
their own worktree. Read the full loop in **[`docs/workflow.md`](docs/workflow.md)**.

## Quickstart

1. **Copy the config into your repo.**
   ```bash
   cp -R claude-code-power-config/.claude your-repo/.claude
   cp claude-code-power-config/CLAUDE.md your-repo/CLAUDE.md
   chmod +x your-repo/.claude/hooks/*.sh your-repo/.claude/statusline.sh
   ```
2. **Adapt it to your stack.** Edit `CLAUDE.md` (workspaces, commands), point the `paths:` in
   `.claude/rules/*.md` at your directories, and replace the placeholder `lint`/`typecheck`/
   `test` scripts in `package.json` with your real toolchain.
3. **Opt into your tools.** Copy `.claude/settings.local.json.example` to
   `.claude/settings.local.json` (gitignored) and add your personal permissions / MCP servers.

The hooks no-op until you touch matching files, so nothing breaks before you've wired your
toolchain.

## The demo project ("Acme")

So the rules, skills, and agents have something concrete to point at, the repo is modeled on a
fictional messaging platform. The shape maps cleanly onto most TypeScript monorepos:

| Path | Role |
| :--- | :--- |
| `apps/acme-api` | Public HTTP API (Hono + Zod OpenAPI) |
| `apps/acme-web` | React SPA (React 19, Vite, Tailwind, TanStack Router) |
| `services/acme-gateway` | Realtime edge: connections, auth, routing (gRPC) |
| `services/acme-session-engine` | Stateful session lifecycle + state machines |
| `packages/acme-db` | Drizzle ORM schema + migrations |
| `packages/acme-providers` | Pluggable delivery providers (email, SMS, push, webhook) |
| `packages/acme-rpc` | Protobuf definitions + generated gRPC stubs |

The conventions for each area live in [`docs/conventions/`](docs/conventions/) and are the
single source of truth the thin `rules/` files import. The [glossary](docs/README.md) anchors
the shared vocabulary.

## Repository structure

```
.
├── CLAUDE.md                  # always-on project memory
├── package.json               # worktree scripts + toolchain hooks
├── scripts/worktrees.sh       # pnpm worktree:create / clean
├── .claude/
│   ├── README.md              # architecture deep-dive (start here)
│   ├── settings.json          # permissions + hook wiring
│   ├── settings.local.json.example
│   ├── statusline.sh
│   ├── rules/  skills/  commands/  agents/  hooks/
└── docs/
    ├── README.md              # glossary + Diátaxis index
    ├── workflow.md            # the worktree-first loop
    ├── conventions/           # source of truth imported by rules/
    ├── reference/  explanation/  adr/
```

## Make it yours

- **Different stack?** The *patterns* transfer even if the libraries don't. Keep the layered
  structure (always-on vs. path-scoped vs. isolated vs. deterministic) and swap the content.
- **Don't want a rule/skill/agent?** Delete the file. Each piece is independent.
- **Add your own?** [`.claude/README.md`](.claude/README.md) has an "Adding new extensions"
  recipe for every layer, and the `write-a-skill` skill scaffolds new skills.

## Acknowledgments & sources

This config stands on ideas and patterns from:

- **[mattpocock/skills](https://github.com/mattpocock/skills)** — Matt Pocock's library of Claude Code skills and the progressive-disclosure approach to skill design.
- **[shanraisshan/claude-code-best-practice](https://github.com/shanraisshan/claude-code-best-practice)** — a practical catalog of Claude Code best practices.
- **[Claude Code documentation](https://code.claude.com/docs)** — the official reference for settings, hooks, skills, subagents, and slash commands.

## License

[MIT](LICENSE).
