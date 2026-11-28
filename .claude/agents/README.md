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
| `migration-reviewer` | After editing a schema definition or a migration (any ORM or migration tool), before committing. Checks the project's database conventions, backwards compatibility (expand/contract), locking, backfills, reversibility, and index safety. | Sonnet · Read/Glob/Grep |
| `security-reviewer` | After editing auth, API endpoints or handlers, socket handlers, forms, or secret handling. Checks injection, broken access control, secrets exposure, OWASP Top 10 against `docs/explanation/security-model.md`. | Opus · Read/Glob/Grep/Bash |
| `architecture-explainer` | A why/how question about component boundaries, data flow, or cross-component interactions. Answers grounded in `docs/reference/architecture.md`, `docs/explanation/`, `docs/adr/`, and code, never invented. | Sonnet · Read/Glob/Grep |

## Dispatched agents

These are workers for a specific skill or hook, not proactive triggers. The `review-*` family
is spawned (relevance-gated, model-tiered) by the [`pr-ci-review` skill](../skills/pr-ci-review/SKILL.md);
`comment-pruner` is dispatched by the `comment-pruner.sh` Stop hook.

| Agent | What it does | Model · tools |
| :---- | :----------- | :------------ |
| `comment-pruner` | Prunes low-value comments added in the current session, delete-when-uncertain, with a hard floor for tooling directives in any language (`@ts-expect-error`, `# noqa`, `//nolint`, `#[allow(...)]`, …). The one agent here that edits. Also usable manually for a repo-wide sweep. | Sonnet · Read/Edit/Grep/Glob/Bash |
| `review-correctness` | Logic and behavior defects deterministic tooling cannot catch, including silent regressions on the surface no test covers; reports every path a root cause reaches. | Opus · Read/Glob/Grep/Bash |
| `review-security` | Real, reachable security issues at trust boundaries, against `docs/explanation/security-model.md`. Agent config is a guardrail, not a boundary. | Opus · Read/Glob/Grep/Bash |
| `review-qa` | Whether the tests a change ships would catch it breaking (Test adequacy), the edge cases they skip, and each missing test as code. | Sonnet · Read/Glob/Grep/Bash |
| `review-perf` | Cost on a path that runs often: allocations per call, unbounded growth, N+1 queries, leaks, blocking I/O. Spawned only when the change touches a hot path. | Sonnet · Read/Glob/Grep/Bash |
| `review-quality` | The change against the written rules (`docs/conventions/*`, accepted ADRs, `AGENTS.md`), the prose it adds, and the contracts and infrastructure around it, quoting what it cites. | Sonnet · Read/Glob/Grep/Bash |
| `review-simplicity` | What the changed code can lose: over-engineering and structural regressions, each cut with its replacement and line delta. Full rounds only. | Sonnet · Read/Glob/Grep/Bash |
| `review-requirements` | The change against the ticket or issue its PR names: a claimed Done item it does not meet, or a line that contradicts the ticket. Spawned only when the PR names one. | Sonnet · Read/Glob/Grep/Bash |
| `review-validator` | Adversarial gate: tries to refute each finding against the actual code before it is posted. | Opus · Read/Glob/Grep/Bash |

**Tuning:** `tools` is the minimum each needs; `model` is matched to the work (Haiku = fast/cheap,
Sonnet = balanced, Opus = deep reasoning). To add an agent, see [`.claude/README.md`](../README.md)
("New agent").
