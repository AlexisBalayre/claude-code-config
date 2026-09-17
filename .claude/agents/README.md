# Agent catalog

Subagents run in their own context window and return only a summary, so deep analysis never
bloats the main conversation.

## Proactive agents

Claude dispatches these automatically when the trigger matches (each is described "Use
PROACTIVELY"); you can also force one — e.g. "run the security-reviewer on these changes."
They are read-only (no edits).

| Agent | When to use | Model · tools |
| :---- | :---------- | :------------ |
| `convention-checker` | Before a commit, or after editing ≥3 source files. Fast audit against `docs/conventions/*`. | Haiku · Read/Glob/Grep |
| `migration-reviewer` | After editing schema definitions or migration files (any ORM or migration tool). Checks backwards compatibility (expand/contract), locking, backfills, reversibility, and index safety. | Sonnet · Read/Glob/Grep |
| `security-reviewer` | After editing auth, API endpoints or handlers, socket handlers, forms, or secret handling. Checks injection, broken access control, secrets exposure, OWASP Top 10 against `docs/explanation/security-model.md`. | Opus · Read/Glob/Grep/Bash |
| `architecture-explainer` | A why/how question about component boundaries, data flow, or cross-component interactions. Answers grounded in `docs/reference/architecture.md`, `docs/explanation/`, `docs/adr/`, and code, never invented. | Sonnet · Read/Glob/Grep |

## Dispatched agents

These are workers for a specific skill or hook, not proactive triggers. The `review-*` family
is spawned (relevance-gated, model-tiered) by the [`pr-ci-review` skill](../skills/pr-ci-review/SKILL.md);
`comment-pruner` is dispatched by the `comment-pruner.sh` Stop hook.

| Agent | What it does | Model · tools |
| :---- | :----------- | :------------ |
| `comment-pruner` | Prunes low-value comments added in the current session, delete-when-uncertain, with a hard floor for tooling directives in any language (`@ts-expect-error`, `# noqa`, `//nolint`, `#[allow(...)]`, …). The one agent here that edits. Also usable manually for a repo-wide sweep. | Sonnet · Read/Edit/Grep/Glob/Bash |
| `review-context` | Spec/protocol contradictions and infrastructure anti-patterns, each finding grounded in the contract it breaks. | Sonnet · Read/Glob/Grep/Bash |
| `review-conventions` | Audits changed files against `docs/conventions/*`, accepted ADRs, and `AGENTS.md`, quoting the exact rule violated. | Sonnet · Read/Glob/Grep/Bash |
| `review-correctness` | Logic/behavior defects deterministic tooling can't catch, including silent regressions on the surface tests don't cover. | Opus · Read/Glob/Grep/Bash |
| `review-docs` | Accuracy and usefulness of prose a change introduces; strict on noise comments and stale docs. | Sonnet · Read/Glob/Grep/Bash |
| `review-maintainability` | Structural regressions — wrong layer, duplication, needless indirection — each with a citable mechanism. | Sonnet · Read/Glob/Grep/Bash |
| `review-security` | Real, reachable security issues at trust boundaries, cross-referenced against the threat model. | Opus · Read/Glob/Grep/Bash |
| `review-validator` | Adversarial gate: tries to refute each finding against the actual code before it is posted or auto-fixed. | Opus · Read/Glob/Grep/Bash |

**Tuning:** `tools` is the minimum each needs; `model` is matched to the work (Haiku = fast/cheap,
Sonnet = balanced, Opus = deep reasoning). To add an agent, see [`.claude/README.md`](../README.md)
("New agent").
