# Session Engine Conventions

Rules for everything under **`services/acme-session-engine`** — the stateful service that owns
the per-Session lifecycle. The thin `@docs/conventions/session-engine.md` import in
`.claude/rules/session-engine-conventions.md` points here.

The engine owns each Session's lifecycle through **explicit state machines** and reaches
external systems through **Adapters**. It is the only tier that holds Session state.

## Never use EventEmitter

**Do not import `node:events` / `events`.** The `convention-spot-check` Stop hook flags it.
`EventEmitter` gives you **untyped event names** (a typo is a silent no-op) and **listener
leaks** (no compiler help to remove handlers). Use **typed callback interfaces** instead:

```ts
// ✗ import { EventEmitter } from "node:events";

export interface SessionListener {
  onParticipantJoined(participant: Participant): void;
  onMessageDispatched(event: MessageDispatched): void;
}

export function createSessionMachine(listener: SessionListener): SessionMachine { … }
```

The compiler now checks every event name and payload, and the listener set is a known shape.

## Atomic Redis slot reservations

Session slot reservations use **atomic Lua scripts**, never `GET`-then-`SET` — the gap between
read and write is a race that lets two clients claim the same slot.

```ts
// ✗ const used = await redis.get(key); if (used < max) await redis.set(key, used + 1);
const reserved = await redis.eval(RESERVE_SLOT_SCRIPT, 1, slotKey, String(maxSlots));
if (!reserved) throw new SlotUnavailableError(sessionId);
```

`RESERVE_SLOT_SCRIPT` reads the counter, checks the cap, and increments in one round trip that
Redis runs atomically.

## Session affinity & discovery

- **Affinity:** a Session is **pinned to one engine instance** for its lifetime. All frames for
  that Session route to the same instance — that's what makes in-memory state machines safe.
- **Discovery:** instances register in a registry keyed by Session, with a **TTL that the owner
  refreshes** on a heartbeat. If an instance dies, its entries expire and the Session can be
  reclaimed elsewhere. The Gateway reads the registry to route frames.

```ts
await registry.refresh(sessionId, instanceId, { ttlMs: config.affinity.ttlMs });
```
