---
name: pr-description
description: Write a PR title and description in this repo's house style, then create or update the PR via gh. Use when asked to draft or rewrite a PR title or body, to open a PR, or when another skill opens a PR. Assumes commits exist on the branch.
---

# Write a PR title and description

> **Setup:** the tracker steps use an issue-tracker MCP server (the examples use a
> Linear-style API; `mcp__linear-server__*` tool names are placeholders — swap them for your
> tracker's tools). The ticket prefix (`PROJ`) comes from `TRACKER_ISSUE_PREFIX` in `.env`.
> No tracker? Skip step 2 and drop the ticket id everywhere.

## Workflow

1. **Gather context** (run together):

   ```sh
   git branch --show-current                              # MUST NOT be "main"; abort if it is
   git log main..HEAD --pretty=format:'%h %s%n%b'         # commits on this branch
   git diff main...HEAD --stat                            # changed files + churn
   gh pr view --json number,url,state,title,body 2>/dev/null   # non-zero exit = no PR yet
   ```

   Read the diff for the key files (`git diff main...HEAD -- <path>`); on large diffs lean on `--stat` plus the important files.

2. **Ground in the tracker (read-only).** Grep branch + commit subjects for `PROJ-\d+`. If found, fetch issue + parent epic with `mcp__linear-server__get_issue` for *What/Why* and the canonical link. NEVER call any `save_*` tool. No id: derive from diff + commits.

3. **Select sections** from the diff (table below). Only include sections that apply.

4. **Draft title + body together.** Title per [Title format](#title-format); body to a temp file (`mktemp`) per [TEMPLATE.md](TEMPLATE.md). For an existing PR, treat the current title as a draft, not a constraint.

5. **Create or update the PR.** Show the drafted **title** and **body** + the exact `gh` command; quick confirm before running (notifies reviewers + CODEOWNERS) unless told to just do it.

   ```sh
   git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || git push -u origin "$(git branch --show-current)"
   # update existing; pass --title only if it changes:
   gh pr edit <number> --body-file <tmp> [--title "<drafted title>"]
   # or create new:
   gh pr create --base main --title "<drafted title>" --body-file <tmp>
   ```

   Report the PR URL.

## Title format

Communicate *what changed and why* at a glance, specific enough that a reviewer can predict the diff. GitHub auto-appends ` (#NNNN)` on merge; do not write it yourself.

```
<type>(<scope>): <summary> (PROJ-XXXX)
```

- **`<type>`**: dominant intent of the diff.

  | Type       | Use for                                     |
  | :--------- | :------------------------------------------ |
  | `feat`     | New feature or capability                   |
  | `fix`      | Bug fix                                     |
  | `refactor` | Code restructuring with no behaviour change |
  | `docs`     | Documentation only (includes ADRs)          |
  | `test`     | Adding or updating tests                    |
  | `chore`    | Build config, dependencies, tooling         |
  | `perf`     | Performance improvement                     |

  Mixed diff: pick the user-visible win, mention the rest in `## Notes`.
- **`<scope>`**: optional but usually present. Lowercase kebab. Common scopes in this repo: `web` (FE SPA), `api`, `db`, `gateway`, `session-engine`, `providers`, `rpc`, `auth`, `admin`, `tooling`, `deps`, `adr`, `skills`. Multi-scope `(api,db)` only when both are non-trivial. Drop the scope when the change is repo-wide.
- **`<summary>`**: imperative present tense, lowercase first word, no trailing period. Proper nouns keep their case (`Drizzle`, `TanStack`, `ADR-0014`); double quotes around identifiers are fine.
- **`(PROJ-XXXX)`**: most specific tracker id (slice over epic); drop entirely if none. Follow-ups with no ticket: `(follow-up to #NNNN)`.

**Lint:**

- No em-dash (`—` / `–`). Hyphen, colon, or rephrase.
- No trailing period; no capital after the colon (proper nouns excepted).
- ≲ 80 chars including `(PROJ-XXXX)`; if over, trim adjectives, not specificity.
- Reject generic verbs (`update`, `improve`, `change`, `various`). Strong fix-title names cause, surface, impact: `fix(gateway): race condition in session cleanup that caused 502s under load`, not `fix bug`.

**Worked examples** (paired bodies in [TEMPLATE.md](TEMPLATE.md)):

| Diff shape                          | Title |
| :---------------------------------- | :---- |
| FE + API slice tied to a ticket     | `feat(admin): super_admin rename and soft-archive an organization (PROJ-2509)` |
| Targeted backend bug, no ticket     | `fix(db): resolve ambiguous "id" in org-admin list/detail reads` |
| Docs/ADR landing ahead of impl      | `docs: add ADR-0014 per-org delivery policies + Delivery Policy glossary term` |
| Follow-up to a prior PR             | `docs: scrub em-dashes from ADR-0014 (follow-up to #1287)` |
| Repo-wide change, no scope          | `feat: collapse the platform-role enum to super_admin and member (PROJ-2504)` |

## Section selection

| Section        | Include when                                                                 |
| :------------- | :--------------------------------------------------------------------------- |
| `## What`      | Always. Concrete bullets of what changed + slice/epic context.               |
| `## How`       | Any code change with a non-obvious approach or notable design decision.      |
| `## Why` / `## Why now` | Docs/ADR PRs, or when motivation is not self-evident from What.     |
| `## Migration` | `packages/acme-db/src/schema/**` or `drizzle/**` changed. Name the file, state additive/nullable + backwards-compat verdict. |
| `## Behaviour` | New business rules, gates, or edge cases worth flagging.                     |
| `## Notes`     | Asides: "docs-only", "no code change", pre-commit green, follow-ups deferred. |

Omit a `## Reviews` section.

## Body rules

- **No em-dash** (`—` / `–`). Hyphen or colon.
- **No hardcoded hostnames/URLs.** Derive from config; link tracker issues by id/URL only.
- **Tracker auto-link + auto-close.** The title's `(PROJ-XXXX)` links the ticket. Add `Closes PROJ-XXXX.` to the body (near the epic-linkage line) so the slice ticket auto-transitions to Done on merge. For epic context use `Part of PROJ-YYYY` (non-closing magic word; links only). Magic words must appear in the PR description, not in a comment. Omit `Closes` only when the PR has no tracker id.
- Close with an epic-linkage line when relevant (`Part of the PROJ-XXXX epic. Remaining: …`).
- **No attribution footer.** Never add `🤖 Generated with Claude Code` (or any agent attribution) to the PR body. The `Co-Authored-By` trailer on commits is the only attribution.
