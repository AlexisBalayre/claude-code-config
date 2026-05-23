# Security Model

The auth, secrets, and input-handling model for Acme. This file is `@imported` by the
**`security-reviewer`** subagent — it is the spec that review checks against, so keep it
concrete and reviewable.

## Authentication

Two distinct trust boundaries, two mechanisms:

- **End users** (web + public API): **BetterAuth sessions**. The browser holds a session
  cookie; `apps/acme-api` and `apps/acme-web` validate it. Sensitive routes are guarded by
  `requireAuth()` / `requireAdmin()` middleware in `src/routes/`.
- **Internal service-to-service** (gRPC between Gateway and Session Engine): **JWT / service
  tokens** verified by `createServerAuthMiddleware`. **No internal call is anonymous** — the
  middleware is wired on every server in `packages/acme-rpc`.

The Gateway bridges the two: it validates the **end-user session at the WebSocket upgrade**
(`server.on("upgrade", …)` must validate before `upgrade()`), then carries a **service token**
inward.

## Secrets

Secrets are read **only** through `src/config/env.ts` for the workspace — never inlined,
never read from `process.env` at a call site. This gives one validated, typed choke point.

```ts
// ✗ const key = process.env.STRIPE_KEY;        // ad-hoc, unvalidated
import { env } from "../config/env";
const key = env.PROVIDER_API_KEY;
```

Provider YAML config carries endpoints and limits, **not credentials** — those come from env.
Reviewers grep for `apiKey` / `token` / `secret` / `password` assignments outside `env.ts`.

## Input validation

**Every endpoint validates input with a Zod schema** in `src/schemas/`. List endpoints carry a
bounded `limit` — no unbounded query parameters. Validation runs before any Service call.

```ts
const ListMessagesQuery = z.object({
  sessionId: z.string().uuid(),
  limit: z.number().int().min(1).max(100).default(50),
});
```

## Output: serializers

**No raw DB entity is ever returned** in an API response. Every response passes through a
`serializer` that selects fields explicitly — this is what prevents accidental exposure of
internal columns, soft-delete flags, or other Members' data.

## Rate limiting

Rate-limiting middleware is applied to **auth endpoints (login, registration)** and
**expensive endpoints** (anything that fans out to Providers or runs heavy queries). This blunts
credential stuffing and abuse.

## Concurrency safety

Session slot reservations use **atomic Lua scripts** (`RESERVE_SLOT_SCRIPT`) in Redis, never
`GET`+`SET` — see [session-engine conventions](../conventions/session-engine.md). The race
between read and write is a real over-allocation bug, not a theoretical one.

## OWASP Top 10 mapping

| OWASP (2021) | Where it's handled |
| :--- | :--- |
| A01 Broken Access Control | `requireAuth`/`requireAdmin` on routes; WS upgrade validation; Service/Repository layering (no route hits the DB directly) |
| A02 Cryptographic Failures | TLS at the edge; secrets via `env.ts`; no plaintext credentials in config |
| A03 Injection | Drizzle parameterised queries (no string-interpolated `.where()`); Zod validation; React escaping (`dangerouslySetInnerHTML` forbidden without sanitization) |
| A04 Insecure Design | Three-tier boundaries; stateless API; affinity/discovery for state |
| A05 Security Misconfiguration | Typed `env.ts` with required-var validation at boot |
| A07 Identification & Auth Failures | BetterAuth sessions; rate-limited login/registration |
| A08 Software & Data Integrity | gRPC stubs generated, never hand-edited; additive proto changes only |
| A09 Logging Failures | Structured logs via `@acme/acme-logger`; **no PII or raw stack traces** logged |
| A10 SSRF | Provider endpoints come from the YAML registry, not user input |
