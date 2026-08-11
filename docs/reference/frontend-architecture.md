# Frontend Architecture (Reference)

The structure of **`apps/acme-web`** — a React 19 SPA (Vite, Tailwind v4, react-router-dom v7 +
TanStack Query, BetterAuth). For coding rules, see
[frontend conventions](../conventions/frontend.md).

## Directory layout

```
apps/acme-web/src/
  app.tsx                the single route table (createBrowserRouter)
  features/              one folder per domain
    sessions/            components + logic for the Sessions domain
      sessions-page.component.tsx
    organizations/
    members/
  components/
    ui/                  reusable primitives
    layout/              AppShell, AuthLayout, PublicLayout, sidebar, topbar
    shared/              AuthGuard, RoleGuard, AuthProvider
  hooks/                 query hooks wrapping TanStack Query (use-sessions.hook.ts)
  transports/            ws adapter + useTransport (raw socket lib never leaves here)
  lib/
    api-client.factory.ts  typed API-client singleton
    cn.utils.ts
```

- **`app.tsx`** holds the whole route table — there is no file-based routing. Routes are added
  to the `children` array of one of three layout groups (`AuthLayout`, `PublicLayout`,
  `AppShell`), with pages `lazy()`-imported from `features/`.
- **`features/<domain>/`** holds the page and view logic for one domain. Pages are plain
  `component`-role files; routes stay thin and compose feature components.
- **`hooks/`** holds **query hooks** — the only place `useQuery`/`useMutation` keys and fetchers
  live, so cache keys stay consistent across the app.
- **`lib/`** holds cross-cutting infra: the api-client factory and helpers.

## Data flow — query hooks

Components read data with **TanStack Query** hooks; all fetches go through the **api-client
singleton** from `lib/api-client.factory.ts` (`credentials: "include"`) — components never call
`fetch` directly.

```ts
// app.tsx
const SessionsPage = lazy(() =>
  import("./features/sessions/sessions-page.component").then((m) => ({ default: m.SessionsPage })),
);
// … inside the AppShell group's children:
{ path: "/sessions", element: <SessionsPage /> }

// hooks/use-sessions.hook.ts
export function useSessions() {
  return useQuery(sessionsQuery()); // keys shared app-wide via the hook
}
```

## Auth guard flow

1. Protected route groups nest under a `<AuthGuard />` parent element (children inherit).
2. The guard checks the **BetterAuth** session via the auth provider.
3. **No session** → redirect to `/login` (with a `redirect` search param to return after sign-in).
4. **Valid session** → the child routes render. Role-gated groups add `<RoleGuard allowedRoles={...} />`,
   whose groups must match the API's guards exactly (see
   [frontend conventions §Role guards](../conventions/frontend.md)).
