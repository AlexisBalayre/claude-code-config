---
name: grill-with-docs
description: Grilling session that stress-tests a plan against this repo's documented language (glossary, conventions, explanation docs, ADRs) and updates those docs inline as decisions crystallise. Use when the user wants to pressure-test a design against the project's existing vocabulary and prior decisions.
---

<what-to-do>

Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.

Ask the questions one at a time, waiting for feedback on each question before continuing.

If a question can be answered by exploring the codebase, explore the codebase instead.

</what-to-do>

<supporting-info>

## Where the "domain model" lives in this repo

During codebase exploration, also look for existing documentation:

| Source                           | What it covers                                                            |
| :------------------------------- | :------------------------------------------------------------------------ |
| `docs/README.md` (Glossary)      | Cross-cutting nouns: Organization, Member, Session, Participant, Gateway, Provider, etc. |
| `docs/conventions/naming.md`     | Role taxonomy and naming stems for modules / classes                      |
| `docs/explanation/<topic>.md`    | Current narrative for a subsystem (system architecture, security model)   |
| `docs/adr/`                      | Dated log of why a hard-to-reverse choice was made                        |

Before grilling, skim the Glossary, the relevant `docs/explanation/` doc, and any ADRs already filed for the area. For *why*/*how* questions that span multiple services, delegate to the `architecture-explainer` subagent rather than re-reading docs in the main context.

## During the session

### Challenge against the existing language

When the user uses a term that conflicts with the Glossary or `naming.md`, call it out. Example: "Glossary defines `Session` as the live client connection context; you're using it for the engine process that runs it. Which do you mean?"

### Sharpen fuzzy language

Propose precise canonical terms; pull from the existing Glossary first, only invent when nothing fits. Common ambiguities here:

- "Session" (the user-facing connection context vs. the engine process running it)
- "Message" (the inbound event vs. the rendered delivery to a Channel)
- "Tenant" vs. `Organization` (BetterAuth term wins)
- "Member" vs. "Participant" vs. "User"

### Stress-test with concrete scenarios

Force precision with edge cases that touch service boundaries:

- "What happens to an in-flight Message dispatch when the Participant disconnects mid-send?"
- "If the Gateway and Session Engine disagree on a Session's active state, who wins?"
- "A Provider for one Channel is down: does the Session block, drop, or fan out to the other Channels?"

### Cross-reference with code

When the user states how something works, verify against the code in the relevant service (`apps/acme-api/`, `services/acme-session-engine/`, `services/acme-gateway/`, `apps/acme-web/`). Surface contradictions: "`session.manager.ts` tears down on the last Participant leaving, but you said Sessions stay warm. Which is right?"

### Update the existing docs inline

When something resolves, update it in place. Capture as it happens; don't batch.

- **New cross-cutting noun?** Add to the Glossary table in `docs/README.md`.
- **Naming stem or role suffix decision?** Update `docs/conventions/naming.md`.
- **Subsystem narrative has drifted from reality?** Update the relevant `docs/explanation/<topic>.md`.
- **Hard-to-reverse choice with non-obvious rejected alternatives?** Open an ADR.

Do not create a parallel `CONTEXT.md`. See [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md) for the underlying glossary discipline if you need a reminder of what a good entry looks like.

### Offer ADRs sparingly

Only offer an ADR when all three are true:

1. **Hard to reverse**: the cost of changing your mind later is meaningful
2. **Surprising without context**: a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off**: there were genuine alternatives and one was picked for specific reasons

If any of the three is missing, skip it. See [ADR-FORMAT.md](./ADR-FORMAT.md) and [docs/adr/README.md](../../../docs/adr/README.md) for the bar and template.

</supporting-info>
