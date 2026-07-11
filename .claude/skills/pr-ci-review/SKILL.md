---
name: pr-ci-review
disable-model-invocation: true
allowed-tools: Bash(git *), Bash(gh *), Bash(pnpm *), Bash(turbo *), Read, Edit, Write, Grep, Glob, Agent, mcp__github_inline_comment__create_inline_comment
description: Cost-optimal multi-agent code review of local changes or a PR, across six relevance-gated, model-tiered areas, with a deterministic pre-flight, record-all reporting, and high-signal posting.
argument-hint: "[pr [<number>]] [--fix | --comment]"
---

# Code Review Protocol

Provide a context-aware code review of local changes or a pull request, across six reviewer areas, and emit a structured record of everything it considered. You orchestrate: you steer, gate, consolidate, and act. You do NOT review code yourself; the area subagents do.

The governing rule is **spend only where it buys recall**. Deterministic tooling and the human in the loop are free, so lean on them first and spawn an agent only when judgment is the thing missing. Recall comes from the deterministic suite plus the correctness and security reviewers; almost everything else is a cut.

> **Setup:** the CI paths (`--json-schema`, the `review-metrics` record) assume your own CI
> wiring — a workflow that invokes `/pr-ci-review` on PRs and appends each run's record to a
> `ci/review-metrics` orphan branch. This template ships no workflow; without one, the local
> `report` / `--fix` and manual `pr --comment` paths work unchanged.

## Review Constraints

- **Grounded, not speculative:** every finding is grounded in the actual changed code. No speculation; a false positive erodes trust.
- **Tag by severity, not by whether to post:** tag each finding `important` (fix before merge), `nit` (real but minor), or `pre-existing` (predates this diff).
- **Record-all, post-high-signal:** the structured record captures every confirmed finding (`important`, `nit`, and `pre-existing`) plus the refuted ones; a PR receives inline comments for `important` findings only. `nit` and `pre-existing` are recorded, never posted.
- **Leave linters alone:** do not raise anything Biome or tsc already catches; that is noise, not a finding.

## 1. Parse arguments -> source + action

- **source = pr** when the invocation carries a `pr` token, a prose `Pull request: #N` reference, or a `--json-schema`. The CI harness invokes `/pr-ci-review` with the PR named in prose and a schema, so that path resolves here. `pr 123` or `#123` targets that PR; a bare `pr` targets the current branch's PR (`gh pr view`). A `--json-schema` or `--comment` makes **action = comment**; otherwise **action = report**.
- **source = local** only when none of the above is present: the working tree is the subject. Default **action = report**; `--fix` -> **action = fix**.
- Reject invalid combinations: `--fix` with `pr`, or `--comment` with local. State the error and stop.

Define **the change** once and reuse it everywhere below:

- **local**: the working-tree diff against the merge-base with `main` (committed + staged + unstaged).
- **pr `<n>`**: the diff of PR #`<n>`.

## 2. Gate the run before spending anything

- **PR freshness** (pr source): before spawning, assert the working tree *is* the PR's current head: `git rev-parse HEAD` must equal `gh pr view <n> --json headRefOid -q .headRefOid`. If they differ, the tree is stale (CI checks out the head, but GitHub can serve a lagging `refs/pull/N/merge`): re-checkout the head with `gh pr checkout <n>`, or stop and say so. A stale tree produces findings that are false on the real head and silently skips the surface the head added. Also assert the tree is **clean** (`git status --porcelain` empty): the SHA match alone passes even when uncommitted edits sit on top of the right HEAD, and those edits make `Read` serve content that disagrees with `gh pr diff`, so reviewers cite code that is not in the PR. If it is dirty, restore it (`git checkout -- .` / `git reset --hard HEAD`) or stop.
- **Pre-flight** (local source): run `turbo run lint typecheck test` **scoped to the affected packages** (a `--filter` per touched workspace; the full suite is slow and cascade-cancels). If it fails, **stop**: report the failures and ask the user to return once the tree is green. Reviewers assume the deterministic layer is clean; spawning them on a red tree pays opus to rediscover what tooling already flagged. Integration tests are infra-gated and stay CI's job. For a pr source running under CI, skip the pre-flight: the build workflow already gates the tree.

## 3. Steer (yourself, no agent)

Gate and brief on the change's *shape* and *intent*, both free to read. Do not spawn a helper for this.

- **Shape**: the changed files, their line counts, and the workspaces they span (`apps` / `services` / `packages`), from `git diff --stat` against the merge-base (local) or `gh pr diff <n>` plus `gh pr view <n> --json files` (pr).
- **Intent**: read it from the commit messages and, for a PR, the title and body. The author already wrote the intent; do not pay an agent to re-derive it from the diff. Only if the messages are junk (`wip`, `fix`) do you skim the diff yourself.
- **Steering brief** (pr): the unresolved comment threads (each with its concern) and what the author declared intentional or out-of-scope.

Do not discover `CLAUDE.md` files: there is one (root), always in context, and per-area `docs/conventions/*` rules auto-load via `.claude/rules` when a reviewer reads a file in that area.

## 4. Gate which reviewers to spawn

Spawn a reviewer (via the **Agent** tool, by its `subagent_type`) **only when its area is actually touched**. An untouched area is the cheapest cut there is. The roster is fixed:

| Reviewer (`subagent_type`) | Default model | Spawn when |
| :--- | :--- | :--- |
| `review-correctness` | opus | code with real logic changed; owns behavioral regressions on the surface tests do not cover |
| `review-security` | opus | a trust boundary is touched (auth, routes, WS handlers, input handling, secrets, config) |
| `review-context` | sonnet | spec/protocol/infra surfaces touched (CORS, HTTP, docker, CI, settings) |
| `review-conventions` | sonnet | almost always: `docs/conventions/*`, `docs/adr/*`, and `CLAUDE.md` govern the rules |
| `review-maintainability` | sonnet | code with real logic changed |
| `review-docs` | sonnet | the change adds or edits prose (docstrings, comments, markdown, copy) |

There is no `requirements` reviewer: this setup deliberately keeps the issue tracker out of the review. If you skip a reviewer that might apply, note it in the final output so coverage is transparent.

**Model dials (asymmetric by stakes).** `correctness` and `security` always run on opus, never downgraded: a miss there is expensive. `maintainability` runs on sonnet for a normal diff and on **opus** once the diff is large (override the model at spawn time). `conventions` and `docs` run on sonnet.

**Instance scaling.** Default every area to **one** instance. Scale only `correctness` / `security` / `maintainability`, and only past a size gate: **> ~400 changed lines OR > ~8 files spanning >= 2 of `apps`/`services`/`packages`** -> 2 instances; a markedly larger diff -> 3; **hard cap 3**. Send each instance in a complementary direction (by subsystem, layer, or risk concentration), never overlapping. Other areas stay single-instance. This is parallel breadth, not repeated passes: review the changed surface well once, do not loop.

## 5. Brief and spawn in parallel

Each subagent's manifest defines its expertise; your brief supplies everything else, in your own words. A good brief gives the reviewer:

- The change and the author's intent, plus the definition of **the change** so it can fetch its own scoped slice.
- Its assignment: the part of the change it owns, and, where you split an area, the specific direction.
- The steering context that concerns it: unresolved threads (do not re-raise their concerns; look harder where they point) and what the author declared intentional or out-of-scope.
- The contract every finding meets: `{file, line (or range), area, confidence (high/medium), tag, description}` with the quote or citation its focus requires.
- An instruction to surface, separately, any impediment that degraded its review. No impediments is the normal case.
- **Under CI (a schema was requested), the review is static**: the checkout has no `node_modules`, and test/build spikes, dependency installs, `git fetch`, and shell redirection to temp files are all off-limits even where the Bash tool itself would accept more than `gh`. Tell the reviewer to confirm from `gh pr diff` and the checked-out tree by static reasoning; a claim about a dependency's internals rests on pinned-version knowledge and caps at `medium` confidence. None of this is an impediment worth logging: it is the CI path's normal shape.
- A reminder that its final message *is* the deliverable for the next stage, not a human-facing report.

## 6. Consolidate (yourself, inline)

You hold every finding and have the only cross-area view, so dedup inline; do not spawn a separate consolidator.

- **Deduplicate across areas.** The same root issue often surfaces from several reviewers (a low-value comment trips `conventions`, `docs`, and `maintainability` at once). Merge into one finding, keeping the clearest description and the most severe tag.
- **Assign one area, citation-first.** A real defect is `correctness` or `security`. Anything citing a written `CLAUDE.md` / `docs/conventions` / ADR rule is `conventions`. Remaining judgment calls are `docs` or `maintainability`.
- **Respect declared scope.** Drop anything the author marked intentional or out-of-scope.
- **Doubt resolves to keep.** A verified mechanism with no currently-reachable trigger stays as a `nit`; only drop a finding whose mechanism is unproven.
- **The tag must match the body.** A finding whose own description concedes there is no reachable break ("no contract violation", "no practical consequence", a provably-equivalent behavior) consolidates as a `nit` regardless of the tag the reviewer chose. Do not forward a self-contradicting `important` to validation; the downgrade is yours to make.

## 7. Validate only where the machine acts on the finding

Validation buys precision, never recall, so it is a cost tax justified only where a false positive does damage you cannot catch yourself. Spawn `review-validator` instances in parallel, tiered (opus for `correctness`/`security` findings, sonnet for the rest). A confirmed finding (with any location correction) survives; a refuted one is dropped from the action and kept aside with its refutation reason.

- **action = report**: spawn **no** validator. The reviewers self-validated, and you are about to show every finding to a human who is the final judge.
- **action = fix**: validate the **`important`** findings only, before editing. `nit` findings are never auto-fixed.
- **action = comment, or a `--json-schema` was requested (CI)**: validate **all** consolidated findings. This is what populates `refuted_findings` in the record (the reviewers' false-positive signal for the retro), so it is not optional on the CI path.

## 8. Act on the result

**report** (default): present each finding in the session: its tag, file and line, the issue and why it was flagged, a suggested fix. Important findings first. If empty, state which areas were reviewed and that nothing was found.

**fix** (local, `--fix`): apply each confirmed finding.

- **Scope discipline**: each fix targets only the flagged issue. Do not refactor adjacent code, touch unrelated docstrings, or remove ticket TODOs.
- **Post-verify**: after editing, re-run `turbo run typecheck test` on the affected packages to prove no regression was introduced. Report the result.
- Summarize what changed (file, line, fix). List anything you noticed but did not touch under "Tangential (not applied)" and ask before editing those.

**comment** (pr, `--comment`): post the confirmed `important` findings as inline comments.

- **If nothing is posted** (also when the review recorded only sub-`important` findings), comment via `gh pr comment`. Keep the marker line and the `No issues found.` sentence verbatim: review-metrics excludes this comment from its posted count by matching them, and a reworded summary such as "No blocking issues found" reads as a phantom posted comment. Only the area list may adapt to what was reviewed:
  > ## Claude code review
  >
  > <!-- pr-ci-review:nothing-posted -->
  >
  > No issues found. Checked correctness, security, conventions, context, maintainability, and docs.
- **If important findings exist:**
  1. Build a private list of unique comments to prevent duplicates.
  2. Post inline comments via `mcp__github_inline_comment__create_inline_comment`.
  3. **Anchor outside the diff:** an inline comment attaches only to a line inside the PR's diff hunks. When an `important` finding's true line is outside them (a context line, or an unchanged file), anchor to the nearest changed line in the same file and name the true location in the body. When the finding's file has **no** changed line at all (a stale reference or dead link in a file the PR never touches), no inline anchor exists: it **must** go into a single consolidated `gh pr comment` (naming each finding's file, line, and concern), never be dropped. A finding silently lost this way is a false negative on an `important` finding, the worst outcome the review can produce.
  4. **Format:** brief description + cite/link to the specific rule or the underlying concern.
  5. **Suggestions:** committable blocks only for small, self-contained fixes that resolve the issue entirely.
  6. **Links:** `<https://github.com/OWNER/REPO/blob/FULL_SHA/path/to/file#L[start]-L[end]>` with the full git SHA.

Impediments surfaced by subagents never appear in the report, the fixes, or the PR comments.

## 9. Structured summary (when a schema is requested)

If the harness runs this skill with a `--json-schema`, your **final message** must be the object that schema validates, in addition to the actions above. Report what the review actually did, not a target:

- `reviewers_spawned`: one entry per reviewer instance that ran, by area (an area repeats when several instances ran).
- `findings`: the confirmed findings, each `{file, line, area, confidence, tag, description}` (the `important` ones you posted and the `nit` ones you recorded but did not post).
- `refuted_findings`: the Step-7 refutations, each finding plus its `refutation`.
- `comments_posted`: how many inline comments you actually posted (ground-truthed against GitHub, not self-reported).
- `process_issues`: anything that degraded the review itself, each `{component, description}`, where `component` is the stage (`orchestrator`, `validator`, an area, or `platform`). Empty when the run was clean. These never appear in the PR comments.

## Todo List

- [ ] Parse arguments into source + action; reject invalid combinations.
- [ ] Gate the run: PR freshness (pr) or pre-flight `turbo` suite (local).
- [ ] Steer: read the change shape and intent yourself.
- [ ] Gate and spawn only the touched areas, tiered and instance-capped.
- [ ] Consolidate inline: dedup and assign one area citation-first.
- [ ] Validate per action (none for report; important for fix; all for comment/CI).
- [ ] Act: report, fix (with post-verify), or comment.
- [ ] Emit the structured summary when a schema is requested.
