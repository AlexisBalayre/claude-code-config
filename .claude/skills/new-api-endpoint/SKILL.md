---
name: new-api-endpoint
description: Scaffold a new API endpoint with route, schema, service, serializer, and test following strict layering conventions. Auto-activates when asked to "add an endpoint", "create a route", or "add an API for".
user-invocable: false
---

Scaffold a new API endpoint. **CRITICAL:** Follow `docs/conventions/api.md` exactly.

## Execution Steps

1. **Determine Domain & Context:** Identify the domain (e.g., `users`, `messages`). Read 2-3 existing examples in `routes/`, `schemas/`, `services/`, and `serializers/` before writing code.
2. **Create Schema (`src/schemas/<domain>.schemas.ts`):**
   - Use `@hono/zod-openapi` (`createRoute`, `z`).
   - Define entity, request body, and param schemas.
   - Include `401` + `500` errors (and `404` for lookups). Import `ErrorSchema`.
3. **Create Service (`src/services/<domain>.service.ts`):**
   - Use stateless exported named functions (NEVER classes). Use standard CRUD names (`create`, `list`, `getById`, `update`, `remove`).
   - First parameter MUST be `db: Database`. Throw `AppError` subclasses for business errors.
4. **Create Serializer (`src/serializers/<domain>.serializer.ts`):**
   - Transform DB entities to API response shapes. NEVER expose raw DB entities.
5. **Create Route (`src/routes/<domain>.routes.ts`):**
   - Export factory: `create<Domain>V1Routes(deps: <Domain>V1Deps)`. Define deps interface (`db`, `env`, `auth`, etc.).
   - **Handlers:** Keep thin (Validate -> Service -> Serialize -> Respond). Return named objects (e.g., `{ entity: ... }`).
   - Add `// @ts-expect-error` above EVERY `router.openapi()` call to bypass Hono strict return types.
6. **Register:** Add the route factory to `registerRoutes` in `src/routes/index.ts`.
7. **Add Tests (`test/unit/routes/<domain>-routes.test.ts`):**
   - Follow `docs/conventions/testing.md`. (Note: Logger is globally suppressed in API tests; mock only to assert).
8. **Verify:**
   - `pnpm typecheck`
   - `pnpm --filter @acme/acme-api test`
