import { type Finding, REVIEWER_AREAS, type RefutedFinding, type ReviewSummary } from "./review-metrics.schemas";

/**
 * Renders the review body the poster submits. Importants anchor
 * inline on the diff; everything else the run confirmed, and everything
 * validation refuted, renders here in collapsed sections, so severity decides
 * how loudly a finding is displayed, never whether it is displayed at all.
 */

type Listed = Pick<Finding, "file" | "line" | "area" | "description"> & { body?: string | null };

export function findingLine(finding: Listed): string {
  const location = finding.file
    ? `\`${finding.file}${finding.line ? `:${finding.line}` : ""}\``
    : "location unrecorded";
  return `- ${location} (${finding.area}): ${finding.body ?? finding.description}`;
}

function refutedLine(finding: RefutedFinding): string {
  return `${findingLine(finding)}\n  - refuted: ${finding.refutation}`;
}

function detailsBlock(label: string, lines: string[]): string[] {
  return ["", "<details>", `<summary>${label}</summary>`, "", ...lines, "", "</details>"];
}

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

export function collapsedSections(summary: ReviewSummary): string[] {
  const nits = summary.findings.filter((finding) => finding.tag === "nit");
  const preExisting = summary.findings.filter((finding) => finding.tag === "pre-existing");
  const sections: string[] = [];
  if (nits.length > 0) {
    // description, not body: sub-important findings carry no body by contract.
    sections.push(...detailsBlock(`${plural(nits.length, "minor issue")} (not blocking)`, nits.map(findingLine)));
  }
  if (preExisting.length > 0) {
    sections.push(
      ...detailsBlock(
        `${plural(preExisting.length, "pre-existing issue")} (predate this diff)`,
        preExisting.map(findingLine),
      ),
    );
  }
  if (summary.refuted_findings.length > 0) {
    // Displayed so a validator that over-killed a real finding is catchable by
    // the one reader who can tell: nothing else surfaces that failure mode.
    sections.push(
      ...detailsBlock(
        `${plural(summary.refuted_findings.length, "finding")} refuted by validation`,
        summary.refuted_findings.map(refutedLine),
      ),
    );
  }
  return sections;
}

export function bodyLines(summary: ReviewSummary | null, unanchored: Finding[], anchoredCount: number): string[] {
  if (!summary || summary.reviewers_spawned.length === 0) {
    const blockers = (summary?.process_issues ?? []).map((issue) => `- **${issue.component}**: ${issue.description}`);
    return [
      "**This PR has not been reviewed.** The run ended before any reviewer reported, so nothing in this diff has been checked.",
      ...(blockers.length ? ["", "What stopped it:", "", ...blockers] : []),
      "",
      "Comment `@claude review` to run it again.",
    ];
  }

  const spawned = [...new Set(summary.reviewers_spawned)];
  const skipped = REVIEWER_AREAS.filter((area) => !spawned.includes(area));
  const coverage = `Checked ${spawned.join(", ")}.${skipped.length ? ` Skipped ${skipped.join(", ")}.` : ""}`;
  const scope =
    summary.review_mode === "incremental" && summary.incremental_from_sha
      ? `Incremental review of the changes since ${summary.incremental_from_sha.slice(0, 7)}. `
      : "";

  const sections = collapsedSections(summary);
  if (anchoredCount === 0 && unanchored.length === 0 && sections.length === 0) {
    return [`No issues found. ${scope}${coverage}`];
  }

  const lines: string[] = [];
  if (anchoredCount > 0) {
    lines.push(`${plural(anchoredCount, "important finding")} on the diff below.`);
  }
  if (unanchored.length > 0) {
    if (lines.length > 0) lines.push("");
    lines.push(
      `${plural(unanchored.length, "important finding")} with no line in this diff to anchor to:`,
      "",
      ...unanchored.map(findingLine),
    );
  }
  if (lines.length === 0) lines.push("No blocking issues found.");
  return [...lines, ...sections, "", `${scope}${coverage}`];
}
