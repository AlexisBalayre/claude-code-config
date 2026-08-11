import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import {
  type MetricsRecord,
  metricsRecordSchema,
  postedRecordSchema,
  REVIEWER_AREAS,
  type ReviewSummary,
  reviewSummarySchema,
  type Tokens,
} from "./review-metrics.schemas";

/**
 * Builds the normalized review-metrics record from a CI review run:
 * the model's structured output, the action's execution log, the poster's
 * hand-off, and the GitHub run context. Writes it as a JSON artifact and a
 * job-summary table. Every external input degrades to null on absence rather
 * than failing the step, so a broken run still produces a record that says it
 * broke.
 */

// Bump whenever the record's meaning changes (a field added or repurposed, a
// pipeline change that alters what a field measures): the retro partitions
// trends by schema_version, so a silent semantic change poisons its baselines.
const SCHEMA_VERSION = 1;

function warn(message: string): void {
  process.stdout.write(`::warning::${message}\n`);
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function asInt(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? Math.trunc(value) : null;
}

function asNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function parseSummary(raw: string): ReviewSummary | null {
  if (!raw.trim()) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    warn("structured output was not valid JSON; recording it as absent");
    return null;
  }
  const result = reviewSummarySchema.safeParse(parsed);
  if (!result.success) {
    warn(`structured output did not match the schema: ${result.error.message}`);
    return null;
  }
  return result.data;
}

interface PlatformMetrics {
  is_error: boolean;
  cost_usd: number | null;
  duration_ms: number | null;
  num_turns: number | null;
  tokens: Tokens | null;
}

const EMPTY_PLATFORM: PlatformMetrics = {
  is_error: false,
  cost_usd: null,
  duration_ms: null,
  num_turns: null,
  tokens: null,
};

function findResultEntry(parsed: unknown): Record<string, unknown> | null {
  const entries = Array.isArray(parsed) ? parsed : [parsed];
  for (let i = entries.length - 1; i >= 0; i--) {
    const entry = asRecord(entries[i]);
    if (entry && (entry.type === "result" || "total_cost_usd" in entry)) {
      return entry;
    }
  }
  return null;
}

function tokensFromUsage(usage: Record<string, unknown> | null): Tokens | null {
  if (!usage) return null;
  const input = asInt(usage.input_tokens);
  const output = asInt(usage.output_tokens);
  const cacheRead = asInt(usage.cache_read_input_tokens);
  const cacheCreation = asInt(usage.cache_creation_input_tokens);
  const parts = [input, output, cacheRead, cacheCreation].filter((n): n is number => n !== null);
  return {
    input,
    output,
    cache_read: cacheRead,
    cache_creation: cacheCreation,
    total: parts.length ? parts.reduce((a, b) => a + b, 0) : null,
  };
}

function platformMetrics(executionFile: string | undefined): PlatformMetrics {
  if (!executionFile || !existsSync(executionFile)) {
    warn("execution file not found; platform metrics unknown");
    return EMPTY_PLATFORM;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(executionFile, "utf8"));
  } catch {
    warn("execution file was not valid JSON; platform metrics unknown");
    return EMPTY_PLATFORM;
  }
  const result = findResultEntry(parsed);
  if (!result) {
    warn("no result entry in execution file; platform metrics unknown");
    return EMPTY_PLATFORM;
  }
  return {
    is_error: result.is_error === true || result.subtype === "error",
    cost_usd: asNumber(result.total_cost_usd),
    duration_ms: asInt(result.duration_ms),
    num_turns: asInt(result.num_turns),
    tokens: tokensFromUsage(asRecord(result.usage)),
  };
}

/**
 * What the poster wrote this round, from its own hand-off rather than from a
 * query over the PR's comments. The poster is the only writer and
 * counts GitHub's response to its own call, so there is no longer a self-report
 * to distrust nor a window of foreign comments to filter out.
 */
function postedCount(path: string | undefined): number | null {
  if (!path || !existsSync(path)) {
    warn("no posted.json; comment count unknown");
    return null;
  }
  try {
    const result = postedRecordSchema.safeParse(JSON.parse(readFileSync(path, "utf8")));
    if (result.success) return result.data.comments_posted;
    warn(`posted.json did not match the schema: ${result.error.message}`);
  } catch {
    warn("posted.json was not valid JSON");
  }
  return null;
}

function reviewersSkipped(spawned: ReviewSummary["reviewers_spawned"]) {
  const seen = new Set(spawned);
  return REVIEWER_AREAS.filter((area) => !seen.has(area));
}

function summaryTable(record: MetricsRecord): string {
  const findings = record.findings ?? [];
  const byArea = new Map<string, number>();
  for (const f of findings) byArea.set(f.area, (byArea.get(f.area) ?? 0) + 1);
  const areas =
    [...byArea.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([area, count]) => `${area}: ${count}`)
      .join(", ") || "-";
  const posted = record.comments_posted_actual === null ? "-" : String(record.comments_posted_actual);
  const mode =
    record.review_mode === "incremental"
      ? `incremental (from ${record.incremental_from_sha?.slice(0, 7) ?? "?"})`
      : record.review_mode;
  const rows: [string, string | number][] = [
    ["Mode", mode],
    ["Findings", findings.length],
    ["Refuted (validation)", (record.refuted_findings ?? []).length],
    ["Process issues", (record.process_issues ?? []).length],
    ["By area", areas],
    ["Comments posted", posted],
    ["Cost (USD est.)", record.cost_usd === null ? "-" : record.cost_usd.toFixed(4)],
    ["Tokens (total)", record.tokens?.total ?? "-"],
    ["Turns", record.num_turns ?? "-"],
    ["Errored", String(record.is_error)],
  ];
  return [
    "## Review metrics",
    "",
    "| Metric | Value |",
    "| --- | --- |",
    ...rows.map(([label, value]) => `| ${label} | ${value} |`),
    "",
  ].join("\n");
}

function main(): void {
  const env = process.env;
  const repository = env.GITHUB_REPOSITORY ?? null;
  const runId = env.GITHUB_RUN_ID ?? null;
  const prRaw = env.REVIEW_PR_NUMBER ?? "";
  const prNumber = /^\d+$/.test(prRaw) ? Number.parseInt(prRaw, 10) : null;
  const commitSha = env.REVIEW_COMMIT_SHA ?? env.GITHUB_SHA ?? null;

  const summary = parseSummary(env.REVIEW_STRUCTURED_OUTPUT ?? "");
  const platform = platformMetrics(env.REVIEW_EXECUTION_FILE);

  // A failed action step often produces no execution log, so its result cannot
  // report the error; without the step outcome the record would read clean. Fold
  // it in so a failed run is never recorded as a successful one. An empty outcome
  // (a local run with no step) is treated as not failed.
  const stepOutcome = env.REVIEW_STEP_OUTCOME ?? "";
  const runFailed = stepOutcome !== "" && stepOutcome !== "success";
  if (runFailed && !platform.is_error) {
    warn(`review step outcome was ${stepOutcome}; marking the record errored`);
  }

  const record: MetricsRecord = {
    schema_version: SCHEMA_VERSION,
    timestamp: new Date().toISOString(),
    repository,
    run_id: runId,
    run_url: repository && runId ? `https://github.com/${repository}/actions/runs/${runId}` : null,
    actor: env.GITHUB_ACTOR ?? null,
    commit_sha: commitSha,
    source: "pr",
    pr_number: prNumber,
    action: "review",
    reviewers_spawned: summary?.reviewers_spawned ?? [],
    reviewers_skipped: summary ? reviewersSkipped(summary.reviewers_spawned) : [],
    // A summary-less (errored) run defaults to "full": mode is unknowable without
    // the orchestrator's report, and full is the population errored runs belong in.
    review_mode: summary?.review_mode ?? "full",
    incremental_from_sha: summary?.incremental_from_sha ?? null,
    prior_importants: summary?.prior_importants ?? null,
    findings: summary?.findings ?? null,
    refuted_findings: summary?.refuted_findings ?? null,
    process_issues: summary?.process_issues ?? null,
    comments_posted_actual: postedCount(env.REVIEW_POSTED_INPUT),
    is_error: platform.is_error || runFailed,
    cost_usd: platform.cost_usd,
    duration_ms: platform.duration_ms,
    num_turns: platform.num_turns,
    tokens: platform.tokens,
  };

  const validated = metricsRecordSchema.parse(record);
  const serialized = `${JSON.stringify(validated, null, 2)}\n`;

  const output = env.REVIEW_METRICS_OUTPUT ?? "review-metrics.json";
  writeFileSync(output, serialized);
  process.stdout.write(serialized);

  if (env.GITHUB_STEP_SUMMARY) {
    appendFileSync(env.GITHUB_STEP_SUMMARY, summaryTable(validated));
  }
}

main();
