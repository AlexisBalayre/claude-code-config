import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

// The first fixture is a real failure: the action skipped itself, the review
// step still reported success, and the round had no structured output at all. The second omits `reviewers_returned`, so it fails
// the schema.
const NO_VERDICT = [
  ["no structured output", ""],
  [
    "a structured output that fails the schema",
    JSON.stringify({
      reviewers_spawned: ["correctness"],
      review_mode: "full",
      incremental_from_sha: null,
      prior_importants: [],
      findings: [],
      refuted_findings: [],
      process_issues: [],
    }),
  ],
] as const;

const VERDICT = JSON.stringify({
  reviewers_spawned: ["correctness"],
  reviewers_returned: ["correctness"],
  review_mode: "full",
  incremental_from_sha: null,
  prior_importants: [],
  findings: [],
  refuted_findings: [],
  process_issues: [],
});

const PACKAGE_DIR = resolve(import.meta.dirname, "../..");

/** Runs a review script as the workflow does, with `gh` answered by a stub that logs each call. */
function runScript(script: string, structuredOutput: string, stepOutcome = "success") {
  const dir = mkdtempSync(join(tmpdir(), "review-tooling-"));
  const ghLog = join(dir, "gh.log");
  writeFileSync(join(dir, "gh"), `#!/bin/sh\ncat >> "${ghLog}"\necho '{}'\n`);
  chmodSync(join(dir, "gh"), 0o755);
  const { GITHUB_STEP_SUMMARY: _summary, ...env } = process.env;
  const result = spawnSync(join(PACKAGE_DIR, "node_modules/.bin/tsx"), [join(PACKAGE_DIR, "src", script)], {
    cwd: PACKAGE_DIR,
    encoding: "utf8",
    env: {
      ...env,
      PATH: `${dir}:${env.PATH ?? ""}`,
      GITHUB_REPOSITORY: "owner/repo",
      GITHUB_RUN_ID: "1",
      REVIEW_PR_NUMBER: "1",
      REVIEW_COMMIT_SHA: "0000000000000000000000000000000000000000",
      REVIEW_STEP_OUTCOME: stepOutcome,
      REVIEW_STRUCTURED_OUTPUT: structuredOutput,
      REVIEW_POSTED_OUTPUT: join(dir, "posted.json"),
      REVIEW_METRICS_OUTPUT: join(dir, "review-metrics.json"),
    },
  });
  const read = (name: string) => {
    try {
      return readFileSync(join(dir, name), "utf8");
    } catch {
      return "";
    }
  };
  return { status: result.status, ghCalls: read("gh.log"), record: read("review-metrics.json") };
}

describe("a review step that succeeds without a verdict", () => {
  it.each(NO_VERDICT)("fails the poster on %s", (_label, structuredOutput) => {
    const { status, ghCalls } = runScript("post-review.script.ts", structuredOutput);
    expect(ghCalls).toContain("Not reviewed: the run left no structured output that matches the schema; re-run.");
    expect(ghCalls).toContain('"state":"failure"');
    expect(status).toBe(1);
  });

  it.each(NO_VERDICT)("records %s as errored, with the reason", (_label, structuredOutput) => {
    const record = JSON.parse(runScript("review-metrics.script.ts", structuredOutput).record);
    expect(record.is_error).toBe(true);
    expect(record.incomplete_reason).toBe(
      "Not reviewed: the run left no structured output that matches the schema; re-run",
    );
  });

  it("gives a failed step that still left a valid output the step's reason", () => {
    const record = JSON.parse(runScript("review-metrics.script.ts", VERDICT, "failure").record);
    expect(record.is_error).toBe(true);
    expect(record.incomplete_reason).toBe("Not reviewed: the review step's outcome was failure; re-run");
  });

  it("leaves a round with a verdict green in both scripts", () => {
    const posted = runScript("post-review.script.ts", VERDICT);
    expect(posted.ghCalls).toContain('"state":"success"');
    expect(posted.status).toBe(0);
    const record = JSON.parse(runScript("review-metrics.script.ts", VERDICT).record);
    expect(record.is_error).toBe(false);
    expect(record.incomplete_reason).toBeNull();
  });
});
