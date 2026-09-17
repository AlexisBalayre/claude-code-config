import { commentableLines, resolveAnchor, trueLine } from "@src/diff-anchor.utils";
import { describe, expect, it } from "vitest";

describe("commentableLines", () => {
  it("expands a hunk header into its right-side range", () => {
    expect(commentableLines("@@ -12,7 +14,3 @@ function foo() {")).toEqual([14, 15, 16]);
  });

  it("treats an omitted count as a single line", () => {
    expect(commentableLines("@@ -12 +14 @@")).toEqual([14]);
  });

  it("collects every hunk in the file, in order", () => {
    const diff = [
      "diff --git a/src/a.ts b/src/a.ts",
      "index 1234567..89abcde 100644",
      "--- a/src/a.ts",
      "+++ b/src/a.ts",
      "@@ -3,0 +4,2 @@",
      "+added",
      "+added",
      "@@ -20,1 +22,1 @@",
      "-old",
      "+new",
    ].join("\n");
    expect(commentableLines(diff)).toEqual([4, 5, 22]);
  });

  it("yields nothing for a pure deletion, which has no right side", () => {
    expect(commentableLines("@@ -5,3 +4,0 @@")).toEqual([]);
  });

  it("yields nothing when the diff is empty", () => {
    expect(commentableLines("")).toEqual([]);
  });

  it("ignores a body line that merely looks like a header", () => {
    expect(commentableLines('+const marker = "@@ -1,1 +99,1 @@";')).toEqual([]);
  });
});

describe("resolveAnchor", () => {
  it("returns the target itself when it is inside a hunk", () => {
    expect(resolveAnchor([14, 15, 16, 22], 15)).toBe(15);
  });

  it("pulls a target above every hunk down to the first line", () => {
    expect(resolveAnchor([14, 15, 16], 3)).toBe(14);
  });

  it("pulls a target below every hunk up to the last line", () => {
    expect(resolveAnchor([14, 15, 16], 900)).toBe(16);
  });

  it("picks the nearest line across a gap between hunks", () => {
    expect(resolveAnchor([10, 40], 33)).toBe(40);
  });

  it("breaks an exact tie towards the earlier line", () => {
    expect(resolveAnchor([10, 20], 15)).toBe(10);
  });

  it("returns null when the file has no changed line to anchor to", () => {
    expect(resolveAnchor([], 42)).toBeNull();
  });
});

describe("trueLine", () => {
  it("reads a plain line number", () => {
    expect(trueLine("105")).toBe(105);
  });

  it("anchors a range at its start", () => {
    expect(trueLine("105-107")).toBe(105);
  });

  it("tolerates surrounding whitespace", () => {
    expect(trueLine("  105 ")).toBe(105);
  });

  it("returns null for an absent or non-numeric line", () => {
    expect(trueLine(null)).toBeNull();
    expect(trueLine("n/a")).toBeNull();
  });
});
