# Testing

**Genre contract:** obligations only.

<!-- TODO(adapt): replace each placeholder with this project's rules (read the test config and 3-5 existing tests first) or delete the section. -->

## Principles

- Test behaviour through public interfaces, not implementation details; a refactor that keeps behaviour must not break tests.
- Mock only at system boundaries (network, clock, filesystem, third-party services), never the module under test or its internal collaborators.
- One reason to fail per test; name tests by the behaviour they pin down.
- A bug fix lands with a regression test that fails without the fix.

## Framework and commands

<!-- TODO(adapt): test framework, how to run unit / integration suites (TEST_CMD in .claude/project.env runs pre-commit), timeouts, coverage targets. -->

## Layout

<!-- TODO(adapt): where tests live (beside source or in a test/ tree), file naming, how a source file maps to its test, where shared fixtures and mocks live. -->

## Setup and mocks

<!-- TODO(adapt): mock/fixture conventions, per-test reset rules, fake timers, test data builders. -->

## Integration tests

<!-- TODO(adapt): what counts as integration here, which real dependencies they use, how external services are stubbed. -->

## Focus

<!-- TODO(adapt): what must always be tested per layer (e.g. authorization, error mapping, persistence queries). -->
