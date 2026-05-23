# Frontend Conventions

Rules for `apps/acme-web` — the React SPA. Auto-loaded via
`.claude/rules/frontend-conventions.md` when you touch this workspace. The
`new-frontend-route` skill scaffolds against this doc.

## Stack

- **React 19** + **Vite**
- **Tailwind v4**
- **TanStack Router** (file-based routing)
- **TanStack Query** (server state)
- **BetterAuth** (sessions / auth)

## Component props

Component props are **always** wrapped in `Readonly<…>`. Props are inputs, never mutated.

```tsx
interface SessionCardProps {
  sessionId: SessionId;
  title: string;
}

export function SessionCard({ sessionId, title }: Readonly<SessionCardProps>) { … }
```

The `convention-spot-check` Stop hook flags any `.tsx` component prop typed as `XProps`
without the `Readonly<…>` wrapper.

## Styling

- **`cn()` for className composition** — never string-concatenate or template-literal
  classes.
  ```tsx
  <div className={cn('rounded-md p-4', isActive && 'bg-accent', className)} />
  ```
- **CSS variables for colors. NEVER hardcode a hex value.** Colors come from the theme tokens
  (`bg-[var(--color-accent)]`, Tailwind theme classes), so dark mode and theming work.
- **CVA for multi-variant components** — define variants with `cva()` rather than branching
  on props with conditionals.

## Data fetching

- Query hooks use the **`queryOptions` pattern** so the same definition powers both `loader`
  prefetch and in-component `useQuery`.
- Build the typed client with **`getApiClient()`** from `src/lib/api-client.factory.ts`.
- Keep the **default 5-minute stale time** unless a resource genuinely needs fresher data.

```ts
export function sessionQueryOptions(id: SessionId) {
  return queryOptions({
    queryKey: ['session', id],
    queryFn: () => getApiClient().sessions[':id'].$get({ param: { id } }),
    // staleTime inherited from the 5-minute default
  });
}
```

## Routing

- **Protected routes** live under `src/routes/_protected/`.
- Check the BetterAuth session in **`beforeLoad`**; redirect unauthenticated users.
- **Prefetch** data in the route `loader` using `routeContext.queryClient`.

```tsx
export const Route = createFileRoute('/_protected/sessions/$id')({
  beforeLoad: ({ context }) => {
    if (!context.auth.session) throw redirect({ to: '/login' });
  },
  loader: ({ context, params }) =>
    context.queryClient.ensureQueryData(sessionQueryOptions(params.id as SessionId)),
  component: SessionPage,
});
```

- **Feature components** live in `src/features/<domain>/` (e.g. `src/features/sessions/`),
  not inline in route files. Route files wire data + layout; features hold the UI.

## Generated files

**NEVER edit `routeTree.gen.ts`.** TanStack Router regenerates it on save from the files in
`src/routes/`. The `protect-generated` hook blocks manual edits — change the route files
instead and let the generator run.
