# Review tooling

The deterministic half of the CI review pipeline. The model half lives in
[`.claude/skills/pr-ci-review`](../../.claude/skills/pr-ci-review/SKILL.md) (orchestrator) and
the `.claude/agents/review-*.md` manifests (reviewers + validator); the wiring is
[`.github/workflows/claude-code-review.yml`](../../.github/workflows/claude-code-review.yml).

## Design

**The model never writes to the PR.** It emits a structured record (schema generated from
[`src/review-metrics.schemas.ts`](src/review-metrics.schemas.ts), the single source of truth),
and the poster renders that record: `important` findings as inline comments anchored to the
diff, everything else (nits, pre-existing, refuted findings) in collapsed sections of one
review body, plus a `claude-review` commit status on the head SHA. Splitting the writer out of
the model buys three guarantees a model-posted review cannot make:

- **A dead run is visible.** The poster runs on `always()`: a run that halted at a permission
  prompt or died mid-flight still leaves a "this PR has not been reviewed" verdict and a red
  status, instead of the green silence that reads as a clean review.
- **Injection has no channel.** The model's tool allowlist is read-only; a PR whose diff tells
  the reviewer to post something has nothing to post with.
- **Anchoring is arithmetic, not judgment.** [`src/diff-anchor.utils.ts`](src/diff-anchor.utils.ts)
  maps each finding onto a line GitHub will accept, relocating out-of-diff findings to the
  nearest changed line with a `Re: file:line` prefix (suggestions turn inert on relocation).

## The scripts

| Script | Workflow step | What it does |
| :----- | :------------ | :----------- |
| `review-preflight.script.ts` | before the model | Asserts the tree is the PR's clean current head, resolves the merge base, and decides **full vs. incremental** against the prior record on `ci/review-metrics` (same-SHA retriggers skip; rebases, force-pushes, and merge-bearing deltas resolve to full). |
| `print-schema.script.ts` | before the model | Emits the structured-output contract as JSON Schema for `--json-schema`, generated from the zod source so the contract cannot drift. |
| `post-review.script.ts` | after the model, `always()` | The review's only writer: posts one review with anchored inline comments (falling back to per-comment posting if the batch 422s), and pins the `claude-review` commit status. |
| `review-metrics.script.ts` | after the poster, `always()` | Builds the normalized run record (findings, refutations, mode, cost, tokens, `is_error`) from the model output + execution log + poster hand-off; the workflow's second job appends it to the `ci/review-metrics` orphan branch for `/review-retro`. |

## Threat model, in short

CI review executes an LLM on untrusted PR content, so the defenses do not rely on the model
behaving: claude-code-action restores startup-read config (`.claude/`, `CLAUDE.md`,
`.mcp.json`, …) from the base branch before the model starts (the workflow extends this to
`AGENTS.md`), preserving the PR's copies unexecuted under `.claude-pr/` for reviewers to read;
the allowlist has no write verbs and no `git checkout`/`git fetch` (which could revert that
restore); the metrics branch is written by a separate job so `contents: write` never coexists
with model-driven steps; and third-party actions are SHA-pinned because the workflow carries
secrets and a write token.

## Using it in your repo

Follow the setup comment at the top of the workflow: add the OAuth token secret and create the
`ci/review-metrics` orphan branch. The package is deliberately
standalone (no workspace, no shared config) so it drops into any repo layout and any stack: the
workflow installs Node and pnpm for this tool alone, whatever the host project builds with. If
the host repo is itself a pnpm workspace, you can fold it in and switch the workflow's `--dir`
invocations to `--filter`.

Verify locally with `pnpm --dir tools/review test` and `pnpm --dir tools/review typecheck`.
