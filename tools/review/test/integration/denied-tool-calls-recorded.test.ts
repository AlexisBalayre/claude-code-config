import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

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

/** Runs the metrics script on an execution file whose result reports `denials`, or no list when undefined. */
function recordFor(denials: object[] | undefined) {
  const dir = mkdtempSync(join(tmpdir(), "review-tooling-"));
  const executionFile = join(dir, "claude-execution-output.json");
  const result = { type: "result", subtype: "success", total_cost_usd: 0.1, permission_denials: denials };
  writeFileSync(executionFile, JSON.stringify([{ type: "system", subtype: "init" }, result]));
  const { GITHUB_STEP_SUMMARY: _summary, ...env } = process.env;
  spawnSync(join(PACKAGE_DIR, "node_modules/.bin/tsx"), [join(PACKAGE_DIR, "src/review-metrics.script.ts")], {
    cwd: PACKAGE_DIR,
    encoding: "utf8",
    env: {
      ...env,
      REVIEW_STEP_OUTCOME: "success",
      REVIEW_STRUCTURED_OUTPUT: VERDICT,
      REVIEW_EXECUTION_FILE: executionFile,
      REVIEW_METRICS_OUTPUT: join(dir, "review-metrics.json"),
    },
  });
  return JSON.parse(readFileSync(join(dir, "review-metrics.json"), "utf8"));
}

describe("a run's denied tool calls", () => {
  it("are recorded by tool, with a Bash call's command as its input", () => {
    const denials = [{ tool_name: "Bash", tool_use_id: "toolu_1", tool_input: { command: "pnpm test" } }];
    expect(recordFor(denials).permission_denials).toEqual([{ tool: "Bash", input: "pnpm test" }]);
  });

  it("keep at most 200 characters of a call's input", () => {
    const denials = [{ tool_name: "Write", tool_use_id: "toolu_1", tool_input: { content: "x".repeat(1000) } }];
    expect(recordFor(denials).permission_denials[0].input).toHaveLength(200);
  });

  it("are recorded as unknown when the result carries no list", () => {
    expect(recordFor(undefined).permission_denials).toBeNull();
  });
});
