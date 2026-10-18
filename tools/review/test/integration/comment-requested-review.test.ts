import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const PACKAGE_DIR = resolve(import.meta.dirname, "../..");

function verdict(findings: object[] = []): string {
  return JSON.stringify({
    reviewers_spawned: ["correctness"],
    reviewers_returned: ["correctness"],
    review_mode: "full",
    incremental_from_sha: null,
    prior_importants: [],
    findings,
    refuted_findings: [],
    process_issues: [],
  });
}

// No file, so the poster lists it in the review body without diffing anything.
const IMPORTANT = {
  file: null,
  line: null,
  area: "correctness",
  confidence: "high",
  tag: "important",
  description: "the description",
  body: null,
  suggestion: null,
};

/** Runs a review script as the workflow does, with `gh` stubbed to log each call; returns what it handed on. */
function runScript(script: string, env: Record<string, string>) {
  const dir = mkdtempSync(join(tmpdir(), "review-tooling-"));
  const ghLog = join(dir, "gh.log");
  writeFileSync(join(dir, "gh"), `#!/bin/sh\ncat >> "${ghLog}"\necho '{}'\n`);
  chmodSync(join(dir, "gh"), 0o755);
  const { GITHUB_STEP_SUMMARY: _summary, ...inherited } = process.env;
  spawnSync(join(PACKAGE_DIR, "node_modules/.bin/tsx"), [join(PACKAGE_DIR, "src", script)], {
    cwd: PACKAGE_DIR,
    encoding: "utf8",
    env: {
      ...inherited,
      PATH: `${dir}:${inherited.PATH ?? ""}`,
      GITHUB_REPOSITORY: "owner/repo",
      GITHUB_RUN_ID: "1",
      GITHUB_OUTPUT: join(dir, "output.txt"),
      REVIEW_PR_NUMBER: "1",
      REVIEW_COMMIT_SHA: "0000000000000000000000000000000000000000",
      REVIEW_STEP_OUTCOME: "success",
      REVIEW_POSTED_OUTPUT: join(dir, "posted.json"),
      REVIEW_METRICS_OUTPUT: join(dir, "review-metrics.json"),
      ...env,
    },
  });
  const read = (name: string) => {
    try {
      return readFileSync(join(dir, name), "utf8");
    } catch {
      return "";
    }
  };
  return { output: read("output.txt"), record: read("review-metrics.json"), ghCalls: read("gh.log") };
}

describe("the review a `@claude review` comment asked for", () => {
  it("is recorded with the run", () => {
    const { record } = runScript("review-metrics.script.ts", {
      REVIEW_STRUCTURED_OUTPUT: verdict(),
      REVIEW_COMMENT: "@claude review quick security docs",
    });
    expect(JSON.parse(record)).toMatchObject({ review_depth: "quick", review_areas: ["security", "docs"] });
  });

  it("records as a standard review of the gated areas when no comment asked", () => {
    const { record } = runScript("review-metrics.script.ts", { REVIEW_STRUCTURED_OUTPUT: verdict() });
    expect(JSON.parse(record)).toMatchObject({ review_depth: "standard", review_areas: [] });
  });
});

describe("the reaction the poster hands to the comment", () => {
  it.each([
    ["no important finding", verdict(), "false", "reaction=rocket"],
    ["no important finding on a partial pass", verdict(), "true", "reaction=+1"],
    ["an important finding", verdict([IMPORTANT]), "false", "reaction=-1"],
    ["no verdict at all", "", "false", "reaction=confused"],
  ])("follows a round with %s", (_label, structuredOutput, partial, expected) => {
    const { output } = runScript("post-review.script.ts", {
      REVIEW_STRUCTURED_OUTPUT: structuredOutput,
      REVIEW_PARTIAL: partial,
    });
    expect(output).toContain(expected);
  });
});

describe("a partial pass's commit status", () => {
  it("is left to the full review when the pass reaches a verdict", () => {
    const { ghCalls } = runScript("post-review.script.ts", {
      REVIEW_STRUCTURED_OUTPUT: verdict(),
      REVIEW_PARTIAL: "true",
    });
    expect(ghCalls).not.toContain('"context":"claude-review"');
  });

  it("is left to the full review when the pass reaches no verdict", () => {
    const { ghCalls } = runScript("post-review.script.ts", {
      REVIEW_STRUCTURED_OUTPUT: "",
      REVIEW_PARTIAL: "true",
    });
    expect(ghCalls).not.toContain('"context":"claude-review"');
  });
});
