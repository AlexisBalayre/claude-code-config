# Agent catalog

Subagents run in their own context window and return only a summary, so deep analysis never
bloats the main conversation. They are read-only (no edits). Claude dispatches them
automatically when the trigger matches (each is described "Use PROACTIVELY"); you can also force
one — e.g. "run the security-reviewer on these changes."

| Agent | When to use | Model · tools |
| :---- | :---------- | :------------ |
| `convention-checker` | Before a commit, or after editing ≥3 files across `apps/`/`services/`/`packages/`. Fast audit against `docs/conventions/*`. | Haiku · Read/Glob/Grep |
| `migration-reviewer` | After editing `packages/acme-db/src/schema/**` or running `pnpm db:generate`. Checks schema + migration safety and backwards-compatibility. | Sonnet · Read/Glob/Grep |
| `security-reviewer` | After editing auth, API routes, WebSocket handlers, forms, or secret handling. Checks injection, broken access control, secrets exposure, OWASP Top 10. | Opus · Read/Glob/Grep/Bash |
| `architecture-explainer` | A why/how question about service boundaries, data flow, or cross-service interactions. Answers grounded in `docs/`, never invented. | Sonnet · Read/Glob/Grep |

**Tuning:** `tools` is the minimum each needs; `model` is matched to the work (Haiku = fast/cheap,
Sonnet = balanced, Opus = deep reasoning). To add an agent, see [`.claude/README.md`](../README.md)
("New agent").
