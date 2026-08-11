# Frontend (`apps/acme-web/`)

React 19, Vite, Tailwind v4, react-router-dom v7, TanStack Query, BetterAuth cookies, Sonner.

**Genre contract:** obligations only. Layout map + guard inventory live in [frontend-architecture.md](../reference/frontend-architecture.md).

**Reference-doc sync:** a PR that changes the route table (`app.tsx`), the guard set, or the transport adapters MUST update [frontend-architecture.md](../reference/frontend-architecture.md) in the same PR.

## Layout

- `features/<domain>/`: feature-scoped components (containers + domain UI).
- `components/ui/`: reusable primitives; `components/layout/`: shells, sidebar, topbar; `components/shared/`: cross-feature guards and providers (`AuthGuard`, `RoleGuard`, `AuthProvider`).
- `hooks/`, `lib/` (`api-client.factory.ts`, `cn.utils.ts`, `runtime-config.utils.ts`, …), `transports/` (`ws/`, `transports.types.ts`, `use-transport.hook.ts`).
- `api/`: TanStack Query hooks per resource (`use-*.hook.ts`), calling `api/adapters/` for the typed API-client requests underneath. `data/`: static datasets. `styles/globals.css`.

## Routing (react-router-dom v7)

- `app.tsx`: single route table via `createBrowserRouter`. Add routes by editing the `children` arrays. Three layout groups: `AuthLayout` (centered, no sidebar), `PublicLayout` (fullscreen, no shell), `AppShell` (sidebar + topbar).
- Auth/role gates use `<AuthGuard />` and `<RoleGuard allowedRoles={...} />` as parent route elements (children inherit).
- Compatibility redirects use a small `useParams` + `<Navigate>` component: `<Navigate to="/join/:code">` does **not** interpolate the param.
- Path alias `@/` → `./src/` (set in `vite.config.ts` + `tsconfig.json`).

## Role guards

- `RoleGuard` groups must match the API guards **exactly**.
- The entire `/admin/*` surface admits the platform-admin tier (`["admin", "super_admin"]`); there is no inner super_admin-only group at the route level.
- Super_admin-reserved writes are enforced by per-handler API guards, not route-level FE concerns.
- Navigation destinations are resolved in `hooks/use-app-nav.hook.ts` (consumed by the sidebar and the command palette); it gates platform-admin nav on `isPlatformAdmin` (`useRoles()`). When adding admin routes, restructure both `app.tsx` and `use-app-nav.hook.ts` together so menus and guards stay aligned.

## Components

- Styling via `cn()` (clsx + tailwind-merge). Variants with CVA: export the variants const.
- Never CSS modules / styled-components. Never hardcode colors (use CSS vars). Inline styles only for dynamic runtime values (framer-motion, dynamic colors).
- Props are `Readonly<T>` (see [`core.md`](core.md)).

## State

- Context + hooks only. Never Redux/Zustand.
- `SessionView` receives all state as props; keep transport logic out of UI.

## Transport abstraction

- UI writes against the transport adapter (`src/transports/ws/ws.adapter.ts`); its event payload types live in `transports.types.ts`. Never import the raw socket library outside `src/transports/`.
- `useTransport(config)` owns connect/teardown and the state its events drive; it does not wrap the adapter's methods. Call them on the returned `adapter` (null until the connect effect builds one). Subscribe in `useEffect`, clean up in the return.

## API client

- `getApiClient()` is a singleton with `credentials: "include"`. Call only inside hooks, event handlers, or `useEffect`. **Never** at module scope.

## Styling (Tailwind v4)

- Theme tokens via `@theme inline` in `globals.css` (CSS vars on `:root` / `.dark`).
- Brand colors: `acme-*` scale. Semantic: `primary`, `secondary`, `destructive`, `success`, `warning`, `muted`, `accent` (+ `-foreground`).
- Dark mode: `[data-theme]` on `<html>` drives CSS vars; `dark:` uses `@custom-variant dark` on `[data-theme=dark]`. Preference: `localStorage` `acme-theme-preference` (`system` | `light` | `dark`).
