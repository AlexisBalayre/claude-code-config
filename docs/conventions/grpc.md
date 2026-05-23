# gRPC Conventions

Rules for everything under **`packages/acme-rpc`** — Protobuf definitions and the generated
gRPC stubs that the Gateway and Session Engine speak. The thin `@docs/conventions/grpc.md`
import in `.claude/rules/grpc-conventions.md` points here.

## The `.proto` is the contract

`.proto` files in `proto/` define the service contract. Generated stubs live in
`src/generated/` and are **never edited by hand** — the `protect-generated.sh` PreToolUse hook
blocks any Edit/Write under `packages/acme-rpc/src/generated/`. To change the API, edit the
`.proto` and regenerate:

```bash
pnpm --filter @acme/acme-rpc proto:gen
```

## Interceptors / middleware

Every server wires the standard interceptor stack:

- **Auth** — `createServerAuthMiddleware` validates the service token on every call. No
  internal call is anonymous.
- **Logging** — structured request/response logging via `@acme/acme-logger`.
- **Deadlines** — see below.

```ts
const server = createRpcServer({
  interceptors: [createServerAuthMiddleware(deps), loggingInterceptor, deadlineInterceptor],
});
```

## Never leave a call unbounded

Every client call sets a per-call deadline / TTL from config. An unbounded call can pin a
connection forever under partial failure.

```ts
// ✗ await client.dispatchMessage(req);
await client.dispatchMessage(req, { deadline: deadlineFrom(config.rpc.dispatchTtlMs) });
```

## Backwards-compatible field changes

The contract evolves additively:

- **Add** new fields with new tag numbers.
- **Never** renumber an existing field, and **never** reuse a tag number that was once used —
  reserve dropped tags with `reserved`.
- Mark removed fields `reserved` so a future field can't silently reuse the wire slot:

```proto
message DispatchRequest {
  reserved 4;            // was `legacy_route`, removed in v2
  string session_id = 1;
  string message_id = 2;
  bytes payload = 3;
  string channel = 5;   // added in v2
}
```
