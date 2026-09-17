---
name: domain-modeling
description: Maintain this repo's documented language as design decisions land. Use when pinning down domain terminology, recording an architectural decision, or when another skill needs the docs kept current during a session.
---

# Domain Modeling

Actively sharpen the project's documented language as you design: challenge terms, stress-test them with edge-case scenarios, and update the docs the moment a decision crystallises. Merely *reading* the docs for vocabulary is not this skill; reach for it when the language is being *changed*, not just consumed.

## Where the documented language lives in this repo

| Source                        | What it covers                                                            |
| :---------------------------- | :------------------------------------------------------------------------ |
| `docs/glossary.md`            | Cross-cutting domain nouns and the aliases to avoid                       |
| `docs/conventions/`           | Naming and structure rules (`core.md`) plus per-area conventions          |
| `docs/explanation/<topic>.md` | Current narrative for a subsystem (system architecture, security model)   |
| `docs/adr/`                   | Dated log of why a hard-to-reverse choice was made                        |

Before a session, skim the Glossary, the relevant `docs/explanation/` doc, and any ADRs already filed for the area. For *why*/*how* questions that span multiple components, delegate to the `architecture-explainer` subagent rather than re-reading docs in the main context.

## During the session

### Challenge against the existing language

When the user uses a term that conflicts with the Glossary or `core.md`, call it out. Example: "Glossary defines `Order` as the customer's request; you're using it for the fulfilment record created from it. Which do you mean?"

### Sharpen fuzzy language

Propose precise canonical terms; pull from the existing Glossary first, only invent when nothing fits. Typical ambiguities to hunt for:

- One word for two things (the request a user makes vs. the record the system keeps for it)
- Two words for one thing ("Account" vs. "Customer" vs. "User")
- A term borrowed from a library or vendor that clashes with the domain's own word (decide which wins and record it)

### Stress-test with concrete scenarios

Force precision with edge cases that touch component boundaries, e.g.:

- "What happens to an in-flight operation when the caller disconnects mid-way?"
- "If two components disagree on an entity's current state, which one is the source of truth?"
- "A request fans out to two downstream systems and one fails: is the result succeeded, partial, or failed?"

### Cross-reference with code

When the user states how something works, verify against the code in the relevant component (`docs/reference/architecture.md` maps where each lives). Surface contradictions: "the cleanup job deletes drafts after 24 hours, but you said drafts are kept forever. Which is right?"

### Update the existing docs inline

When something resolves, update it in place. Capture as it happens; don't batch.

- **New cross-cutting noun?** Add to the Glossary table in `docs/glossary.md`.
- **Naming or structure decision?** Update `docs/conventions/core.md` (or the area doc in `docs/conventions/` it belongs to).
- **Subsystem narrative has drifted from reality?** Update the relevant `docs/explanation/<topic>.md`.
- **Hard-to-reverse choice with non-obvious rejected alternatives?** Open an ADR.

Do not create a parallel `CONTEXT.md`. See [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md) for the underlying glossary discipline if you need a reminder of what a good entry looks like.

### Offer ADRs sparingly

Only offer an ADR when all three are true:

1. **Hard to reverse**: the cost of changing your mind later is meaningful
2. **Surprising without context**: a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off**: there were genuine alternatives and one was picked for specific reasons

If any of the three is missing, skip it. See [ADR-FORMAT.md](./ADR-FORMAT.md) and [docs/adr/README.md](../../../docs/adr/README.md) for the bar and template.
