import { REVIEW_DEPTHS, REVIEWER_AREAS, type ReviewDepth, type ReviewerArea } from "./review-metrics.schemas";

/**
 * Pure logic behind the review preflight. Head freshness,
 * tree cleanliness, and the full/incremental mode choice are arithmetic over
 * git facts, so they run as a deterministic CI step before the model starts
 * instead of as model turns that could stall on a permission prompt and exit
 * green over an unreviewed diff.
 */

/** A review-metrics record as the preflight reads it off `ci/review-metrics`. */
export interface PriorRecord {
  /** Branch-relative path, e.g. `records/123.json`; what the model `git show`s. */
  path: string;
  commit_sha: string | null;
  timestamp: string;
  is_error: boolean;
  /** A quick or area-restricted review, which left the rest of its head unreviewed. */
  partial: boolean;
}

/**
 * What a `@claude review` comment asks for: the words after the trigger on its
 * line, in any order, up to the first that is not a keyword, so prose after
 * the trigger (`@claude review, thanks!`) never reads as part of the request.
 * `areas` is empty when the comment named none, leaving the orchestrator to
 * gate them.
 */
export function parseReviewRequest(comment: string): { full: boolean; depth: ReviewDepth; areas: ReviewerArea[] } {
  const words =
    /@claude review\b(.*)/i
      .exec(comment)?.[1]
      ?.toLowerCase()
      .split(/[\s,]+/)
      .filter(Boolean) ?? [];
  let full = false;
  let depth: ReviewDepth = "standard";
  const areas = new Set<ReviewerArea>();
  for (const word of words) {
    const named = REVIEW_DEPTHS.find((candidate) => candidate === word);
    const area = REVIEWER_AREAS.find((candidate) => candidate === word);
    if (word === "full") full = true;
    else if (named) depth = named;
    else if (area) areas.add(area);
    else break;
  }
  return { full, depth, areas: [...areas] };
}

/** Working-tree paths from `git status --porcelain`, rename targets included. */
export function porcelainPaths(porcelain: string): string[] {
  const paths: string[] = [];
  for (const line of porcelain.split("\n")) {
    if (line.length < 4) continue;
    for (const path of line.slice(3).split(" -> ")) {
      paths.push(path.replace(/^"|"$/g, ""));
    }
  }
  return paths;
}

/**
 * The newest non-errored, complete record, or null when the PR has never had
 * one. A partial review is never the prior: an incremental round diffed from
 * its head would leave the areas it skipped unreviewed on everything before it.
 */
export function newestCleanRecord(records: PriorRecord[]): PriorRecord | null {
  let newest: PriorRecord | null = null;
  for (const record of records) {
    if (record.is_error || record.partial || record.commit_sha === null) continue;
    if (newest === null || record.timestamp > newest.timestamp) newest = record;
  }
  return newest;
}

export type ModeDecision =
  | { kind: "skip-duplicate"; priorPath: string }
  | { kind: "full" }
  | { kind: "incremental"; fromSha: string; priorPath: string };

export function decideMode(args: {
  prior: PriorRecord | null;
  headSha: string;
  trigger: string;
  forceFull: boolean;
  isAncestor: boolean;
  hasMerges: boolean;
}): ModeDecision {
  const { prior, headSha, trigger, forceFull, isAncestor, hasMerges } = args;
  if (prior === null || prior.commit_sha === null) return { kind: "full" };
  // An already-reviewed head re-reviews only on an explicit human request: a
  // synchronize or opened retrigger of the same SHA has nothing new to look at,
  // while a comment is a person asking again on purpose.
  if (prior.commit_sha === headSha) {
    return trigger === "comment" ? { kind: "full" } : { kind: "skip-duplicate", priorPath: prior.path };
  }
  if (forceFull) return { kind: "full" };
  // A rebase or force-push breaks ancestry, and a merge from the base folds
  // upstream code into the delta; neither leaves a delta that is this PR's own
  // change to review.
  if (!isAncestor || hasMerges) return { kind: "full" };
  return { kind: "incremental", fromSha: prior.commit_sha, priorPath: prior.path };
}
