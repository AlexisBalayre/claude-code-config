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
       ├─ TDD: red → green → refactor       (red first)
       ├─ Stop hook gates every response     (format/lint dirty files)
       ├─ convention-checker before commit   (≥3 source files changed)
       ├─ commit (pre-commit lints staged files; lands on feature/ branch, never main)
       ├─ bash scripts/verify.sh             (lint, typecheck, tests; stamps the tree green)
       └─ open PR  →  /pr-description  →  CI review (pr-ci-review)  →  /address-review-comments
scripts/worktree-remove.sh my-feature   # the worktree, then its branch once no work is lost
scripts/worktree-remove.sh --merged     # the same for every worktree whose tip a merged PR shipped
```

## What enforces it

| Mechanism | File | What it guarantees |
| :-------- | :--- | :----------------- |
| Branch protection | `.claude/hooks/git-safety.sh` | Blocks `git checkout -b` on the trunk, blocks pushes to the trunk, blocks `git reset --hard` and `rm -rf`. |
| Worktree creation | `scripts/worktree-create.sh` | `scripts/worktree-create.sh <name>` always makes `.worktrees/<name>` on `feature/<name>` and runs `INSTALL_CMD`. |
| Worktree removal | `scripts/worktree-remove.sh` | `scripts/worktree-remove.sh <name>...` keeps a worktree with uncommitted or untracked files (ignored ones go with it), a detached HEAD, or a branch outside `<prefix>/` (`WORKTREE_BRANCH_PREFIX`). It deletes the branch once a PR merged into `GIT_TRUNK` shipped its tip, along with `origin`'s copy while that is still at the tip, or once `origin`'s copy holds every commit. `--merged` does the same for every worktree whose tip a merged PR shipped, and keeps the rest. |
| Quality gate | `.claude/hooks/quality-checks.sh` | On every `Stop`, runs `FORMAT_FIX_CMD` and `LINT_CMD` (from `.claude/project.env`) on the dirty files of the session's checkout or worktree; blocks on a lint failure. An empty key skips that check. |
| Push gate | `scripts/verify.sh`, `scripts/pre-push` | `verify.sh` runs `LINT_CMD`, `TYPECHECK_CMD` and `TEST_CMD` on the committed tree and records it as green; `pre-push` refuses a tree without that record, and a PR above the size cap. `scripts/setup.sh` installs the hooks. |
| First push and merge | `.claude/hooks/push-gate.sh` | Asks the user before a push that can create a branch on `origin` and before a `gh` call that can merge. |
| Context survival | `.claude/hooks/post-compact-state.sh` | Re-injects the session's directory, the project checkout's branch, worktrees and open PRs after compaction. |
| Visibility | `.claude/statusline.sh` | Shows the active worktree branch, context usage, and cost in the statusline. |
| `CLAUDE.md` | repo root | States the rule in always-on context: PRs only, worktrees only, never the trunk. |

## The review pipeline

Every PR is reviewed in CI by `.github/workflows/claude-code-review.yml`; nothing runs the paid panel by hand.

1. A deterministic preflight (`tools/review`) checks the head, resolves the merge base and decides the mode: a first review is full, a push after it gets an incremental review of the delta.
2. The `pr-ci-review` skill orchestrates: it spawns only the `review-*` agents whose area the change touches (`.claude/agents/README.md`), consolidates their findings, and has `review-validator` try to refute each `important`.
3. The model has no write channel. It emits a structured record, and the poster script renders it: each `important` as an inline comment, minors and refuted findings in collapsed sections of one review, and a `claude-review` commit status on the head.
4. Each run's record lands on the `ci/review-metrics` orphan branch, which the `tooling` skill's retro mode reads.

`.github/workflows/pr-state-labels.yml` shows in the PR list what each open PR waits on. A draft carries no label; any other PR carries exactly one review label, plus the conflict label when it cannot merge:

| Label | Meaning |
| :--- | :--- |
| `Needs a Review` | no verdict yet from a reviewer with write access or from the `claude-review` status |
| `Changes Requested` | a reviewer requested changes, or the review found an `important` |
| `Approved` | an approval and no changes requested |
| `Needs Conflict Resolution` | merge conflicts with the base |

The workflow never creates a label and fails while one is missing, so create the four once per repository:

```sh
gh label create "Needs a Review" --color FBCA04
gh label create "Changes Requested" --color D93F0B
gh label create "Approved" --color 0E8A16
gh label create "Needs Conflict Resolution" --color B60205
```

A comment steers a round: `@claude review [quick|deep|full] [areas]`. A quick or area-restricted round is partial and leaves the status to a full pass. The round and merge rules (importants block, minors never do, when a PR is ready) are in `.claude/skills/pr-ci-review/REVIEW.md`; `/address-review-comments` works the open threads with you deciding each call. `claude-review-base-change.yml` marks a retargeted PR's head pending, and `pr-size.yml` warns at 400 added production lines and fails at 800.

## Releases

`.github/workflows/release.yml` releases on its own: after each merge to the trunk, `scripts/next-version.sh` reads the commit subjects since the last `vX.Y.Z` tag and the workflow tags that commit and publishes a GitHub release with generated notes. The PR title is the squash commit's subject, so it decides the version:

| Since the last tag | Next version |
| :--- | :--- |
| a `type!:` subject or a `BREAKING CHANGE:` footer | major |
| `feat` | minor |
| `fix` or `perf` | patch |
| anything else (`docs`, `chore`, `test`, `refactor`, `ci`, a revert) | no release |

Mark a PR that removes or renames a file adopters copy with `!` and say what to do in its body: the generated notes link the PR, and they are the upgrade guide. The trunk name in the workflow's `branches:` follows `GIT_TRUNK`.

## Dispatching agents into worktrees

For independent tasks, subagents can run in their own worktree so their edits never collide
with yours:

```
isolation: worktree   # in an agent definition or Agent tool call
```

The agent gets a temporary worktree, does its work, and the worktree is cleaned up if nothing
changed. A rule of thumb for removals: *open a worktree per category; do not bundle unrelated
removals into one PR.*

## Adapting it

The trunk and branch prefix are `GIT_TRUNK` (default `main`) and `WORKTREE_BRANCH_PREFIX`
(default `feature`) in `.claude/project.env`. The worktree scripts and `.claude/hooks/git-safety.sh`
both read them from there, so the helper and the guard stay in agreement automatically. The loop
above assumes the defaults and an `origin` remote.
