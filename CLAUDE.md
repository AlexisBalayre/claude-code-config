# Acme

@AGENTS.md

## Claude Code specifics

- `docs/conventions/*.md` reach you through `.claude/rules/`: the first Read, Edit, or Write of a file in an area injects that area's doc for the rest of the session. Never Read a conventions doc yourself; that duplicates 1-4.5k tokens already in context.
- MCP results (codegraph) do not fire rules. Before your first edit in an area, Read one existing file there with the Read tool; the "read 2-3 similar files" rule already asks for this.
