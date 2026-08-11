# Glossary

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
> appear. The `domain-modeling` skill (which `grill-with-docs` delegates to) and the
> `improve-codebase-architecture` skill both read it to keep naming consistent.
