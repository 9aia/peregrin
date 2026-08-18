# Effect Conventions

Rules for writing idiomatic Effect code.

## Use `Effect.gen` for sequencing

- Use `Effect.gen` + `yield*` for sequential effectful code.
- `yield*` is the Effect equivalent of `await`. It short-circuits on the first failure.
- Prefer `Effect.gen` over `Effect.flatMap` chains for more than two steps.

## Use `pipe` for transformations

- Use `pipe` for linear data and effect transformations.
- Keep pipelines readable. If a `pipe` chain is longer than 5 steps, introduce a named intermediate.

## Services and layers

- Define services with `Context.Tag` or `Effect.Service`.
- Create a `Layer` for construction and dependency wiring.
- Provide layers at the application entry point, not in library code.
- Name the service tag the same as its interface: `class FileSystem` or `const FileSystem = Context.Tag<FileSystem, FileSystemImpl>(...)`.

## Resource safety

- Use `Effect.acquireRelease` for resources that need cleanup.
- Use `Scope` to manage the lifecycle of many resources.
- Never call `dispose` manually; let `Layer` or `Scope` handle it.

## Error handling

- Return typed errors. Use `Effect.fail` with a `Data`/`Schema` error record.
- Use `Effect.catchTag` and `Effect.catchAll` at boundaries, not in the middle of a pipeline.
- Convert recoverable errors to defects only in `main.ts` or where an invariant holds.

## Logging and tracing

- Use `Effect.log` / `Console.log` inside `Effect` programs. No bare `console.log` in library code.
- Add `span` to `Effect` programs for tracing. Use `Effect.withSpan` around operation boundaries.

## What to avoid

- Don't mix `async`/`await` with `Effect` inside the same package.
- Don't call `Effect.runSync` in library code. Effects are executed at the app boundary.
- Don't leak `Fiber` handles. Prefer `Effect.forkScoped` or `Effect.race` for concurrency.
