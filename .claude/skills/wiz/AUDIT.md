# Wiz audit

`/wiz audit [question]`: answer a question about the project's cloud security posture. The routing traps, IDs, false positives and guardrail are in [SKILL.md](SKILL.md).

Every query takes the same four steps:

1. **Route.** Call `discover()` with no argument to list the domains. Pick the one domain that owns the answer by the *type of finding* you want, not by nouns in the question (SKILL.md's Routing traps).
2. **Spec.** Call `discover(domain)` for the exact tool `name` and its parameter schema. Read it: params that sound singular are usually **arrays** (`group_by`, `severity`, `status`, `asset_name`, `container_repository`), some are **required** (e.g. `list_issues_grouped` needs `group_by` + `type`), and the `first` / `limit` cap differs per tool (often 20). Never guess.
3. **Query.** Call `execute(tool_name, parameters)` with the exact `name` from step 2, scoped per SKILL.md's project IDs. If `execute` rejects the name, re-run `discover(domain)` to confirm spelling (some names repeat across domains).
4. **Summarize.** A result too large for one response is auto-saved to a `tool-results/*.txt` file instead of returned. Parse it with jq, never re-read it whole. Report a ranked summary (severity, then affected resource) with the Wiz finding IDs so the user can open each in the portal. Never paste raw JSON back.

Prefer a `*_grouped` tool with a small `first` over a raw list: fewer tokens, and it answers "where is the load" directly.
