import { bodyLines, collapsedSections, findingLine } from "@src/review-body.utils";
import type { Finding, ReviewSummary } from "@src/review-metrics.schemas";
import { describe, expect, it } from "vitest";

function finding(overrides: Partial<Finding> = {}): Finding {
  return {
    file: "src/a.ts",
    line: "10",
    area: "correctness",
    confidence: "high",
    tag: "important",
    description: "the description",
    body: null,
    suggestion: null,
    ...overrides,
  };
}

function summary(overrides: Partial<ReviewSummary> = {}): ReviewSummary {
  const spawned: ReviewSummary["reviewers_spawned"] = overrides.reviewers_spawned ?? [
    "correctness",
    "security",
    "conventions",
    "context",
    "maintainability",
    "docs",
  ];
  return {
    reviewers_spawned: spawned,
    reviewers_returned: spawned,
    review_mode: "full",
    incremental_from_sha: null,
    prior_importants: [],
    findings: [],
    refuted_findings: [],
    process_issues: [],
    ...overrides,
  };
}

describe("findingLine", () => {
  it("prefers body over description and cites the location", () => {
    expect(findingLine(finding({ body: "the body" }))).toBe("- `src/a.ts:10` (correctness): the body");
  });

  it("degrades to description and survives a missing location", () => {
    expect(findingLine(finding({ file: null, line: null }))).toBe(
      "- location unrecorded (correctness): the description",
    );
  });
});

describe("collapsedSections", () => {
  it("renders nothing when there is nothing sub-important to show", () => {
    expect(collapsedSections(summary({ findings: [finding()] }))).toEqual([]);
  });

  it("renders nits, pre-existing, and refuted findings in their own blocks", () => {
    const rendered = collapsedSections(
      summary({
        findings: [finding({ tag: "nit", description: "a nit" }), finding({ tag: "pre-existing", description: "old" })],
        refuted_findings: [
          {
            file: "src/a.ts",
            line: "10",
            area: "correctness",
            confidence: "high",
            tag: "important",
            description: "killed",
            refutation: "not reachable",
          },
        ],
      }),
    ).join("\n");
    expect(rendered).toContain("<summary>1 minor issue (not blocking)</summary>");
    expect(rendered).toContain("a nit");
    expect(rendered).toContain("<summary>1 pre-existing issue (predate this diff)</summary>");
    expect(rendered).toContain("<summary>1 finding refuted by validation</summary>");
    expect(rendered).toContain("refuted: not reachable");
  });

  it("pluralizes the section labels", () => {
    const rendered = collapsedSections(summary({ findings: [finding({ tag: "nit" }), finding({ tag: "nit" })] })).join(
      "\n",
    );
    expect(rendered).toContain("2 minor issues (not blocking)");
  });
});

describe("bodyLines", () => {
  it("declares an unreviewed diff when there is no summary", () => {
    const rendered = bodyLines(
      null,
      "Not reviewed: the run left no structured output that matches the schema; re-run",
      [],
      0,
    ).join("\n");
    expect(rendered).toContain("**Not reviewed: the run left no structured output that matches the schema; re-run.**");
    expect(rendered).toContain("Comment `@claude review` to run it again.");
  });

  it("declares an incomplete round, never a clean bill, when a reviewer did not return", () => {
    const rendered = bodyLines(
      summary({
        reviewers_returned: ["correctness"],
        findings: [finding({ tag: "nit" })],
        process_issues: [{ component: "orchestrator", description: "four reviewers were still out" }],
      }),
      "Review incomplete: 1 of 6 reviewers returned; re-run",
      [],
      0,
    ).join("\n");
    expect(rendered).toContain("**Review incomplete: 1 of 6 reviewers returned; re-run.**");
    expect(rendered).toContain("- **orchestrator**: four reviewers were still out");
    expect(rendered).toContain("Comment `@claude review` to run it again.");
    expect(rendered).not.toContain("No issues found");
    expect(rendered).not.toContain("minor issue");
  });

  it("says an incremental round that spawned no reviewer found nothing to review, with the orchestrator's notes", () => {
    const rendered = bodyLines(
      summary({
        reviewers_spawned: [],
        review_mode: "incremental",
        incremental_from_sha: "abcdef0123456789",
        process_issues: [{ component: "orchestrator", description: "the delta is a one-line version bump" }],
      }),
      null,
      [],
      0,
    ).join("\n");
    expect(rendered).toContain("No reviewer was spawned for the changes since abcdef0.");
    expect(rendered).toContain("- **orchestrator**: the delta is a one-line version bump");
    expect(rendered).not.toContain("Not reviewed");
    expect(rendered).not.toContain("to run it again");
  });

  it("keeps the plain clean-bill line when the record is empty", () => {
    expect(bodyLines(summary(), null, [], 0)).toEqual([
      "No issues found. Checked correctness, security, conventions, context, maintainability, docs.",
    ]);
  });

  it("names the skipped areas", () => {
    const rendered = bodyLines(summary({ reviewers_spawned: ["correctness", "docs"] }), null, [], 0).join("\n");
    expect(rendered).toContain("Checked correctness, docs.");
    expect(rendered).toContain("Skipped security, conventions, context, maintainability.");
  });

  it("counts anchored importants and lists unanchored ones", () => {
    const rendered = bodyLines(
      summary({ findings: [finding(), finding()] }),
      null,
      [finding({ file: "gone.ts" })],
      1,
    ).join("\n");
    expect(rendered).toContain("1 important finding on the diff below.");
    expect(rendered).toContain("1 important finding with no line in this diff to anchor to:");
    expect(rendered).toContain("`gone.ts:10`");
  });

  it("headlines no-blocking when only sub-important content exists", () => {
    const rendered = bodyLines(summary({ findings: [finding({ tag: "nit" })] }), null, [], 0).join("\n");
    expect(rendered).toContain("No blocking issues found.");
    expect(rendered).toContain("minor issue");
  });

  it("states the incremental scope", () => {
    const rendered = bodyLines(
      summary({ review_mode: "incremental", incremental_from_sha: "abcdef0123456789" }),
      null,
      [],
      0,
    ).join("\n");
    expect(rendered).toContain("Incremental review of the changes since abcdef0.");
  });
});
