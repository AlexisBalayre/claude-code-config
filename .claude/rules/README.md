---
paths:
  - ".claude/rules/**"
---

# Rule catalog

Path-scoped convention rules. Each one is a symlink to a doc in `docs/conventions/`, and the doc's
own `paths:` frontmatter says which files load it: you never invoke a rule. `docs/conventions/` is
the single source of truth, so always-on context stays small while full detail loads on demand.

A rule must not `@import` its doc: an import loads at session start whatever `paths:` says.

| Rule | Auto-loads for | Enforces (full doc) |
| :--- | :------------- | :------------------ |
| `core-conventions` | source files of the project's languages | Altitude/YAGNI, comments, language and types, errors, logging, structure, naming: `core.md` |
| `testing-conventions` | `test/`, `tests/`, `__tests__/`, `*.test.*`, `*.spec.*`, `*_test.*`, `test_*.*` | Behaviour-over-internals, framework, layout, mocks, integration tests: `testing.md` |

`/adapt-to-project` narrows these globs to the project and adds one rule per area whose conventions
differ from core (e.g. `api-conventions` -> `docs/conventions/api.md`); list each new rule here.
`bash scripts/conventions-rules.test.sh` checks that every doc has `paths:` and a symlink.

**How to use:** just edit files in a matching path; the doc loads automatically. To add a rule, see
[`.claude/README.md`](../README.md) ("New rule").
