# Core Conventions

Universal rules for every TypeScript file. Area docs ([`backend.md`](backend.md), [`services.md`](services.md), [`frontend.md`](frontend.md), [`testing.md`](testing.md)) inherit from here; they never relax these rules.

**Genre contract:** obligations only (rules, gotchas, decision rules, naming). System description lives in `docs/reference/`.

## Altitude / YAGNI

Build the smallest thing that meets the requirement. Prefer a deep module (simple surface, logic hidden) over several shallow ones; a module too thin to justify its file gets folded into its caller (see the decision-tree fallback in Naming, and the ≥2-impl rule for `interface`/`adapter`).

- **Inline by default.** No new file, helper, wrapper, param, option, interface, or generic with a single caller, unless it is a taxonomy role (service / serializer / repository / routes / middleware) or a second caller exists today. A thin serializer is house style; a one-call `formatX` helper or one-field `options` object is the bloat this targets.
- **No unreachable defensiveness.** No guard, `catch`, or `?? fallback` for a state the types or surrounding code already guarantee. Test: if you cannot write the input that reaches the branch, delete it. Real error handling at seams (provider calls, I/O, user input) and the sanctioned `runDegraded()` / `Result` / `AppError` / `{ success, errorMessage }` patterns are exempt.
- **Delete when replacing.** No shims, no "removed" markers, no backwards-compat re-exports.

## TypeScript

- Explicit return types on every exported function.
- `interface` for object shapes; `type` for unions, intersections, mapped types.
- `Readonly<T>` for React props and config objects.
- `override` required on every subclass method (`noImplicitOverride`).
- Prefer `@ts-expect-error <reason>` over `@ts-ignore`.
- No-any rule (biome `noExplicitAny`): no `any` outside system boundaries, and only with a justification comment there.
- Classes only for stateful resources with a lifecycle; factories/named functions for stateless logic.

## Type system

- **Branded types** (`SessionId`, `OrganizationId`, `MemberId`, `ParticipantId`, `MessageId`, `ProviderId`) from `packages/acme-domain/src/types/branded.types.ts`. Apply to **new** signatures; never mass-refactor. Cast from raw strings only at system boundaries (`row.id as SessionId`).
- String literal unions over `enum`, always. Discriminated unions key on `status` or `kind`.
- Exhaustive `switch`: `default` throws `new Error(\`unhandled: ${value as string}\`)` after narrowing.
- **Result pattern**: `Result<TValue, TError>` (`Ok()`, `Err()`) from `packages/acme-domain/src/types/result.types.ts`. **Use** for pure utilities that fail expectedly (parsing, validation). **Don't use** in the API layer (throw `AppError`), gateway (return `{ success, errorMessage }`), or engine (use `runDegraded()`).

## JSDoc

- **Required** on every exported symbol in `packages/*`; **not required** in `apps/*` / `services/*` (write it only when the WHY is non-obvious).
- Imperative verb summary (≤120 chars) is the contract; the rest is optional. Tags: `@param name - desc`, `@returns`, `@throws {ErrorType} condition`. No `@type` / inline type annotations. `@example` only when usage is non-obvious.
- Document interface/enum/prop members only where the WHY is non-obvious (units, direction, per-environment defaults, lifecycle, constraints not in the type). Self-explanatory members may go undocumented even in `packages/*`.
- Do NOT write: prose paraphrasing the symbol name or a param's type (`@param userId - The user id`); rambling multi-paragraph bodies; file-header banners (`@fileoverview …`); `// === Section ===` dividers; journal/changelog comments; commented-out code.

## Comments

Default to no inline comment. `//` explains **WHY** (invariant, unit, ordering, gotcha), never WHAT. A comment whose removal wouldn't confuse a future reader should not exist.

```ts
retries++; // increment the retry counter           (BAD: restates the code)
retries++; // 429s are transient, so retry first    (GOOD: explains the why)
```

**Seam-duplication test:** a call-site WHY comment that restates the callee's JSDoc or class doc is a duplicate; the explanation lives at the seam, hover-visible from the call site. Before keeping a comment that explains another module's behaviour, read that module's docs; if they already say it, delete the call-site copy.

This section is deliberately mirrored in `AGENTS.md`; keep both in sync.

## Errors

- Custom errors in a dedicated module: `src/lib/<package>.errors.ts` in apps, `src/errors/` in packages.
- Never inline `class FooError extends Error` next to business logic.

## Logging

- Structured JSON via `@acme/acme-logger` (`json-stdout` target in prod). One event = one line.
- **Console off in production.** Backend services build their logger via `createServiceLogger({ isProduction, identity })`, never by hand-assembling `targets`: in prod the seam wires `json-stdout` at minimum level `info` with the console target off; in dev, console only. `MultiTargetLogger` stays for tests and scripts.
- **Join keys are snake_case** — `service`, `env`, `version`, `request_id`, `trace_id`, `span_id`, and `event_type` for new structured events. Existing camelCase **domain** fields (`sessionId`, `durationMs`, …) are grandfathered; do **not** mass-rename them.
- Never log secrets or PII (passwords, tokens, cookies, email, name). `userId` / `orgId` are the allowed identity fields.
- **Sample high-volume success logs.** Errors (4xx/5xx) always log; successful (2xx) request lines log at a small configured rate (`REQUEST_LOG_2XX_SAMPLE_RATE`, default 2%). Success *volume* is the request-rate metric's job.
- Don't log healthcheck / `/metrics` paths; metrics answer "how many / how fast", not logs.

Compliance checklist for a new service or a new structured event:

1. Logger built via `createServiceLogger` (console-off-in-prod comes with it).
2. New events carry a top-level `event_type`; join keys snake_case.
3. No secrets or PII; provider-raw identifiers (e.g. raw SMS delivery receipts, which can embed phone numbers) sanitized before they reach a structured field.
4. Hot paths report a counter plus one episodic WARN, never one line per frame or per call.
5. High-volume success paths sampled or left to metrics; healthcheck and `/metrics` paths never logged.

## Structure

- Interface contracts belong in `interfaces/`, never beside implementations.
- Types/interfaces live in dedicated `types/` or `interfaces/` files, never inline in service/route files. Re-export from the consumer to keep call sites stable.
- `config/*.yaml` lives at the app/package root (e.g. `apps/acme-api/config/`), not the monorepo root.
- **No hardcoded values**: pull from YAML config or `env.ts`. Never inline ports, URLs, sample rates, pool sizes, timeouts.

## Naming

### File pattern

Every source file matches `kebab-case.<role>.ts` (or `.tsx`), hook-enforced by `.claude/hooks/validate-file-naming.sh` (PascalCase/camelCase stems and missing-role names are rejected). The role suffix locks in a calling convention; pick the role for what the file *does*, not the domain it lives in. Exempt filenames (no role): `index.ts`, `main.ts`, `app.ts`, `env.ts`, `setup.ts`, `*.config.ts`, `*.d.ts`.

### The role list (24 roles)

#### Behavioral (11)

Modules with a runtime calling convention.

| Role         | Calling convention                                                                                          |
| :----------- | :---------------------------------------------------------------------------------------------------------- |
| `service`    | Stateless verbs over a domain: `create` / `list` / `getById` / `update` / `remove`. No lifecycle.           |
| `repository` | DB persistence: `create<Entity>` / `get<Entity>ById` / `list<Entities>By<Filter>` / `update<Entity>` / `delete<Entity>`. Lives only in `packages/acme-db/src/repositories/`. |
| `serializer` | Pure transform: `serialize<Entity>(...)`. No state, no I/O.                                                 |
| `middleware` | HTTP/WS pipeline step: `authenticate<Type>()` / `require<Condition>()` / `resolve<Property>()`.             |
| `routes`     | Route factory: `create<Domain>V1Routes()` (or `createInternalRoutes()`). For WS, `create<Type>UpgradeHandler()` / `create<Type>Proxy()`. |
| `manager`    | Class owning a stateful resource. Exposes `start()` / `stop()` (or `dispose()`) plus domain methods.        |
| `factory`    | Pure construction: `create<Thing>(...)` returns a fresh instance. No state, no domain verbs.                |
| `client`     | Wrapper around one external system (HTTP / gRPC / SDK). Methods mirror the external API verbs.              |
| `adapter`    | Implementation of a project-defined interface at a seam with ≥2 impls. Methods come from the interface.     |
| `registry`   | Key→value lookup: `register(key, value)` / `get(key)` / `has(key)` / `list()`.                              |
| `config`     | Loads/parses runtime configuration: `load<Domain>Config()` returning a typed object.                        |

#### Declarations (6)

No runtime behaviour; classify by content kind.

| Role         | Content                                                                                                       |
| :----------- | :----------------------------------------------------------------------------------------------------------- |
| `types`      | TypeScript-only declarations: types, unions, params, request/response shapes, branded IDs.                    |
| `interface`  | A seam contract with ≥2 implementations or consumed across a package boundary. **Must live in `interfaces/`.** |
| `schemas`    | Zod / Drizzle / OpenAPI schemas (runtime validation or column definitions).                                   |
| `enums`      | Enum declarations.                                                                                            |
| `constants`  | Runtime values (no types added).                                                                              |
| `errors`     | Custom `Error` subclasses (location: see [Errors](#errors)).                                                  |

#### Frontend (3)

| Role        | Used for                                                                |
| :---------- | :--------------------------------------------------------------------- |
| `component` | React component (function or class).                                    |
| `hook`      | Single React hook (`useFoo`). One hook per file.                        |
| `store`     | Client state store (Context + hooks; see `frontend.md`. No Redux / Zustand). |

Pages and layouts under `apps/acme-web/src/features/<domain>/` are `component`; react-router-dom has no file-based-routing exemption.

#### Tests & test data (2)

| Role               | Used for                                                                                  |
| :----------------- | :---------------------------------------------------------------------------------------- |
| `test`             | Unit test (Vitest).                                                                       |
| `mock`             | Test double or fixture builder: `mock<Thing>()` / `build<Thing>Fixture()`.                |

#### Constrained utility (1)

| Role    | Rule                                                                                                                   |
| :------ | :--------------------------------------------------------------------------------------------------------------------- |
| `utils` | Pure functions only: no classes, no module-level state. Names are domain verbs (`normalizeText`), not CRUD. **Cannot import** `service`, `manager`, `repository`, or `client`; utils sit at the bottom of the dep graph. |

#### Entrypoint (1)

| Role     | Rule                                                                                                                                |
| :------- | :-------------------------------------------------------------------------------------------------------------------------------- |
| `script` | Runnable entrypoint: has a `main()` block or `if (isMainModule())` runner, invoked via `tsx`/`node` from a `package.json` script or shell, not imported by production code. May export library surface alongside the runner; if the file is **only** importable, pick a different role. |

### Decision tree

Walk top-to-bottom; first match wins.

```
Pure declaration, no runtime behaviour?  → types | interface | schemas | enums | constants | errors
React file?                              → component | hook | store
Test or test data?                       → test | mock
External system wrapper?                 → repository (DB) | client (HTTP/gRPC/SDK) | adapter (project interface, ≥2 impls)
Owns a stateful resource (start/stop/dispose)?  → manager
Pure construction (build an instance, return it)?  → factory
Pure format transform (domain ↔ wire)?   → serializer
Lookup-by-key is the entire interface?   → registry
HTTP / WS plumbing?                      → routes | middleware
Loads runtime configuration?             → config
Stateless domain verbs over data?        → service
Pure functions, none of the above fit?   → utils (constrained; see rule above)
Runnable entrypoint (main() / isMainModule guard, run via tsx/node)?  → script
```

If nothing fits cleanly, the module is probably shallow: deepen it or fold it into a role above.

### Plurality rule

One canonical form per role (hook-enforced): plural `types` / `schemas` / `enums` / `constants` / `errors` / `utils`; singular `interface` / `hook` (one seam contract per file, one hook per file).

### Folder rules

- `*.interface.ts` MUST live in an `interfaces/` directory.
- `*.types.ts` SHOULD live in a `types/` directory when shared across more than one neighbour; local types may sit beside their consumer.
- Repositories live only in `packages/acme-db/src/repositories/`, never in apps.

### Imports

- Named exports only; never `export default` (biome `noDefaultExport`).
- Extensionless imports in source (`.js` added at build time by a post-build step).
- Services and repos imported as namespaces: `import * as messagesService from "..."`.
- Import order is biome-managed (`organizeImports`).

### Identifier shapes

#### Types

- Generics: descriptive `T`-prefixed PascalCase (`TResponse`, `TConfig`). Bare `T` only for fully generic containers.
- Booleans use `is/has/should/can` prefixes; `enabled`/`active` are exceptions.
- API deps: `<Domain>V1Deps`. Params: `<Action><Entity>Params`. Errors: `<Reason>Error`.
- DB row inference: `typeof <table>.$inferSelect`.

#### Zod / OpenAPI

- `<Entity>Schema`, `<Action><Entity>BodySchema`, `<Entity>IdParamSchema`, `<Action><Entity>Response`.

### Banned roles (and where they go)

Rejected by `.claude/hooks/validate-file-naming.sh`; its error message re-teaches the role list. Rename to the role matching the file's calling convention.

| Banned                                                          | Rename to                                                                          |
| :-------------------------------------------------------------- | :--------------------------------------------------------------------------------- |
| `helper` / `helpers`                                            | `utils` (pure functions) or `mock` (test fixtures)                                 |
| `util` (singular)                                              | `utils`                                                                            |
| `interfaces` / `error` / `schema` / `hooks`                    | `interface` / `errors` / `schemas` / `hook`                                        |
| `builder`                                                       | `factory`                                                                          |
| `controller`                                                    | `service` (logic) or `routes` (route factory)                                      |
| `monitor` / `pool`                                             | `manager`                                                                          |
| `fixture`                                                       | `mock`                                                                             |
| `router`                                                        | `routes`                                                                           |
| `handler`                                                       | `routes` (HTTP/WS) or `service` (domain logic)                                     |
| `ws`                                                           | `routes` (factory shape) or `manager` (lifecycle)                                  |
| `service.test` / `repository.test` / `registry.test` / `serializer.test` / `routes.test` | `test`                                          |
| `integration.test`                                              | `test` under `test/integration/` (see [`testing.md`](testing.md))                   |
| `page` / `layout` / `provider` / `context`                     | `component`                                                                        |

Domain words are also banned as roles (`email`, `sms`, `push`, `webhook`, `dispatch`, `delivery`, `pipeline`, `runtime`, `processor`, `coordinator`, `persister`, `generator`, `stream`, `server`, `tokens`, `keys`, `singleton`, `resolver`, `publisher`, `history`, `machine`, `renderer`): the domain belongs in the kebab portion, the role names the calling convention.
