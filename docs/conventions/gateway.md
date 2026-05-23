# Gateway Conventions

Rules for everything under **`services/acme-gateway`** — the edge service. The thin
`@docs/conventions/gateway.md` import in `.claude/rules/gateway-conventions.md` points here.

The Gateway terminates client connections (**WebSocket + gRPC**), runs the **auth handshake**,
and routes frames to the **Session Engine**. It holds connection state but no Session state.

## Factories only — never `new XService()`

Services are wired through factory functions that take their dependencies explicitly. The
`convention-spot-check` Stop hook flags any `new [A-Z]…Service(`.

```ts
// connection.service.ts
export function createConnectionService(deps: ConnectionDeps): ConnectionService { … }

// ✗ const svc = new ConnectionService(pool);   // flagged by the hook
const svc = createConnectionService({ pool, sessionClient });
```

Explicit deps keep services testable (pass fakes) and the dependency graph readable. Classes
are reserved for stateful resources with a lifecycle (pools), not for grouping logic.

## WebSocket upgrades validate the session first

An `"upgrade"` handler **must** validate the session before calling `socket.upgrade()`. An
unvalidated upgrade is an open door — the `security-reviewer` subagent rejects it.

```ts
server.on("upgrade", async (req, socket, head) => {
  const session = await auth.validateUpgrade(req);
  if (!session) {
    socket.write("HTTP/1.1 401 Unauthorized\r\n\r\n");
    socket.destroy();
    return;
  }
  wss.handleUpgrade(req, socket, head, (ws) => wss.emit("connection", ws, session));
});
```

## Service-to-service auth

Every internal gRPC call to the Session Engine (or any internal peer) wires
`createServerAuthMiddleware`. No internal call is anonymous. See [grpc.md](grpc.md).

## Pools and timeouts come from config

Connection and resource pools are **managed objects with a lifecycle** (create → drain →
close). Pool sizes, idle timeouts, and call deadlines are read from config — **never
hardcoded**.

```ts
// ✗ const pool = createPool({ max: 50, idleMs: 30000 });
const pool = createPool({ max: config.engine.poolMax, idleMs: config.engine.idleMs });
```
