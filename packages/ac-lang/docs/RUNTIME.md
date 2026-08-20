# AC Runtime

**Version:** 0.1.0
**Status:** Draft

The AC runtime is the execution environment for AC language documents. It is analogous to Node.js — it provides the host functions, module resolution, environment access, and tooling pipeline that make AC documents executable and validatable.

For the language specification itself, see [LANG.md](./LANG.md).

## 1. Overview

The runtime provides:

- **VM** — evaluates expression trees with host-provided functions.
- **Host functions** — `add`, `mul`, `concat`, `len`, `import`, and user-defined function bodies.
- **Environment access** — `env` namespace exposes process environment variables.
- **Module resolution** — file, HTTP, and Git resolvers for `import()` calls.
- **Schema validation** — JSON Schema (draft 2020-12) validation of field values.
- **Source location tracking** — line:column mapping for error reporting.
- **Engine pipeline** — composable passes for parsing, extraction, import resolution, type checking, and type generation.
- **CLI** — the `ac` command-line tool.

## 2. Architecture

The runtime is organized into five layers, each building on the one below. No layer depends on a layer above it.

```
Layer 5 — CLI                     CLI tool
Layer 4 — Engine pipeline         engine, pipeline core, pass implementations, benchmarks
Layer 3 — Type system & generation type checking, type generation, module resolution
Layer 2 — Domain logic            fields, host functions, frontend factory
Layer 1 — Core                    AST, values, VM, schema, source locations, format frontends
```

### 2.1 Layer responsibilities

#### Layer 1 — Core

| Component         | Responsibility                                                 |
| ----------------- | -------------------------------------------------------------- |
| **AST**           | Expression trees and field bindings — generic, no dependencies |
| **Values**        | JSON-compatible value type, frontend interface                 |
| **VM**            | Expression evaluator, host interface, execution context        |
| **Schema**        | Schema validation interface, validation error                  |
| **Schema (JSON)** | JSON Schema implementation of the schema interface             |
| **Source**        | Source location, source map, line index, key matcher           |
| **Frontend-JSON** | JSON parse/serialize to values + source map                    |
| **Frontend-YAML** | YAML parse/serialize to values + source map                    |
| **Frontend-TOML** | TOML parse/serialize to values + source map                    |

#### Layer 2 — Domain logic

| Component            | Responsibility                                                             |
| -------------------- | -------------------------------------------------------------------------- |
| **Fields**           | Field extraction, interpolation, computed fields, merge, scope, validation |
| **Host**             | Default host with `add`, `mul`, `concat`, `len` primitives                 |
| **Frontend factory** | Maps format names to frontend implementations                              |

#### Layer 3 — Type system, type generation, module resolution

| Component                 | Responsibility                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------- |
| **Typecheck core**        | Type representation, type environment, type errors, schema interpreter, compatibility |
| **Typecheck inference**   | Expression type inference, expression AST, function table                             |
| **Typecheck schema-JSON** | JSON Schema → type mapping                                                            |
| **Typecheck key checks**  | Per-key type checking: `expr`, `arg`, `merge`, `inheritable`, `mutable`, `import`     |
| **Typegen core**          | Typegen context, backend interface, key stripping                                     |
| **Typegen key handlers**  | Strip AC keys, filter merge fields, detect imports                                    |
| **Typegen import**        | Follow `import()` expressions, merge imported schemas                                 |
| **Typegen backend-JSON**  | JSON Schema output backend                                                            |
| **Resolver core**         | Module resolver interface, module cache, caching resolver, chain resolver             |
| **Resolver file**         | Filesystem-based module loading                                                       |
| **Resolver virtual**      | In-memory module loading (for tests)                                                  |
| **Resolver HTTP**         | HTTP(S) module loading                                                                |
| **Resolver Git**          | Git repository module loading                                                         |

#### Layer 4 — Engine pipeline

| Component           | Responsibility                                                                                 |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| **Engine core**     | Compilation state, configuration, pass interface, pipeline builder                             |
| **Pass: parse**     | Parse source text into values + source map                                                     |
| **Pass: extract**   | Extract fields, functions, data from parsed document                                           |
| **Pass: resolve**   | Resolve imports, build typegen context                                                         |
| **Pass: typecheck** | Typecheck all AC keys and cross-module imports (parallelizable)                                |
| **Pass: generate**  | Generate output from resolved typegen context                                                  |
| **Engine**          | Preset pipelines + non-pipeline operations (parse, eval, validate, fields, typegen, typecheck) |
| **Benchmarks**      | Performance benchmarks for core, pipeline, inference, typecheck                                |

#### Layer 5 — CLI

| Component | Responsibility                                          |
| --------- | ------------------------------------------------------- |
| **CLI**   | `ac` command-line tool — thin wrapper around the engine |

### 2.2 Dependency constraints

- The AST has zero external dependencies.
- The schema interface depends only on source locations (for error reporting).
- Source location tracking is format-agnostic.
- The value type is the only component that couples to a JSON value library.
- The fields module does not depend on the VM — the caller bridges the two via a function resolver interface.
- The typecheck core and typegen core have no knowledge of AC-specific keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`). Each key has its own handler.
- The resolver core defines the module resolver and module cache interfaces; implementations (file, virtual, HTTP, Git) do the I/O.
- The engine core is pass-agnostic — it defines the pass interface and pipeline builder but depends on no specific passes.
- The engine depends on all pass implementations and provides preset pipelines.
- The CLI depends only on the engine — all logic lives in the engine.

## 3. Value type

### 3.1 Runtime value

The concrete value type used throughout the runtime. It wraps a JSON-compatible value, supporting: null, boolean, number, string, array, and object.

The type provides:

- **Predicates** to test the kind of a value (null, bool, number, string, array, object).
- **Accessors** to read typed values (boolean, number, string, array, object elements/properties).
- **Conversions** from and to JSON values, and from primitive types (bool, number, string).

### 3.2 Host value

An alias for the runtime value used at the VM/host boundary to clarify intent.

## 4. Virtual machine

### 4.1 Execution engine

The VM walks expression trees and produces values. It is parameterized by a host that provides functions.

**Evaluation rules:**

1. A `Value` node evaluates to itself.
2. A `Call` node evaluates each argument left-to-right, resolves the function by name via the host, then invokes the function with the evaluated arguments and the current context.

### 4.2 Host interface

The host provides function lookups by name. The VM has no built-in functions — all capabilities come from the host implementation.

### 4.3 Default host functions

The default host provides these primitives:

| Function | Params                     | Returns  | Description               |
| -------- | -------------------------- | -------- | ------------------------- |
| `add`    | `x: number, y: number`     | `number` | Arithmetic addition       |
| `mul`    | `x: number, y: number`     | `number` | Arithmetic multiplication |
| `concat` | `x: string, y: string`     | `string` | String concatenation      |
| `len`    | `x: array\|string\|object` | `number` | Length of collection      |

### 4.4 Function interface

A function receives evaluated arguments and the execution context, and returns a value or an error. Functions must be callable without exclusive access to themselves; functions requiring mutable state must use interior mutability patterns.

### 4.5 Execution context

The context maintains a stack of scopes for variable bindings.

| Operation                  | Behavior                                                                           |
| -------------------------- | ---------------------------------------------------------------------------------- |
| **Create**                 | Initializes with one empty global scope.                                           |
| **Push scope / Pop scope** | Model function call boundaries. The global scope is never popped.                  |
| **Define**                 | Sets or overwrites a variable in the current scope.                                |
| **Resolve**                | Searches from the innermost scope outward. Returns `UnknownVariable` if not found. |
| **Depth**                  | Reports the current scope depth (global = 1).                                      |

### 4.6 VM errors

| Error             | Meaning                                                   |
| ----------------- | --------------------------------------------------------- |
| `UnknownFunction` | A function name was not found in the host.                |
| `UnknownVariable` | A variable name was not found in any scope.               |
| `PrimitiveError`  | A primitive operation failed with a message.              |
| `TypeMismatch`    | A value did not match the expected type.                  |
| `ArgumentCount`   | The number of arguments did not match the expected count. |

Errors are comparable for testing and replay purposes.

## 5. Schema validation

### 5.1 Schema interface

A schema validates a value of type `V`, returning a validation error if the value does not conform. The interface is generic over `V` and independent of any specific schema language.

### 5.2 Validation error

A validation error carries:

- **message** — human-readable description of the failure.
- **path** — dotted path to the offending element within the value (empty for root).
- **location** — optional source location (line:column), populated by the engine from the source map after validation.

### 5.3 JSON Schema validator

The default schema implementation validates values against a JSON Schema document using a full JSON Schema draft 2020-12 validator. Supports the complete JSON Schema vocabulary (`$ref`, `allOf`, `oneOf`, `anyOf`, `format`, `pattern`, `minimum`/`maximum`, `minLength`/`maxLength`, `minItems`/`maxItems`, `uniqueItems`, `enum`, `const`, etc.).

The validator also exposes a `type_name()` helper that returns the declared type (first type for multi-type schemas), used by the fields pipeline for reporting.

## 6. Frontends

### 6.1 Frontend interface

A frontend parses a text input into a value and serializes a value back to text. The interface is defined in the core layer; each frontend provides one implementation.

The interface includes an optional `parse_with_locations()` method that returns both the parsed value and a source map mapping dotted paths to source locations. Each frontend overrides this with format-specific scanning logic (indentation for YAML, braces for JSON, sections for TOML). The default implementation returns an empty source map.

### 6.2 Built-in frontends

| Frontend | Format |
| -------- | ------ |
| JSON     | JSON   |
| YAML     | YAML   |
| TOML     | TOML   |

### 6.3 Frontend factory

The frontend factory provides a `frontend_for(format)` function that returns a frontend for the requested format name. Supported formats: `yaml`/`yml`, `json`, `toml`. This is interface-agnostic — usable from CLI, MCP, or any consumer that needs to parse/serialize AC documents.

Adding a new format (XML, INI, etc.) requires only implementing the frontend interface and adding a match arm to the factory. The core, VM, and schema system are unaffected.

## 7. Source location tracking

A source location is a 1-indexed line:column pair. A source map maps dotted paths (e.g. `"server.port"`) to source locations. Both are pure data structures in the source component — format-specific scanning lives in each frontend.

The source map is built during parsing by each frontend's `parse_with_locations()` method, which scans the source text to locate key positions. The map is stored in the compilation state and used post-hoc to resolve error locations.

Shared scanning utilities:

- **Line index** — precomputed line-start offsets for efficient byte-offset → line:column conversion.
- **Key matcher** — matches unquoted, double-quoted, and single-quoted keys with a configurable separator (`:` for YAML/JSON, `=` for TOML).
- **Walk value** — recursively walks a parsed value tree, calling a callback for each object key with its dotted path and byte-offset context.
- **Leading whitespace** — counts leading spaces/tabs for indentation-based scanning.

## 8. Module resolution

Module resolution abstracts how `import()` paths are loaded. The module resolver interface defines the contract; implementations do the I/O.

### 8.1 Module resolver interface

A module resolver resolves a module path (relative to an optional base path) and returns a resolved module containing:

- **schemas** — field name → raw schema fragment pairs (AC keys still present).
- **merge** — field names marked `merge: true` in the imported module.
- **functions** — function signatures declared in the module.
- **data** — the document's data values.
- **base_path** — base path for resolving nested imports from this module.

The module resolver interface requires thread-safety — resolvers may be shared across pipeline passes and threads (e.g., parallel type checking).

### 8.2 Module cache

The module cache interface abstracts resolver-level caching. Implementations:

| Implementation | Behavior                                   |
| -------------- | ------------------------------------------ |
| Memory cache   | Thread-safe in-memory cache. Default.      |
| No cache       | No-op cache — every lookup returns a miss. |

Each resolver is parameterized over a module cache, allowing callers to choose caching behavior. Resolvers use resolver-specific cache keys:

- **File resolver** — canonical filesystem path.
- **HTTP resolver** — full URL string.
- **Git resolver** — `repo_url@ref/path` (module-level), plus an internal repo cache for clone-level dedup.

### 8.3 Caching resolver

The caching resolver wraps any module resolver with a module cache. The cache key is `path:base` (stringified). Subsequent calls with the same arguments return a cached value without re-invoking the inner resolver. This eliminates redundant disk I/O, parsing, and field extraction for repeated imports — including when the resolve pass and typecheck pass both resolve the same import graph.

For resolvers that need resolver-specific caching (e.g., Git's repo-level clone cache), prefer passing a module cache directly to the resolver instead of wrapping with the caching resolver.

### 8.4 Chain resolver

The chain resolver tries multiple resolvers in order, returning the first successful result. This enables mixed import sources in a single document — e.g., local files, HTTP URLs, and Git URLs — without the caller needing to know which resolver handles which path.

Each resolver returns a miss for paths it doesn't handle, so the chain falls through to the next. Resolvers that can quickly determine a path is not theirs (e.g., file resolver sees `git+` prefix) should short-circuit early.

### 8.5 Implementations

| Implementation   | I/O                                   |
| ---------------- | ------------------------------------- |
| File resolver    | Filesystem — reads from disk          |
| Virtual resolver | In-memory hash map                    |
| HTTP resolver    | HTTP(S) client                        |
| Git resolver     | Git repositories (bare shallow clone) |

**File resolver** auto-detects format by file extension (YAML/JSON/TOML), parses, and extracts fields. **Virtual resolver** stores pre-parsed documents in memory — ideal for tests and embedded scenarios.

**HTTP resolver** fetches documents over HTTP(S). Module paths are URLs. Relative paths are resolved against the base URL (the URL of the importing document). Format is auto-detected from the URL path extension.

**Git resolver** fetches documents from Git repositories using the `git+` URL scheme:

```text
import('git+https://github.com/org/repo.git@main/schemas/base.yml')
```

Repositories are cloned to a cache directory keyed by a hash of `(repo_url, ref)`. Clones are **bare** and **shallow** (depth=1 by default, configurable). Subsequent resolves from the same repo+ref fetch updates from the remote before reading. Files are read from the Git object store (not the working tree). Relative paths within a Git repo resolve against the `git+` base URL.

## 9. Function resolver — VmResolver

The VmResolver bridges the function resolver interface to the VM + host. Key behaviors:

- **Recursion depth limit** — a depth counter with `MAX_DEPTH = 64`. Incremented before each call, decremented after. Prevents infinite recursion in user-defined functions.
- **Import handling** — `import()` calls are intercepted before VM dispatch. The resolver uses a module resolver to load the file, caches results, and merges imported function signatures.
- **Param validation** — before dispatching to the VM or evaluating a body, args are validated against param schemas using the schema validator.
- **Return validation** — after the call, the return value is validated against the return schema (if declared).

## 10. Engine pipeline

The engine provides a composable pipeline for compile-time operations: parsing, field extraction, import resolution, type checking, and type generation. The pipeline is a sequence of stateless passes that run in order, each receiving the compilation state and configuration.

### 10.1 Compilation state

A compilation flows through the pipeline, accumulating state as each pass runs:

| Field         | Populated by     | Description                       |
| ------------- | ---------------- | --------------------------------- |
| `source`      | Created at start | Input text + format + path        |
| `parsed`      | Parse pass       | Parsed value                      |
| `extracted`   | Extract pass     | Extracted fields, functions, data |
| `typegen_ctx` | Resolve pass     | Typegen context                   |
| `type_errors` | Typecheck pass   | List of type errors               |
| `output`      | Generate pass    | Generated output                  |
| `source_map`  | Parse pass       | Path → location mapping           |

After all passes complete, the pipeline resolves error locations using the source map, populating the `location` field on any type errors that don't already have one.

### 10.2 Configuration

The configuration holds the pluggable components:

- **resolver** — how modules are loaded (module resolver).
- **interpreter** — how schema fragments become types (schema interpreter).
- **backend** — how types are emitted (typegen backend, optional for typecheck-only runs).

The engine provides default configurations:

- **Default** — file resolver (with memory cache) + JSON Schema interpreter + JSON Schema backend.
- **Typecheck-only** — file resolver (with memory cache) + JSON Schema interpreter + no backend.

### 10.3 Pass interface

Each pass implements:

- **`run(compilation, config)`** — execute the pass, mutating the compilation state.
- **`should_run(compilation, config)`** — return false to skip when preconditions are not met.

Custom passes (linting, optimization, JIT) implement the pass interface and slot into the pipeline without core changes.

### 10.4 Built-in passes

| Pass      | Input → Output                                  |
| --------- | ----------------------------------------------- |
| Parse     | source text → parsed value + source map         |
| Extract   | parsed value → extracted fields                 |
| Resolve   | extracted fields → typegen context              |
| Typecheck | extracted fields → type errors (parallelizable) |
| Generate  | typegen context → output                        |

### 10.5 Preset pipelines

| Preset      | Passes                                           | Use case                         |
| ----------- | ------------------------------------------------ | -------------------------------- |
| `typegen`   | parse → extract → resolve → generate             | Generate JSON Schema from fields |
| `typecheck` | parse → extract → typecheck                      | Static type checking             |
| `full`      | parse → extract → resolve → typecheck → generate | Type-safe code generation        |

### 10.6 Parallel type checking

The typecheck pass parallelizes independent checks. Six leaf checks run concurrently:

- **Schema-level**: `expr` key check, `arg` key check, `merge` key check
- **Data-level**: `inheritable` key check, `mutable` key check, import type check

The module resolver and module cache must be thread-safe to support this.

## 11. CLI

The `ac` command-line tool is a thin wrapper around the engine. It provides argument parsing and delegates all logic to the engine.

Commands: `parse`, `eval`, `validate`, `fields`, `typegen`, `typecheck`.

The CLI provides default host functions (`add`, `mul`, `concat`, `len`, `import`) via the default host and VmResolver.

## 12. Facade

The top-level package is a batteries-included facade that re-exports all public components in a single dependency. It provides a prelude for convenient imports. This is the recommended entry point for most users.

## 13. Testing and benchmarking

### 13.1 Unit tests

Unit tests are co-located with implementation, keeping tests close to the code they test.

### 13.2 Integration tests

Integration tests cover the full pipeline (parse → extract → resolve → typecheck → generate) with real YAML/JSON/TOML inputs.

### 13.3 Fuzzing

The runtime includes fuzzing infrastructure with targets for expression parsing, interpolation, scoped interpolation, and expression type inference. Curated seed corpora cover edge cases: paths, unary operators, multi-arg calls, empty inputs, comma handling, unmatched parentheses.

### 13.4 Benchmarks

Benchmarks measure:

| Benchmark | What it measures                                              |
| --------- | ------------------------------------------------------------- |
| Core      | Schema interpretation and type compatibility (isolated)       |
| Inference | Expression type inference (parsing + inference)               |
| Typecheck | Per-key typecheck checks (expr, arg, merge, inherit, mutable) |
| Pipeline  | End-to-end pipeline (parse → extract → typecheck) at scale    |

Benchmarks cover flat documents, `expr:`-heavy documents, inheritable fields with nesting, and mixed-feature documents at varying sizes (10–1000 fields). Individual pass costs are also measured (parse-only, parse+extract).

## 14. Extension points

| Extension                | How                                                   | Touches            |
| ------------------------ | ----------------------------------------------------- | ------------------ |
| New format               | Implement the frontend interface + add to the factory | New component only |
| New schema language      | Implement the schema interface                        | New component only |
| New schema interpreter   | Implement the schema interpreter for type checking    | New component only |
| New typegen backend      | Implement the typegen backend for type generation     | New component only |
| New module resolver      | Implement the module resolver interface               | New component only |
| New module cache         | Implement the module cache interface                  | New component only |
| New host function        | Implement the function interface, register with host  | Host impl only     |
| Custom namespace         | Insert into the namespaces map                        | Caller code        |
| Custom function resolver | Implement the function resolver interface             | Caller code        |
| Custom pipeline pass     | Implement the pass interface, add to pipeline         | New component only |
| Native values            | Extend the value type with a native variant           | Value type only    |
| Multi-source imports     | Compose resolvers with the chain resolver             | Caller code        |

## 15. Porting notes

When porting the runtime to another language:

1. **Start with the AST** — define expression trees and field bindings as generic types with no dependencies.
2. **Define the value type** — wrap the target language's JSON value type. Implement predicates, accessors, and conversions.
3. **Implement the frontend interface** — at minimum YAML and JSON. Use the target language's native YAML/JSON parsers.
4. **Implement the schema interface** — use a JSON Schema validator library for the target language.
5. **Implement the VM** — a simple recursive tree walker. The host interface provides function lookup.
6. **Implement field extraction** — single-pass extraction of schemas, functions, data, inheritable, immutable, computed, merge.
7. **Implement interpolation** — recursive `${...}` replacement with path lookup and function call resolution.
8. **Implement the expression parser** — recursive descent for function calls, path lookups, and literals. Handle string literals, nested parens, and comma-separated arguments.
9. **Implement module resolvers** — at minimum a file resolver. Use the module cache pattern for memoization.
10. **Implement the engine pipeline** — pass interface + pipeline builder. Each pass reads/writes a compilation state.
11. **Source location tracking** — scan source text during parsing to build a path → location map. Resolve error locations after all passes.
12. **Parallel type checking** — if the target language supports parallelism, parallelize independent type checks. The module resolver and module cache must be thread-safe.
