# Project

<!-- TODO(adapt): replace the heading with the project name and write 1-2 sentences on what it is, its stack, and its shape (single package, monorepo, services). Run /adapt-to-project to fill every TODO(adapt) in this repo. -->

Instructions for AI coding agents, whatever the tool. Tool-specific layers build on this file: Claude Code adds `CLAUDE.md` and `.claude/`, Cursor adds `.cursor/`.

## Role

<!-- TODO(adapt): the expertise to bring, e.g. "Expert Python / FastAPI engineer working in a strict-convention service". -->

**Core rule:** Before creating or modifying code, read 2-3 similar files in the same directory and match their patterns exactly.

## Precedence

When two instructions disagree, the higher one wins:

1. The user's word in this session, including an approval a lead session relays.
2. `AGENTS.md` and `CLAUDE.md`.
3. `docs/conventions/` and the accepted ADRs in `docs/adr/`.
4. Skills.
5. Machine-local notes and memories.

Hooks and `.claude/settings.json` are the mechanical layer under all five. Change a hook only in your own worktree, as a diff in a PR, never to unblock yourself. When a hook or permission blocks you, stop and hand the user the command.

## Conventions

`docs/conventions/` is the single source of truth. The conventions for every area covering a file must be in context before you touch it: Claude Code injects them through `.claude/rules/`; other tools read them from this table.

| Area | Paths | Conventions |
| --- | --- | --- |
| Core | any source file | `docs/conventions/core.md` |
| Testing | test files and test directories | `docs/conventions/testing.md` |

<!-- TODO(adapt): add one row per area with its own rules (e.g. API, database, frontend), each with a matching docs/conventions/<area>.md and .claude/rules/<area>-conventions.md. -->

`docs/README.md` (Diátaxis-structured) covers anything outside these areas; shared vocabulary lives in `docs/glossary.md`.

## Layout

Topology, components and data flow: `docs/reference/architecture.md`.

## Comments

Comments carry only a non-obvious WHY, in every language. The rule and its example: `docs/conventions/core.md` §Comments.

## Altitude

Build the smallest thing that works; the obligation lives in `docs/conventions/core.md` §Altitude / YAGNI. Before finishing a change that introduced any discretionary construct (new file, helper, wrapper, option, param, interface, generic, or defensive branch), list each one with a keep-or-inline verdict; keep means you can name a second caller or a real WHY. Default to inline/delete. Don't list constructs a convention prescribes, and skip the audit silently when a change added none. Unjustifiable bloat is a `/simplify` candidate.

## Git workflow (CRITICAL)

- NEVER work on or push to the trunk (`GIT_TRUNK` in `.claude/project.env`, default `main`). PRs only.
- ALWAYS use `scripts/worktree-create.sh <name>` (creates `.worktrees/<name>` on `feature/<name>`). NEVER `git checkout -b` in the main worktree.
- `git branch --show-current` MUST NOT be the trunk before committing.
- Remove a worktree only with `scripts/worktree-remove.sh <name>`: it keeps one that still holds unpushed work. Never `git worktree remove` or `git branch -D` yourself.

## Key commands

Lint, typecheck, test and install commands live in `.claude/project.env` (`LINT_CMD`, `TYPECHECK_CMD`, `TEST_CMD`, `INSTALL_CMD`).

<!-- TODO(adapt): list the commands an agent needs beyond those (dev server, integration tests, codegen, migrations) and any gotcha (services that must be up, env files). -->

- **Before a push:** commit, then run `bash scripts/verify.sh`. It runs lint, typecheck and tests on the committed tree and stamps it green; `scripts/pre-push` refuses a tree without that stamp.
- **Heavy commands** (a whole-repo typecheck or test run outside verify): `bash scripts/gate-lock.sh <cmd>`, so parallel worktrees take turns instead of starving each other.
- **Formatting and lint:** run the lint command before handing work back. (Claude Code only: don't run it manually; its `Stop` hook formats and lints the files you touched.)
- **After cloning:** `bash scripts/setup.sh` installs the git hooks.
