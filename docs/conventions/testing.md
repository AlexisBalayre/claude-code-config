# Testing Conventions

Rules for test files across the monorepo. Auto-loaded via
`.claude/rules/testing-conventions.md` when you touch a test. The runner is **Vitest**.

## Test kinds

| Kind        | File                       | Runs against                                       |
| :---------- | :------------------------- | :------------------------------------------------- |
| Unit        | `*.test.ts`                | Pure logic in isolation; colocated or under `test/`.|
| Integration | `*.integration.test.ts`    | A real server + database (`pnpm … test:integration`).|

Unit tests must not touch the network or a real DB — mock those boundaries. Integration tests
are where you exercise the wired-up stack end to end.

## Mocking

- **Mock external SDKs** (provider clients, gRPC stubs, BetterAuth) — never hit a third party
  in a test.
- **Mock ordering matters.** `vi.mock` is hoisted, but factory references and spies are not:
  declare and configure mocks **before** importing the unit under test, or the unit captures
  the real implementation at import time.

  ```ts
  import { vi, describe, it, expect } from 'vitest';

  vi.mock('../providers/email.client', () => ({
    sendViaEmail: vi.fn().mockResolvedValue({ delivered: true }),
  }));

  // import AFTER the mock is declared
  import { dispatchMessage } from './message-dispatch.service';
  ```

- **Use fake timers** for anything time-dependent (retry backoff, debounce, TTL expiry):
  ```ts
  vi.useFakeTimers();
  // …
  vi.advanceTimersByTime(5_000);
  vi.useRealTimers();
  ```

## The logger

The logger is **globally suppressed in API tests** — you don't need to silence it per file.
Mock it **only** when a test needs to assert that something was logged (e.g. a warning on a
dropped Event).

## Structure

- **Arrange / Act / Assert.** Keep the three phases visually distinct.
- **Test behaviour and public contracts, not implementation details.** Assert on what a
  caller observes (return value, thrown `AppError`, emitted Event), not on private internals.
  Tests coupled to internals break on every refactor.
- **One logical assertion focus per test.** A test verifies a single behaviour; multiple
  `expect`s are fine when they describe one outcome.

```ts
describe('dispatchMessage', () => {
  it('marks the Message delivered when the Provider accepts it', async () => {
    // arrange
    const db = makeTestDb();
    const message = await seedMessage(db, { body: 'hello' });

    // act
    const result = await dispatchMessage(db, message.id);

    // assert
    expect(result.status).toBe('delivered');
  });
});
```

## Cross-links

- The **`tdd`** skill drives the red-green-refactor loop — write the failing test first, make
  it pass, then refactor.
- The `testing-conventions` rule auto-loads this doc whenever you open a test file, so the
  conventions are in context while you write.
