# Acme Docs

Documentation for **Acme**, a fictional real-time messaging platform used to demonstrate
this Claude Code configuration. The docs are organised with [Diátaxis](https://diataxis.fr/):

| Folder              | Answers                | Read when…                                  |
| :------------------ | :--------------------- | :------------------------------------------ |
| `conventions/`      | *How must I code this?* | You're editing files in an area (auto-loaded by `.claude/rules/`). |
| `reference/`        | *What is the shape?*    | You need the structure of a service or schema. |
| `explanation/`      | *Why is it like this?*  | You're questioning a design decision.       |
| `adr/`              | *What did we decide?*   | You need the record of a past decision.     |

`conventions/` is the **single source of truth** for code style; conventions files contain
obligations only, while system description lives in `reference/`. Unfamiliar term? See the
[Glossary](glossary.md).

## Conventions index

Per-area rules. Read the one for the area you are changing.

- [core](conventions/core.md) — universal TypeScript, type system, JSDoc/comments, logging, the `kebab-case.role.ts` naming taxonomy
- [backend](conventions/backend.md) — API (`acme-api`) + database (`acme-db`)
- [services](conventions/services.md) — gateway, session engine, gRPC (`acme-rpc`), delivery providers
- [frontend](conventions/frontend.md) · [testing](conventions/testing.md)
