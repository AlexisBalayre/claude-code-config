# Acme

Instructions for AI coding agents, whatever the tool. Tool-specific layers build on this file: Claude Code adds `CLAUDE.md` and `.claude/`, Cursor adds `.cursor/`.

Acme is a fictional real-time messaging platform used as a worked example for this configuration. It is a strict-convention pnpm + Turborepo monorepo.

## Role

Expert TypeScript / Node.js / React architect working in a strict-convention pnpm monorepo.

**Core rule:** Before creating or modifying code, read 2-3 similar files in the same directory and match their patterns exactly.

## Conventions

`docs/conventions/` is the single source of truth. Before touching a file, read the conventions for every area that covers it:

| Area | Paths | Conventions |
| --- | --- | --- |
| Core | any `*.ts` / `*.tsx` | `docs/conventions/core.md` |
| Backend | `apps/acme-api/**`, `packages/acme-db/**` | `docs/conventions/backend.md` |
| Frontend | `apps/acme-web/**` | `docs/conventions/frontend.md` |
| Services | `services/acme-gateway/**`, `services/acme-session-engine/**`, `packages/acme-rpc/**`, `packages/acme-providers/**` | `docs/conventions/services.md` |
| Testing | `**/*.test.ts`, `**/*.test.tsx`, `**/*.mock.ts`, `**/test/**`, `**/vitest*.config.ts`, `tools/vitest/**` | `docs/conventions/testing.md` |

`docs/README.md` (Diátaxis-structured) covers anything outside these areas; shared vocabulary lives in `docs/glossary.md`.

## Layout

Topology + end-to-end session flow: `docs/reference/backend-architecture.md`.

## Comments

Default to no inline comment. Add `//` only for a non-obvious WHY (invariant, unit, ordering, gotcha), never to narrate WHAT the code does. If removing a comment wouldn't confuse a competent reader, delete it. A WHY comment that restates the callee's JSDoc or class doc is a duplicate: read the callee's docs before keeping it; if the seam already says it, delete the call-site copy.

```ts
retries++; // increment the retry counter           (BAD: restates the code)
retries++; // 429s are transient, so retry first    (GOOD: explains the why)
```

## Altitude

Build the smallest thing that works; the obligation lives in `docs/conventions/core.md` §Altitude / YAGNI. Before finishing a change that introduced any discretionary construct (new file, helper, wrapper, option, param, interface, generic, or defensive branch), list each one with a keep-or-inline verdict; keep means you can name a second caller or a real WHY. Default to inline/delete. Don't list prescribed taxonomy roles, and skip the audit silently when a change added none. Unjustifiable bloat is a `/simplify` candidate.

## Frontend

Single SPA at `apps/acme-web/`. Conventions in `docs/conventions/frontend.md`; architecture in `docs/reference/frontend-architecture.md`.

## Git workflow (CRITICAL)

- NEVER work on or push to `main`. PRs only.
- ALWAYS use `pnpm worktree:create <name>` (creates `.worktrees/<name>` with `feature/<name>`). NEVER `git checkout -b` in the main worktree.
- `git branch --show-current` MUST NOT be `main` before committing.

## Key commands

`pnpm test:integration` runs in-process (no services need to be up); `pnpm dev` reads `.env`.

**Formatting/typechecking:** run `pnpm lint` and `pnpm typecheck` before handing work back. (Claude Code only: don't run them manually; its `Stop` hook runs Biome + tsc automatically.)
