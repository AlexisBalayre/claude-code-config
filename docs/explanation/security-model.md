# Security Model

The trust, auth, secrets, and input-handling model for this project. This file is `@imported` by
the **`security-reviewer`** subagent: it is the spec that review checks against, so keep it
concrete and reviewable. State each rule as an obligation a reviewer can verify in a diff.

<!-- TODO(adapt): fill each section from the code; delete a section only when it truly does not apply (e.g. no users means no authn), and say so in one line instead. -->

## Trust boundaries

Every boundary where data or control crosses from a less-trusted to a more-trusted side must be
listed here, with the check that guards it. Anything not on this list is assumed untrusted.

| Boundary | Untrusted side | Guarded by |
| :------- | :------------- | :--------- |
| <!-- TODO(adapt): e.g. public HTTP API --> | <!-- TODO(adapt): e.g. browser / API clients --> | <!-- TODO(adapt): e.g. session middleware in `src/api/middleware/` --> |

## Authentication

<!-- TODO(adapt): the mechanism per kind of caller (end users, service-to-service, CLI/API keys), where it is verified, and the rule that no internal call is anonymous if that holds. -->

## Authorization

<!-- TODO(adapt): the permission model (roles, ownership, tenancy), the single place checks are enforced, and how a reviewer spots a missing check. -->

- Authorization is checked server-side on every request; a client-side guard is UX, never a
  control.

## Secrets handling

<!-- TODO(adapt): the one module or mechanism that reads secrets (validated config, secret manager) and the pattern reviewers grep for. -->

- Secrets are read through one validated config choke point, never inlined and never read ad hoc
  at a call site.
- Config files carry endpoints and limits, not credentials.
- Reviewers grep for `apiKey` / `token` / `secret` / `password` assignments outside that choke
  point.

## Input validation

<!-- TODO(adapt): the validation library or layer, where it runs, and any project-specific bounds. -->

- Every external input (request body, query, headers, messages, files, env) is validated at the
  boundary, before any business logic runs.
- List endpoints carry a bounded limit; no unbounded query parameters.
- Queries are parameterised; no string-interpolated SQL, shell commands, or paths built from
  input.
- Outbound requests never target a URL taken from user input without an allowlist (SSRF).

## Output and data protection

<!-- TODO(adapt): what counts as PII or sensitive data here, where it is stored, retention, and encryption at rest/in transit. -->

- No raw storage entity is returned from an external interface; responses select fields
  explicitly.
- Logs never contain secrets, PII, or raw stack traces sent to clients.
- Rendered output is escaped by default; any raw-HTML escape hatch requires sanitization.

## Abuse controls

<!-- TODO(adapt): rate limiting, quotas, lockouts, and which endpoints carry them. -->

- Authentication endpoints and expensive endpoints (fan-out, heavy queries) are rate-limited.

## Known accepted risks

| Risk | Why accepted | Revisit when |
| :--- | :----------- | :----------- |
| <!-- TODO(adapt): or "none recorded" --> | | |
