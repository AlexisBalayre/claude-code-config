import { z } from "zod";

/**
 * The single source of truth for the automated-review structured-output contract.
 * `reviewSummarySchema` is emitted as JSON Schema by
 * `print-schema.script.ts` and handed to claude-code-action's `--json-schema` at
 * run time; `metricsRecordSchema` is the normalized, sink-agnostic record that
 * `review-metrics.script.ts` writes from that output plus the run context.
 *
 * By design the summary is not a companion to the PR comments, it is the
 * source they are rendered from: `post-review.script.ts` is the only writer, so
 * anything the author must read has to travel through `findingSchema`.
 */

export const REVIEWER_AREAS = ["correctness", "security", "conventions", "context", "maintainability", "docs"] as const;

export const reviewerAreaSchema = z.enum(REVIEWER_AREAS);

const tagSchema = z.enum(["important", "nit", "pre-existing"]);
const confidenceSchema = z.enum(["high", "medium"]);

const processComponentSchema = z.enum([
  "correctness",
  "security",
  "conventions",
  "context",
  "maintainability",
  "docs",
  "consolidator",
  "validator",
  "orchestrator",
  "platform",
]);

const reviewModeSchema = z.enum(["full", "incremental"]);

const priorImportantSchema = z.object({
  file: z.string().nullable().describe("Repo-relative file path from the prior record."),
  line: z.string().nullable().describe("Line number or range from the prior record."),
  status: z.enum(["resolved", "unresolved"]).describe("Whether the current head resolves the prior finding."),
});

const findingSchema = z.object({
  file: z.string().nullable().describe("Repo-relative file path."),
  line: z.string().nullable().describe("Line number or range."),
  area: reviewerAreaSchema.describe("Reviewer area that raised it."),
  confidence: confidenceSchema,
  tag: tagSchema,
  description: z
    .string()
    .describe(
      "One-line description of the issue. On nit and pre-existing findings this is the line the author reads in the review body's collapsed sections, so write it for a person, not a trend column.",
    ),
  body: z
    .string()
    .nullable()
    .describe(
      "Markdown the PR author reads: the concern, the rule or code it cites, and a link. Required on important findings, which are the only ones posted; null on nit and pre-existing. Falls back to description when null.",
    ),
  suggestion: z
    .string()
    .nullable()
    .describe(
      "Replacement code for the flagged lines, posted as a committable suggestion block. Only for a small self-contained fix that resolves the finding entirely; null otherwise.",
    ),
});

// Refuted findings render from description + refutation in a collapsed section,
// never inline, so the author-facing pair would only ever be null
// on them; omitting it keeps it out of the model's output.
const refutedFindingSchema = findingSchema.omit({ body: true, suggestion: true }).extend({
  refutation: z.string().describe("Why validation refuted the finding, grounded in the code."),
});

const processIssueSchema = z.object({
  component: processComponentSchema.describe(
    "The pipeline part that misbehaved; platform covers failures outside any agent.",
  ),
  description: z.string().describe("What went wrong, with specifics that help debugging."),
});

export const reviewSummarySchema = z.object({
  reviewers_spawned: z.array(reviewerAreaSchema).describe("One entry per instance; an area repeats when several ran."),
  review_mode: reviewModeSchema.describe(
    "full reviews the PR diff; incremental reviews the delta since the prior review.",
  ),
  incremental_from_sha: z
    .string()
    .nullable()
    .describe("The prior review's head SHA the delta ran from; null on a full review."),
  prior_importants: z
    .array(priorImportantSchema)
    .describe(
      "Incremental only: the prior record's important findings checked against the current head. Empty on full.",
    ),
  findings: z.array(findingSchema).describe("The confirmed findings the run acted on."),
  refuted_findings: z
    .array(refutedFindingSchema)
    .describe("Important findings validation refuted and dropped; sub-important findings are not validated."),
  process_issues: z
    .array(processIssueSchema)
    .describe("Problems in the review process itself, not the code. Empty when it ran clean."),
});

const tokensSchema = z.object({
  input: z.number().int().nullable(),
  output: z.number().int().nullable(),
  cache_read: z.number().int().nullable(),
  cache_creation: z.number().int().nullable(),
  total: z.number().int().nullable(),
});

export const metricsRecordSchema = z.object({
  schema_version: z.number().int(),
  timestamp: z.string(),
  repository: z.string().nullable(),
  run_id: z.string().nullable(),
  run_url: z.string().nullable(),
  actor: z.string().nullable(),
  commit_sha: z.string().nullable(),
  source: z.literal("pr"),
  pr_number: z.number().int().nullable(),
  action: z.literal("review"),
  reviewers_spawned: z.array(reviewerAreaSchema),
  reviewers_skipped: z.array(reviewerAreaSchema).describe("Derived: the reviewer areas absent from reviewers_spawned."),
  review_mode: reviewModeSchema,
  incremental_from_sha: z.string().nullable(),
  prior_importants: z.array(priorImportantSchema).nullable(),
  findings: z.array(findingSchema).nullable(),
  refuted_findings: z.array(refutedFindingSchema).nullable(),
  process_issues: z.array(processIssueSchema).nullable(),
  comments_posted_actual: z
    .number()
    .int()
    .nullable()
    .describe("What the poster wrote, counted from GitHub's response to its own call."),
  is_error: z.boolean(),
  cost_usd: z.number().nullable(),
  duration_ms: z.number().int().nullable(),
  num_turns: z.number().int().nullable(),
  tokens: tokensSchema.nullable(),
});

/** The poster's hand-off to the metrics step, written as `posted.json`. */
export const postedRecordSchema = z.object({
  comments_posted: z.number().int(),
});

export type Finding = z.infer<typeof findingSchema>;
export type RefutedFinding = z.infer<typeof refutedFindingSchema>;
export type ReviewSummary = z.infer<typeof reviewSummarySchema>;
export type MetricsRecord = z.infer<typeof metricsRecordSchema>;
export type Tokens = z.infer<typeof tokensSchema>;
