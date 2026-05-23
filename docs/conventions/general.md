# General Conventions

Universal rules for **every** `*.ts` / `*.tsx` file in the monorepo. Area-specific docs
([api](api.md), [frontend](frontend.md), [testing](testing.md), …) layer on top of these;
they never relax them. The thin `@docs/conventions/general.md` import in
`.claude/rules/universal-conventions.md` points here.

## File naming

Files follow `kebab-case.role.ts`. The `role` suffix is not decoration — it locks in a
calling convention. The full taxonomy, plurality rules, and banned roles live in
[naming.md](naming.md), enforced by `.claude/hooks/validate-file-naming.sh`.

## Exports

**Named exports only. Never `export default`.**

```ts
// session.service.ts
export function createSession(db: Database, input: CreateSessionInput): Promise<Session> { … }

// ✗ export default createSession
```

Default exports break namespace imports, defeat tree-shaking hints, and make rename-across-
files unreliable. The `convention-spot-check` Stop hook flags any `export default`.

## Imports

- **Extensionless relative imports in source.** Write `from './session.service'`, not
  `'./session.service.js'`. The `.js` extensions are appended at build time by a post-build
  step. Adding them by hand fights the build.
- **Prefer namespace imports for services** so call sites read as `Service.method`:

  ```ts
  import * as SessionService from './session.service';
  const session = await SessionService.getById(db, id);
  ```

## Type separation

Types and interfaces live in dedicated `types/` or `interfaces/` folders. **Never inline an
exported `type`/`interface` in a `*.service.ts` or `*.routes.ts` file** — the spot-check hook
warns on this. Re-export from the consumer if you want a stable call-site path.

## Type safety

- **No `any` outside system boundaries.** Where an external SDK or untyped payload forces it,
  isolate it and add a one-line justification comment:
  ```ts
  // any: third-party SDK ships no types for this webhook envelope
  const payload = raw as any;
  ```
- **Branded IDs.** Use `SessionId`, `OrganizationId`, `MemberId`, … from
  `packages/acme-domain/src/types/branded.types.ts`. Cast a raw `string` to a branded ID
  **only at a boundary** (request parsing, DB row mapping), never deep in business logic.
- **Boolean names** start with `is` / `has` / `should` / `can` (`isConnected`, `hasQuorum`).
- **Generics are descriptive** — `TConfig`, `TResponse`, `TEvent`. Never bare `T` / `U`.
- **`override` is required** on every method that overrides a superclass method.
- **Factories for stateless logic; classes only for stateful resources** with a real
  lifecycle (a connection pool, a session state machine). A bag of pure functions is a
  module of named exports, not a class.

## No hardcoded values

Ports, URLs, timeouts, model names, channel limits — none are inlined. Pull them from YAML
config or the typed `env.ts` for the workspace.

```ts
// ✗ const client = connect('localhost:50051', { timeoutMs: 5000 });
const client = connect(env.GATEWAY_ADDR, { timeoutMs: config.rpc.timeoutMs });
```

## JSDoc

- **`packages/*`:** JSDoc is **required on every exported symbol** (function, class,
  interface, type, const). Cross-workspace consumers read it on IDE hover. Summary is an
  imperative verb phrase, ≤120 chars.
  ```ts
  /** Resolve the Provider for a Channel, falling back to the Organization default. */
  export function resolveProvider(registry: ProviderRegistry, channel: Channel): Provider { … }
  ```
- **`apps/*` and `services/*`:** JSDoc only when the *why* is non-obvious — invariants,
  units, ordering guarantees, gotchas. These are leaf workspaces with no external consumers.

## Comments

- Inline `//` comments explain **WHY, not WHAT**. If deleting the comment wouldn't confuse a
  reader, delete it.
- No `// === Section ===` banners.
- No journal / changelog comments (`// 2026-05: changed by …`).
- No commented-out code. Git is the history.

## Clean code

- **Delete old code when you replace it.** No compatibility shims, no `// removed` markers,
  no backwards-compat re-exports. The replacement *is* the change.
- **YAGNI.** Don't add parameters, abstractions, or config for requirements that don't exist
  yet. Build for what is in front of you.
