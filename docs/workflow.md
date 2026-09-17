# How I work: the worktree-first loop

This config is built around one habit: **never work on the trunk (`main` by default), one git
worktree per unit of work.** Everything else (the safety hook, the statusline, the compaction hook, the way agents
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
scripts/worktree-create.sh my-feature   # .worktrees/my-feature on branch feature/my-feature
  └─ cd .worktrees/my-feature
       ├─ explore + plan (read 2-3 neighbours; match patterns)
       ├─ TDD: red → green → refactor       (tdd skill)
       ├─ Stop hook gates every response     (format/lint dirty files · typecheck)
       ├─ convention-checker before commit   (≥3 source files changed)
       ├─ commit (pre-commit runs TEST_CMD; lands on feature/ branch, never main)
       └─ open PR  →  /pr-description  →  /pr-ci-review
scripts/worktree-clean.sh               # remove worktrees whose remote branch is gone
```

## What enforces it

| Mechanism | File | What it guarantees |
| :-------- | :--- | :----------------- |
| Branch protection | `.claude/hooks/git-safety.sh` | Blocks `git checkout -b` on the trunk, blocks pushes to the trunk, blocks `git reset --hard` and `rm -rf`. |
| Worktree creation | `scripts/worktree-create.sh` | `scripts/worktree-create.sh <name>` always makes `.worktrees/<name>` on `feature/<name>` and runs `INSTALL_CMD`. |
| Worktree cleanup | `scripts/worktree-clean.sh` | Removes worktrees whose remote branch is gone (e.g. after merge). |
| Quality gate | `.claude/hooks/quality-checks.sh` | On every `Stop`, runs `FORMAT_FIX_CMD` and `LINT_CMD` on the dirty files and `TYPECHECK_CMD` on the repo (all from `.claude/project.env`); blocks on failure. An empty key skips that check. |
| Test gate | `scripts/pre-commit` | Runs `TEST_CMD` from `.claude/project.env` before every commit. |
| Context survival | `.claude/hooks/pre-compact-preserve.sh` | Preserves the current branch + worktree path + test results across compaction. |
| Visibility | `.claude/statusline.sh` | Shows the active worktree branch, context usage, and cost in the statusline. |
| `CLAUDE.md` | repo root | States the rule in always-on context: PRs only, worktrees only, never the trunk. |

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

The trunk and branch prefix are `GIT_TRUNK` (default `main`) and `WORKTREE_BRANCH_PREFIX`
(default `feature`) in `.claude/project.env`. The worktree scripts and `.claude/hooks/git-safety.sh`
both read them from there, so the helper and the guard stay in agreement automatically. The loop
above assumes the defaults and an `origin` remote.
