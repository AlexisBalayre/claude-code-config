# Slice mode

Break an Epic, a plan or the conversation into child tickets that are tracer bullets: each one cuts a narrow but complete path through every layer the feature touches (schema, service, transport, UI, tests) for one behavior.

## 1. Source

With a ticket id or URL as input, read it with `get_issue` and `list_comments`; with a path (`docs/plans/<feature-slug>/spec.md`), Read it. Otherwise work from the conversation.

Done when you can state the Goal and Done-when items the slices must add up to.

## 2. Explore and prefactor

Read the code the work touches, at the trunk (`GIT_TRUNK` in `.claude/project.env`). Look for a prefactor that makes the work easier: make the change easy, then make the easy change. A prefactor is its own slice, and it comes first.

Done when you can name each module the slices touch, and each prefactor or "none".

## 3. Slice

For each slice:

- **Title:** `file-ticket` §5's Title rule, with the behavior the slice delivers as a reader can verify it.
- **Type** from the Type and size table of `templates.md`; **points** your estimate on that section's scale. Together they decide whether the slice needs a Design (the table's Design column), which goes through `Specify` before code (`agent-workflow` §2).
- **Blocked by:** the slices that must land first; "None" when it can start now.
- **User stories covered**, from the parent Epic when it has them.
- **Size:** the PR's estimated added production lines, counted as `scripts/added-lines.sh` counts them.

Each slice:

- is a complete path end to end; a schema-only or UI-only slice is a smell;
- is demoable or verifiable on its own;
- fits in a single fresh context window, so one worker session can carry it from Design to merge;
- stays under the size cap (`agent-workflow` WORKER.md §4). Split a bigger one into thinner tracer bullets.

Prefer many thin slices to a few thick ones, and mark every truly independent slice "None", so slices can run in parallel.

A **wide refactor** (one mechanical change, such as renaming a column or retyping a shared symbol, whose blast radius breaks call sites across the codebase at once) is the exception: slice it as expand-contract. Expand: add the new form beside the old, so nothing breaks. Migrate the call sites in batches sized by blast radius (per package, per directory), each its own slice blocked by the expand, CI green batch to batch. Contract: delete the old form once no caller remains, in a slice blocked by every batch. When even the batches cannot stay green alone, they share an integration branch and all block a final integrate-and-verify slice. A single behavior still over the size cap splits the same way by layer: the backend first, its endpoint live but unused, then the UI.

Done when every Done-when item of the source maps to a slice.

## 4. Review

Show the breakdown as a numbered list, and below it the Epic's `## Slices` table and order diagram as they will read once filed (placeholders for ids not yet filed). Ask:

- Does the granularity feel right (too coarse, too fine)?
- Are the blocking edges right, and could any slice run in parallel?
- Should any slice merge or split?
- Is each slice's type and estimate right?

Iterate until the user approves. That yes covers writing the parent Epic's `## Slices` table and diagram.

## 5. File

Write each slice through `file-ticket` §2, §3 and §5, in dependency order so each blocker has a real id. Each slice gets `parentId` set to the Epic and a native `blockedBy` relation per blocker. With no tracker, each slice is one file under the Epic's `docs/plans/<feature-slug>/issues/`, as `file-ticket`'s Tracker metadata says, never a single combined file. Its body is the shared core of `templates.md` plus its type's extras, then:

- `## Blocked by`: the blocker ids, or "None, can start immediately";
- `## Notes`: the relevant ADRs, conventions, prior art (by module), open questions.

Then write the parent Epic once: its `## Slices` table (ticket, status) and its mermaid order diagram, as the user approved them. Touch no other part of the Epic. Read it back as `file-ticket` §5 says.

Done when every slice was read back, the Epic's read-back shows every slice in its table, and the user has the digest of ids.
