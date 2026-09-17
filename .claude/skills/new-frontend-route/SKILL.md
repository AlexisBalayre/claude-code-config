---
name: new-frontend-route
description: Scaffold a new frontend route with page component, query hooks, and feature components following the react-router-dom v7 single-route-table conventions. Auto-activates when asked to "add a page", "create a frontend route", or "add a frontend view".
user-invocable: false
---

Scaffold a new frontend route. The routing, guard, styling, and API-client obligations live in `docs/conventions/frontend.md` (component rules in `docs/conventions/core.md`), injected by the frontend and core rules as soon as you Read a file under `apps/acme-web/`; follow them exactly without re-reading them. This skill fixes only the file set and order of operations.

## Execution Steps

1. **Determine Placement & Context:**
   - All routes live in the single route table in `apps/acme-web/src/app.tsx` (`createBrowserRouter`); there is no file-based routing. Pick the layout group (`AuthLayout` / `PublicLayout` / `AppShell`) per frontend.md §Routing.
   - Read 2-3 existing routes in `app.tsx` and their feature components before writing code.
2. **Page Component** - `src/features/<domain>/<name>-page.component.tsx`:
   - Pages are plain React components with the `component` role - no routing exemptions.
   - Register in `app.tsx`: `lazy(() => import("./features/<domain>/<name>-page.component").then((m) => ({ default: m.<Name>Page })))`, then add `{ path: "...", element: <...Page /> }` to the right layout group's `children`.
   - Guards, admin-sidebar pairing, and param-redirect gotchas: frontend.md §Routing and §Role guards.
3. **Feature Components** - `src/features/<domain>/`, per frontend.md §Components and §Styling.
4. **Query Hooks** - `src/hooks/use-<name>.hook.ts`: TanStack Query with `getApiClient()` per frontend.md §API client.
5. **Verify:** Run `pnpm typecheck` to confirm types resolve, then load the route on the dev port (frontend.md §Routing).
