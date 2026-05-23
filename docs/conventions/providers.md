# Provider Conventions

Rules for everything under **`packages/acme-providers`** — pluggable adapters to external
delivery systems. The thin `@docs/conventions/providers.md` import in
`.claude/rules/providers-conventions.md` points here.

A **Provider** is an adapter to one external delivery system, scoped to a single **Channel**
(`email`, `sms`, `push`, `webhook`). The **`new-provider`** skill scaffolds everything below.

## Channel interface + factory

Each Channel has an interface in `src/interfaces/` (`EmailProvider`, `SmsProvider`, …). Each
adapter implements its Channel interface. A single **factory** switches on the provider id:

```ts
// provider.factory.ts
export function createProvider(channel: Channel, id: ProviderId, deps: ProviderDeps): Provider {
  switch (id) {
    case "ses":      return createSesEmailProvider(deps);
    case "twilio":   return createTwilioSmsProvider(deps);
    default:         throw new UnknownProviderError(channel, id);
  }
}
```

Adding a provider means adding its id literal to the **union type** in the Channel interface, so
the factory's `switch` fails to compile until the new case is handled:

```ts
// src/interfaces/email.interface.ts
export type EmailProviderId = "ses" | "sendgrid";   // add "postmark" here first
```

## YAML registry

Per-provider config lives in `config/<channel>/<provider>.yaml` — endpoints, options, rate
limits. **No hardcoded credentials**: secrets are read via env, never inlined in YAML or code.

```yaml
# config/email/ses.yaml
endpoint: https://email.eu-west-1.amazonaws.com
rateLimit: { perSecond: 14 }
# credentials come from env (AWS_*), never written here
```

## Packaging & tests

- **Subpath exports** in `src/index.ts` so consumers import exactly one Channel:
  ```ts
  // src/index.ts
  export * as email from "./email";
  export * as sms from "./sms";
  ```
- **Unit tests mock the external SDK.** A provider test never hits the network — it asserts the
  adapter maps domain calls to SDK calls and translates errors correctly.
