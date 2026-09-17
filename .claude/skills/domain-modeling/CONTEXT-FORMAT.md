# Glossary Format

The domain language lives in one file, `docs/glossary.md`. There is no `CONTEXT.md` or `CONTEXT-MAP.md` in this repo, and no per-context glossaries: every term goes in that one table.

## Structure

```md
| Term         | Meaning |
| :----------- | :------ |
| **Order**    | {A one or two sentence description of the term} _Avoid_: purchase, transaction |
| **Invoice**  | A request for payment sent to a customer after delivery. _Avoid_: bill, payment request |
| **Customer** | A person or organization that places orders. _Avoid_: client, buyer, account |
```

One row per term: the term in bold, then its meaning. Aliases to avoid go at the end of the meaning cell, after `_Avoid_:`. Replace the template's placeholder row when you add the first real term.

## Rules

- **Be opinionated.** When multiple words exist for the same concept, pick the best one and list the others under `_Avoid_`.
- **Keep definitions tight.** One or two sentences max. Define what it IS, not what it does.
- **Only include terms specific to this project's domain.** General programming concepts (timeouts, error types, utility patterns) don't belong even if the project uses them extensively. Before adding a term, ask: is this a concept unique to this project, or a general programming concept? Only the former belongs.
- **Group terms under subheadings** when natural clusters emerge: split the table into one table per `###` subheading. If all terms belong to a single cohesive area, one table is fine.
