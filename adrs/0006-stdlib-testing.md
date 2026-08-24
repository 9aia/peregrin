# ADR 0006: First-Class Testing Standard Library for AC

## Status

Proposed

## Context

AC has a growing set of runtime standard-library modules under `packages/ac-lang/crates/lib-*` (`lib-utils`, `lib-rand`, `lib-collections`, etc.). These are exposed as `Module` implementations that `ModuleHost` dispatches by a single dotted prefix such as `math.abs`, `rand.int`, or `array.map`. The AC standard library has no testing surface yet, which is a major adoption barrier for a functional language where property-based testing should be first-class.

We need a testing system that is:

- discoverable as ordinary language modules;
- deterministic and parallel by default;
- able to cover unit tests, assertions, property tests, benchmarks, fuzzing, and snapshots;
- built on the existing `ac-lib-rand` seeded RNG so randomness is reproducible.

## Decision

Introduce `packages/ac-lang/crates/lib-test`, a new runtime module crate that provides the following AC modules:

| AC module   | Primary responsibility                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------ |
| `test`      | Test suite definition, registration, parallel execution, fixtures, and report emission                 |
| `assert`    | Assertion primitives (`equal`, `not_equal`, `true`, `false`, `match`, `contains`)                      |
| `property`  | Property-based testing with generators, `for_all`, shrinkable counterexamples, and deterministic seeds |
| `benchmark` | Wall-clock timing of functions across iterations with simple statistics                                |
| `fuzz`      | Structure-agnostic random-input stress testing with seed control                                       |
| `snapshot`  | Snapshot record, match, and update for language values                                                 |

### Module routing

`ModuleHost` splits function names on the **first** `.` only. Each module above is a separate `Module` implementation. Calls look like `test.run(...)`, `assert.equal(...)`, `property.for_all(...)`, `benchmark.run(...)`, `fuzz.run(...)`, `snapshot.match(...)`. In a later phase, a `std` router module can be added to re-export these under the `std.test.*`, `std.assert.*`, etc., names.

### Crate design

- `ac-lib-test` follows the `ac-lib-utils`/`ac-lib-rand` pattern:
  - one Rust crate under `crates/lib-test`;
  - each logical module lives in its own file with a `Module` struct;
  - shared helpers live in `shared.rs`;
  - module-specific errors live in `error.rs`.
- All functions are `#[derive(Clone, Copy)]` and stateless, fitting the existing `Function<NonFunctionValue>` contract.
- The `test` module uses `rayon` for parallel execution when running suites with independent test cases.
- `property` and `fuzz` use the workspace `rand` crate with `StdRng` and a user-supplied or fixed seed for deterministic reproducibility.
- `assert` and `snapshot` operate purely on `NonFunctionValue` JSON data and report failures as `TestError` converted into `ac_runtime::Error`.

### Determinism and randomness

- Every function that uses randomness accepts an optional integer `seed` argument.
- If `seed` is omitted, the current system time is used; if provided, the same seed always yields the same sequence.
- This reuses the `rand` / `rand_chacha` ecosystem already depended on by `ac-lib-rand`.

### Error model

- Expected failures (assertion failures, property counterexamples, snapshot mismatches) are library-level errors returned through the `Effect`/`Result` channel, never thrown.
- The crate defines `TestError` with a `Display` impl and a `From<TestError> for ac_runtime::Error` impl.

## Consequences

### Positive

- AC gains a first-class testing experience that mirrors the rest of the standard library.
- Test runs can be parallel and deterministic from a single seed.
- Each testing concern is isolated in its own module, matching the existing `lib-*` decomposition.
- The design is forward-compatible with `std.*` namespacing when a router module is introduced.

### Trade-offs

- `rayon` adds a new dependency for parallel execution.
- Snapshot matching will eventually need filesystem access; the initial version is in-memory / value-comparison only.
- Property-based shrinking and custom generators are complex; they will be added in follow-up iterations.

## References

- `packages/ac-lang/crates/lib-utils`
- `packages/ac-lang/crates/lib-rand`
- `packages/ac-lang/crates/runtime-core/src/module/module.rs`
- `packages/ac-lang/crates/runtime-core/src/host/host.rs`
