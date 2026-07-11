---
name: review-retro
description: Mine past automated-review runs for recurring process and judgment failures, then propose fixes to the review setup as one evidence-cited PR. Use when running the review retrospective or tuning the PR reviewer.
allowed-tools: Bash(git *), Bash(gh *), Read, Edit, Write, Glob, Grep
model: opus
disable-model-invocation: true
---

# Review retro

You run the retrospective for the automated review setup (the learn stage of the review pipeline). The review pipeline reviews code; you review the pipeline. Your deliverable is not findings about code: it is improvements to the setup itself, grounded in past runs.

**Dormant until there is history.** The retro needs a window of real review runs and the human outcomes on them. Until the merged producer has reviewed a stretch of PRs, there is nothing to mine: say so and stop rather than inventing patterns from a handful of runs.

> **Setup:** the `ci/review-metrics` branch is produced by your own CI wiring (a workflow that
> runs `/pr-ci-review` with a `--json-schema` and appends each run's record); this template
> ships no workflow. Without it, the retro degrades gracefully — it reports there is no history
> to mine and stops.

## Evidence

Three sources, all reachable through `gh` or `git`:

- **Run records.** Every CI review appends its `review-metrics` record to the `ci/review-metrics` orphan branch as `records/<run_id>.json` (read via `git fetch origin ci/review-metrics`; the branch holds the full history, so no artifact downloads are needed). Each record carries `process_issues` (self-reported process failures), `refuted_findings` with reasons (the validator's false-positive catches), the reviewer spawn shape (`reviewers_spawned` / `reviewers_skipped`), `comments_posted_claimed` vs `comments_posted_actual` (drift between them is its own signal), cost, and tokens. Not every record is a review: a run can fail before producing output (`is_error: true`, null findings). Tell those apart from a clean review that genuinely found nothing.
- **PR thread outcomes.** For each PR those runs reviewed, what humans did with the posted comments. A finding whose code was fixed or whose suggestion was applied earned its place; one dismissed, resolved with pushback, or retracted as a conceded challenge (via `address-review-comments`) is a false positive that survived validation.
- **Human-found misses.** On those same PRs, the review comments humans wrote that the automated review never raised. Each is an issue the pipeline could have caught and did not. Count a miss only against a PR the pipeline actually reviewed; a comment on a PR whose review never ran is an outage, not a coverage gap.

**Window**: from the last retro PR (title contains `[review-retro]`) included, to now.

**Memory**: the prior retro PRs are your ledger. A merged one is done; a closed-unmerged one was rejected, so do not re-propose its change unless the new window adds materially different evidence.

## Judgment

You are looking for patterns, not incidents; "recurring" is your call, with no numeric threshold. Wherever the evidence shows the pipeline misfiring — in its process, its judgment, or its coverage — trace the failure to the mechanism in the setup that produced it and propose the fix there: a reviewer brief or area in `pr-ci-review`, the consolidator's citation-first assignment, the validator, the workflow, or the schema. Never blunt a reviewer wholesale to make the numbers look better, and the review's philosophy (record-all / post-high-signal, precision over speculation, small inconsistencies compound) is not yours to rewrite.

## Act

Walk the user through what the evidence shows and what you propose to change, and let them steer before you write any diff. For what survives the discussion: branch off `main`, apply the changes, and open a PR (via the `pr-description` skill) with a title of the form `[review-retro] <short summary>`, whose description cites the evidence per change. Wait for the user's sign-off. If nothing recurring emerged, say so and stop: open no PR in that case.
