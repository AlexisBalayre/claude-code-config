# Claude Code Configuration

This directory contains all Claude Code customizations for the Acme project. Everything here extends Claude's agentic loop: the cycle of reasoning, tool use, and iteration that powers every session.

## How It All Fits Together

```
┌─────────────────────────────────────────────────────────┐
│                    Always-On Context                     │
│  CLAUDE.md (project rules, build commands)               │
│  Rules without paths (unconditional)                     │
│  Skill descriptions (names + one-liners)                 │
│  MCP tool schemas                                        │
├─────────────────────────────────────────────────────────┤
│                    On-Demand Context                      │
│  Rules with paths (load when matching files opened)      │
│  Skill full content (load when invoked/relevant)         │
│  Subdirectory CLAUDE.md files                            │
├─────────────────────────────────────────────────────────┤
│                    Isolated Context                       │
│  Subagents (own context window, return summary)          │
├─────────────────────────────────────────────────────────┤
│                    External (Zero Cost)                   │
│  Hooks (shell scripts, no LLM, deterministic)            │
└─────────────────────────────────────────────────────────┘
```

**Context budget matters.** Everything in "Always-On" consumes tokens every turn. Rules with `paths:` and skills with descriptions load on-demand, saving context. Subagents run in isolated windows. Hooks cost zero context.

---

## Directory Structure

```
.claude/
├── settings.json              # Shared project config (permissions, hooks)
├── settings.local.json.example # Template for personal overrides (real file gitignored)
│
├── rules/                 # Path-scoped convention rules (auto-load)
│   ├── universal-conventions.md
│   ├── api-conventions.md
│   ├── frontend-conventions.md
│   ├── testing-conventions.md
│   ├── database-conventions.md
│   ├── session-engine-conventions.md
│   ├── gateway-conventions.md
│   ├── grpc-conventions.md
│   └── providers-conventions.md
│
├── skills/                # Auto-discoverable knowledge + workflows (each is <name>/SKILL.md)
│   ├── new-api-endpoint/   new-frontend-route/   new-provider/   # scaffolding
│   ├── tdd/   diagnose/   resolve-merge-conflicts/               # engineering
│   ├── find-dead-code/   improve-codebase-architecture/          # engineering (manual)
│   ├── grilling/   grill-me/   grill-with-docs/                  # thinking / design
│   ├── codebase-design/   domain-modeling/   zoom-out/   prototype/
│   ├── pr-description/   pr-ci-review/                           # PR & review
│   ├── address-review-comments/   review-retro/
│   ├── write-a-skill/   handoff/   caveman/                      # meta / workflow
│   ├── obsidian-vault/   daily-note/   to-issues/   to-epic/     # personal integrations (.env)
│   └── backfill-issues/   fix-sonar/   wiz/   fix-wiz/
│
├── agents/                # Custom subagents for specialized tasks
│   ├── convention-checker.md      migration-reviewer.md          # proactive
│   ├── security-reviewer.md       architecture-explainer.md
│   ├── review-context.md          review-conventions.md          # dispatched by pr-ci-review
│   ├── review-correctness.md      review-docs.md
│   ├── review-maintainability.md  review-security.md
│   ├── review-validator.md
│   └── comment-pruner.md          # dispatched by the comment-pruner Stop hook
│
└── hooks/                    # Deterministic shell scripts (zero LLM cost)
    ├── quality-checks.sh          # Stop: lint/format dirty files + repo typecheck
    ├── convention-spot-check.sh   # Stop: advisory file-level convention scan
    ├── comment-pruner.sh          # Stop: dispatch the comment-pruner subagent on new comments
    ├── git-safety.sh              # PreToolUse(Bash): block dangerous git/shell ops
    ├── protect-generated.sh       # PreToolUse(Edit|Write): block generated files
    ├── validate-file-naming.sh    # PreToolUse(Write): enforce kebab-case.role.ts
    └── pre-compact-preserve.sh    # PreCompact: inject must-preserve context
```

---

## Extension Points Explained

### 1. `CLAUDE.md` — Project Memory

The root `CLAUDE.md` contains universal rules Claude sees every session: coding standards, git workflow, key commands. Kept under ~60 lines to minimize context cost.

**When to edit:** Add universal rules that apply to every file. For area-specific rules, use `rules/` instead.

### 2. `rules/` — Path-Scoped Convention Rules

Markdown files with `paths:` frontmatter that auto-load when Claude works with matching files. Each rule contains a quick-reference (~15 lines) plus an `@docs/conventions/X.md` import for the full doc.

```yaml
---
paths:
  - "apps/acme-api/**"
---
# API Conventions — Quick Reference
- STRICT layering: Routes → Services → Repositories → Database
...
## Full conventions
@docs/conventions/api.md
```

**Key insight:** Rules follow the "split pattern" — lightweight recognition triggers (quick facts) pointing to detailed knowledge (full convention docs). This keeps always-on context small while ensuring full detail loads when needed.

**When to add a rule:** When conventions are specific to a file path pattern and should auto-load when editing those files.

**Catalog:** [`rules/README.md`](rules/README.md) — every rule, its path scope, and what it enforces.

### 3. `skills/` — Auto-Discoverable Workflows

Skills are directories with a `SKILL.md` that Claude discovers automatically. Claude sees the description at session start (tiny context cost) and loads the full content when the skill is relevant.

This repo ships **30 skills** across scaffolding, engineering, thinking/design, PR & review, meta, and personal integrations. The **[skill catalog](skills/README.md)** lists when each one fires and how to invoke it (auto-trigger, `/slash-command`, Claude-only, or manual-only).

**Frontmatter options:**
- `name` — identifier and `/slash-command` name
- `description` — when to auto-activate (critical for discovery)
- `user-invocable: false` — Claude-only (hidden from `/` menu)
- `disable-model-invocation: true` — user-only (Claude can't auto-trigger)
- `allowed-tools` — tools available without permission prompts
- `context: fork` — run in isolated subagent context
- `model` — override model when active

**When to add a skill:** When Claude should auto-discover and apply knowledge or follow a workflow without being asked. Skills are for things Claude should know to do on its own. For repeatable workflows only the user should trigger — anything with side effects like posting PR comments or filing issues — set `disable-model-invocation: true`: a manual-only skill is invoked as `/<name>` and replaces the deprecated `commands/` layer.

### 4. `agents/` — Custom Subagents

Specialized AI workers that run in their own context window. Claude delegates to them and gets summarized results back — zero bloat in the main conversation.

| Agent | Model | Tools | Purpose |
|-------|-------|-------|---------|
| `convention-checker` | Haiku | Read, Glob, Grep | Fast convention compliance audit |
| `migration-reviewer` | Sonnet | Read, Glob, Grep | Schema change safety review |
| `security-reviewer` | Opus | Read, Glob, Grep, Bash | Deep security analysis |
| `architecture-explainer` | Sonnet | Read, Glob, Grep | Answer why/how architecture questions, grounded in `docs/` |
| `review-*` (7 agents) | Sonnet/Opus | Read, Glob, Grep, Bash | Area reviewers + adversarial validator, dispatched by the `pr-ci-review` skill |
| `comment-pruner` | Sonnet | Read, Edit, Grep, Glob, Bash | Prune low-value comments; dispatched by the `comment-pruner.sh` Stop hook |

**Frontmatter options:**
- `tools` — allowlist of tools (restricts to only what's needed)
- `model` — `haiku` (fast/cheap), `sonnet` (balanced), `opus` (deep reasoning)
- `memory: project` — persistent knowledge across sessions
- `isolation: worktree` — run in temporary git worktree
- `skills` — preload specific skills into subagent context
- `mcpServers` — scope MCP servers to this subagent only
- `maxTurns` — limit agentic turns

**When to add an agent:** For tasks that need deep analysis without polluting the main context, or for work that benefits from model-specific strengths (Haiku for speed, Opus for reasoning).

**Catalog:** [`agents/README.md`](agents/README.md) — every agent, when it fires, and its model/tools.

### 5. `hooks/` — Deterministic Automation

Shell scripts that run outside the LLM loop on lifecycle events. Zero context cost, zero hallucination risk — purely deterministic.

| Hook | Event | What it does |
|------|-------|-------------|
| `quality-checks.sh` | Stop | Lint/format dirty files, typecheck the repo (blocks on failure; tests live in pre-commit) |
| `convention-spot-check.sh` | Stop | Advisory scan for `export default`, inline types, missing JSDoc (`packages/` only) |
| `comment-pruner.sh` | Stop | Dispatch the `comment-pruner` subagent when the session added net-new comments |
| `git-safety.sh` | PreToolUse(Bash) | Block `rm -rf`, `git reset --hard`, force push, `checkout -b` on main, push to main |
| `protect-generated.sh` | PreToolUse(Edit\|Write) | Block edits to `*.gen.ts` and gRPC stubs |
| `validate-file-naming.sh` | PreToolUse(Write) | Enforce `kebab-case.role.ts` on new files |
| `pre-compact-preserve.sh` | PreCompact | Preserve branch, modified files, test output across compaction |

**Exit codes:**
- `0` — success, continue
- `1` — error (shown to user, continues)
- `2` — **block the operation** (PreToolUse: prevents tool; Stop: feedback to Claude)

**When to add a hook:** For deterministic checks that should always run. If it doesn't need LLM reasoning, it's a hook.

**Catalog:** [`hooks/README.md`](hooks/README.md) — every hook, the event it fires on, and what it does.

### 6. `settings.json` — Permissions & Hook Wiring

Shared project configuration. Contains:
- **`permissions.allow`** — pre-approved tool patterns (pnpm, git read-only, MCP tools)
- **`permissions.deny`** — explicitly blocked operations (force push, hard reset, rm -rf)
- **`hooks`** — wires hook scripts to lifecycle events

**`settings.local.json`** (gitignored) extends this with personal preferences — additional MCP servers, machine-specific permissions, etc. Copy `settings.local.json.example` to `settings.local.json` to start; it is merged on top of `settings.json`, never replacing it.

---

## Decision Framework

| I want... | Use... |
|-----------|--------|
| Claude to always know this | `CLAUDE.md` |
| Claude to know this when editing specific files | `rules/` with `paths:` |
| Claude to auto-discover and use this knowledge | `skills/` |
| A workflow I trigger explicitly | `skills/` with `disable-model-invocation: true` |
| Isolated analysis that won't bloat context | `agents/` |
| A check that runs every time, deterministically | `hooks/` |
| External service access | MCP (`.mcp.json`) |

---

## Adding New Extensions

### New rule
1. Create `.claude/rules/<name>.md` with `paths:` frontmatter
2. Add ~15 lines of quick-reference facts
3. Point to full docs with `@docs/conventions/<area>.md`

### New skill
1. Create `.claude/skills/<name>/SKILL.md` with `name` and `description` frontmatter
2. Write the full workflow/knowledge content
3. Set `user-invocable: false` if Claude-only, `disable-model-invocation: true` if user-only

### New agent
1. Create `.claude/agents/<name>.md` with `name`, `description`, and `tools`
2. Choose `model` based on task complexity (haiku/sonnet/opus)
3. Restrict `tools` to minimum needed

### New hook
1. Create `.claude/hooks/<name>.sh` (must be executable)
2. Wire it in `settings.json` under the appropriate event
3. Use exit code `2` to block operations
