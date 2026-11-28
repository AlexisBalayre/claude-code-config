---
name: notes
disable-model-invocation: true
allowed-tools: Bash(gh pr list:*), Bash(git log:*), Bash(git ls-remote:*), Bash(date:*), Read, Edit, Write, Glob, Grep, Agent, mcp__linear-server__list_issues
description: Write the user's Obsidian notes (the daily logbook, a cited study note) or search the vault.
---

# Notes

`/notes <mode> [args]`. Read the mode's file in this folder, then follow it.

| Mode | File | Use |
| -- | -- | -- |
| `daily plan\|summary\|status\|checkin` | [daily.md](daily.md) | the day's logbook note |
| `study <topic>` | [study.md](study.md) | one researched, cited note the user reviews |
| `search <terms>` | [search.md](search.md) | notes, backlinks and index notes |

With no mode, or one not in the table, list the modes and ask which one.

> **Setup:** a personal integration. Set `OBSIDIAN_VAULT` and `OBSIDIAN_DAILY_DIR` in `.env` (see `.env.example`). `daily` also reads the tracker through an issue-tracker MCP server; the `mcp__linear-server__*` tool names are placeholders, swap them for your tracker's tools.

## The vault

- **Path:** `$OBSIDIAN_VAULT` from `.env`, written `<vault>` in the mode files. When it is unset, ask the user for the path.
- **Folders:** `$OBSIDIAN_DAILY_DIR/` (one `YYYY-MM-DD.md` per day), `Templates/`, `Research/<Topic>/Notes/`. Rename them here to match the vault.
- **Writers:** the user and the lead session. Any other session (a worker, a subagent) writes its draft under `$TMPDIR` and hands the lead the path.
- **Names:** Title Case. An index note is `<Topic> Index.md`, a list of `[[wikilinks]]`.
- **Links:** `[[Note Title]]` between notes, each note's related notes at its bottom. PRs and tickets as Markdown links to their URLs: `[#NNN](<PR URL>)`, `[<KEY>-NNN](<ticket URL>)`.
- **The user's text:** every line the user wrote survives an update; write around it.
- **Your prose:** commas, colons or parentheses where a dash would go.
