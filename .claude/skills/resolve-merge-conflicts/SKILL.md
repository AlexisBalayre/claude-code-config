---
name: resolve-merge-conflicts
description: Resolve an in-progress git merge or rebase conflict. Use when a merge, rebase, or cherry-pick stops on conflicts.
---

# Resolve Merge Conflicts

The repo-specific rules; resolve the rest of each hunk as usual.

- Always resolve; never `--abort`.
- **Regenerate, don't hand-merge.** Conflict markers in generated files are never resolved by hand:
  - The package manager's lockfile: take either side wholesale, then re-run `INSTALL_CMD` (`.claude/project.env`) to regenerate. A sandboxed worker never holds this conflict: `worker-git.sh merge` refuses it and changes nothing, so tell the lead.
  - Any path matching `GENERATED_PATHS_REGEX` (`.claude/project.env`): take either side, then re-run the generator against the merged sources.
  <!-- TODO(adapt): one bullet per generated artifact with its own regeneration command (migrations and their journal, schema diagrams, codegen output), or delete this comment. -->
- A branch already pushed takes a merge of the trunk (`GIT_TRUNK` in `.claude/project.env`), never a rebase.

Done when `git diff --name-only --diff-filter=U` prints nothing and the merge or rebase is continued.
