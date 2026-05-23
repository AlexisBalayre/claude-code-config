# Command catalog

Slash commands you invoke explicitly. They carry side effects (posting PR comments, writing
files), so they never auto-trigger.

| Command | When to use | How to invoke |
| :------ | :---------- | :------------ |
| `/code-review` | You want a high-signal review of a pull request before merging — real bugs, `CLAUDE.md` violations, and spec/config correctness, with style nits and linter-catchable issues filtered out. | `/code-review <PR number or URL>`. Runs a context pass + 6 parallel review agents + a validation pass, then posts inline comments (or a clean-bill summary). |

**Requires:** `gh` authenticated; for inline comments, the GitHub inline-comment MCP tool
(`mcp__github_inline_comment__create_inline_comment`) — both declared in the command's
`allowed-tools`.

To add a command, see [`.claude/README.md`](../README.md) ("New command").
