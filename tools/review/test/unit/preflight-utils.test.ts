import { decideMode, newestCleanRecord, type PriorRecord, porcelainPaths } from "@src/preflight.utils";
import { describe, expect, it } from "vitest";

function record(overrides: Partial<PriorRecord> = {}): PriorRecord {
  return {
    path: "records/1.json",
    commit_sha: "aaa",
    timestamp: "2026-08-01T00:00:00.000Z",
    is_error: false,
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
