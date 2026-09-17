# Architecture Decision Records

An ADR records a decision that is **hard to reverse**, **surprising without context**, and
**the result of a real trade-off**. If a choice is none of those, it does not need an ADR.

ADRs are append-only. To change a past decision, write a new ADR that supersedes the old one
(link both ways); never rewrite history.

## Naming

`NNNN-short-kebab-title.md`, e.g. `0001-use-postgres-for-primary-store.md`. Numbers are sequential and
never reused. No ADRs ship with the template; the first real decision is `0001`.

## Template

```markdown
# NNNN. Title

- Status: proposed | accepted | superseded by [NNNN](NNNN-...md)
- Date: YYYY-MM-DD

## Context
What forces are at play? What makes this hard to reverse?

## Decision
What we chose, stated plainly.

## Consequences
What becomes easier, what becomes harder, what we are now committed to.

## Alternatives considered
The genuine options we rejected, and why.
```

The `domain-modeling` skill (which `grill-with-docs` runs) offers to open an ADR when a decision clears the three-part bar above.
