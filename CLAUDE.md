# Acme

Acme is a fictional real-time messaging platform used as a worked example for this
Claude Code configuration. It is a strict-convention pnpm + Turborepo monorepo.

## Role

Expert TypeScript / Node.js / React architect working in a strict-convention pnpm monorepo.

**Core rule:** Before creating or modifying code, read 2-3 similar files in the same directory and match their patterns exactly.

## Workspaces

| Path                            | What it is                                              |
| :------------------------------ | :------------------------------------------------------ |
| `apps/acme-api`                 | Public HTTP API (Hono + Zod OpenAPI)                    |
| `apps/acme-web`                 | React SPA (React 19, Vite, Tailwind, TanStack Router)   |
| `services/acme-gateway`         | Edge gateway: client connections, auth, routing (gRPC)  |
| `services/acme-session-engine`  | Stateful session lifecycle + state machines             |
| `packages/acme-db`              | Drizzle ORM schema + migrations                         |
| `packages/acme-providers`       | Pluggable delivery providers (email, SMS, push, webhook)|
| `packages/acme-rpc`             | Protobuf definitions + generated gRPC stubs             |
| `packages/acme-domain`          | Shared domain types (branded IDs, enums)                |
| `packages/acme-logger`          | Structured logger (no PII in logs)                      |

## Conventions

Path-scoped rules in `.claude/rules/*.md` auto-load the matching `docs/conventions/<area>.md` when you touch a file in that area. `docs/conventions/` is the single source of truth; `docs/README.md` (Diátaxis-structured) is the glossary + index for anything outside the path-scoped areas.

## Git workflow (CRITICAL)

- NEVER work on or push to `main`. PRs only.
- ALWAYS use `pnpm worktree:create <name>` (creates `.worktrees/<name>` with `feature/<name>`). NEVER `git checkout -b` in the main worktree.
- `git branch --show-current` MUST NOT be `main` before committing.

## Key commands

| Command                                           | Purpose                                            |
| :------------------------------------------------ | :------------------------------------------------- |
| `pnpm worktree:create <name>`                     | Create worktree at `.worktrees/<name>`             |
| `pnpm worktree:clean`                             | Remove worktrees with deleted remote branches      |
| `pnpm db:generate` / `db:migrate`                 | Generate migration SQL / apply pending migrations  |
| `pnpm --filter @acme/acme-api test:integration`   | Run integration tests (needs server + DB)          |
| `pnpm dev`                                         | Start the dev stack (reads `.env`)                 |

**Formatting/typechecking:** Biome + tsc run automatically via the `Stop` hook. Don't run them manually.

## Subagents (invoke proactively via Agent tool)

- `convention-checker` — before commit, or after ≥3 files across `apps/`/`services/`/`packages/`.
- `migration-reviewer` — after editing `packages/acme-db/src/schema/**` or running `pnpm db:generate`.
- `security-reviewer` — after editing auth, API routes, WS handlers, forms, or secret handling.
- `architecture-explainer` — for *why*/*how* questions about service boundaries or cross-service flows.
