# House-style PR body

Adaptive: always include `## What`; add other sections only when the section-selection table in SKILL.md says so. Lead with user-visible change, then implementation, then reviewer-critical info (migrations, edge cases, deferred work).

## Annotated skeleton

```md
## What

<One sentence: what this PR does + slice/epic context, e.g. "Nth slice of the …
surface (PROJ-XXXX, under the PROJ-YYYY epic).">

- <Concrete change, user- or API-visible first.>
- <`METHOD /path` for new routes; name the component/hook for FE; etc.>

## How

<Approach + notable decisions. Reference files plainly by basename; don't
narrate every file.>

## Migration            <!-- only if schema/migration changed -->

`NNNN_name.sql`: <additive nullable column / index / etc.> (<no default,
backfill rule>). <Backwards-compat verdict.>

## Behaviour            <!-- only if rules/edge cases worth flagging -->

- <Gate, default, or edge case a reviewer should verify.>

## Notes                <!-- optional asides -->

- <docs-only / no code change / pre-commit green / follow-up deferred.>

Closes PROJ-XXXX. Part of the PROJ-YYYY epic. Remaining: **PROJ-ZZZZ** (…).   <!-- Closes is the slice ticket from the title; auto-transitions to Done on merge. Omit when the PR has no tracker id. -->
```

## Example: code PR with a migration

Title:

```
feat(admin): rename and soft-archive a workspace (PROJ-2509)
```

Body:

```md
## What

Third slice of the workspace-administration surface (PROJ-2509, under the
PROJ-2497 epic). Adds **rename** and **soft-archive** for any workspace, plus
the gate that stops archived workspaces from creating new projects.

- **`PATCH /admin/workspaces/{id}`** (admin-only): partial update
  `{ name?, archived? }`.
- Workspace detail page gains **Rename** and **Archive/Unarchive** actions and an
  archived badge.
- Workspace list gains a **"Show archived"** toggle (archived excluded by default).

## Migration

`0056_workspace_archived_at.sql`: additive nullable `workspaces.archived_at timestamptz`
(no default, no backfill; null = not archived). Backwards-compatible.

## Behaviour

- **Soft-archive**, not delete: projects, members, and billing are retained;
  unarchive clears the flag.
- **New-project gate**: project creation refuses an archived workspace on
  every path. Writes to existing projects still allowed.

Closes PROJ-2509. Part of the PROJ-2497 epic. Remaining: **PROJ-2510** (usage/billing panel).
```

## Example: docs / ADR PR

Title:

```
docs: add ADR-0014 per-workspace retention policies + Retention Policy glossary term
```

Body:

```md
## What

Docs-only. Records the decision behind Epic PROJ-2517 (per-workspace retention
policies):

- **`docs/adr/0014-per-workspace-retention-policies.md`** (status `proposed`).
- **`docs/glossary.md`** adds the **Retention Policy** glossary term.
- **`docs/adr/README.md`** index entry.

## Why now

Implementation is sliced into PROJ-2519 → 2523; landing the ADR + glossary first
gives those slices a shared, reviewed vocabulary.

## Notes

- No code changes; pre-commit green.
- ADR is `proposed`, flips to `accepted` once the system reflects it.
```
