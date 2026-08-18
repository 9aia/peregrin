# Testing Rules

Rules for tests in the Effect + Vitest workspace.

## Test framework

- Use Vitest. Effect tests use `@effect/vitest` (`it.effect`, `it.live`).
- Every package and app has its own `tests/` or `*.test.ts` co-located with source.
- Test files mirror the source structure. `src/parse.ts` → `tests/parse.test.ts`.

## Writing tests

- Test behavior, not implementation. Assert on outcomes and error cases.
- Use `it.effect` for effectful tests. Provide a `Layer` with `it.effect.provide(layer)`.
- Use `Effect.gen` in tests to sequence assertions with `yield*`.
- Prefer `Effect.runPromiseExit` to assert failure cases explicitly.
- Use `TestClock` for time-dependent code. Avoid real `setTimeout` in tests.

## Layer and test dependencies

- Build a `Layer` for the test environment. Do not call live services unless the test is specifically `it.live`.
- Share a single test layer across a file using `describe.provide(layer)`.

## What to avoid

- Don't test only the happy path. Include missing input, invalid input, and edge cases.
- Don't use real network, filesystem, or timers in unit tests. Use `Effect` test context or mocks.
- Don't leave `console.log` in committed tests.
