---
name: new-frontend-route
description: Scaffold a new frontend route with page component, query hooks, and feature components following TanStack Router file-based conventions. Auto-activates when asked to "add a page", "create a route", or "add a frontend view".
user-invocable: false
---

Scaffold a new frontend route. **CRITICAL:** Follow `docs/conventions/frontend.md` exactly.

## Execution Steps

1. **Determine Placement & Context:**
   - _Protected:_ `src/routes/_protected/<path>.tsx`
   - _Public:_ `src/routes/<path>.tsx`
   - _Dynamic:_ `src/routes/.../$param.tsx` (e.g., `$id.live.tsx`)
   - _Rule:_ Read 2-3 existing routes and feature components before writing code.
2. **Create Route File (`src/routes/.../<name>.tsx`):**
   - Use `createFileRoute` from `@tanstack/react-router`.
   - **Auth Guard:** Check BetterAuth session in `beforeLoad` for protected routes.
   - **Data:** Use `routeContext.queryClient` for TanStack Query integration and `loader` for prefetching.
3. **Create Feature Components (`src/features/<domain>/`):**
   - **Props:** ALWAYS use `Readonly<Props>`.
   - **Styling:** Use `cn()` for className composition. Use CSS variables for colors (NEVER hardcode). Use CVA for multi-variant components.
4. **Create Query Hooks (`src/hooks/` or co-locate):**
   - Use TanStack Query with the `queryOptions` pattern.
   - Use `getApiClient()` (`src/lib/api-client.factory.ts`).
   - Keep the default 5-minute stale time.
5. **Auto-Generate & Verify:**
   - TanStack Router auto-generates `routeTree.gen.ts` on save. **Rule: NEVER edit `routeTree.gen.ts` manually.**
   - Run `pnpm typecheck` to confirm types resolve.
