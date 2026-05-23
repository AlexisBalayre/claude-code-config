# How I work: the worktree-first loop

This config is built around one habit: **never work on `main`, one git worktree per unit of
work.** Everything else (the safety hook, the statusline, the compaction hook, the way agents
are dispatched) exists to support that habit. This page explains the loop and how the config
enforces it.

## Why worktrees

A [git worktree](https://git-scm.com/docs/git-worktree) is a second checkout of the same repo
on its own branch, in its own directory. Instead of stashing and switching branches in one
working copy, each task gets an isolated directory under `.worktrees/`.

This matters more with an agent in the loop:

- **No accidental commits to `main`.** Plain `git commit` from a worktree lands on that
  worktree's branch, not `main`.
- **Parallel work is real, not simulated.** A long review can run in one worktree while you
  start the next feature in another. Subagents can be dispatched with `isolation: worktree`
  to work on an isolated copy without touching yours.
- **Clean blast radius.** A throwaway experiment lives in its own directory and is deleted
  wholesale, not unwound commit by commit.

## The loop

```
pnpm worktree:create my-feature      # .worktrees/my-feature on branch feature/my-feature
  └─ cd .worktrees/my-feature
       ├─ explore + plan (read 2-3 neighbours; match patterns)
       ├─ TDD: red → green → refactor       (tdd skill)
       ├─ Stop hook gates every response     (lint · typecheck · test affected pkgs)
       ├─ convention-checker before commit   (≥3 files across apps/services/packages)
       ├─ commit (lands on feature/ branch, never main)
       └─ open PR  →  /code-review
pnpm worktree:clean                   # remove worktrees whose remote branch is gone
```

## What enforces it

| Mechanism | File | What it guarantees |
| :-------- | :--- | :----------------- |
| Branch protection | `.claude/hooks/git-safety.sh` | Blocks `git checkout -b` on `main`, blocks pushes to `main`, blocks `git reset --hard` and `rm -rf`. |
| Worktree creation | `scripts/worktrees.sh` + `package.json` | `pnpm worktree:create <name>` always makes `.worktrees/<name>` on `feature/<name>`. |
| Quality gate | `.claude/hooks/quality-checks.sh` | On every `Stop`, runs lint + typecheck + tests for affected packages; blocks on failure. |
| Context survival | `.claude/hooks/pre-compact-preserve.sh` | Preserves the current branch + worktree path + test results across compaction. |
| Visibility | `.claude/statusline.sh` | Shows the active worktree branch, context usage, and cost in the statusline. |
| `CLAUDE.md` | repo root | States the rule in always-on context: PRs only, worktrees only, never `main`. |

## Dispatching agents into worktrees

For independent tasks, subagents can run in their own worktree so their edits never collide
with yours:

```
isolation: worktree   # in an agent definition or Agent tool call
```

The agent gets a temporary worktree, does its work, and the worktree is cleaned up if nothing
changed. This pairs well with the `find-dead-code` rule of thumb: *open a worktree per
category; do not bundle unrelated removals into one PR.*

## Adapting it

The scripts assume `feature/<name>` branches and an `origin` remote. If your team uses a
different branch prefix or trunk name, edit `scripts/worktrees.sh` and the patterns in
`.claude/hooks/git-safety.sh` together so the helper and the guard stay in agreement.
