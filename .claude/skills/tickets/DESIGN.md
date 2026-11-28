# Design mode

Turn a plan into one Epic and file it. An Epic is work that spans more than one PR. Work that fits one PR is a ticket: file it in `file` mode, and its Design follows `agent-workflow` DESIGN.md when the worker picks it up.

## 1. Grill

Call the Skill tool twice, for `grilling` and `domain-modeling`. Work from what the conversation already settled: a decision it settled is not asked again.

Done when the grilling frontier is empty and every term the Epic will use matches `docs/glossary.md` or was changed there.

## 2. Ground truth

Explore the area if the conversation has not: `docs/reference/` for the current service boundaries and wiring, `docs/explanation/` for the rationale of the surrounding subsystem, `docs/adr/` for decisions already locked in.

Done when you can name each module the Epic builds or changes, and each ADR it touches.

## 3. Seams

Call the Skill tool with `codebase-design` for the vocabulary, and sketch the modules to build or change, looking for deep modules: a simple, testable interface over a complex implementation.

Then sketch the seams the work will be tested at. Existing seams are preferred to new ones. Use the highest seam possible. If new seams are needed, propose them at the highest point you can. The fewer seams across the codebase, the better: the ideal number is one.

Done when the user confirmed the module breakdown, the seams, and which modules need test coverage first.

## 4. Draft

Write the Epic from the Epic block of `templates.md` (`## Goal`, `## Why`, `## Slices`, `## Done when`). Leave the Slices table with its header row and the diagram with no node: `slice` mode fills both. Add these sections after `## Why`:

- `## User stories`: a numbered list, one line each: *As an `<actor>`, I want `<feature>`, so that `<benefit>`.* Cover every actor the Epic changes something for.
- `## Implementation decisions`: modules to build or change, interface shapes, schema changes, API contracts, provider interactions, transport choices. Name any decision that supersedes or refines an ADR, and whether a new ADR is warranted.
- `## Testing decisions`: the seams from step 3, the modules covered and at which layer (unit, integration, e2e), and prior art cited by module. A good test checks behavior through the seam, never the implementation.
- `## Out of scope`: what the Epic deliberately leaves out.
- `## Further notes`: links to ADRs, prior Epics, related tickets, open questions.

Show the draft. Done when the user approved it.

## 5. File

Write it through `file-ticket` §2 and §5 with the `Epic` label and your estimate of the whole Epic on the 1, 2, 4, 8 or 16 scale (no slice exists yet to sum). With no tracker, the Epic is `docs/plans/<feature-slug>/spec.md`. Then tell the user the next step: `/tickets slice <the Epic's id>`.

Done when the Epic's read-back holds every section of the draft, and the user has its id or its path.
