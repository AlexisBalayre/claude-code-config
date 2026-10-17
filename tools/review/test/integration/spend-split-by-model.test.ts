import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const VERDICT = JSON.stringify({
  reviewers_spawned: ["conventions"],
  reviewers_returned: ["conventions"],
  review_mode: "full",
  incremental_from_sha: null,
  prior_importants: [],
  findings: [],
  refuted_findings: [],
  process_issues: [],
});

// The shape claude-code-action writes to its execution file: the raw result
// message, whose `usage` covers the orchestrator's own turns only while
// `total_cost_usd` and `modelUsage` cover every subagent too.
const RESULT = {
  type: "result",
  subtype: "success",
  is_error: false,
  duration_ms: 44969,
  num_turns: 6,
  total_cost_usd: 0.45,
  usage: { input_tokens: 8, output_tokens: 2900, cache_read_input_tokens: 104026, cache_creation_input_tokens: 38025 },
  modelUsage: {
    "claude-opus-5-5": {
      inputTokens: 8,
      outputTokens: 2900,
      cacheReadInputTokens: 104026,
      cacheCreationInputTokens: 38025,
      webSearchRequests: 0,
      costUSD: 0.3,
      contextWindow: 1000000,
      maxOutputTokens: 128000,
    },
    "claude-sonnet-5-5": {
      inputTokens: 12,
      outputTokens: 4100,
      cacheReadInputTokens: 210000,
      cacheCreationInputTokens: 30000,
      webSearchRequests: 0,
      costUSD: 0.15,
      contextWindow: 1000000,
      maxOutputTokens: 128000,
    },
  },
};

const PACKAGE_DIR = resolve(import.meta.dirname, "../..");

function runMetrics(result: object) {
  const dir = mkdtempSync(join(tmpdir(), "review-tooling-"));
  const executionFile = join(dir, "claude-execution-output.json");
  writeFileSync(executionFile, JSON.stringify([{ type: "system", subtype: "init" }, result]));
  spawnSync(join(PACKAGE_DIR, "node_modules/.bin/tsx"), [join(PACKAGE_DIR, "src/review-metrics.script.ts")], {
    cwd: PACKAGE_DIR,
    encoding: "utf8",
    env: {
      ...process.env,
      REVIEW_STEP_OUTCOME: "success",
      REVIEW_STRUCTURED_OUTPUT: VERDICT,
      REVIEW_EXECUTION_FILE: executionFile,
      REVIEW_METRICS_OUTPUT: join(dir, "review-metrics.json"),
      GITHUB_STEP_SUMMARY: join(dir, "summary.md"),
    },
  });
  return {
    record: JSON.parse(readFileSync(join(dir, "review-metrics.json"), "utf8")),
    summary: readFileSync(join(dir, "summary.md"), "utf8"),
  };
}

describe("a run's spend", () => {
  it("is split by model across the orchestrator and its subagents", () => {
    expect(runMetrics(RESULT).record.model_usage).toEqual([
      {
        model: "claude-opus-5-5",
        input: 8,
        output: 2900,
        cache_read: 104026,
        cache_creation: 38025,
        cost_usd: 0.3,
      },
      {
        model: "claude-sonnet-5-5",
        input: 12,
        output: 4100,
        cache_read: 210000,
        cache_creation: 30000,
        cost_usd: 0.15,
      },
    ]);
  });

  it("keeps the orchestrator-only tokens apart from the split", () => {
    expect(runMetrics(RESULT).record.tokens.total).toBe(8 + 2900 + 104026 + 38025);
  });

  it("shows each model's cost and the orchestrator's tokens in the job summary", () => {
    const { summary } = runMetrics(RESULT);
    expect(summary).toContain("| Cost: claude-opus-5-5 | 0.3000 |");
    expect(summary).toContain("| Cost: claude-sonnet-5-5 | 0.1500 |");
    expect(summary).toContain("| Tokens (orchestrator) | 144959 |");
  });

  it("records no split when the result carries none", () => {
    const { modelUsage: _modelUsage, ...withoutSplit } = RESULT;
    expect(runMetrics(withoutSplit).record.model_usage).toBeNull();
  });
});
