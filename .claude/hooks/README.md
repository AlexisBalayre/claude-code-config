# Hook catalog

Hooks are deterministic shell scripts wired in [`settings.json`](../settings.json) that run on
lifecycle events — zero LLM cost, zero context, no hallucination. You never invoke them; they
fire automatically. Exit code `2` blocks the operation, `0` allows it.

| Hook | Fires on | What it does |
| :--- | :------- | :----------- |
| `quality-checks.sh` | `Stop` | When `.ts/.tsx` files are dirty: Biome auto-fix + check on the dirty files, then a whole-repo `typecheck`. Blocks (exit 2) on any failure. No-ops when no TS changed. Tests are deliberately not run here — the pre-commit hook owns the full turbo suite. |
| `convention-spot-check.sh` | `Stop` | Advisory scan of changed TS: `export default`, inline types in service/route files, missing JSDoc (`packages/` only), non-`Readonly` React props, `EventEmitter` in the session engine, direct `new XService()` in the gateway. Always advisory (exit 0). Comment quality is owned by `comment-pruner.sh`. |
| `comment-pruner.sh` | `Stop` | When the session added net-new comments (hashed against a memo of already-adjudicated ones): exits 2 so the main loop dispatches the `comment-pruner` subagent over the touched files, then seals the memo. A cheap pre-filter — a response that adds no comment pays nothing. |
| `git-safety.sh` | `PreToolUse(Bash)` | Blocks `rm -rf`, `DROP TABLE`, `git push --force`, `git reset --hard`, `checkout -b` on the trunk, and pushes to the trunk. Trunk name from `.env` `GIT_TRUNK` (default `main`). |
| `protect-generated.sh` | `PreToolUse(Edit\|Write)` | Blocks edits to `*.gen.ts`, `routeTree.gen.ts`, and `packages/acme-rpc/src/generated/**` (regenerate from `.proto` instead). |
| `validate-file-naming.sh` | `PreToolUse(Write)` | Blocks new `.ts/.tsx` files that do not match `kebab-case.role.ts` (25 valid roles), with documented exceptions. |
| `pre-compact-preserve.sh` | `PreCompact` | Injects must-preserve context (current branch + worktree path, modified files, test results) so it survives compaction. |

**Configure / disable:** edit the entry under `hooks` in `settings.json`. Scripts must stay
executable (`chmod +x`). To add a hook, see [`.claude/README.md`](../README.md) ("New hook").
