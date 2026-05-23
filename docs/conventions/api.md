# API Conventions

Rules for `apps/acme-api` — the public HTTP API built on **Hono** +
**`@hono/zod-openapi`**. Auto-loaded via `.claude/rules/api-conventions.md` when you touch
this workspace. The `new-api-endpoint` skill scaffolds against this doc.

## Strict layering

```
Routes  →  Services  →  Repositories  →  Database
```

Each arrow is the **only** legal call direction. **Never skip a layer.** A route handler must
not touch the database; it calls a service. A service must not run raw SQL; it calls a
repository. Crossing a layer is the most common review rejection.

## Schemas — `src/schemas/<domain>.schemas.ts`

Use `@hono/zod-openapi` (`createRoute`, `z`). Each domain file defines:

- the **entity** schema (the API shape of the resource),
- **request body** schemas,
- **param** schemas (path/query),
- **responses** that always include `401` and `500`, plus `404` for any by-id lookup.

Import the shared `ErrorSchema` rather than redefining error shapes.

```ts
import { z } from '@hono/zod-openapi';
import { ErrorSchema } from './error.schemas';

export const MessageSchema = z.object({
  id: z.string().uuid(),
  sessionId: z.string().uuid(),
  body: z.string(),
}).openapi('Message');

export const getMessageRoute = createRoute({
  method: 'get',
  path: '/v1/messages/{id}',
  request: { params: z.object({ id: z.string().uuid() }) },
  responses: {
    200: { content: { 'application/json': { schema: z.object({ message: MessageSchema }) } }, description: 'OK' },
    401: { content: { 'application/json': { schema: ErrorSchema } }, description: 'Unauthorized' },
    404: { content: { 'application/json': { schema: ErrorSchema } }, description: 'Not found' },
    500: { content: { 'application/json': { schema: ErrorSchema } }, description: 'Server error' },
  },
});
```

## Services — `src/services/<domain>.service.ts`

- **Stateless exported named functions. NEVER classes.**
- Standard CRUD names: `create`, `list`, `getById`, `update`, `remove`.
- **First parameter is `db: Database`.**
- Throw `AppError` subclasses for business errors (`NotFoundError`, `ForbiddenError`); the
  route layer maps them to status codes.

```ts
export async function getById(db: Database, id: MessageId): Promise<MessageEntity> {
  const row = await MessageRepository.findById(db, id);
  if (!row) throw new NotFoundError('message', id);
  return row;
}
```

## Serializers — `src/serializers/<domain>.serializer.ts`

Transform DB entities into API response shapes. **Never expose a raw DB entity** from a
handler — it leaks column names, internal flags, and soft-delete fields.

```ts
export function serializeMessage(row: MessageEntity): Message {
  return { id: row.id, sessionId: row.sessionId, body: row.body };
}
```

## Routes — `src/routes/<domain>.routes.ts`

- Export a **factory** named `create<Domain>V1Routes(deps)` (e.g. `createMessagesV1Routes`,
  `createUsersV1Routes`).
- Define a **deps interface** for what the routes need (`db`, `env`, `auth`, …).
- Handlers stay **thin**: **Validate → Service → Serialize → Respond**, returning a named
  object (`{ message }`, `{ users }`).
- Add `// @ts-expect-error` directly above **every** `router.openapi()` call — Hono's strict
  return types don't yet model the OpenAPI response union.
- Apply auth middleware (`requireAuth()`, `requireAdmin()`) to sensitive routes.

```ts
export interface MessagesV1Deps {
  db: Database;
  env: Env;
  auth: Auth;
}

export function createMessagesV1Routes(deps: MessagesV1Deps) {
  const router = new OpenAPIHono();

  // @ts-expect-error — Hono strict return type vs. OpenAPI response union
  router.openapi(getMessageRoute, async (c) => {
    const { id } = c.req.valid('param');
    const entity = await MessageService.getById(deps.db, id as MessageId);
    return c.json({ message: serializeMessage(entity) }, 200);
  });

  return router;
}
```

## Register the factory

Every route factory is registered in `registerRoutes` (`src/routes/index.ts`). A route that
isn't registered there is unreachable.

```ts
export function registerRoutes(app: OpenAPIHono, deps: AppDeps) {
  app.route('/', createUsersV1Routes(deps));
  app.route('/', createMessagesV1Routes(deps));
}
```

The `users` and `messages` domains are the canonical examples — read them before adding a new
one.
