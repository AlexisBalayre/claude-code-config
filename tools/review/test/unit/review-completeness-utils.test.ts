import { incompleteReason, noVerdictReason } from "@src/review-completeness.utils";
import type { ReviewSummary } from "@src/review-metrics.schemas";
import { describe, expect, it } from "vitest";

function summary(overrides: Partial<ReviewSummary> = {}): ReviewSummary {
  return {
    reviewers_spawned: ["correctness", "correctness", "conventions"],
    reviewers_returned: ["correctness", "correctness", "conventions"],
    review_mode: "full",
    incremental_from_sha: null,
    prior_importants: [],
    findings: [],
    refuted_findings: [],
    process_issues: [],
    ...overrides,
  };
}

// Verbatim from a real run that was posted as "No issues found".
const HOLLOW_ADMISSION =
  "The harness forced the structured output while all four reviewers (correctness, context, conventions, docs) were still running in the background. None had returned, so this record has no findings.";

describe("incompleteReason", () => {
  it("passes a round every spawned reviewer returned from", () => {
    expect(incompleteReason(summary())).toBeNull();
  });

  it("counts the reviewers that never returned", () => {
    expect(incompleteReason(summary({ reviewers_returned: [] }))).toBe(
      "Review incomplete: 0 of 3 reviewers returned; re-run",
    );
    expect(incompleteReason(summary({ reviewers_returned: ["correctness", "conventions"] }))).toBe(
      "Review incomplete: 2 of 3 reviewers returned; re-run",
    );
  });

  it("believes the orchestrator's admission over a full count", () => {
    const reason = incompleteReason(
      summary({ process_issues: [{ component: "orchestrator", description: HOLLOW_ADMISSION }] }),
    );
    expect(reason).toBe("Review incomplete: the orchestrator reports reviewers that never returned; re-run");
  });

  it.each([
    "The harness demanded structured output before any of the five spawned reviewers returned: two correctness instances (core; tooling), plus maintainability (opus), conventions and docs.",
    "INCOMPLETE REVIEW, NOT A CLEAN ONE: the harness required structured output while all 10 reviewer instances were still running (3 correctness, 2 security, 2 maintainability on opus, conventions, docs, context).",
  ])("reads the other recorded admissions: %s", (description) => {
    expect(incompleteReason(summary({ process_issues: [{ component: "orchestrator", description }] }))).not.toBeNull();
  });

  it.each([
    "Security and context reviewers were deliberately skipped: the change is eval tooling plus three docs, and touches no trust boundary or spec/protocol/infra surface.",
    "review-context was not spawned. The spec, protocol and infra surface of the change is limited to one data-only migration and its journal entry, and correctness was briefed to cover migration safety and the rollout window.",
  ])("ignores orchestrator notes that are not an admission: %s", (description) => {
    expect(incompleteReason(summary({ process_issues: [{ component: "orchestrator", description }] }))).toBeNull();
  });

  it("ignores an admission on a record whose reviewers demonstrably reported", () => {
    // From a complete review that carried findings.
    const superseded =
      "A Stop hook forced the StructuredOutput call before any of the eight spawned reviewers had returned, so an empty, inaccurate record was emitted mid-run (reported zero findings and characterised the PR as unreviewed). All eight reviewers and all four validators subsequently completed normally and this record supersedes that one.";
    const reported = summary({
      process_issues: [{ component: "orchestrator", description: superseded }],
      findings: [
        {
          file: "src/a.ts",
          line: "10",
          area: "correctness",
          confidence: "high",
          tag: "nit",
          description: "a nit",
          body: null,
          suggestion: null,
        },
      ],
    });
    expect(incompleteReason(reported)).toBeNull();
  });

  it("reads only the orchestrator's own notes", () => {
    expect(
      incompleteReason(summary({ process_issues: [{ component: "correctness", description: HOLLOW_ADMISSION }] })),
    ).toBeNull();
  });
});

describe("noVerdictReason", () => {
  it("passes a finished round", () => {
    expect(noVerdictReason(summary(), "success")).toBeNull();
  });

  it("calls a full review that spawned no reviewer not reviewed, though 0 of 0 is no shortfall", () => {
    expect(noVerdictReason(summary({ reviewers_spawned: [], reviewers_returned: [] }), "success")).toBe(
      "Not reviewed: no reviewer was spawned; re-run",
    );
  });

  it("passes an incremental round that spawned no reviewer, whose delta can hold nothing to review", () => {
    const round = summary({
      reviewers_spawned: [],
      reviewers_returned: [],
      review_mode: "incremental",
      incremental_from_sha: "abcdef0123456789",
    });
    expect(noVerdictReason(round, "success")).toBeNull();
  });

  it("names a step that did not succeed, whatever it left behind", () => {
    expect(noVerdictReason(summary(), "cancelled")).toBe(
      "Not reviewed: the review step's outcome was cancelled; re-run",
    );
  });

  it("calls a round with no structured output not reviewed", () => {
    expect(noVerdictReason(null, "success")).toBe(
      "Not reviewed: the run left no structured output that matches the schema; re-run",
    );
  });
});
