import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { commentableLines, resolveAnchor, trueLine } from "./diff-anchor.utils";
import { bodyLines } from "./review-body.utils";
import { type Finding, postedRecordSchema, type ReviewSummary, reviewSummarySchema } from "./review-metrics.schemas";

/**
 * Writes the round's verdict to the pull request. By design this is the
 * review's only writer: the model reports findings and never touches GitHub, so
 * a run that halts at a gate, loses a command to a permission prompt, or dies
 * mid-flight cannot leave an untouched PR that reads as a clean review (a
 * failure mode this design exists to prevent). Every round leaves
 * one review: the important findings anchored to the code, a clean bill, or the
 * reason there is neither. It also pins a `claude-review` commit status to the
 * head SHA, so an unreviewed diff is visible on the PR's check list
 * rather than only in the Actions tab.
 */

const REVIEW_HEADING = "## Claude code review";

function warn(message: string): void {
  process.stdout.write(`::warning::${message}\n`);
}

function gh(args: string[], input?: string): string {
  return execFileSync("gh", args, { encoding: "utf8", input });
}

function parseSummary(raw: string): ReviewSummary | null {
  if (!raw.trim()) return null;
  try {
    const result = reviewSummarySchema.safeParse(JSON.parse(raw));
    if (result.success) return result.data;
    warn(`structured output did not match the schema: ${result.error.message}`);
  } catch {
    warn("structured output was not valid JSON");
  }
  return null;
}

interface AnchoredComment {
  path: string;
  line: number;
  side: "RIGHT";
  body: string;
  finding: Finding;
}

function fileDiff(mergeBase: string, file: string): string {
  try {
    // `:/` anchors the pathspec at the repo root: `pnpm --filter` runs this from
    // the package directory, where a plain repo-relative path would match nothing
    // and quietly send every finding to the summary instead of the diff.
    // Two-dot from the resolved merge base, never three-dot from `origin/main`,
    // which the action's depth-1 base fetch grafts parentless.
    return execFileSync("git", ["diff", `${mergeBase}..HEAD`, "--unified=0", "--", `:/${file}`], { encoding: "utf8" });
  } catch {
    warn(`could not diff ${file}; its findings go in the review body`);
    return "";
  }
}

function commentBody(finding: Finding, relocatedFrom: number | null): string {
  const parts: string[] = [];
  if (relocatedFrom !== null) parts.push(`Re: \`${finding.file}:${relocatedFrom}\``, "");
  parts.push(finding.body ?? finding.description);
  if (finding.suggestion !== null) {
    // A committable suggestion rewrites the line it is attached to, so on a
    // relocated anchor it would replace code the finding never mentioned. Same
    // text, inert fence.
    parts.push("", relocatedFrom === null ? "```suggestion" : "```", finding.suggestion, "```");
  }
  return parts.join("\n");
}

function prepare(summary: ReviewSummary, mergeBase: string) {
  const anchored: AnchoredComment[] = [];
  const unanchored: Finding[] = [];
  for (const finding of summary.findings.filter((candidate) => candidate.tag === "important")) {
    const target = trueLine(finding.line);
    const anchor =
      finding.file && target !== null
        ? resolveAnchor(commentableLines(fileDiff(mergeBase, finding.file)), target)
        : null;
    if (finding.file === null || target === null || anchor === null) {
      unanchored.push(finding);
      continue;
    }
    anchored.push({
      path: finding.file,
      line: anchor,
      side: "RIGHT",
      body: commentBody(finding, anchor === target ? null : target),
      finding,
    });
  }
  return { anchored, unanchored };
}

function postReview(repository: string, prNumber: number, body: string, comments: AnchoredComment[]): void {
  gh(
    ["api", "--method", "POST", `repos/${repository}/pulls/${prNumber}/reviews`, "--input", "-"],
    JSON.stringify({
      event: "COMMENT",
      body,
      // Omitted rather than sent empty: a clean review is the commonest round, and
      // if the endpoint ever objected to a bare `comments: []` it would redden
      // every one of them.
      ...(comments.length > 0
        ? {
            comments: comments.map(({ path, line, side, body: text }) => ({ path, line, side, body: text })),
          }
        : {}),
    }),
  );
}

/**
 * The observable outcome: a status on the head SHA that a
 * cancelled, skipped, or dead run leaves red, visible on the PR itself. Not a
 * required check, so it never blocks a merge; its own failure only warns,
 * because the review comment is the primary contract and this the signal.
 */
function setCommitStatus(repository: string, sha: string, state: "success" | "failure", description: string): void {
  const runUrl = `https://github.com/${repository}/actions/runs/${process.env.GITHUB_RUN_ID ?? ""}`;
  try {
    gh(
      ["api", "--method", "POST", `repos/${repository}/statuses/${sha}`, "--input", "-"],
      JSON.stringify({ state, context: "claude-review", description, target_url: runUrl }),
    );
  } catch {
    warn("could not set the claude-review commit status");
  }
}

function postEach(repository: string, prNumber: number, commitSha: string, comments: AnchoredComment[]) {
  const rejected: Finding[] = [];
  let accepted = 0;
  for (const comment of comments) {
    try {
      gh(
        ["api", "--method", "POST", `repos/${repository}/pulls/${prNumber}/comments`, "--input", "-"],
        JSON.stringify({
          commit_id: commitSha,
          path: comment.path,
          line: comment.line,
          side: comment.side,
          body: comment.body,
        }),
      );
      accepted++;
    } catch {
      rejected.push(comment.finding);
    }
  }
  return { accepted, rejected };
}

function main(): void {
  const env = process.env;
  const repository = env.GITHUB_REPOSITORY ?? "";
  const runId = env.GITHUB_RUN_ID ?? "";
  const prRaw = env.REVIEW_PR_NUMBER ?? "";
  if (!repository || !runId || !/^\d+$/.test(prRaw)) {
    warn("repository, run id or PR number missing; cannot post the review");
    return;
  }
  const prNumber = Number.parseInt(prRaw, 10);

  // A failed action step often produces no structured output at all, but it can
  // also produce a stale or partial one; either way the run did not finish, so
  // its output is not a verdict to post.
  const stepOutcome = env.REVIEW_STEP_OUTCOME ?? "";
  const summary =
    stepOutcome === "" || stepOutcome === "success" ? parseSummary(env.REVIEW_STRUCTURED_OUTPUT ?? "") : null;
  const { anchored, unanchored } = summary
    ? prepare(summary, env.REVIEW_MERGE_BASE ?? "")
    : { anchored: [], unanchored: [] };

  const runUrl = `https://github.com/${repository}/actions/runs/${runId}`;
  const body = (findings: Finding[], anchoredCount: number) =>
    [REVIEW_HEADING, "", ...bodyLines(summary, findings, anchoredCount), "", `<sub>[review run](${runUrl})</sub>`].join(
      "\n",
    );

  let posted = 0;
  let reviewPosted = false;
  try {
    try {
      postReview(repository, prNumber, body(unanchored, anchored.length), anchored);
      posted = anchored.length;
    } catch {
      // The reviews endpoint is all-or-nothing on positions, so one anchor GitHub
      // disagrees with (the head moved after our checkout) would cost every
      // comment. Individually, only the bad one is lost, and it lands in the body.
      warn("batch review post failed; falling back to individual comments");
      const { accepted, rejected } = postEach(repository, prNumber, env.REVIEW_COMMIT_SHA ?? "", anchored);
      posted = accepted;
      postReview(repository, prNumber, body([...unanchored, ...rejected], accepted), []);
    }
    reviewPosted = true;
    process.stdout.write(`posted a review on #${prNumber} with ${posted} inline comment(s)\n`);
  } finally {
    const sha = env.REVIEW_COMMIT_SHA ?? "";
    if (sha) {
      const importants = anchored.length + unanchored.length;
      if (!summary) setCommitStatus(repository, sha, "failure", "the diff was not reviewed");
      else if (!reviewPosted) setCommitStatus(repository, sha, "failure", "the review could not be posted");
      else {
        setCommitStatus(
          repository,
          sha,
          "success",
          importants > 0 ? `${importants} important finding(s)` : "no blocking issues",
        );
      }
    } else {
      warn("no commit sha; skipping the claude-review status");
    }
    const output = env.REVIEW_POSTED_OUTPUT ?? "posted.json";
    writeFileSync(output, `${JSON.stringify(postedRecordSchema.parse({ comments_posted: posted }), null, 2)}\n`);
  }
}

// ESM-native `require.main === module`; keeps unit-test imports side-effect free.
if (import.meta.url === `file://${process.argv[1]}`) {
  main();
}
