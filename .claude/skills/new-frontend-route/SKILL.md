---
name: new-frontend-route
description: Scaffold a new frontend route with page component, query hooks, and feature components following TanStack Router file-based conventions. Auto-activates when asked to "add a page", "create a frontend route", or "add a frontend view".
user-invocable: false
---

Scaffold a new frontend route. The routing, guard, styling, and API-client obligations live in `docs/conventions/frontend.md` (universal component rules in `docs/conventions/general.md`) - read them first and follow them exactly. This skill fixes only the file set and order of operations.

## Execution Steps

1. **Determine Placement & Context:**
   - Routing is file-based under `apps/acme-web/src/routes/`: _Protected:_ `src/routes/_protected/<path>.tsx`; _Public:_ `src/routes/<path>.tsx`; _Dynamic:_ `src/routes/.../$param.tsx` (e.g., `$id.live.tsx`). Pick the placement per frontend.md §Routing.
   - Read 2-3 existing routes and their feature components before writing code.
2. **Route File** - `src/routes/.../<name>.tsx`:
   - Use `createFileRoute` from `@tanstack/react-router`.
   - Auth guards (BetterAuth session in `beforeLoad`) and `loader` prefetching via `routeContext.queryClient`: frontend.md §Routing.
3. **Feature Components** - `src/features/<domain>/`, per frontend.md §Component props and §Styling.
4. **Query Hooks** - `src/hooks/use-<name>.hook.ts` (or co-locate): TanStack Query with the `queryOptions` pattern and `getApiClient()` from `src/lib/api-client.factory.ts`, per frontend.md §Data fetching.
5. **Verify:**
   - TanStack Router auto-generates `routeTree.gen.ts` on save. **Rule: NEVER edit `routeTree.gen.ts` manually** (frontend.md §Generated files).
   - Run `pnpm typecheck` to confirm types resolve, then load the route on the dev port.
