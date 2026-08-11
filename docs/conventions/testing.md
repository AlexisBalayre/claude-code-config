# Testing

**Genre contract:** obligations only.

Vitest (`globals: true`, `env: node`), V8 coverage (85% lines / 80% branches). Timeouts: 5s unit / 10s integration (`packages/acme-db` may extend). One exception: `apps/acme-web` runs `jsdom` with a coverage **ratchet** instead of the 85/80 target (thresholds pinned in its `vitest.config.ts`; raise as untested areas gain tests, never lower).

## Layout

Tests live in `test/`, never beside source. Per package: `test/unit/` (mirrors `src/`, one file per module), `test/integration/` (cross-module scenarios; may use real services), `test/helpers/` (shared mocks and fixture builders), optional `test/setup.ts` (global hooks). `apps/acme-web` adds a third category: `test/e2e/` (smoke specs making real network calls against a live deployment), run via `test:e2e` with its own `vitest.e2e.config.ts` (30s timeout, single fork), deliberately outside the shared factories below.

- Unit mirror is mechanical: `src/foo/bar.service.ts` → `test/unit/foo/bar-service.test.ts` (replace the dot between kebab and role with a dash, append `.test.ts`). No compound test roles. The naming hook validates only the test file's own suffix, not the src↔test correspondence, so keep the mapping honest yourself.
- Integration tests are named by **scenario**, not module: `two-providers-failover.test.ts`, not `session.manager.test.ts`.
- Coverage thresholds apply to unit tests only.
- The legacy `.integration.test.ts` suffix is **banned** (the naming hook rejects it) — folder placement is the source of truth.

## Imports

Tests use `@src/*` and `@test/*` aliases — never `../../src/...`. Each package wires them in its own `tsconfig.json` `compilerOptions.paths`; the shared Vitest factory wires the same aliases — keep both in sync.

## Vitest configuration

Use the shared factories at `tools/vitest/`:

```ts
// vitest.config.ts
import { defineUnitConfig } from "../../tools/vitest/unit.factory";
export default defineUnitConfig(import.meta.dirname);

// vitest.integration.config.ts (when integration tests exist)
import { defineIntegrationConfig } from "../../tools/vitest/integration.factory";
export default defineIntegrationConfig(import.meta.dirname);
```

Pass overrides as a 2nd arg — deep-merged via `mergeConfig` (array fields like `coverage.exclude` extend, not replace). Scripts: `test` / `test:watch` / `test:integration`.

## File order (strict)

`vi.mock(...)` → imports of mocked modules → import of module under test → shared fixtures → `describe` blocks.

## Setup & mocks

- Always `vi.clearAllMocks()` in `beforeEach`. Use `vi.mocked(fn).mockResolvedValue(...)` for type safety.
- **Logger:** mock `@acme/acme-logger`'s `MultiTargetLogger` (instance methods `info/error/warn/debug`); prefer `@test/helpers/logger`'s `createMockLogger()` where available.
- **DB:** API uses `const mockDb = {} as any` + repo module mocks. Repo tests use a local chainable `createMockDb()` (`from`, `where`, `select`, `insert`).

## API route tests

- Mock `middleware/auth` and `middleware/session-auth` as pass-throughs that set context. Mock services at module level.
- Wrap the sub-router in a mini `OpenAPIHono` with an `onError` mapping `AppError`. Drive via `app.request()`.

## Integration tests

- Real implementations; mock external services (email, SMS, push providers) via factory helpers + spread overrides.
- `vi.useFakeTimers()` in `beforeEach`, `vi.advanceTimersByTimeAsync()` for async pipelines, `vi.useRealTimers()` in `afterEach`.

## Focus

- Never skip repository tests.
- Services: happy path, ownership (`NotFoundError` on mismatch), errors, fallbacks.
- Routes: status, shape, service args, roles, error mapping.
- Repositories: query construction, parameter mapping, return values.
- Gateway pools: acquire/release, lifecycle, capacity limits, queue behavior, concurrency, shutdown draining.
- Gateway resilience: CB closed → open → half-open; health score windows.
- Gateway routing: selection, fallback, degraded skipping. Sessions: admission thresholds, duplicate rejection.
