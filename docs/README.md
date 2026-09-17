# Docs

Documentation for this project, organised with [Diátaxis](https://diataxis.fr/):

| Folder              | Answers                | Read when…                                  |
| :------------------ | :--------------------- | :------------------------------------------ |
| `conventions/`      | *How must I code this?* | You're editing files in an area (auto-loaded by `.claude/rules/`). |
| `reference/`        | *What is the shape?*    | You need the structure of a component or data flow. |
| `explanation/`      | *Why is it like this?*  | You're questioning a design decision.       |
| `adr/`              | *What did we decide?*   | You need the record of a past decision.     |

`conventions/` is the **single source of truth** for code style; conventions files contain
obligations only, while system description lives in `reference/`. Unfamiliar term? See the
[Glossary](glossary.md).

Pages still carrying `TODO(adapt)` markers are skeletons; `/adapt-to-project` fills them from the
codebase.

## Conventions index

Per-area rules. Read the one for the area you are changing.

- [core](conventions/core.md) — language-agnostic obligations: altitude/YAGNI, comments, errors, logging, naming and structure
- [testing](conventions/testing.md) — testing obligations
- Area docs (`conventions/<area>.md`, each with a matching `.claude/rules/<area>-conventions.md` loader) are added per project by `/adapt-to-project`

## Reference and explanation

- [reference/architecture.md](reference/architecture.md) — components, layout, request/data flow, external dependencies, deployment
- [explanation/security-model.md](explanation/security-model.md) — trust boundaries, authn/authz, secrets, input validation (the `security-reviewer` spec)
- [adr/](adr/README.md) — how to record a decision, plus the template
- [glossary.md](glossary.md) — shared vocabulary
