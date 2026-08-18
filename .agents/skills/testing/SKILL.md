---
name: testing
description: Testing conventions for the Peregrin project. Use when writing tests, setting up test files, or mocking dependencies.
---

# Testing

## Framework

- Use Vitest. Effect tests use `@effect/vitest` (`it.effect`, `it.live`).
- Test files mirror source structure: `src/parse.ts` → `tests/parse.test.ts`.

## Effect tests

- Use `it.effect` for effectful tests.
- Provide a `Layer` with `it.effect.provide(layer)`.
- Use `Effect.gen` + `yield*` for test sequencing.
- Assert failures with `Effect.runPromiseExit`.
- Use `TestClock` for time-dependent code.

## Best practices

- Test happy path, errors, and edge cases.
- Use layers, not live services, in unit tests.
- Remove `console.log` from committed tests.
