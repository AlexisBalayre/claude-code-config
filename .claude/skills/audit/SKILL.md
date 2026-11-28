---
name: audit
disable-model-invocation: true
description: Audit the codebase for deepening opportunities, dead code or dependency-rule debt, and file the rows the user picks.
---

# Audit

One entry point for codebase audits. The first word of `$ARGUMENTS` picks the mode; the rest is its scope. With no mode, or one not in the table, list the modes and ask which one.

| Mode | Use it to find | Read |
| -- | -- | -- |
| `arch` | shallow modules worth deepening | [ARCH.md](ARCH.md) |
| `dead` | unused files, exports and feature flags | [DEAD.md](DEAD.md) |
| `deps` | import cycles, orphans and the exceptions the dependency rules list | [DEPS.md](DEPS.md) |

Read the mode's file now and follow it; the run is done when that file's last step is.

## Shared rules

- **Scope first.** One package, service or subsystem per run, named by the user or taken from the mode's own scoping step. A whole-repo list is too long to pick from.
- **Report, never fix.** An audit edits no code and deletes nothing: each finding is a candidate, a hypothesis the user rules on.
- **Use the project's tools.** A mode runs the detector the project already configures; it never installs one. With none configured, say which would help and fall back to grep.
- **Pick sheet last.** Each mode ends in an `html-artifact` pick sheet, one row per candidate keyed `A1`, `A2`, and so on, each row's evidence beside it. A run with no candidate ends in its clean report instead: what ran, over which scope. Run from a worker, the sheet goes to the lead, and the picks count only as `agent-workflow` §1 relays them.
- **Every write goes through `file-ticket`.** File only the rows the user picked: its §2 search (a picked row an open ticket already covers folds into it), its §3 grouping, its §5 write and read-back. Done when every picked row is a ticket or folded into one, and the user has `file-ticket`'s digest.
- **Repo vocabulary.** Name modules with `docs/glossary.md` and the role taxonomy in `docs/conventions/core.md`; a candidate that re-litigates an ADR in `docs/adr/` says so in its row.
