import {
  decideMode,
  isPartialReview,
  newestCleanRecord,
  type PriorRecord,
  parseReviewRequest,
  porcelainPaths,
} from "@src/preflight.utils";
import { describe, expect, it } from "vitest";

function record(overrides: Partial<PriorRecord> = {}): PriorRecord {
  return {
    path: "records/1.json",
    commit_sha: "aaa",
    timestamp: "2026-08-01T00:00:00.000Z",
    is_error: false,
    partial: false,
    ...overrides,
  };
}

describe("porcelainPaths", () => {
  it("extracts the path from a status entry", () => {
    expect(porcelainPaths(" M src/a.ts\n?? src/b.ts")).toEqual(["src/a.ts", "src/b.ts"]);
  });

  it("includes both sides of a rename", () => {
    expect(porcelainPaths("R  old.ts -> new.ts")).toEqual(["old.ts", "new.ts"]);
  });

  it("strips the quoting git applies to special characters", () => {
    expect(porcelainPaths('?? "with space.ts"')).toEqual(["with space.ts"]);
  });

  it("yields nothing for a clean tree", () => {
    expect(porcelainPaths("")).toEqual([]);
  });
});

describe("newestCleanRecord", () => {
  it("picks the newest by timestamp", () => {
    const older = record({ path: "records/1.json", timestamp: "2026-08-01T00:00:00.000Z" });
    const newer = record({ path: "records/2.json", timestamp: "2026-08-02T00:00:00.000Z" });
    expect(newestCleanRecord([older, newer])).toBe(newer);
    expect(newestCleanRecord([newer, older])).toBe(newer);
  });

  it("ignores errored records and records without a sha", () => {
    const errored = record({ timestamp: "2026-08-03T00:00:00.000Z", is_error: true });
    const shaless = record({ timestamp: "2026-08-03T00:00:00.000Z", commit_sha: null });
    const clean = record();
    expect(newestCleanRecord([errored, shaless, clean])).toBe(clean);
  });

  it("never takes a partial review as the prior", () => {
    const partial = record({ timestamp: "2026-08-03T00:00:00.000Z", partial: true });
    const complete = record();
    expect(newestCleanRecord([partial, complete])).toBe(complete);
  });

  it("returns null when nothing usable exists", () => {
    expect(newestCleanRecord([record({ is_error: true })])).toBeNull();
    expect(newestCleanRecord([])).toBeNull();
  });
});

describe("decideMode", () => {
  const base = {
    headSha: "bbb",
    trigger: "synchronize",
    forceFull: false,
    isAncestor: true,
    hasMerges: false,
  };

  it("resolves full when the PR has never been reviewed", () => {
    expect(decideMode({ ...base, prior: null })).toEqual({ kind: "full" });
  });

  it("skips a synchronize retrigger of an already-reviewed head", () => {
    expect(decideMode({ ...base, prior: record({ commit_sha: "bbb" }) })).toEqual({
      kind: "skip-duplicate",
      priorPath: "records/1.json",
    });
  });

  it("re-reviews an already-reviewed head when a human asked by comment", () => {
    expect(decideMode({ ...base, prior: record({ commit_sha: "bbb" }), trigger: "comment" })).toEqual({ kind: "full" });
  });

  it("resolves incremental from the prior head on a clean fast-forward", () => {
    expect(decideMode({ ...base, prior: record() })).toEqual({
      kind: "incremental",
      fromSha: "aaa",
      priorPath: "records/1.json",
    });
  });

  it("honors an explicit full request over an eligible delta", () => {
    expect(decideMode({ ...base, prior: record(), forceFull: true })).toEqual({ kind: "full" });
  });

  it("falls back to full when a rebase broke ancestry", () => {
    expect(decideMode({ ...base, prior: record(), isAncestor: false })).toEqual({ kind: "full" });
  });

  it("falls back to full when the delta contains a merge from the base", () => {
    expect(decideMode({ ...base, prior: record(), hasMerges: true })).toEqual({ kind: "full" });
  });
});

describe("isPartialReview", () => {
  it("counts a quick review as partial", () => {
    expect(isPartialReview("quick", [])).toBe(true);
  });

  it("counts a review restricted to named areas as partial", () => {
    expect(isPartialReview("deep", ["security"])).toBe(true);
  });

  it("counts a standard or deep review of the gated areas as complete", () => {
    expect(isPartialReview("standard", [])).toBe(false);
    expect(isPartialReview("deep", [])).toBe(false);
  });

  it("reads a record from before the request fields as complete", () => {
    expect(isPartialReview(undefined, undefined)).toBe(false);
  });
});

describe("parseReviewRequest", () => {
  it("asks for a standard review of the gated areas by default", () => {
    expect(parseReviewRequest("@claude review")).toEqual({ full: false, depth: "standard", areas: [] });
  });

  it("reads full, a depth, and areas in any order", () => {
    expect(parseReviewRequest("@claude review security deep full docs")).toEqual({
      full: true,
      depth: "deep",
      areas: ["security", "docs"],
    });
  });

  it("ignores case and commas between words", () => {
    expect(parseReviewRequest("@Claude Review Quick Correctness,Security")).toEqual({
      full: false,
      depth: "quick",
      areas: ["correctness", "security"],
    });
  });

  it("stops at the first word that is not a keyword", () => {
    expect(parseReviewRequest("@claude review, thanks for the security fix")).toEqual({
      full: false,
      depth: "standard",
      areas: [],
    });
  });

  it("reads only the trigger's own line", () => {
    expect(parseReviewRequest("@claude review\nsecurity looks fine to me")).toEqual({
      full: false,
      depth: "standard",
      areas: [],
    });
  });
});
