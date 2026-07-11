---
name: new-api-endpoint
description: Scaffold a new API endpoint with route, schema, service, serializer, and test following strict layering conventions. Auto-activates when asked to "add an endpoint", "create an API route", or "add an API for".
user-invocable: false
---

Scaffold a new API endpoint. The per-layer obligations (schema error shapes, service signatures, serializer and handler rules) live in `docs/conventions/api.md` - read it first and follow it exactly. This skill fixes only the file set and order of operations.

## Execution Steps

1. **Determine Domain & Context:** Identify the domain (e.g., `sessions`, `messages`). Read 2-3 existing examples in `routes/`, `schemas/`, `services/`, and `serializers/` before writing code.
2. **Schema** - `src/schemas/<domain>.schemas.ts` with `@hono/zod-openapi` (`createRoute`, `z`): entity, request body, and param schemas, per api.md §Schemas.
3. **Service** - `src/services/<domain>.service.ts`, per api.md §Services.
4. **Serializer** - `src/serializers/<domain>.serializer.ts`, per api.md §Serializers.
5. **Route** - `src/routes/<domain>.routes.ts`: export factory `create<Domain>V1Routes(deps: <Domain>V1Deps)` with a deps interface (`db`, `env`, `auth`, etc.); handlers stay thin (Validate -> Service -> Serialize -> Respond), per api.md §Routes.
6. **Register:** Add the route factory to `registerRoutes` in `src/routes/index.ts`.
7. **Tests** - `test/unit/routes/<domain>-routes.test.ts`, per `docs/conventions/testing.md`. (Logger is globally suppressed in API tests; mock only to assert.)
8. **Verify:**
   - `pnpm typecheck`
   - `pnpm --filter @acme/acme-api test`
