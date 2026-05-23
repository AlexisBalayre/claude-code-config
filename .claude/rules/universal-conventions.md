---
paths:
  - "**/*.ts"
  - "**/*.tsx"
---

# Universal TypeScript Conventions — Quick Reference

**CRITICAL:** These rules apply to EVERY TypeScript file in the project. See `@docs/conventions/general.md` for the full doc.

## File Naming: `kebab-case.role.ts`

| DO | DON'T |
|----|-------|
| `my-service.service.ts` | `MyService.service.ts` (PascalCase) |
| `audio-frame.types.ts` | `audioFrame.types.ts` (camelCase) |
| `session.adapter.ts` | `session-adapter.ts` (missing role) |
| `index.ts`, `env.ts`, `main.ts` | Exception files — no role needed |

**Valid roles (25).** The role suffix locks in a calling convention, not just a label. Pick the role that matches what the file *does*. Full taxonomy + per-role conventions in `@docs/conventions/naming.md`.

| Group              | Roles                                                                                              |
| :----------------- | :------------------------------------------------------------------------------------------------- |
| Behavioral (11)    | `service`, `repository`, `serializer`, `middleware`, `routes`, `manager`, `factory`, `client`, `adapter`, `registry`, `config` |
| Declarations (6)   | `types`, `interface`, `schemas`, `enums`, `constants`, `errors`                                    |
| Frontend (3)       | `component`, `hook`, `store`                                                                       |
| Tests + data (3)   | `test`, `integration.test`, `mock`                                                                 |
| Constrained (1)    | `utils` (pure functions only; cannot import `service`/`manager`/`repository`/`client`)             |
| Entrypoint (1)     | `script` (runnable `main()` block, invoked via `tsx`/`node` from `package.json` or shell)          |

**Plurality is locked.** Plural: `types`/`schemas`/`enums`/`constants`/`errors`/`utils`. Singular: `interface`/`hook`. Inverses are banned.

**Banned roles** are rejected by `.claude/hooks/validate-file-naming.sh`. If the validator complains, the rename target is in `@docs/conventions/naming.md` ("Banned roles" table). Common ones: `helper`→`utils`/`mock`, `builder`→`factory`, `controller`→`service`/`routes`, `monitor`/`pool`→`manager`, `router`→`routes`, `handler`→`routes`/`service`, domain verbs (`notify`/`dispatch`/…)→role matching the calling convention.

## Exports & Imports

- **Named exports ONLY** — NEVER `export default`.
- **Extensionless imports** in source (`.js` added at build time by a post-build step).
- Prefer namespace imports for services: `import * as SessionService from './session.service'`.

## Type Separation

Types/interfaces live in dedicated `types/` or `interfaces/` folders. NEVER inline in service or route files. Re-export from the consumer to keep call sites stable.

## Type Safety

- No `any` outside system boundaries (with a justification comment).
- Branded IDs (`SessionId`, `OrganizationId`, …) from `packages/acme-domain/src/types/branded.types.ts`. Cast raw strings only at boundaries.
- Boolean prefixes: `is/has/should/can`. Generics: descriptive (`TConfig`, `TResponse`), never bare `T`/`U`.
- `override` keyword required on every subclass method. Factories for stateless logic; classes only for stateful resources with a lifecycle.

## No Hardcoded Values

Pull from YAML config or `env.ts`. Never inline ports, URLs, sample rates, model names, timeouts.

## JSDoc & Comments

- **`packages/*`:** JSDoc required on every exported symbol (imperative-verb summary, ≤120 chars). Cross-workspace consumers see it in IDE hover.
- **`apps/*` / `services/*`:** JSDoc only when the WHY is non-obvious (invariants, units, ordering, gotchas).
- Inline `//` comments explain WHY, never WHAT. If removing the comment wouldn't confuse a reader, delete it. No `// === Section ===`, no journal/changelog comments, no commented-out code.

## Clean Code

- DELETE old code when replacing — no shims, no "removed" markers, no backwards-compat re-exports.
- YAGNI: don't design for hypothetical requirements.
