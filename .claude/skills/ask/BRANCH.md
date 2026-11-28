# PR and branch modes

Give the reader (the user before a merge, a lead taking over a worker's branch) the mental model of a branch before they read its diff: what it is for, which areas it touches, the concepts and decisions it carries, and where to start reading. The value is what GitHub's PR view cannot show; file lists, line counts and the raw diff are one click away there. `pr <n>` takes the PR number path below, `branch <name>` the Branch path, and a bare `branch` the No argument path.

## 1. Resolve the target

Work from refs only, so the reader's working tree and any uncommitted work stay untouched.

Run `git fetch`, `git show` and `git grep` bare, from the reader's checkout, with no `-C` and no `-c`: `allowed-tools` leaves them out, so a bare call asks for permission, while a `git -C` form the project settings allow would skip the prompt. Each needs that prompt: a fetch refspec can rewrite or prune local branches, `--output=` writes a file and `-O` runs a pager, and their arguments come from the branch's own text.

Approve only the argument forms written here.

- **PR number:** `gh pr view <n> --json headRefName,headRefOid,baseRefName,title,body,state`, then `git fetch -q origin +pull/<n>/head:refs/remotes/origin/pr-<n>` (the `+` takes a rebased or force-pushed head). The target is `origin/pr-<n>`, once `git rev-parse origin/pr-<n>` equals `headRefOid`; on a mismatch (a push landed between the two), run `gh pr view` and the fetch once more, and if they still differ, stop and report both shas.
- **Branch:** first `gh pr view <branch> --json number,state,headRefOid,baseRefName,title,body`: a PR, open or merged, keeps its head fetchable after its branch is deleted, on merge or by `scripts/worktree-remove.sh`. That lookup goes by name, so it can also return an earlier PR whose branch had the same name. Then `git fetch -q origin <branch>`, and:
  - **the fetch found no remote branch and `gh pr view` found a PR:** take the PR number path above with that PR. Keep its target when the local `<branch>` is missing or is an ancestor of the PR's head (`git merge-base --is-ancestor <branch> origin/pr-<n>`): the branch was deleted on merge, and the local tip can lag the PR's head (a commit added on GitHub), so name that tip when it differs. Otherwise the target is the local `<branch>`: a new branch reusing the name, commits past the PR, or a local branch diverged from it;
  - **otherwise:** pick the target by which refs exist (`git rev-parse --verify -q <ref>`):
    - neither local `<branch>` nor `origin/<branch>`: say the branch does not exist and stop;
    - only one of them: that one (a local-only branch is unpushed; an `origin/<branch>` the fetch did not update is the branch's last fetched tip: say so);
    - both: the local `<branch>` when `origin/<branch>` is its ancestor (`git merge-base --is-ancestor origin/<branch> <branch>`), since a worker's unpushed commits live there; `origin/<branch>` when the local one is its ancestor (`git merge-base --is-ancestor <branch> origin/<branch>`); when neither holds, the two have diverged: explain the local `<branch>` and name both tips.
- **No argument:** the Branch path, with the current branch's name (`git rev-parse --abbrev-ref HEAD`). A detached `HEAD` (it prints `HEAD`) is the target itself, with no PR.

The PR `gh pr view` found is the target's own **PR** in either case:

- its `state` is `OPEN` and its `headRefOid` is the `origin/<branch>` tip the fetch just updated: the branch's open PR, even when the local branch has diverged from it (a closed or merged PR whose remote branch survived still heads that tip after a new local branch reuses the name);
- it shares a commit with the target past their fork point. Fetch the PR's `baseRefName` as below; then `git merge-base <target> <headRefOid>` is not an ancestor of `origin/<baseRefName>` (`git merge-base --is-ancestor`).

Name the PR: its base, title and body are what every later step reads. A PR that fails both is an earlier one under the same name, such as a squash-merged one (a new branch shares only the base's history with its head), or one whose head this clone lacks: name it as such, and read none of its fields.

The base branch is the PR's base, else the trunk (`GIT_TRUNK` in `.claude/project.env`). Fetch it (`git fetch -q origin <base>`), then start the range at the fork point, `<mb>` = `git merge-base origin/<base> <target>`, and use `<mb>..<target>` for every log and diff: a merged-in trunk stays out, and a merged PR (squash included) still shows its own change. When the target is the trunk itself or the range is empty, say there is nothing to explain and stop.

Done when the target ref, the base branch, `<mb>` and the range are named.

## 2. Gather the evidence

- **Intent:** the commit messages (`git log --format='%s%n%b' <mb>..<target>`), the PR title and body.
- **The ticket:** every ticket id (`<KEY>-NNN`, the prefix from `TRACKER_ISSUE_PREFIX` in `.env`) in the branch name, the PR title and the body's Goal line (`pr-description` names an id nowhere else), or, on a body written before it had a Goal line, its closing `Closes` / `Refs` / `Part of` line. Read each with `get_issue` (its Goal, Done when and Design; on an older ticket, Problem and Fix, or What to build and Acceptance criteria) and `list_comments`, where a later decision can supersede the description.
- **Rationale files:** any `docs/adr/` file the range adds or changes, read in full with `git show <target>:<path>`: ADRs state the design decisions most directly.
- **The diff:** `git diff --stat <mb>..<target>`, then every source hunk; test and doc files from their names and the stat, opening a hunk only where it carries the change.
- **Areas:** map every changed path to the most specific `docs/conventions/*.md` whose `paths:` frontmatter covers it (Core is implied for every source file), placing a test under the area it tests; `docs/` paths are "docs", any other uncovered path "other".

Done when every changed path has an area and every ticket id found has been read.

## 3. Write the explanation

Inline in the session; to a file only when the reader asks for one. Open with `## <head branch> -> <base branch>` and a TL;DR of two sentences. Then cover these in the order that reads clearest. Always include Stated intent and Where to start reading; include the others only when they apply:

- **Stated intent:** the ticket's Goal, quoted (with no ticket, the purpose the PR body or the commits state, named as theirs), and how the branch serves it.
- **Areas touched:** each area with its files' role and the `docs/conventions/` doc that governs it.
- **What changed structurally:** seams moved, new modules, call paths that now run differently.
- **Concepts:** terms introduced, changed or removed, named as `docs/glossary.md` names them.
- **Decisions:** each with its source (the Design, an ADR, a commit message).
- **Done items:** where each Done-when item this PR claims lands in the diff. A ticket with no Done-when list takes its items from the Design's requirements, else from its Fix or Acceptance criteria; on a multi-PR ticket, only the items this PR's body claims.
- **Where to start reading:** an ordered list of `path:line` entry points, with lines read from the target ref (`git grep -n <symbol> <target> -- <path>`, dropping its leading `<target>:`), since the checkout may hold another commit.

**Code is the source of truth.** The PR body, the commits and the ticket are context. Where the diff and the stated intent disagree, say so plainly.

Done when the TL;DR and every section that applies are written.
