# Search mode

`/notes search <terms>`: find notes in the vault. Read-only. The vault rules are in [SKILL.md](SKILL.md).

Run the Glob and Grep tools with `<vault>` as their path, `*.md` only:

| Find | How |
| -- | -- |
| a note by name | Glob `**/*<term>*.md` with the term as typed, then in Title Case (Glob matches case) |
| a note by content | Grep `<term>`, files-with-matches |
| a note's backlinks | Grep `\[\[<Note Title>(\|[^]]*)?\]\]` |
| the index notes | Glob `**/*Index*.md` |

Print each hit as its vault-relative path with its first heading, newest first. For a daily note, search `$OBSIDIAN_DAILY_DIR/` and read only the matching section ([daily.md](daily.md)).
