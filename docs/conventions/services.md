# Realtime Services

Covers `services/acme-gateway/`, `services/acme-session-engine/`, `packages/acme-rpc/`, and `packages/acme-providers/`. Inherits [`core.md`](core.md); testing in [`testing.md`](testing.md).

**Genre contract:** obligations only. For what each subsystem is and does, see [backend-architecture.md](../reference/backend-architecture.md) and [system-architecture.md](../explanation/system-architecture.md).

**Reference-doc sync:** a PR that changes a composition root (`main.ts`, `bootstrap.service.ts`), the per-session factory, or a proto service surface MUST update the matching reference doc's element list and diagram in the same PR.

## Gateway (`services/acme-gateway/`)

### Layout

- `<name>.<role>.ts`; tests live in `test/`, never beside source (see [testing.md](testing.md)). No barrel `index.ts`.
- gRPC services are **always** factories: `create<Name>Service(deps)` returning RPC handlers; provider client factories injected via `deps`.
- Dedicated dirs: `types/` (data shapes), `interfaces/` (DI contracts), `constants/`, `utils/` (stateless helpers).
- Config split: `config/loader.config.ts` (entry), `env.schemas.ts`, `gateway.schemas.ts`, `routing.schemas.ts`. Use Zod `.transform()` to map snake_case YAML → camelCase TS; config types inferred from the schemas.

### Pools

- `ConcurrencyPool` (HTTP delivery providers): use `pool.execute(accountId, fn)`. **Never** acquire/release manually. Pass `maxAccounts` (counted per channel from routing rules) for utilization math.
- `ProviderWsPool` (persistent-connection providers, partitioned by `provider:channel`): reserve the slot **before** the async factory call (prevents TOCTOU). `release()` itself decides reuse from `state` (`idle`/`active`/`dead`) via `isConnectionAlive()`/`isExpired()`; callers pass the connection through without setting `state` themselves.
- Concurrency/rate limits belong in **gateway** config, never in the providers package.

### Accounts & reservations

- Prefer `await using reservation = accountManager.selectAndReserve(...)` (or `try { … } finally { await reservation.release() }`) over the legacy `release(id, slotId)` form. The reservation handle binds `release()` / `renew()` / `[Symbol.asyncDispose]`.

### Resilience

- `HealthMonitor.record(provider, channel, { latencyMs, success, errorCode? })` on success **and** failure.
- `IFailoverService` per gRPC service, built at the composition root with knobs bound at construction. Stream-only instances omit `executeDeps` and cannot call `execute()`.
- Two retry layers: inner Cockatiel retry (same provider, transient errors), outer `failover.execute()` / `failover.stream()` loop that pushes the failed `provider:channel` onto the exclude set and re-routes.

### Composition root (`main.ts`)

- **Wiring order (strict):** Config → AccountManager → HealthMonitor → PolicyFactory → Router → Pools → FailoverServices → gRPC Services → gRPC Server → Shutdown.
- Derive `wsProviders` from routing rules with `transport: "websocket"`. `addProtobufTsService` requires `as any` + biome-ignore.
- **Shutdown order (strict):** set draining (SessionManager) → graceful gRPC (with force timeout) → Pools (`Promise.allSettled`) → HealthMonitor.

## Session Engine (`services/acme-session-engine/`)

### Layout

- `<name>.<role>.ts`. No internal barrel `index.ts` (exception: the package entry `src/index.ts`).
- Stateful classes take exactly two constructor params: `config` (immutable) and `deps` (injected). No god-objects. Types in `types/`.
- Providers are **never** hardcoded: inject factories via `deps` (e.g. `this.deps.emailProviderFactory(channelConfig)`).
- Stateless construction extracted to factories (e.g. `buildSessionSubsystems()`) that return a typed aggregate and own no lifecycle.

### Config

- A key exists in `config/*.yaml` only when there is a realistic scenario for changing the value without touching code (per-env divergence, incident lever). Protocol-fixed or never-tuned values are named constants next to their consumer. The YAMLs are baked into the image, so a YAML edit ships exactly like a code edit; "configurable" buys nothing on its own.
- The YAML is the single source of a value: schema fields are required, and consumer sinks carry no `?? DEFAULT` fallbacks or shadow constants. A field may be optional only when *absence* is a distinct behavior (e.g. an unset threshold = the check off), never as a way to store a default in a second place.
- Schemas follow the gateway pattern: Zod `.transform()` maps snake_case YAML to camelCase, and config types are inferred from the schemas. Per-value rationale lives as comments in the YAML next to the values.

### Lifecycle & state machine

- Always `start()` / `stop()`. **Never** do async work in constructors.
- Idempotent stop: cache the stop promise (`if (this.stopPromise) return this.stopPromise`). Children via `Promise.allSettled()`. Null out refs after cleanup.
- `transition("starting")` at the top of `start()`. Guard public entry points with `assertState()`. (State graph: see reference.)

### Events & errors

- Typed callback arrays (`FrameHandler[]`, `MessageHandler[]`) or direct property assignment (`onFlushError`). Register **before** `start()`. No Node `EventEmitter`. No `off()` (lifetime = session).
- `runDegraded(label, fn, logger)` for side-effect failures; `runBackground(label, signal, fn, logger)` for loops. One component's failure must not cascade.

### Adapters & streams

- Extend `BaseAdapter<S>`; implement inbound/outbound/bidirectional streams + `dispose()`. Frame streams extend `BaseFrameStream`.

### Dispatch pipeline

- Deliveries are serialized via a single-consumer task FIFO (`enqueue(task)`, drained one at a time) so per-Session ordering holds; preview/typing events run in parallel with per-segment `AbortController`.
- Monotonic counter (`revision++`) prevents stale overwrites: a slower in-flight render must never clobber a newer one.

### Graceful shutdown (strict order)

1. `shuttingDown = true` → health `NOT_SERVING` → `service.setDraining()` → `engineRegistry.markDraining()`. Stop taking new sessions, but keep the engine entry **and** the session-TTL refresh running.
2. `sessionManager.waitForDrain(drainTimeoutMs)`; force-stop remaining on timeout.
3. Stop the session-TTL refresh and `engineRegistry.stop()` (deregister from Redis) **only after** the drain completes.
4. gRPC → WebSocket → adapters → gateway leases → DB → Redis → `process.exit(0)`.

Deregistering or freezing the session-TTL refresh before the drain finishes orphans the still-running sessions: their `session:{id}:engine` keys expire within one TTL (~30s) while the pod keeps serving for up to `drainTimeoutMs`, so the API can no longer route to them. `markDraining()` advertises zero free capacity so least-loaded routing skips this engine for new work while the entry stays alive for the draining ones.

Keep `drainTimeoutMs` (config key `drain_timeout_ms`, default 300s) below K8s `terminationGracePeriodSeconds` (360s = drain + 60s teardown buffer).

## gRPC (`packages/acme-rpc/`)

Read 2-3 existing RPCs (proto + client/server) before adding one. Four subpath exports: `@acme/acme-rpc` (product-agnostic bootstrap: auth middleware, channel/server factories, the protobuf-ts ↔ nice-grpc bridge, log redaction), `@acme/acme-rpc/protos` (`.proto` source in `proto/`, generated stubs in `src/protos/generated/`), `/testing` (fake auth middleware for tests), and `/config-schema` (channel/auth config schemas).

### Protobuf

- Enum default always `_UNSPECIFIED = 0`.
- `optional` only when truly optional (proto3 scalars default to 0). Use `bytes` + JSON for variable payloads.
- Shared enums/messages live in `common.proto`. **Never** add enums to `acme-session-engine.proto` (use `string`); gateway-specific enums go in `acme-gateway.proto`.
- Codegen after any change: `pnpm --filter @acme/acme-rpc proto:gen`.
- Per-call structured payloads ride request-level fields; **never** push variable per-call data into a cached or shared prefix.

### Auth & errors

- Auth is global via `create{Server,Client}AuthMiddleware` (shared-secret Bearer). **Never** hardcode secrets.
- Wrap handlers in `try/catch`: return `{ success: false, errorMessage }` for expected errors, throw only for unrecoverable ones.
- Resource-creating RPCs must check `service.draining` and fail fast.

### Discovery & Redis

- Discovery: scan `engine:*`, filter by capacity, pick least loaded.
- Atomic slot reservation via Lua (`RESERVE_SLOT_SCRIPT` / `RELEASE_SLOT_SCRIPT`). **Never** separate GET+INCR.
- Key builders live in each service's `constants/redis-keys.constants.ts` (never inline key strings). Key/TTL inventory: see reference.

### Client & server

- Build clients from `createChannel(address)` + `createClientFactory().use(authMiddleware).create(definition, channel)`. **Never** create per-request. A consumer with one fixed target builds it once at bootstrap (session-engine → gateway); a consumer fanning out to many targets caches per `address:port` itself (the API's `EngineClientManager.getGrpcClient()`); `acme-rpc` has no shared client-cache helper.
- Compute service defs as module-scope singletons (`fromProtobufTsServiceDefinition(Service)`).
- Server: `addProtobufTsService`.

## Delivery Providers (`packages/acme-providers/`)

Subpath exports per area (`interfaces`, `providers`, `errors`); no root `.` export, so import from the specific subpath. See `package.json`'s `exports` map.

### Layout

- Type-only contracts live **only** in `interfaces/`.
- One `<provider>-<channel>.adapter.ts` per (provider, channel) under `providers/clients/{email,sms,push,webhook}/<provider>/`.
- Shared external-SDK wrappers in `providers/clients/sdk/` (`<system>.client.ts`). Factories in `providers/factories/` (`createEmailProvider` / `createSmsProvider` / `createPushProvider` / `createWebhookProvider`).

### Factories & registry

- **Never** instantiate providers directly; always go through `providers/factories/`. They auto-resolve defaults via `ProviderRegistry` (YAML) + env config.
- Source of truth: YAML in `config/{email,sms,push}/`, keyed by `provider:channel`. **Never** hardcode endpoints, sender identities, or formats: query the registry.
- Access via `getProviderRegistry()` (lazy singleton). For tests: `setProviderRegistry()`; `resetProviderRegistry()` forces re-load.
- Concurrency/rate limits belong in **gateway** config, not here.

### File naming

- Each SDK-backed provider is an `adapter` of its interface: `<provider>-<channel>.adapter.ts` → class `<Provider><Channel>` (e.g. `twilio-sms.adapter.ts` → `TwilioSms`). External-SDK wrappers used by multiple adapters live in `providers/clients/sdk/<system>.client.ts`.
