# 0001. Postgres over DynamoDB for the primary store

- Status: accepted
- Date: 2026-01-15

## Context

Acme needs a primary store for Organizations, Members, Sessions, and Message
history. Access patterns are relational (joins across tenants, members, and
sessions) and not yet fully known. The team is small and already fluent in SQL.

## Decision

Use Postgres (via Drizzle ORM) as the system of record. Model relations
explicitly; partition the high-volume `messages` table by time.

## Consequences

- Easier: ad-hoc relational queries, transactions, and schema evolution through
  reviewed migrations (`docs/conventions/backend.md`).
- Harder: horizontal write scaling later may require partitioning or sharding
  work that a wide-column store would have given for free.
- Committed to: one relational schema as the source of truth; the Session Engine
  keeps only ephemeral state in Redis, never a second copy of record data.

## Alternatives considered

- **DynamoDB** — rejected: single-table design demands the access patterns be
  known up front, and the relational reporting needs would fight the model.
- **Postgres + a separate document store** — rejected as premature: two stores
  to keep consistent before there is evidence one store cannot cope.
