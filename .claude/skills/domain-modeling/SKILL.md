---
name: domain-modeling
description: Maintain this repo's documented language as design decisions land. Use when discussing codebase terminology, writing or editing `docs/glossary.md`, or recording or editing an ADR.
---

# Domain Modeling

Actively sharpen the project's documented language as you design: challenge terms, stress-test them with edge-case scenarios, and update the docs the moment a decision crystallises. Merely *reading* the docs for vocabulary is not this skill; reach for it when the language is being *changed*, not just consumed.

## Where the documented language lives in this repo

| Source                           | What it covers                                        |
| :------------------------------- | :---------------------------------------------------- |
| `docs/glossary.md`               | Cross-cutting nouns of the domain                     |
| `docs/conventions/core.md`       | Naming rules for modules and classes                  |
| `docs/explanation/<topic>.md`    | Current narrative for a subsystem                     |
| `docs/reference/architecture.md` | System shape: components, flows, layout               |
| `docs/adr/`                      | Dated log of why a hard-to-reverse choice was made    |

There is no `CONTEXT.md` or `CONTEXT-MAP.md`; never create one. Before a session, grep `docs/glossary.md` for the area's terms, and read the relevant `docs/explanation/` doc and any ADRs already filed for the area.

## During the session

### Challenge against the existing language

When the user uses a term that conflicts with the glossary or `core.md`, call it out. Example: "The glossary defines 'cancellation' as X, but you seem to mean Y. Which is it?"

### Sharpen fuzzy language

Propose precise canonical terms; pull from the existing glossary first, only invent when nothing fits. Example: "You're saying 'account': do you mean the Customer or the User? Those are different things."

### Stress-test with concrete scenarios

Force precision with edge cases that touch the boundaries between concepts: invent a specific scenario and make the user say what happens in it.

### Cross-reference with code

When the user states how something works, verify against the code (`docs/reference/architecture.md` maps where each component lives). Surface contradictions: "Your code cancels entire Orders, but you just said partial cancellation is possible. Which is right?"

### Record each doc change as it resolves

When something resolves, write the change down at once; don't batch. Before the Design is approved (Specify), list it as a row of the Design's Change table (`agent-workflow/DESIGN.md`), and the PR lands it; after, edit the doc in place.

- **New cross-cutting noun?** Add to the table in `docs/glossary.md`.
- **Naming or structure rule?** Update the relevant doc in `docs/conventions/`.
- **Subsystem narrative has drifted from reality?** Update the relevant `docs/explanation/<topic>.md`.
- **System shape has drifted from reality?** Update `docs/reference/architecture.md`.
- **Hard-to-reverse choice with non-obvious rejected alternatives?** Open an ADR.

A good glossary entry is opinionated: one term per concept, its rivals listed as aliases to avoid (after `_Avoid_:` at the end of the meaning cell). It says what the term IS in one or two sentences, and covers project terms only, never general programming concepts. Keep the language in these docs.

### Offer ADRs sparingly

Offer an ADR only when the three-part bar that opens [docs/adr/README.md](../../../docs/adr/README.md) holds. The same file holds the template and numbering.
