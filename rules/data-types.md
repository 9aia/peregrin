# Data Type Rules

Rules for choosing and constructing types across the workspace.

## Branded types

- Use `Brand` and `Schema.Brand` to distinguish values with the same primitive but different semantics.
- Build brands from `Schema`: `const InstanceName = Schema.String.pipe(Schema.brand("InstanceName"))`.
- Decode at boundaries. Once branded, code does not need to re-validate.

## Option and Either

- Use `Option` for values that may be absent. No `null`/`undefined` in public APIs.
- Use `Either` for values that may be either a result or an error where `Effect` is not needed.
- Use `Option.match` / `Either.match` for extraction. Avoid `if (x !== null)`.

## Collections

- Prefer immutable arrays and records. Use `Array` from `effect/Array`.
- Use `Chunk` for lazily produced, effectful collections and `HashMap`/`HashSet` for fast lookups.
- Avoid `Map`/`Set` from JS when you need structural equality or hashing semantics — use `HashMap`/`HashSet` from `effect`.

## Time and duration

- Use `DateTime` from `effect` for all timestamps. Do not pass raw `Date` objects across package boundaries.
- Use `Duration` from `effect` for delays, timeouts, and intervals. No raw milliseconds in APIs.

## Schema

- Define `Schema` for every data structure that crosses a boundary (file, network, user input, env).
- Use `Schema.decode` / `Schema.encode` explicitly. Don't rely on `as` casts or `JSON.parse`.
- Mark optional fields with `Schema.optional`. Don't use `undefined` as a sentinel inside a value.

## Secrets

- Use `Redacted` for secret strings. Never log or serialize a `Redacted`.
- Validate that a secret is present with `Schema.Redacted(Schema.String)`.
