# Frontend Architecture (Reference)

The structure of **`apps/acme-web`** — a React 19 SPA (Vite, Tailwind, TanStack Router +
TanStack Query, BetterAuth). For coding rules, see
[frontend conventions](../conventions/frontend.md).

## Directory layout

```
apps/acme-web/src/
  routes/                file-based routes (TanStack Router)
    __root.tsx
    _protected/          guarded subtree — requires an authenticated session
      sessions.tsx
      organizations.tsx
    login.tsx
    routeTree.gen.ts     GENERATED — never edited (protect-generated hook blocks it)
  features/              one folder per domain
    sessions/            components + logic for the Sessions domain
    organizations/
    members/
  hooks/                 query hooks wrapping TanStack Query (useSessions, useMember)
  lib/
    api-client.ts        typed API-client factory
    auth.ts              BetterAuth client + guard helpers
```

- **`routes/`** is file-based — the file tree *is* the route tree. The generated
  `routeTree.gen.ts` is off-limits to manual edits (the `protect-generated.sh` hook blocks
  `*.gen.ts`). The `_protected/` prefix marks the authenticated subtree.
- **`features/<domain>/`** holds the components and view logic for one domain. Routes stay thin
  and compose feature components.
- **`hooks/`** holds **query hooks** — the only place `useQuery`/`useMutation` keys and fetchers
  live, so cache keys stay consistent across the app.
- **`lib/`** holds cross-cutting infra: the api-client factory and the auth client.

## Data flow — loader + query

Routes prefetch with a **TanStack Router loader**; components read the same data with a
**TanStack Query** hook. The loader warms the cache so the view renders without a spinner; the
hook keeps it live and re-fetching.

```ts
// routes/_protected/sessions.tsx
export const Route = createFileRoute("/_protected/sessions")({
  loader: ({ context }) => context.queryClient.ensureQueryData(sessionsQuery()),
  component: SessionsPage,
});

// hooks/use-sessions.ts
export function useSessions() {
  return useQuery(sessionsQuery()); // same query key the loader primed
}
```

All fetches go through the **api-client factory** in `lib/api-client.ts`, which injects the
BetterAuth credentials and base URL — components never call `fetch` directly.

## Auth guard flow

1. The `_protected/` route subtree runs a `beforeLoad` guard.
2. The guard checks the **BetterAuth** session via `lib/auth.ts`.
3. **No session** → redirect to `/login` (with a `redirect` search param to return after sign-in).
4. **Valid session** → the loader runs and the protected view renders.

```ts
// routes/_protected/route.tsx
export const Route = createFileRoute("/_protected")({
  beforeLoad: async ({ location }) => {
    const session = await getSession();
    if (!session) throw redirect({ to: "/login", search: { redirect: location.href } });
  },
});
```
