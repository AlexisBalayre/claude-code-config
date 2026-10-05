import type { ReviewSummary } from "./review-metrics.schemas";

/**
 * Whether a round's record covers every reviewer it spawned. A reviewer spawned
 * in the background reports in a later turn, and an orchestrator that ends its
 * turn first hands the `--json-schema` harness a record with no findings in it:
 * rendered as it stands, that is a clean bill for a diff nobody reviewed, and
 * the preflight then takes its head as reviewed.
 */

// Tuned on real records: it matches the orchestrator notes of empty records
// whose reviewers never returned, and of complete reviews recounting an earlier
// premature emission that the final record superseded, and no others.
const UNRETURNED_REVIEWERS =
  /(?:\bnone|\bnever|\bnot|n't)(?:\s+(?:had|have|has|yet|all|any|of them|been))*\s+returned\b|\bbefore\b[^.;:,]{0,60}\breturned\b|\bstill (?:running|in flight|outstanding)\b/i;

/** The verdict the poster posts and the record keeps in place of a review, or null when every reviewer returned. */
export function incompleteReason(summary: ReviewSummary): string | null {
  const spawned = summary.reviewers_spawned.length;
  const returned = summary.reviewers_returned.length;
  if (returned < spawned) return `Review incomplete: ${returned} of ${spawned} reviewers returned; re-run`;
  // The count is the orchestrator's own claim, so on an empty record its
  // admission that reviewers were still out outranks it. A record holding
  // findings proves reports came back, which is what separates a superseded
  // premature emission from a hollow record.
  const empty = summary.findings.length === 0 && summary.refuted_findings.length === 0;
  const admitted = summary.process_issues.some(
    (issue) => issue.component === "orchestrator" && UNRETURNED_REVIEWERS.test(issue.description),
  );
  return empty && admitted ? "Review incomplete: the orchestrator reports reviewers that never returned; re-run" : null;
}

/**
 * Why a round is no verdict on the diff, or null when it is one: a review step
 * that did not succeed, a successful one that left no structured output
 * matching the schema (the action can skip itself and still report success), a
 * full review that spawned no reviewer, or reviewers that never returned. An
 * empty outcome is a local run with no step, not a failure.
 */
export function noVerdictReason(summary: ReviewSummary | null, stepOutcome: string): string | null {
  if (stepOutcome !== "" && stepOutcome !== "success") {
    return `Not reviewed: the review step's outcome was ${stepOutcome}; re-run`;
  }
  if (!summary) return "Not reviewed: the run left no structured output that matches the schema; re-run";
  // An incremental round may rightly find nothing to review in its delta (a
  // version bump, the author's fixes for the prior round). Red there could
  // never clear, because an errored record is never the next round's prior and
  // the same delta comes back.
  if (summary.review_mode === "full" && summary.reviewers_spawned.length === 0) {
    return "Not reviewed: no reviewer was spawned; re-run";
  }
  return incompleteReason(summary);
}
