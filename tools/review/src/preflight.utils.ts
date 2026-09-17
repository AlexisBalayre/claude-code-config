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

/** The newest non-errored record, or null when the PR has never been reviewed. */
export function newestCleanRecord(records: PriorRecord[]): PriorRecord | null {
  let newest: PriorRecord | null = null;
  for (const record of records) {
    if (record.is_error || record.commit_sha === null) continue;
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
