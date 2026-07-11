---
name: find-dead-code
description: Detect dead-code candidates in the monorepo and hand back a ranked list with per-category verification checklists.
disable-model-invocation: true
---

# Find Dead Code

Surface candidates. Never delete. The user verifies and removes.

## Rules

1. **Detect and report only.** Do not run `rm`, `git rm`, or edit imports as part of this skill.
2. **A candidate is a hypothesis, not a verdict.** Every candidate ships with the verification recipe below; the user runs the final eye.
3. **Scope before running.** Ask the user which area: a single package, a service, or the whole repo. Repo-wide scans are slow and noisy; one subsystem at a time produces actionable lists.
4. **No em-dash in output.** Project convention.

## Categories

Pick the category the user asked about, or run all four if they said "find dead code" with no scope.

### A. Unreferenced files

Files (typically role-suffixed: `*.service.ts`, `*.manager.ts`, `*.adapter.ts`, `*.component.tsx`) that no other file imports.

Detection: for each candidate file `path/to/foo.service.ts`, grep the repo for `from ".*foo.service"`, `from ".*foo\.service"`, and the bare basename `foo.service` as a string. If zero non-self matches across `apps/`, `services/`, `packages/`, flag it.

### B. Unused exports

Named exports that nothing imports. If knip is configured (`knip.json` at the repo root), run `pnpm knip`, adding `--include-entry-exports` to also surface unused exports of entry files; otherwise grep per export. Cross-check survivors by grep: for `export const fooBar`, search `import.*\bfooBar\b` across the repo.

### C. Dead feature flags

Flags in the `feature_flags` table whose `key` is never checked in code.

1. Dump current flag keys via `pnpm --filter @acme/acme-db exec tsx src/seed/dump-feature-flags.script.ts` (or read `packages/acme-db/src/seed/feature-flags.script.ts` directly).
2. For each key, grep for `useFeatureFlag("<key>")`, `hasFeatureFlag("<key>")`, and the bare quoted key string across `apps/` and `services/`.
3. Zero hits = candidate. Also flag flags where the only hit is the seed file itself.

### D. Orphan kebab-role modules across workspaces

A `packages/*` export that no `apps/*` or `services/*` workspace consumes. This is the **Explore-agent blind spot**: package-scoped searches miss the consumer side. Always grep from the repo root, not from inside the package.

For each entry in `packages/<pkg>/src/index.ts`, grep `apps/`, `services/`, and other `packages/` for an `import.*from.*@acme/<pkg>` that pulls that symbol.

## Mandatory verification recipe (per candidate, before flagging)

Run all five. A single hit moves the candidate from "dead" to "live" (or "uncertain").

1. **Repo-wide grep, basename without extension.** `grep -rn "foo.service" apps services packages --include="*.ts" --include="*.tsx"`.
2. **String-literal grep.** The basename or kebab key may appear inside a string: registry YAMLs, factory maps, MCP tool names, route paths, env var names, log tags.
3. **Dynamic imports.** Search `import\(.*foo` and template-literal imports (`import(\`./providers/${name}\`)`). These hide static references.
4. **Registry and factory check.** If the file matches a known registry pattern (`*.provider.ts`, `*.adapter.ts`, `*.client.ts`), open the corresponding factory/registry (e.g. `acme-providers` registry YAML) and look for the name.
5. **Cross-workspace check.** For `packages/*` candidates, grep `services/` and `apps/` separately. For `services/*` candidates, grep `apps/` and other services. Package-scoped Explore is not enough.

## Known false-positive sources

Surface these alongside the candidate list so the user can sanity-check fast.

- **String-keyed registries:** `packages/acme-providers` resolves providers by string key; provider files look unreferenced via `import` grep.
- **Route file conventions:** TanStack Router files under `apps/acme-web/src/routes/` are picked up by the generator, not imported by name. Skip this directory unless the user opts in.
- **Drizzle schema files:** referenced via `packages/acme-db/src/schema/index.ts` re-exports; unique-export grep needs the re-export traversed.
- **Test setup, seed scripts, eval-runner harness:** entry points run by tooling, not imported.
- **Build-time inlined files:** anything referenced from a Vite config, `turbo.json`, or shell scripts under `scripts/`.
- **Feature flags awaiting rollout:** the flag row exists before the code lands. Check the latest 4 weeks of `git log` against the flag key before flagging.

## Output format

Hand back a markdown table per category, ranked by confidence (high = passed all five checks). Example row:

```
| Confidence | Path                                       | Last touched | Why suspect                          | Verification status            |
| ---------- | ------------------------------------------ | ------------ | ------------------------------------ | ------------------------------ |
| High       | packages/acme-domain/src/legacy-format.utils.ts | 9 months ago | 0 imports, 0 string hits, no factory | Passed all 5 checks            |
| Medium     | packages/acme-db/src/schema/old_audit.schemas.ts | 4 months ago | 0 direct imports                    | Re-exported via schema/index.ts; needs human review |
```

End with: "Verify each row before removing. Open a worktree per category; do not bundle removals across categories in a single PR."
