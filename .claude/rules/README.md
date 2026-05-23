# Rule catalog

Path-scoped convention rules. Each auto-loads when you open or edit a file matching its `paths:`
frontmatter — you never invoke them. A rule is a thin trigger that imports the full convention
doc (the "split pattern"), so always-on context stays small while full detail loads on demand.

| Rule | Auto-loads for | Enforces (full doc) |
| :--- | :------------- | :------------------ |
| `universal-conventions` | every `**/*.ts`, `**/*.tsx` | Naming, named-exports-only, type separation, JSDoc, comment discipline → `general.md` + `naming.md` |
| `api-conventions` | `apps/acme-api/**` | Routes → Services → Repositories → DB layering, Hono + Zod OpenAPI → `api.md` |
| `frontend-conventions` | `apps/acme-web/**` | React 19, `Readonly` props, `cn()`/CVA, TanStack Router/Query → `frontend.md` |
| `database-conventions` | `packages/acme-db/**` | Drizzle, UUID PKs, snake_case columns, migration safety → `database.md` |
| `testing-conventions` | `**/*.test.ts`, `**/*.integration.test.ts` | Vitest, mock ordering, fake timers, behaviour-not-internals → `testing.md` |
| `gateway-conventions` | `services/acme-gateway/**` | Factories only, WS-upgrade auth, config-driven pools → `gateway.md` |
| `grpc-conventions` | `packages/acme-rpc/**` | Proto contracts, never-edit generated stubs, per-call deadlines → `grpc.md` |
| `session-engine-conventions` | `services/acme-session-engine/**` | State machines, no `EventEmitter`, atomic Redis Lua slots → `session-engine.md` |
| `providers-conventions` | `packages/acme-providers/**` | Channel interface + factory + YAML registry → `providers.md` |

**How to use:** just edit files in a matching path; the rule and its `docs/conventions/*.md`
import load automatically. To add a rule, see [`.claude/README.md`](../README.md) ("New rule").
