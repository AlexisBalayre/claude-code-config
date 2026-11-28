# Arch mode

Surface **deepening opportunities**: refactors that turn shallow modules into deep ones, for testability and AI-navigability. Call the Skill tool with "codebase-design" first and use its vocabulary exactly.

## 1. Explore

Deepening pays off where the code keeps changing, so decide where to look before you look:

- If the user named a direction (a module, a subsystem, a pain point), take it.
- Otherwise count the last 3 months of churn per path, `git log --since=3.months --name-only --format= -- <scope> | grep . | sort | uniq -c | sort -rn`, and take the 3-5 most-changed files or directories as the hot spots.

Read the area's ADRs and grep `docs/glossary.md` for its terms. Then walk the code with `Explore` subagents and note where you feel friction:

- understanding one concept means bouncing between many small modules;
- a module is **shallow**, its interface nearly as complex as its implementation;
- pure functions were extracted for testability, but the bugs hide in how they are called (no **locality**);
- tightly coupled modules leak across their seams;
- code is untested, or hard to test through its current interface.

Apply the deletion test to each suspect: would deleting it concentrate complexity, or only move it? "Concentrates" is the signal.

Done when each candidate has its files, its problem, the deepening in plain words, its benefit in locality and leverage (and how tests improve), and a strength: `Strong`, `Worth exploring` or `Speculative`. Propose no interfaces yet.

## 2. Pick sheet

Build the `html-artifact` pick sheet with the choices `file`, `grill` and `skip`. Beside each row show the files, problem, solution, benefits and strength, and a before/after inline SVG drawn in the markup: boxes for modules, a dashed line for a seam, labels that are module names. A candidate that contradicts an ADR carries a warning line naming it and why the friction is worth reopening it; drop the ones an ADR settles. The page header names your top recommendation and why.

Done when the user's picks are back, verbatim.

## 3. Grill

For each `grill` row, call the Skill tool with "grilling" to walk its decision tree with the user: constraints, dependencies, the shape of the deepened module, what sits behind the seam, which tests survive. [DEEPENING.md](../codebase-design/DEEPENING.md) classifies the dependencies and how the module is tested across its seam; [INTERFACE-DESIGN.md](../codebase-design/INTERFACE-DESIGN.md) designs alternative interfaces in parallel.

Call the Skill tool with "domain-modeling" as decisions land:

- a deepened module named after a concept missing from the glossary or `core.md`: add it there;
- a fuzzy term sharpened: update it in place;
- a candidate rejected for a load-bearing reason: offer an ADR, "Want me to record this as an ADR so future audits don't re-suggest it?", only when a future explorer would need the reason.

Done when each grilled row is decided: filed with the grilling's outcome in its body, or dropped.

## 4. File

File each `file` row and each grilled row kept, as the shared rules say: type `Improvement`, its `## Before / After` table from the row.
