# Backend

Covers `apps/acme-api/` and `packages/acme-db/`. Inherits [`core.md`](core.md); testing in [`testing.md`](testing.md).

**Genre contract:** obligations only. System description lives in [backend-architecture.md](../reference/backend-architecture.md).

**Reference-doc sync:** a PR that changes the composition root (`server.script.ts`, `app.ts`, `routes/index.ts`) or adds a cross-service edge MUST update [backend-architecture.md](../reference/backend-architecture.md) (element list, diagram) in the same PR.

## API (`apps/acme-api/`)

### Layering (strict)

Routes → Services → Repositories → Database. **Never** call repos from routes. Repos live in `packages/acme-db/src/repositories/` (`@acme/acme-db/repositories`).

| Dir | Pattern | Notes |
|---|---|---|
| `routes/` | `<domain>.routes.ts` | Thin handlers: validate → service → serialize → respond. |
| `schemas/` | `<domain>.schemas.ts` | `@hono/zod-openapi`. May split (e.g. `sessions-lifecycle.schemas.ts`). |
| `services/` | `<domain>.service.ts` | Stateless named functions. No classes for business logic. |
| `serializers/` | `<domain>.serializer.ts` | Pure. Dates → ISO strings. No I/O. |
| `ws/` | `<type>.routes.ts` | WebSocket route factories (raw-`ws` upgrade handlers). |

### Routes

- Export `create<Domain>V1Routes(deps)` / `createInternalRoutes(deps)`. Import services as namespaces. Register in `routes/index.ts`.
- Responses are named objects (`{ message: ... }`); `201` on create, `200` otherwise.
- Pass the explicit success status to `c.json` (`c.json(body, 200)`, `201` on create). Without it the handler's return type does not narrow to one member of the route's declared status union, and `router.openapi()` rejects the handler.
- Never blanket-suppress that rejection. `// @ts-expect-error` above `router.openapi()` disables type checking on the whole route-to-handler wiring, so a genuinely wrong response shape passes silently. Add one only for a cause you have diagnosed, and state that cause in the directive rather than the generic "Hono OpenAPI strict return types". The one cause known to survive an explicit status is a serializer whose output type is wider than the declared schema; that is a bug to fix, not a steady state.
- **Org-scoped path shape:** `/api/v1/organizations/:orgId/<resource>` is the norm for org-role-gated self-service resources (`orgAdminChain`, e.g. `org-members`, `org-join-links`); `:orgId` still names the caller's own org, checked by the chain. Platform-admin cross-tenant routes live under `/api/v1/admin/<resource>` (`platformAdminChain`), never under `:orgId`. `/api/v1/org/<resource>` (org from the session, no `:orgId` param) is the exception shape: `api-keys` and `billing-details` only.

### Schemas

- Naming in [`core.md`](core.md#zod--openapi). Path params: `z.uuid().openapi({ param: { name, in: "path" } })`.
- Always include `401` + `500`; add `404` on lookups. Use shared `ErrorSchema`.

### Services

- First param is always `db: Database`. Standard methods: `create`, `list`, `getById`, `update`, `remove`.
- Verify org ownership: a private `getOwned<Entity>(db, id, orgId)` helper (throws `NotFoundError` on mismatch) is the pattern when a service repeats the same check across methods (`messages.service.ts`); a service with one call site checks ownership inline instead.
- Throw `AppError` subclasses; catch/log/return a safe default only for non-fatal ops. Classes only for stateful resources (`EngineClient`, `JWTValidator`).

### Errors

Extend `AppError` in `src/lib/api.errors.ts`. Shape: `{ error: "NOT_FOUND", message: "..." }`. No stack traces in prod. Status map: `NotFoundError` 404, `BadRequestError` 400, `UnauthorizedError` 401, `ForbiddenError` 403, `ConflictError`/`IdempotencyConflictError` 409, `UnprocessableEntityError` 422 (well-formed body, semantically out of bounds, e.g. a pick outside the caller's entitlements), `TooManyRequestsError` 429, `UsageLimitExceededError`/`SubscriptionLapsedError` 402 (billing-quota exhaustion or a lapsed subscription; clears by fixing billing rather than by waiting), `ServiceUnavailableError`/`SessionCapacityError`/`IdempotencyWaitTimeoutError` 503. `IdempotencyTransientHandlerError` extends plain `Error`, not `AppError`: it never reaches the client, it only tells the idempotency middleware to release its cache slot for retry.

### Auth & Middleware

- Auth chain: pick a named chain from `auth-chain.middleware.ts` and spread it (`router.use("*", ...chain(deps))`): `authenticatedChain` (any signed-in caller; pass `{ jwt: false }` for cookie/key-only surfaces), `orgAdminChain(...roles)` (`:orgId` gate), `platformAdminChain(min)` (platform-role gate). All build on `credentialPrefix(deps, { jwt })` — cookie → key → `resolveUserRole`, bracketed by `optionalAuth` + `resolveUserRoleById` when `jwt` (they pair: the JWT path is the only producer of the `userId`-without-email state). Routes with an interleaved chain (sessions, api-keys) spread `credentialPrefix` and append their own gates.
- Helpers: `getSessionUser(c)`, `checkUserRole()`. Gate routes with the named chains above, not a hand-rolled guard.
- **Global middleware order (do not change):** ReqID → Logging → Security headers → Body limit (1MB) → CORS → CSRF origin check → Rate limit (200/min on `/api/v1/*`). CORS **must** precede the rate limiter: a 429 short-circuits `next()`, so middleware after it never runs, and the browser would report a misleading CORS error instead of the rate-limit response. The CSRF check **must** sit between them: after CORS so its 403 carries CORS headers, before the rate limiter so forged cross-site requests cannot burn the victim's cookie-keyed bucket.

### WebSocket

- **Critical paths (session / realtime frames): raw `ws` only**, routed via `server.on("upgrade")` so we control framing and error handling. Factories `create<Type>UpgradeHandler(deps)` / `create<Type>Proxy(deps)`, registered in `server.script.ts`.
- Plain Hono upgrades may use `@hono/node-ws` (`createNodeWebSocket`); go raw if a path needs custom lifecycle control.
- **Hono upgrade hack (raw `ws` paths):** monkey-patch `socket.end` before `wss.handleUpgrade()`, or Hono's catch-all responds 400 and closes the socket before our handler runs.

## Database (`packages/acme-db/`)

Drizzle ORM + Postgres 18. Config: `casing: "snake_case"`.

### Layout

- `src/schema/`: one file per domain, re-exported from `index.ts`. `src/types/`: domain type files (re-exported via `types/index.ts`). `src/repositories/`: export inferred types here (not from schema). `src/seed/`: idempotent scripts (run via `tsx` with `.env` loaded).

### Schema rules

- PK: always `uuidV7("id").primaryKey()` (time-ordered `uuidv7()` default, PG 18). Never auto-increment. Exception: BetterAuth uses `text("id")`.
- Composite PKs: pass an explicit `name:` (`primaryKey({ name: "<table>_pkey", columns: [...] })`) whenever the Drizzle-generated name would exceed 63 bytes: Postgres silently truncates, and `db:generate` then diffs the name forever.
- Tables: plural snake_case. Columns: `camelCase` in TS (auto snake_case). Enums: `<domain>Enum` in TS, snake_case in SQL, declared at top via `pgEnum()`.
- Indexes: `idx_<table>_<columns>`, defined in the table's 3rd arg. Relations: `<table>Relations`, immediately after the table.
- Timestamps: always `{ withTimezone: true }`. `createdAt`: `.notNull().defaultNow()`. `updatedAt`: set manually (`new Date()`), never `$onUpdateFn()`.
- JSONB: `.$type<T>().default({})`. Shared column builders (`uuidV7`, `tsvector`) from `src/schema/custom-types.schemas.ts`.
- FKs: `.references(() => table.column, { onDelete: "cascade" | "set null" })`.
- Churny value sets (provider names, channel identifiers) stay free-text `text`, never `pgEnum`: the value sets churn and enum migrations are destructive + partition-hostile.
- CHECK constraints (`check("chk_<table>_<rule>", sql\`...\`)`) guard invariants such as non-negative counts/latencies. Add them on non-partitioned tables; on large partitioned tables a CHECK-add scans under `ACCESS EXCLUSIVE` in Drizzle's single-transaction migration, so apply those out-of-band.

### Partitioned tables

- Composite PKs required: `primaryKey(table.id, table.orgId)`.
- No DB-level FK constraints: enforce in app code.
- Declare every app-level FK via `relations()` on the partitioned table; a bare `uuid` column with no relation is invisible to schema tooling.
- **Dev has zero partitioned tables; prod has several**, so `pnpm db:migrate` against dev proves nothing about partition behaviour and partition-hostile DDL (e.g. `ALTER COLUMN TYPE` on a partition-key column, which Postgres refuses) goes green in dev and fails in prod. Rehearse DDL touching those tables against a scratch schema with real partitions attached (`CREATE TABLE ... PARTITION BY` + `PARTITION OF`, apply, insert via parent and direct).

### Queries

- Use the query builder (`db.select().from().where()`). **Never** `db.query.*` (relational API).
- Dynamic filters: accumulate in an array, spread into `and(...conditions)`.
- Mutations always `.returning()` → `const [result] = ...` → `result ?? null`.
- Upserts via `.onConflictDoUpdate()` on a unique index. Raw `sql` only for JSONB and conditional updates.
- Comparisons binding a `Date`: the typed operators (`gt`/`gte`/`lt`/`lte`/`eq`) map the value only when the **left operand is a `Column`**. With an expression on the left (e.g. `gte(sql\`COALESCE(a, b)\`, date)`) the `Date` reaches postgres-js unmapped and throws at query time, so bind `.toISOString()` yourself. Mocked-`db` unit tests cannot catch this class (they assert the arguments, never that the SQL binds); cover it with a real-postgres integration test.
- Raw `db.execute<T>` rows keep the DB's snake_case names (`casing: "snake_case"` remaps the query builder only) and `numeric` arrives as a string: type row shapes snake_case and coerce with `Number(...)`. A camelCase generic on a raw execute is a bug, and a camelCase mock fixture for one proves nothing.

### Ops

- Client pools (config fact): default `3`, API `50`, session engine `20`.
- Schema changes: edit `src/schema/**` → `pnpm db:generate` → review the SQL → commit → `pnpm db:migrate`. **Never** `db:push` against shared DBs: it applies DDL directly with no migration file, so there is no audit trail, no PR review of the SQL, and no record for `db:migrate` to replay later.
- Bootstrapping a DB previously kept in sync via `db:push`: backfill the migration journal first (dry-run, review, then apply) before the first `pnpm db:migrate`, or Drizzle's migrator sees zero applied rows and re-runs `0000_initial` from scratch.
- Handle PG error codes `23503` (FK violation) and `23505` (unique): extract constraint/detail.
