# Rule catalog

Path-scoped convention rules. Each auto-loads when you open or edit a file matching its `paths:`
frontmatter — you never invoke them. A rule is a **pure loader**: `paths:` frontmatter plus a
single `@docs/conventions/<area>.md` import, no content of its own. `docs/conventions/` is the
single source of truth, so always-on context stays small while full detail loads on demand.

| Rule | Auto-loads for | Enforces (full doc) |
| :--- | :------------- | :------------------ |
| `core-conventions` | every `**/*.ts`, `**/*.tsx` | Altitude/YAGNI, type system, JSDoc, comments, logging, naming taxonomy → `core.md` |
| `backend-conventions` | `apps/acme-api/**`, `packages/acme-db/**` | Routes → Services → Repositories → DB layering, Hono + Zod OpenAPI, auth chains, Drizzle + migration safety → `backend.md` |
| `services-conventions` | `services/acme-gateway/**`, `services/acme-session-engine/**`, `packages/acme-rpc/**`, `packages/acme-providers/**` | Factories only, pools, state machines, no `EventEmitter`, proto contracts, atomic Redis Lua slots, provider factory + YAML registry → `services.md` |
| `frontend-conventions` | `apps/acme-web/**` | React 19, react-router-dom v7 single route table, `Readonly` props, `cn()`/CVA, TanStack Query → `frontend.md` |
| `testing-conventions` | `**/*.test.ts(x)`, `**/*.mock.ts`, `**/test/**`, vitest configs, `tools/vitest/**` | Vitest shared factories, `test/` layout, mock ordering, fake timers, behaviour-not-internals → `testing.md` |

**How to use:** just edit files in a matching path; the rule and its `docs/conventions/*.md`
import load automatically. To add a rule, see [`.claude/README.md`](../README.md) ("New rule").
