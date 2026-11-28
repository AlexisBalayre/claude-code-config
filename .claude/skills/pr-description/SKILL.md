---
name: pr-description
description: Write a PR title and description in this repo's house style, then create or update the PR via gh. Use when drafting or rewriting a PR title or body, or opening a PR.
---

# Write a PR title and description

Use this template for writing the PR body:

```markdown
## Goal

<One line: what this PR delivers.> Closes <KEY>-NNN.

## What changed

<Up to 5 bullets or a table, plus the updated mermaid diagram when a flow changed.>

## Done

- [x] <The ticket's Done-when item, copied>: <its evidence: a test name, a link, a figure>

Proof: <test|check|removal>: <evidence>

## Merge danger

<One line: the door and the blast radius, or "None".>
```

The Why stays on the ticket: the PR links it and never restates it. The Goal line's closing word follows the [tracker rule](#body-rules).

> **Setup:** the tracker steps use an issue-tracker MCP server (`mcp__linear-server__*` tool
> names are placeholders: swap them for your tracker's tools). The ticket prefix (`<KEY>`) comes
> from `TRACKER_ISSUE_PREFIX` in `.env`. No tracker? Skip step 2, drop the ticket id and the
> closing word everywhere, and write `## Done` from the plan's acceptance criteria.

## Sections

Skip all preambles and keep prose brief. Grep `docs/glossary.md` for a term before naming it.

### What changed

Up to 5 bullets or a table. When a view helps (pseudocode, a call tree, a component or file tree, Mermaid, a `diff` of one of them, or a whole block), pick the smallest that makes the point and place it next to the text it supports; keep only the calls, files, props, states and boundaries a reviewer needs. [VIEWS.md](VIEWS.md) has an example of each.

A change within the Goal that the approved plan did not name gets one `Scope: <what and why>` line, last in this section.

### Done

Copy the ticket's `## Done when` items, in order, and tick each one with its evidence after a colon or a spaced hyphen: the test that failed before and passes now, a screenshot for a visual change, a link, or a measured figure. An item this PR does not finish stays unticked and the Goal line says `Refs` instead of `Closes`.

End the section with one `Proof:` line after a blank line (glued under a ticked item, it reads as that item's evidence) that shows the change works. The form is one of three:

- `Proof: test: <name>`: a test that fails without the fix (the mutant run).
- `Proof: check: <what you ran>`: a dev or browser check, required for any visible change.
- `Proof: removal: <tests and search>`: for a pure-removal chore only, green tests plus a search showing nothing still uses what was removed.

### Merge danger

One line: the door (two-way when a revert undoes it, one-way for a destructive or hard-to-reverse change) and the blast radius, or "None".

When a schema definition or a migration changed, add one line per migration file: what it adds (additive or nullable, default, backfill) and the backwards-compat verdict. A destructive migration (a drop, a rename, a narrowed type) is a one-way door.

## Workflow

1. **Gather context** (run together):

   The trunk is `GIT_TRUNK` from `.claude/project.env` (default `main`); substitute it below.

   ```sh
   git fetch origin <trunk>
   git branch --show-current                                   # MUST NOT be the trunk; abort if it is
   git log origin/main..HEAD --pretty=format:'%h %s%n%b'       # commits on this branch
   git diff origin/main...HEAD --stat                          # changed files + churn
   gh pr view --json number,url,state,title,body 2>/dev/null   # non-zero exit = no PR yet
   ```

   Read the diff for the key files (`git diff origin/main...HEAD -- <path>`); on large diffs lean on `--stat` plus the important files. Generated files (`GENERATED_PATHS_REGEX` in `.claude/project.env`) can dwarf the change: report the code churn without them.

2. **Ground in the tracker (read-only).** Grep branch + commit subjects for `<KEY>-\d+`. If found, fetch the issue and its parent with `mcp__linear-server__get_issue` for its Goal, its `## Done when` items (the PR's `## Done` copies them), the canonical link, and whether this merge finishes the ticket. NEVER call any `save_*` tool. No id: derive from diff + commits.

3. **Draft title + body together.** Title per [Title format](#title-format); body to a temp file (`mktemp "${TMPDIR:-/tmp}/pr.XXXXXX"`: a bare `mktemp` fails in a sandboxed worker) per the template and [Sections](#sections), obeying [Body rules](#body-rules). For an existing PR, treat the current title and body as a draft, not a constraint.

4. **Create or update the PR.** Show the drafted **title** and **body** + the exact `gh` command; quick confirm before running (notifies reviewers + CODEOWNERS) unless told to just do it. This step never pushes: `gh pr create` needs the branch on origin, so a branch with no upstream waits for its push, which is the user's call.

   ```sh
   # update existing; pass --title only if it changes:
   gh pr edit <number> --body-file <tmp> [--title "<drafted title>"]
   # or create new:
   gh pr create --draft --base <trunk> --title "<drafted title>" --body-file <tmp>
   ```

   A new PR always opens as a draft. Report the PR URL.

## Title format

Communicate *what changed and why* at a glance, specific enough that a reviewer can predict the diff. GitHub auto-appends ` (#NNNN)` on merge; do not write it yourself.

```
<type>(<scope>): <summary> (<KEY>-NNN)
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

  Mixed diff: pick the user-visible win, mention the rest in What changed.
- **`<scope>`**: optional but usually present. Lowercase kebab, naming the component or area touched. Common scopes in this repo: <!-- TODO(adapt): list the repo's scopes, e.g. `api`, `web`, `db`, `auth`, `tooling`, `deps`, `adr`, `skills` -->. Multi-scope `(api,db)` only when both are non-trivial. Drop the scope when the change is repo-wide.
- **`<summary>`**: imperative present tense, lowercase first word, no trailing period. Proper nouns keep their case (`PostgreSQL`, `GitHub`, `ADR-0014`); double quotes around identifiers are fine.
- **`(<KEY>-NNN)`**: most specific tracker id (slice over epic); drop entirely if none. Follow-ups with no ticket: `(follow-up to #NNNN)`.

**Lint:**

- No em dash or en dash (U+2014, U+2013). Hyphen, colon, or rephrase.
- No trailing period; no capital after the colon (proper nouns excepted).
- ≲ 80 chars including `(<KEY>-NNN)`; if over, trim adjectives, not specificity.
- Reject generic verbs (`update`, `improve`, `change`, `various`). Strong fix-title names cause, surface, impact: `fix(api): race condition in cache invalidation that caused 502s under load`, not `fix bug`.

**Worked examples:**

| Diff shape                          | Title |
| :---------------------------------- | :---- |
| FE + API slice tied to a ticket     | `feat(admin): rename and soft-archive a workspace (<KEY>-2509)` |
| Targeted backend bug, no ticket     | `fix(db): resolve ambiguous "id" in admin list/detail reads` |
| Docs/ADR landing ahead of impl      | `docs: add ADR-0014 per-workspace retention policies + Retention Policy glossary term` |
| Follow-up to a prior PR             | `docs: scrub em-dashes from ADR-0014 (follow-up to #1287)` |
| Repo-wide change, no scope          | `feat: collapse the user-role enum to admin and member (<KEY>-2504)` |

## Body rules

- **No em dash or en dash** (U+2014, U+2013). Hyphen or colon.
- **About 200 words of prose**, fenced code, tables, diagrams and headings excluded.
- **400 production lines or more** (the PR Size warning, the first line `scripts/added-lines.sh` prints): What changed says in one line why the change can't split.
- **No hardcoded hostnames/URLs.** Derive from config; link tracker issues by id/URL only.
- **Tracker rule.** The title's `(<KEY>-NNN)` links the ticket; the Goal line ends with what the merge does to it:
  - `Closes <KEY>-NNN.` when the merge finishes the ticket: the tracker moves it to Done.
  - `Refs <KEY>-NNN.` when the ticket stays open after the merge (a dev check, a later PR).
  - `Part of <KEY>-MMM` for an epic: it links without closing.

  Magic words work in the PR description only, not in a comment. Drop the closing word when the PR has no tracker id. Name a ticket id only on that line and in the title: a tracker integration attaches the PR to every id its text names, and a closing word before an id closes that ticket.
- **No attribution footer and no model name.** Never add `Generated with Claude Code`, a robot line, or the name of the model to the PR body. The `Co-Authored-By` trailer on commits is the only attribution.
