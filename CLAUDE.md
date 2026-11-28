@AGENTS.md

## Claude Code specifics

- `docs/conventions/*.md` reach you through `.claude/rules/`: the first Read, Edit, or Write of a file in an area injects that area's doc for the rest of the session. Never Read a conventions doc yourself; that duplicates tokens already in context. To load an area's doc early, Read one file in the area rather than the doc.
- MCP results do not fire rules. Before your first edit in an area, Read one existing file there with the Read tool; the "read 2-3 similar files" rule already asks for this.
- The `Stop` hook runs the formatter and the lint on the dirty source files of the session's checkout or worktree. Typecheck and tests run in `scripts/verify.sh`.
- `/simplify` applies the Altitude audit's cuts.
