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

<Approach + notable decisions. Reference files plainly (`service.ts`); don't
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
feat(admin): super_admin rename and soft-archive an organization (PROJ-2509)
```

Body:

```md
## What

Third slice of the super_admin org-administration surface (PROJ-2509, under the
PROJ-2497 epic). Adds **rename** and **soft-archive** for any organization, plus
the gate that stops archived orgs from starting new sessions.

- **`PATCH /admin/organizations/{id}`** (super_admin-only): partial update
  `{ name?, archived? }`.
- Org detail page gains **Rename** and **Archive/Unarchive** actions and an
  archived badge.
- Org list gains a **"Show archived"** toggle (archived excluded by default).

## Migration

`0056_mute_shape.sql`: additive nullable `organizations.archived_at timestamptz`
(no default, no backfill; null = not archived). Backwards-compatible.

## Behaviour

- **Soft-archive**, not delete: sessions/messages/usage/billing are retained;
  unarchive clears the flag.
- **New-session gate**: `sessions.service.create()` refuses an archived org on
  both capped and uncapped paths. Existing-session writes still allowed.

Closes PROJ-2509. Part of the PROJ-2497 epic. Remaining: **PROJ-2510** (usage/billing panel).
```

## Example: docs / ADR PR

Title:

```
docs: add ADR-0014 per-org delivery policies + Delivery Policy glossary term
```

Body:

```md
## What

Docs-only. Records the decision behind Epic
[PROJ-2517](https://linear.app/acme/issue/PROJ-2517) (per-organization delivery
policies):

- **`docs/adr/0014-per-org-delivery-policies.md`** (status `proposed`).
- **`docs/glossary.md`** adds the **Delivery Policy** glossary term.
- **`docs/adr/README.md`** index entry.

## Why now

Implementation is sliced into PROJ-2519 → 2523; landing the ADR + glossary first
gives those slices a shared, reviewed vocabulary.

## Notes

- No code changes; pre-commit green.
- ADR is `proposed`, flips to `accepted` once the system reflects it.
```
