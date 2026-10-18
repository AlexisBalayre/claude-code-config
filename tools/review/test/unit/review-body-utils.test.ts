import { bodyLines, collapsedSections, commentBody, findingLine } from "@src/review-body.utils";
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
    "spec",
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

describe("commentBody", () => {
  it("keeps the suggestion fence it opens committable", () => {
    expect(commentBody(finding({ suggestion: "fixed();" }), null)).toBe(
      "the description\n\n```suggestion\nfixed();\n```",
    );
  });

  it("makes a tag in a relocated finding's path inert", () => {
    expect(commentBody(finding({ file: "a`<pre lang=suggestion>`.ts" }), 12)).toBe(
      "Re: `a`<\u200bpre lang=suggestion>`.ts:12`\n\nthe description",
    );
  });

  it("offers a relocated finding's suggestion in an inert fence", () => {
    expect(commentBody(finding({ suggestion: "fixed();" }), 12)).toBe(
      "Re: `src/a.ts:12`\n\nthe description\n\n```\nfixed();\n```",
    );
  });

  it.each([
    ["a backtick fence", "```suggestion\nevil();\n```", "```text suggestion\nevil();\n```"],
    ["a tilde fence", "~~~suggestion\nevil();\n~~~", "~~~text suggestion\nevil();\n~~~"],
    ["a quoted fence", "> ```suggestion\n> evil();", "> ```text suggestion\n> evil();"],
    ["a raw tag", '<pre lang="suggestion">evil();</pre>', '<​pre lang="suggestion">evil();</pre>'],
  ])("keeps %s in a relocated suggestion inert after a body that leaves a fence open", (_label, suggestion, inert) => {
    const body = commentBody(finding({ body: "See below\n````", suggestion }), 12);
    expect(body).toContain(`\n${inert}\n`);
  });

  it("posts a suggestion holding backtick runs verbatim, in a fence they cannot close", () => {
    const suggestion = 'const fence = "```";\n```\n```suggestion\nevil();';
    expect(commentBody(finding({ suggestion }), null)).toBe(
      `the description\n\n\`\`\`\`suggestion\n${suggestion}\n\`\`\`\``,
    );
  });

  it.each([
    ["a bare fence", "```suggestion\nevil();\n```", "```text suggestion\nevil();\n```"],
    [
      "a tilde fence with a line range",
      "~~~~ Suggestion:-0+1\nevil();\n~~~~",
      "~~~~text  Suggestion:-0+1\nevil();\n~~~~",
    ],
    ["an entity in the language", "```&#115;uggestion\nevil();\n```", "```text &#115;uggestion\nevil();\n```"],
    ["a form feed before the language", "```\fsuggestion\nevil();\n```", "```text \fsuggestion\nevil();\n```"],
    ["a quoted fence", "> ```suggestion\n> evil();\n> ```", "> ```text suggestion\n> evil();\n> ```"],
    [
      "a fence in nested containers",
      "> - 1. ```suggestion\n>      evil();",
      "> - 1. ```text suggestion\n>      evil();",
    ],
    ["a fence after a footnote label", "[^1]: ```suggestion\n    evil();", "[^1]: ```text suggestion\n    evil();"],
    ["a fence after a lone carriage return", "a\r```suggestion\revil();\r```", "a\r```text suggestion\revil();\r```"],
    [
      "a language that is not a plain token",
      "```ts title=x\nconst a = 1;\n```",
      "```text ts title=x\nconst a = 1;\n```",
    ],
    [
      "a tilde fence with a backtick in its info",
      "~~~suggestion `x`\nevil();\n~~~",
      "~~~text suggestion `x`\nevil();\n~~~",
    ],
    ["a raw pre block", '<pre lang="suggestion">evil();</pre>', '<\u200bpre lang="suggestion">evil();</pre>'],
    ["an upper-case tag", "<PRE LANG=suggestion>", "<\u200bPRE LANG=suggestion>"],
    [
      "a real tag behind a tag-like code span",
      '`<pre a="`\n<pre lang="suggestion">evil();</pre>',
      '`<\u200bpre a="`\n<\u200bpre lang="suggestion">evil();</pre>',
    ],
    [
      "a tag continued on a quoted line",
      '> <pre\n> lang="suggestion">evil();</pre>',
      '> <\u200bpre\n> lang="suggestion">evil();</pre>',
    ],
    [
      "a tag with a no-break space before a quote",
      '<pre a=\u00a0"x lang=suggestion ">evil();</pre>',
      '<\u200bpre a=\u00a0"x lang=suggestion ">evil();</pre>',
    ],
  ])("makes %s quoted in a finding inert", (_label, quoted, inert) => {
    expect(commentBody(finding({ body: quoted }), null)).toBe(inert);
  });

  // 28 labels: milliseconds when the label matches one way, seconds when it backtracks.
  it("reads a line of footnote labels in linear time", () => {
    const labels = `${"[^a]:".repeat(28)}x`;
    const started = performance.now();
    commentBody(finding({ body: labels }), null);
    expect(performance.now() - started).toBeLessThan(100);
  });

  it("makes a fence quoted in the description inert when there is no body", () => {
    expect(commentBody(finding({ description: "```suggestion\nevil();\n```" }), null)).toBe(
      "```text suggestion\nevil();\n```",
    );
  });

  it.each([
    ["a fence with a plain language", "```ts\nconst a = 1;\n```"],
    ["a fence named in inline code", "use a ```` ```suggestion ```` fence here"],
    ["a mid-line backtick run", "Wrap the fix in a ```ts block before posting"],
    ["quoted code holding a fence", '```ts\nconst fence = "```";\n```'],
    [
      "a permalink, generics and other tags",
      "See <https://github.com/o/r/blob/abc/a.ts#L1-L2>: `Array<string>`, a < b, <details>, <prefix>, <!-- x -->, </pre>.",
    ],
    ["a lang= outside a pre block", 'const lang = detect(text);\nrender(lang=code);\nif (lang === "suggestion") {}'],
    ["a footnote label that never closes", "[^a\\]: ```suggestion\n    evil();"],
    ["inline code opening with a backtick run", "```foo``` is wrong, and ```` ```suggestion ```` is what they wrote"],
  ])("leaves %s as written", (_label, text) => {
    expect(commentBody(finding({ body: text }), null)).toBe(text);
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
      "No issues found. Checked correctness, security, conventions, context, maintainability, docs, spec.",
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
