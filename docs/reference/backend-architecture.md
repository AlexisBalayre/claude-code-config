# Backend Architecture (Reference)

The topology of the Acme backend: three services + four shared packages, and the two end-to-end
paths through them. For *why* it's split this way, see
[system-architecture.md](../explanation/system-architecture.md).

## Topology

```
                  ┌──────────────┐   HTTPS    ┌─────────────────┐
   browser  ─────▶│  acme-web     │──────────▶│   apps/acme-api  │──┐
                  │ (React SPA)   │           │ (Hono + Zod)     │  │
                  └──────────────┘            └─────────────────┘  │
                                                                   ▼
   client ── WS/gRPC ─▶ services/acme-gateway ── gRPC ─▶ services/acme-session-engine
                         (edge: auth, routing)            (stateful: state machines)
                                                                   │
                                                                   ▼
                                                    packages/acme-providers ──▶ Channel
                                                    (email · sms · push · webhook)

   shared:  packages/acme-db   packages/acme-rpc   packages/acme-domain   packages/acme-providers
```

## What each owns

| Workspace | Owns |
| :--- | :--- |
| `apps/acme-api` | Public HTTP API: routes, Zod schemas, services, serializers. Stateless. |
| `services/acme-gateway` | Client connection termination (WS + gRPC), auth handshake, frame routing. Connection state only. |
| `services/acme-session-engine` | Per-Session state machines, slot reservations, Adapters. Session state. |
| `packages/acme-db` | Drizzle schema + migrations (Postgres). The only place SQL is defined. |
| `packages/acme-rpc` | Protobuf contracts + generated gRPC stubs; auth/logging/deadline interceptors. |
| `packages/acme-domain` | Shared domain types — branded IDs (`SessionId`, `MemberId`), enums. |
| `packages/acme-providers` | Pluggable Channel adapters behind a factory + YAML registry. |

## Path A — an HTTP API request

1. `acme-web` calls the API with the BetterAuth session cookie.
2. **Route** (`src/routes/`) — `requireAuth()` middleware validates the session.
3. **Schema** (`src/schemas/`) — Zod validates and types the request body/query.
4. **Service** (`src/services/`) — business logic; calls the Repository.
5. **Repository** — Drizzle queries via `packages/acme-db`. No route touches the DB directly.
6. **Serializer** — maps entities to the response shape. No raw entity leaves the API.

## Path B — a realtime Message

1. Client sends a Message frame over its **WebSocket** to `acme-gateway`.
2. **Gateway** has already validated the session at upgrade; it looks up the Session's engine
   instance in the **discovery registry** (affinity) and forwards the frame over **gRPC**,
   carrying a service token (`createServerAuthMiddleware`).
3. **Session Engine** advances the Session's **state machine**, persists/updates Session state,
   and emits Events (e.g. `MessageDispatched`).
4. For each target **Channel**, the engine resolves a **Provider** from
   `packages/acme-providers` (`createProvider(channel, id, deps)`) and dispatches.
5. The **Provider** adapts the call to the external delivery system; results/errors flow back
   to the engine, which updates state and may notify Participants via the Gateway.
