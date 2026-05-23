# System Architecture

*Why* Acme is split into three backend tiers instead of one server. Read this when you're
questioning a service boundary. The `architecture-explainer` subagent grounds its answers here.

## Three backend tiers

| Tier | Workspace | Holds state? | Scales by |
| :--- | :--- | :--- | :--- |
| Public API | `apps/acme-api` | No | Replicas (horizontal) |
| Edge gateway | `services/acme-gateway` | Connection state only | Open connections |
| Session engine | `services/acme-session-engine` | Session state | Session affinity + discovery |

They are separate **because their scaling axes and failure modes differ**, and folding them
together would force the cheapest tier to inherit the constraints of the most expensive one.

### Why a separate public API

`apps/acme-api` is plain request/response HTTP (Hono + Zod OpenAPI): CRUD on Organizations,
Members, and Provider config; auth; reads of historical Messages. It is **stateless**, so it
scales by adding replicas behind a load balancer and can be deployed and rolled back freely. It
must not hold a socket open or pin a Session — that's what the other two tiers are for.

### Why a separate Gateway

Realtime clients hold **long-lived WebSocket/gRPC connections**. That's a fundamentally
different resource than a short HTTP request: it scales by **concurrent connections**, not
request rate, and a deploy must drain connections gracefully. Isolating the edge lets us tune
it for connection count (file descriptors, pools) without dragging the stateless API along. The
Gateway terminates the connection, runs the **auth handshake once**, then routes frames inward.

### Why a separate Session Engine

A live Session needs **in-memory state** — a state machine, the participant set, slot counters.
That state is the thing you cannot trivially replicate, so it lives in **one tier, on one
instance per Session**. Keeping it out of the Gateway means the edge stays nearly stateless and
restartable; keeping it out of the API means CRUD traffic never competes with live Sessions.

## Why Providers are pluggable

Delivery systems (email, SMS, push, webhook vendors) change far more often than the core
domain. Putting them behind a **factory + YAML registry** (`packages/acme-providers`) means
adding or swapping a vendor is a new adapter + a config file — not a change to the engine. The
engine asks for a Channel; the registry resolves the concrete Provider at runtime.

## How it scales

- **API** — stateless, so just add replicas.
- **Gateway** — add instances as connection count grows; each owns a slice of connections.
- **Session Engine** — a Session is **pinned to one instance** (affinity) and instances
  **register in a discovery registry with a TTL heartbeat**. Add instances to host more
  Sessions; if one dies, its registry entries expire and its Sessions are reclaimed elsewhere.

## The trade-off

Three tiers cost network hops, a gRPC contract to maintain (`packages/acme-rpc`), and
distributed-systems concerns (affinity, discovery, partial failure). We accept that because the
alternative — one process that is simultaneously stateless-and-replicated *and* stateful-and-
pinned — can't be scaled or deployed sanely.
