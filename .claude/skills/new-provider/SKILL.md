---
name: new-provider
description: Scaffold a new delivery Provider (email, SMS, push, or webhook Channel) following the factory + YAML registry pattern. Auto-activates when asked to "add a provider", "integrate a new email/SMS/push provider", or "add support for <provider>".
user-invocable: false
---

Scaffold a new delivery Provider. **CRITICAL:** Follow `docs/conventions/providers.md` exactly.

## Execution Steps

1. **Determine Channel & Context:** Which Channel — `email`, `sms`, `push`, or `webhook`? Read 2-3 existing examples in `packages/acme-providers/src/providers/clients/` and `src/interfaces/` before writing code.
2. **Create Adapter (`src/providers/clients/<channel>/<provider>/<provider>-<channel>.adapter.ts`):**
   - Implement the Channel interface (`EmailProvider`, `SmsProvider`, `PushProvider`, or `WebhookProvider` from `src/interfaces/`).
   - Use shared SDK clients (`src/providers/clients/sdk/<system>.client.ts`) if applicable. Handle retries and partial failures explicitly.
   - Add JSDoc to every exported method (packages require it).
3. **Register in Factory (`src/providers/factories/<channel>.factory.ts`):**
   - Add the new provider ID literal to the union type in `src/interfaces/<channel>.ts`.
   - Add the new case to the factory switch (`createProvider(channel, id, deps)`).
4. **Add Config (`config/<channel>/<provider>.yaml`):**
   - Define endpoints, options, and rate limits. NEVER hardcode credentials — read them via env.
5. **Update Exports:** Ensure correct exposure via subpath exports in `src/index.ts`.
6. **Add Tests (`test/unit/providers/clients/<channel>/<provider>/<provider>-<channel>.test.ts`):**
   - Mock the external SDK. ALWAYS mock `@acme/acme-logger`.
   - Test success, error handling, retry/backoff, and config mapping.
7. **Verify:**
   - `pnpm --filter @acme/acme-providers typecheck`
   - `pnpm --filter @acme/acme-providers test`
