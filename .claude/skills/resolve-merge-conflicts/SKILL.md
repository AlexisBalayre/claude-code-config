---
name: resolve-merge-conflicts
description: Resolve an in-progress git merge or rebase conflict. Use when a merge, rebase, or cherry-pick stops on conflicts.
---

# Resolve Merge Conflicts

1. **See the current state** of the merge/rebase: `git status`, the history of both sides, and the conflicting files.

2. **Find the primary sources** for each conflict. Understand deeply why each change was made and what the original intent was. Read the commit messages and the PRs (`gh pr view`); PR bodies carry `Closes PROJ-XXXX`, so fetch the tracker issue when the intent is still unclear.

3. **Resolve each hunk.** Preserve both intents where possible. Where incompatible, pick the one matching the merge's stated goal and note the trade-off. Do **not** invent new behaviour. Always resolve; never `--abort`.

4. **Regenerate, don't hand-merge.** Conflict markers in generated files are never resolved by hand:
   - `pnpm-lock.yaml`: take either side wholesale, then re-run `pnpm install` to regenerate.
   - Drizzle migrations and `packages/acme-db/src/migrations/meta/_journal.json`: drop this branch's generated migration, re-run `pnpm db:generate` against the merged schema, then `biome check --write` the `_journal`.
   - Any other generated artifact (schema diagrams, generated API references, gRPC stubs): re-run its generator against the merged sources; never edit the output.

5. **Run the tests** for the affected packages and fix anything the merge broke (formatting and typechecking run automatically via the Stop hook). The pre-commit hook lints, typechecks, and tests the whole repo; failures that already exist on the base branch are not the merge's fault, and `--no-verify` is acceptable only for those.

6. **Finish the merge/rebase.** Stage everything and commit. If rebasing, continue (`git rebase --continue`) until all commits are rebased.
