# Acme Docs

Documentation for **Acme**, a fictional real-time messaging platform used to demonstrate
this Claude Code configuration. The docs are organised with [Diátaxis](https://diataxis.fr/):

| Folder              | Answers                | Read when…                                  |
| :------------------ | :--------------------- | :------------------------------------------ |
| `conventions/`      | *How must I code this?* | You're editing files in an area (auto-loaded by `.claude/rules/`). |
| `reference/`        | *What is the shape?*    | You need the structure of a service or schema. |
| `explanation/`      | *Why is it like this?*  | You're questioning a design decision.       |
| `adr/`              | *What did we decide?*   | You need the record of a past decision.     |

`conventions/` is the **single source of truth** for code style. Everything else explains
or records.

---

## Glossary

The shared vocabulary. Names in code, docs, and conversation should match these exactly.
When a term is fuzzy, sharpen it here first.

| Term             | Meaning                                                                                          |
| :--------------- | :----------------------------------------------------------------------------------------------- |
| **Organization** | Top-level tenant. Owns Members, Sessions, and provider configuration.                            |
| **Member**       | A user belonging to an Organization, with a role (`owner`, `admin`, `member`).                   |
| **Session**      | A live real-time context a client connects to (think: a room or channel).                        |
| **Participant**  | A Member or guest currently connected to a Session.                                              |
| **Gateway**      | Edge service terminating client connections (WebSocket + gRPC): auth handshake, frame routing.   |
| **Session Engine** | Stateful service owning the Session lifecycle and per-Session state machine; emits Events.     |
| **Provider**     | Pluggable adapter to an external delivery system, selected at runtime via factory + YAML registry.|
| **Channel**      | The delivery medium a Provider implements: `email`, `sms`, `push`, `webhook`.                    |
| **Message**      | A unit of content routed through a Session; may fan out to Channels via Providers.               |
| **Event**        | An internal domain event emitted by the Session Engine (`ParticipantJoined`, `MessageDispatched`).|

> This glossary is intentionally small. In a real project it grows as cross-cutting nouns
> appear. The `grill-with-docs` skill and `improve-codebase-architecture` skill both read it
> to keep naming consistent.

---

## Conventions index

- [General / universal](conventions/general.md) — applies to every TypeScript file
- [Naming taxonomy](conventions/naming.md) — the `kebab-case.role.ts` role system
- [API](conventions/api.md) · [Frontend](conventions/frontend.md) · [Database](conventions/database.md)
- [Testing](conventions/testing.md) · [Gateway](conventions/gateway.md) · [gRPC](conventions/grpc.md)
- [Session Engine](conventions/session-engine.md) · [Providers](conventions/providers.md)
