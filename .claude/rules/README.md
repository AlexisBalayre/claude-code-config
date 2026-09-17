# Rule catalog

Path-scoped convention rules. Each auto-loads when you open or edit a file matching its `paths:`
frontmatter — you never invoke them. A rule is a **pure loader**: `paths:` frontmatter plus a
single `@docs/conventions/<area>.md` import, no content of its own. `docs/conventions/` is the
single source of truth, so always-on context stays small while full detail loads on demand.

| Rule | Auto-loads for | Enforces (full doc) |
| :--- | :------------- | :------------------ |
| `core-conventions` | source files of the project's languages | Altitude/YAGNI, comments, language and types, errors, logging, structure, naming → `core.md` |
| `testing-conventions` | `test/`, `tests/`, `__tests__/`, `*.test.*`, `*.spec.*`, `*_test.*`, `test_*.*` | Behaviour-over-internals, framework, layout, mocks, integration tests → `testing.md` |

`/adapt-to-project` narrows these globs to the project and adds one rule per area whose conventions
differ from core (e.g. `api-conventions` → `docs/conventions/api.md`); list each new rule here.

**How to use:** just edit files in a matching path; the rule and its `docs/conventions/*.md`
import load automatically. To add a rule, see [`.claude/README.md`](../README.md) ("New rule").
