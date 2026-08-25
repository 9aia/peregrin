# handl Runtime

**Version:** 0.1.0
**Status:** Draft

The handl runtime is the execution environment for handl language documents. It is analogous to Node.js — it provides the host functions, module resolution, environment access, and tooling pipeline that make handl documents executable and validatable.

For the language specification itself, see [LANG.md](./LANG.md). For the dialect interface that abstracts the syntax in implementations, see [DIALECT.md](./DIALECT.md).

## 1. Overview

The runtime provides:

- **VM** — evaluates expression trees with host-provided functions, operations, and lazy functions.
- **Host functions** — `add`, `mul`, `concat`, `len`, `import`, `apply`, and user-defined function bodies.
- **Lazy functions** — short-circuit evaluation for `if`, `and`, `or` via the `LazyFunction` trait.
- **Async functions** — non-blocking I/O via the `AsyncFunction` trait and `eval_async`.
- **Operations & handlers** — algebraic effect handling via the `Operation` trait and `handle` expressions.
- **Resumable evaluation** — serializable continuations for cross-machine evaluation migration via `eval_resumable`.
- **Caching host** — `CallCache` + `CacheHost` for cross-machine resumability without re-execution.
- **Content-addressed storage** — `DefinitionStore`, `NameRegistry`, `EvalCache` for hash-based dedup and caching.
- **Module system** — `Module` trait + `ModuleHost` for composing qualified function namespaces (`fs.read`, `http.get`, `math.abs`).
- **Platform services** — abstract `Fs`, `Env`, `Path`, `Time`, `HttpClient`, `Cmd`, `Tmp` service traits with stub and stdlib implementations.
- **Standard library** — math, string, datetime, crypto, random, key-value, and internationalization modules.
- **Environment access** — `env` namespace exposes process environment variables.
- **Module resolution** — file, HTTP, Git, virtual, and noop resolvers for `import()` calls.
- **Schema validation** — JSON Schema (draft 2020-12) validation of field values.
- **Source location tracking** — line:column mapping for error reporting.
- **Dialect system** — `Dialect` trait abstracting syntax tokens, key names, and schema vocabulary behind a swappable interface.
- **Compiler pipeline** — composable passes for parsing, extraction, import resolution, type checking, and type generation.
- **CLI** — the `handl` command-line tool.

## 2. Architecture

The runtime is organized into six layers, each building on the one below. No layer depends on a layer above it.

```
Layer 6 — CLI                     CLI tool
Layer 5 — Compiler pipeline         compiler, pipeline core, pass implementations, benchmarks
Layer 4 — Type system & generation type checking, type generation, module resolution
Layer 3 — Domain logic            fields, host functions, frontend factory, dialect, platform services
Layer 2 — Runtime                 values, VM, host, continuation, store, modules, standard library
Layer 1 — Core                    AST, schema, source locations, format frontends
```

### 2.1 Layer responsibilities

#### Layer 1 — Core

| Component         | Responsibility                                                 |
| ----------------- | -------------------------------------------------------------- |
| **AST**           | Expression trees and field bindings — generic, no dependencies |
| **Schema**        | Schema validation interface, validation error                  |
| **Schema (JSON)** | JSON Schema implementation of the schema interface             |
| **Source**        | Source location, source map, line index, key matcher           |
| **Frontend-JSON** | JSON parse/serialize to values + source map                    |
| **Frontend-YAML** | YAML parse/serialize to values + source map                    |
| **Frontend-TOML** | TOML parse/serialize to values + source map                    |

#### Layer 2 — Runtime

| Component             | Responsibility                                                                                                                     |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Values**            | `NonFunctionValue` (wraps `serde_json::Value`), `FunctionValue`, `Frontend` trait                                                  |
| **VM**                | Expression evaluator, `Host`/`Function`/`AsyncFunction`/`LazyFunction`/`Operation` traits, `Context`, `Continuation`, `EvalResult` |
| **Host**              | Default host with `add`, `mul`, `concat`, `len` primitives                                                                         |
| **Host-cache**        | `CallCache` + `CacheHost` for cross-machine resumability                                                                           |
| **Store**             | `DefinitionStore`, `NameRegistry`, `EvalCache` for content-addressed storage                                                       |
| **Runtime-core**      | `Module` trait + `ModuleHost` for composing qualified function namespaces                                                          |
| **Dialect**           | `Dialect`/`Syntax`/`ExprParser`/`DocumentVocabulary`/`SchemaVocabulary` traits                                                     |
| **Dialect-canonical** | Canonical handl dialect implementation (JSON Schema vocabulary, `${...}` syntax)                                                   |
| **lib-utils**         | Math, string, and utility modules                                                                                                  |
| **lib-crypto**        | Cryptographic functions module                                                                                                     |
| **lib-datetime**      | Date/time functions module                                                                                                         |
| **lib-rand**          | Random number generation module                                                                                                    |
| **lib-kv**            | Key-value store interface + hashmap implementation                                                                                 |
| **lib-intl**          | Internationalization (number formatting, string comparison, segmentation, display names, duration formatting, supported locales)   |
| **lib-platform**      | Abstract `Fs`, `Env`, `Path`, `Time`, `HttpClient`, `Cmd`, `Tmp` service traits                                                    |
| **lib-platform-std**  | Concrete stdlib implementations of platform service traits                                                                         |

#### Layer 3 — Domain logic

| Component            | Responsibility                                                             |
| -------------------- | -------------------------------------------------------------------------- |
| **Fields**           | Field extraction, interpolation, computed fields, merge, scope, validation |
| **Frontend factory** | Maps format names to frontend implementations                              |

#### Layer 4 — Type system, type generation, module resolution

| Component                 | Responsibility                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------- |
| **Typecheck core**        | Type representation, type environment, type errors, schema interpreter, compatibility |
| **Typecheck inference**   | Expression type inference, expression AST, function table                             |
| **Typecheck schema-JSON** | JSON Schema → type mapping                                                            |
| **Typecheck key checks**  | Per-key type checking: `expr`, `arg`, `merge`, `inheritable`, `mutable`, `import`     |
| **Typegen core**          | Typegen context, backend interface, key stripping                                     |
| **Typegen key handlers**  | Strip handl keys, filter merge fields, detect imports                                 |
| **Typegen import**        | Follow `import()` expressions, merge imported schemas                                 |
| **Typegen backend-JSON**  | JSON Schema output backend                                                            |
| **Resolver core**         | Module resolver interface, module cache, caching resolver, chain resolver             |
| **Resolver file**         | Filesystem-based module loading                                                       |
| **Resolver virtual**      | In-memory module loading (for tests)                                                  |
| **Resolver HTTP**         | HTTP(S) module loading                                                                |
| **Resolver Git**          | Git repository module loading                                                         |
| **Resolver memory**       | In-memory module cache                                                                |
| **Resolver noop**         | No-op resolver (for sandboxed/no-import hosts)                                        |

#### Layer 5 — Compiler pipeline

| Component           | Responsibility                                                                                 |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| **Compiler core**   | Compilation state, configuration, pass interface, pipeline builder                             |
| **Pass: parse**     | Parse source text into values + source map                                                     |
| **Pass: extract**   | Extract fields, functions, data from parsed document                                           |
| **Pass: resolve**   | Resolve imports, build typegen context                                                         |
| **Pass: typecheck** | Typecheck all handl keys and cross-module imports (parallelizable)                             |
| **Pass: generate**  | Generate output from resolved typegen context                                                  |
| **Engine**          | Preset pipelines + non-pipeline operations (parse, eval, validate, fields, typegen, typecheck) |
| **Benchmarks**      | Performance benchmarks for core, pipeline, inference, typecheck                                |

#### Layer 6 — CLI

| Component | Responsibility                                               |
| --------- | ------------------------------------------------------------ |
| **CLI**   | `handl` command-line tool — thin wrapper around the compiler |

### 2.2 Dependency constraints

- The AST has zero external dependencies.
- The schema interface depends only on source locations (for error reporting).
- Source location tracking is format-agnostic.
- The value type is the only component that couples to a JSON value library.
- The VM depends only on the AST and value type — it knows nothing about modules, fields, or I/O.
- The continuation system depends only on the AST and value type — all data is JSON-serializable.
- The dialect traits depend only on the value type — no syntax or I/O dependencies.
- The platform service traits have no platform dependencies — stubs are in-memory, stdlib implementations are in a separate crate.
- The module system (`Module` / `ModuleHost`) depends only on the VM `Host` trait — the VM is unaware of modules.
- The standard library modules depend only on the value type and module trait — no VM or compiler dependencies.
- The fields module does not depend on the VM — the caller bridges the two via a function resolver interface.
- The typecheck core and typegen core have no knowledge of handl-specific keys (`expr`, `arg`, `merge`, `inheritable`, `mutable`). Each key has its own handler.
- The resolver core defines the module resolver and module cache interfaces; implementations (file, virtual, HTTP, Git, noop) do the I/O.
- The compiler core is pass-agnostic — it defines the pass interface and pipeline builder but depends on no specific passes.
- The compiler depends on all pass implementations and provides preset pipelines.
- The CLI depends only on the compiler — all logic lives in the compiler.

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

The VM walks expression trees and produces values. It is parameterized by a host that provides functions, operations, and lazy functions.

**Evaluation rules:**

1. A `Value` node evaluates to itself.
2. A `Call` node evaluates each argument left-to-right, resolves the function by name via the host, then invokes the function with the evaluated arguments and the current context.
3. A `Handle` node evaluates its body with handler clauses active. When an operation signals, the VM searches handler clauses innermost-first for a match.

The VM checks `Host::lazy_function` before `Host::function` and `Host::operation`. If a name resolves to a lazy function, the VM passes the raw expression tree instead of pre-evaluated values.

### 4.2 Host interface

The host provides function lookups by name. The VM has no built-in functions — all capabilities come from the host implementation.

| Trait           | Purpose                                                                            |
| --------------- | ---------------------------------------------------------------------------------- |
| `Host`          | Resolve function names to callables; the primary interface                         |
| `Function`      | Sync function — receives evaluated args, returns a value or error                  |
| `AsyncFunction` | Async function — returns a `Future` for non-blocking I/O                           |
| `LazyFunction`  | Lazy function — receives unevaluated args, decides which to evaluate               |
| `Operation`     | Effect operation — signals (yields) instead of completing; intercepted by handlers |

### 4.3 Default host functions

The default host provides these primitives:

| Function | Params                     | Returns  | Description                        |
| -------- | -------------------------- | -------- | ---------------------------------- |
| `add`    | `x: number, y: number`     | `number` | Arithmetic addition                |
| `mul`    | `x: number, y: number`     | `number` | Arithmetic multiplication          |
| `concat` | `x: string, y: string`     | `string` | String concatenation               |
| `len`    | `x: array\|string\|object` | `number` | Length of collection               |
| `apply`  | `fn: function, ...args`    | _any_    | Apply a first-class function value |

The `apply` function is the bridge between data and computation: it takes a function value (an object with a `body` field, params, and optional return schema) and remaining arguments, binds the arguments to the function's params, and evaluates the body. This is what makes functions first-class — `double(5)` desugars to `apply(double, 5)` when `double` resolves to a function value via path lookup.

### 4.4 Operations and handlers

Operations are the language's primary mechanism for **control flow as data**. An operation signals (yields) instead of returning. The VM captures the current continuation and searches handler clauses for a match.

The `Operation` trait defines:

- **`signature()`** — operation name, param schemas, return schema.
- **`perform()`** — called by the VM; returns `Err(Error::Operation { name, args })` to signal, or `Ok(value)` to complete without signaling.

The VM checks `Host::operation` before `Host::function`. If a name resolves to an operation, the operation's `perform` method is called instead of a function's `call` method.

When an operation signals and a matching handler clause is found:

1. Operation arguments are bound to the handler's params.
2. The captured continuation (frame stack) is bound to the handler's continuation variable.
3. The handler body is evaluated. The handler can resume the continuation (`k(42)`), discard it (return a value), or re-raise (perform another operation).

If no handler matches, the operation propagates to the caller as an unhandled operation yield.

### 4.5 Lazy functions

The `LazyFunction` trait enables short-circuit evaluation. A lazy function receives unevaluated argument expressions and an `evaluate` closure. It decides which arguments to evaluate and in what order.

This enables `if(cond, then, else)`, `and(a, b)`, and `or(a, b)` as host-provided functions without baking control flow into the kernel. The VM checks `Host::lazy_function` before `Host::function` and `Host::operation`.

### 4.6 Async evaluation

The `AsyncFunction` trait enables non-blocking I/O. Async functions return `Pin<Box<dyn Future<...>>>`, allowing network requests, timers, and cooperative yielding without blocking the calling thread.

The async VM (`eval_async`) checks `Host::async_function` after `Host::function` returns `None`. A host can provide both sync and async functions — the VM uses whichever is available for a given name.

An async function can yield control to the executor by awaiting a pending future, or yield a value to the caller by returning `Err(Error::Yield(value))` for explicit resumability.

### 4.7 Resumable evaluation

The VM provides `eval_resumable`, which uses an explicit frame stack instead of the native call stack. This enables cross-machine resumability.

**How it works:**

1. When a host function returns `Err(Error::Yield(value))`, the VM captures the current frame stack into a `Continuation`.
2. The caller receives `EvalResult::Yielded { value, continuation }`.
3. The continuation is serializable via `Continuation::to_json()` — all data is JSON-compatible.
4. To resume, call `eval_resumable` with `Continuation::from_json(json)` and a potentially different `Vm` with a different `Host`.
5. The resumed host can use a `CallCache` to return cached results for calls already made on the original host.

The `Continuation` contains:

- **Frame stack** — each `Frame` records the function name, evaluated arguments, pending expressions, and active handler clauses.
- All data is JSON-compatible: function names are strings, values are `serde_json::Value`, expressions are `Expr<NonFunctionValue>`, handler clauses are `HandlerClause<NonFunctionValue>`.

The `EvalResult` enum:

| Variant                           | Meaning                                     |
| --------------------------------- | ------------------------------------------- |
| `Done(value)`                     | Evaluation completed successfully           |
| `Yielded { value, continuation }` | Evaluation paused — a host function yielded |
| `Error(err)`                      | Evaluation failed                           |

### 4.8 Execution context

The context maintains a stack of scopes for variable bindings.

| Operation                  | Behavior                                                                           |
| -------------------------- | ---------------------------------------------------------------------------------- |
| **Create**                 | Initializes with one empty global scope.                                           |
| **Push scope / Pop scope** | Model function call boundaries. The global scope is never popped.                  |
| **Define**                 | Sets or overwrites a variable in the current scope.                                |
| **Resolve**                | Searches from the innermost scope outward. Returns `UnknownVariable` if not found. |
| **Depth**                  | Reports the current scope depth (global = 1).                                      |

### 4.9 VM errors

| Error             | Meaning                                                              |
| ----------------- | -------------------------------------------------------------------- |
| `UnknownFunction` | A function name was not found in the host.                           |
| `UnknownVariable` | A variable name was not found in any scope.                          |
| `PrimitiveError`  | A primitive operation failed with a message.                         |
| `TypeMismatch`    | A value did not match the expected type.                             |
| `ArgumentCount`   | The number of arguments did not match the expected count.            |
| `Yield`           | A host function yielded — captures a value for resumable evaluation. |
| `Operation`       | An operation signaled — carries the operation name and arguments.    |

Errors are comparable for testing and replay purposes.

## 5. Schema validation

### 5.1 Schema interface

A schema validates a value of type `V`, returning a validation error if the value does not conform. The interface is generic over `V` and independent of any specific schema language.

### 5.2 Validation error

A validation error carries:

- **message** — human-readable description of the failure.
- **path** — dotted path to the offending element within the value (empty for root).
- **location** — optional source location (line:column), populated by the compiler from the source map after validation.

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

The frontend factory provides a `frontend_for(format)` function that returns a frontend for the requested format name. Supported formats: `yaml`/`yml`, `json`, `toml`. This is interface-agnostic — usable from CLI, MCP, or any consumer that needs to parse/serialize handl documents.

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

- **schemas** — field name → raw schema fragment pairs (handl keys still present).
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

## 10. Module system

The `Module` trait groups host functions by namespace, enabling qualified calls like `fs.read("config.yml")`, `math.abs(x)`, `http.get("https://...")`. A `ModuleHost` composes multiple modules into a single `Host` implementation — the VM sees only `Host::function(name)`.

### 10.1 Module trait

A module provides function lookups by qualified name. The `ModuleHost` routes `name.subname` calls to the appropriate module. The VM is unaware of modules — it only calls `Host::function(name)`.

### 10.2 ModuleHost composition

```rust
let host = ModuleHost::new()
    .with(MathModule)
    .with(StrModule)
    .with(FsModule::new(StdFs))
    .with(EnvModule::new(StdEnv))
    .with(HttpClientModule::new(StdHttpClient));
```

Each module is independent. Adding or removing a module only affects which function names are available — no core changes needed.

## 11. Platform services

Abstract service traits bridge host capabilities (filesystem, environment, path manipulation, time, HTTP, command execution, temp files) into callable `Module` implementations.

### 11.1 Service traits

| Service     | Trait        | Functions                                     | Description           |
| ----------- | ------------ | --------------------------------------------- | --------------------- |
| Filesystem  | `Fs`         | `fs.read`, `fs.write`, `fs.exists`, `fs.list` | File I/O              |
| Environment | `Env`        | `env.get`, `env.set`, `env.keys`              | Environment variables |
| Path        | `Path`       | `path.join`, `path.dirname`, `path.basename`  | Path manipulation     |
| Time        | `Time`       | `time.now`, `time.format`                     | Time access           |
| HTTP        | `HttpClient` | `http.get`, `http.post`                       | HTTP client           |
| Command     | `Cmd`        | `cmd.run`                                     | Process execution     |
| Temp        | `Tmp`        | `tmp.file`, `tmp.dir`                         | Temporary files/dirs  |

### 11.2 Implementations

- **Stub implementations** — in-memory, no I/O. Ideal for tests and sandboxed hosts.
- **Stdlib implementations** (`handl-lib-platform-std`) — concrete implementations using Rust stdlib + common crates.

Each service trait is abstract — no platform dependencies in the interface crate. The stdlib crate provides concrete implementations. A `ModuleHost` composes services into a single host — plug in only what each environment needs.

## 12. Content-addressed storage

### 12.1 DefinitionStore

Stores `FunctionValue` definitions indexed by SHA-256 hash of their canonical JSON. Enables deduplication, reproducible builds, and cross-machine sharing.

### 12.2 NameRegistry

Maps human-readable names to content-addressed hashes. Resolves names to definitions at runtime.

### 12.3 EvalCache

Caches evaluation results keyed by expression + input context. Enables memoization and replay — if the same expression is evaluated with the same inputs, the cached result is returned without re-evaluation.

Content addressing is a storage layer, not a language-level constraint — documents evaluate correctly without ever touching a `DefinitionStore`.

## 13. Caching host

The `CacheHost` wraps another host with a `CallCache` — a serializable cache mapping `(function, args)` keys to result values.

### 13.1 Cross-machine resumability

When a VM evaluation yields, the caller captures a `Continuation` and the yielded value. To resume on another machine, the new host needs to return results for calls already made — without re-executing them.

```rust
// Server: register functions, evaluate, hit a yield
let mut host = CacheHost::new();
host.register("add", Box::new(AddFunction));
host.register("fetch", Box::new(FetchFunction)); // yields
let vm = Vm::new(host);
let result = vm.eval_resumable(&expr, &mut ctx, None);

// Ship continuation + cache to client
let cont_json = continuation.to_json();
let cache_json = vm.host().cache().to_json();

// Client: restore cache, resume with different host
let cache = CallCache::from_json(&received_cache_json);
let mut host = CacheHost::with_cache(cache);
host.register("add", Box::new(AddFunction));
host.register("fetch", Box::new(ClientFetchFunction));
let vm = Vm::new(host);
let result = vm.eval_resumable(&expr, &mut ctx, Some(cont));
```

The `CallCache` is serializable to JSON, enabling cross-machine migration of in-progress evaluation without re-executing completed calls.

## 14. Dialect system

The `Dialect` trait abstracts the language's syntax tokens, reserved key names, and schema vocabulary behind a swappable interface. The canonical dialect is the only shipped implementation, but custom dialects are architecturally possible.

### 14.1 Dialect trait

The `Dialect` trait is a facade grouping four sub-traits:

| Sub-trait            | Abstracts                                                   |
| -------------------- | ----------------------------------------------------------- |
| `Syntax`             | Interpolation markers, path separator, call delimiters      |
| `ExprParser`         | Expression parsing rules                                    |
| `DocumentVocabulary` | Reserved key names (`$`, `expr`, `body`, `functions`, etc.) |
| `SchemaVocabulary`   | Schema language vocabulary (JSON Schema by default)         |

### 14.2 Canonical dialect

`HandlDialect` is a zero-sized struct implementing the `Dialect` trait with the canonical handl language values:

- Interpolation: `${` ... `}`
- Path separator: `.`
- Call delimiters: `(` ... `)`
- Schema vocabulary: JSON Schema draft 2020-12

It is a `#[derive(Clone, Copy)]` static singleton — a value that can be easily passed around and used as the language definition in compiler configuration.

### 14.3 Compiler configuration

The compiler `Config` holds a `Box<dyn Dialect>`, meaning the specific language definition (dialect) is a configurable and swappable value at runtime. A custom dialect may use different key names, interpolation markers, or schema vocabulary without changing the compiler, VM, or fields pipeline.

## 15. Standard library

The runtime provides standard library modules that can be composed into a `ModuleHost`:

| Module    | Crate                | Functions                                                                                                                           |
| --------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Math      | `handl-lib-utils`    | `math.abs`, `math.floor`, `math.ceil`, `math.round`, `math.min`, `math.max`, etc.                                                   |
| String    | `handl-lib-utils`    | `str.upper`, `str.lower`, `str.trim`, `str.split`, `str.replace`, etc.                                                              |
| DateTime  | `handl-lib-datetime` | `datetime.now`, `datetime.format`, `datetime.parse`, etc.                                                                           |
| Crypto    | `handl-lib-crypto`   | `crypto.sha256`, `crypto.hmac`, etc.                                                                                                |
| Random    | `handl-lib-rand`     | `rand.int`, `rand.float`, `rand.pick`, etc.                                                                                         |
| Key-Value | `handl-lib-kv`       | `kv.get`, `kv.set`, `kv.delete` (interface + hashmap impl)                                                                          |
| Intl      | `handl-lib-intl`     | `intl.format_number`, `intl.compare_strings`, `intl.segment`, `intl.display_name`, `intl.format_duration`, `intl.supported_locales` |

Each module is independent — include only what the host environment needs.

## 16. Compiler pipeline

The compiler provides a composable pipeline for compile-time operations: parsing, field extraction, import resolution, type checking, and type generation. The pipeline is a sequence of stateless passes that run in order, each receiving the compilation state and configuration.

### 16.1 Compilation state

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

### 16.2 Configuration

The configuration holds the pluggable components:

- **resolver** — how modules are loaded (module resolver).
- **interpreter** — how schema fragments become types (schema interpreter).
- **backend** — how types are emitted (typegen backend, optional for typecheck-only runs).

The compiler provides default configurations:

- **Default** — file resolver (with memory cache) + JSON Schema interpreter + JSON Schema backend.
- **Typecheck-only** — file resolver (with memory cache) + JSON Schema interpreter + no backend.

### 16.3 Pass interface

Each pass implements:

- **`run(compilation, config)`** — execute the pass, mutating the compilation state.
- **`should_run(compilation, config)`** — return false to skip when preconditions are not met.

Custom passes (linting, optimization, JIT) implement the pass interface and slot into the pipeline without core changes.

### 16.4 Built-in passes

| Pass      | Input → Output                                  |
| --------- | ----------------------------------------------- |
| Parse     | source text → parsed value + source map         |
| Extract   | parsed value → extracted fields                 |
| Resolve   | extracted fields → typegen context              |
| Typecheck | extracted fields → type errors (parallelizable) |
| Generate  | typegen context → output                        |

### 16.5 Preset pipelines

| Preset      | Passes                                           | Use case                         |
| ----------- | ------------------------------------------------ | -------------------------------- |
| `typegen`   | parse → extract → resolve → generate             | Generate JSON Schema from fields |
| `typecheck` | parse → extract → typecheck                      | Static type checking             |
| `full`      | parse → extract → resolve → typecheck → generate | Type-safe code generation        |

### 16.6 Parallel type checking

The typecheck pass parallelizes independent checks. Six leaf checks run concurrently:

- **Schema-level**: `expr` key check, `arg` key check, `merge` key check
- **Data-level**: `inheritable` key check, `mutable` key check, import type check

The module resolver and module cache must be thread-safe to support this.

## 17. CLI

The `handl` command-line tool is a thin wrapper around the compiler. It provides argument parsing and delegates all logic to the compiler.

Commands: `parse`, `eval`, `validate`, `fields`, `typegen`, `typecheck`.

The CLI provides default host functions (`add`, `mul`, `concat`, `len`, `import`, `apply`) via the default host and VmResolver.

## 18. Facade

The top-level package is a batteries-included facade that re-exports all public components in a single dependency. It provides a prelude for convenient imports. This is the recommended entry point for most users.

## 19. Testing and benchmarking

### 19.1 Unit tests

Unit tests are co-located with implementation, keeping tests close to the code they test.

### 19.2 Integration tests

Integration tests cover the full pipeline (parse → extract → resolve → typecheck → generate) with real YAML/JSON/TOML inputs.

### 19.3 Fuzzing

The runtime includes fuzzing infrastructure with targets for expression parsing, interpolation, scoped interpolation, and expression type inference. Curated seed corpora cover edge cases: paths, unary operators, multi-arg calls, empty inputs, comma handling, unmatched parentheses.

### 19.4 Benchmarks

Benchmarks measure:

| Benchmark | What it measures                                              |
| --------- | ------------------------------------------------------------- |
| Core      | Schema interpretation and type compatibility (isolated)       |
| Inference | Expression type inference (parsing + inference)               |
| Typecheck | Per-key typecheck checks (expr, arg, merge, inherit, mutable) |
| Pipeline  | End-to-end pipeline (parse → extract → typecheck) at scale    |

Benchmarks cover flat documents, `expr:`-heavy documents, inheritable fields with nesting, and mixed-feature documents at varying sizes (10–1000 fields). Individual pass costs are also measured (parse-only, parse+extract).

## 20. Extension points

| Extension                  | How                                                         | Touches            |
| -------------------------- | ----------------------------------------------------------- | ------------------ |
| New format                 | Implement the frontend interface + add to the factory       | New component only |
| New schema language        | Implement the schema interface                              | New component only |
| New schema interpreter     | Implement the schema interpreter for type checking          | New component only |
| New typegen backend        | Implement the typegen backend for type generation           | New component only |
| New module resolver        | Implement the module resolver interface                     | New component only |
| New module cache           | Implement the module cache interface                        | New component only |
| New host function          | Implement the `Function` interface, register with host      | Host impl only     |
| New async function         | Implement the `AsyncFunction` interface, register with host | Host impl only     |
| New lazy function          | Implement the `LazyFunction` interface, register with host  | Host impl only     |
| New operation              | Implement the `Operation` trait, register with host         | Host impl only     |
| New module                 | Implement the `Module` trait, add to `ModuleHost`           | New component only |
| New platform service       | Implement the service trait, add as a `Module`              | New component only |
| New dialect                | Implement the `Dialect` trait, use in compiler `Config`     | New component only |
| Custom namespace           | Insert into the namespaces map                              | Caller code        |
| Custom function resolver   | Implement the function resolver interface                   | Caller code        |
| Custom pipeline pass       | Implement the pass interface, add to pipeline               | New component only |
| Native values              | Extend the value type with a native variant                 | Value type only    |
| Multi-source imports       | Compose resolvers with the chain resolver                   | Caller code        |
| Cross-machine resumability | Use `eval_resumable` + `CacheHost` + `CallCache`            | Caller code        |

## 21. Porting notes

When porting the runtime to another language:

1. **Start with the AST** — define expression trees and field bindings as generic types with no dependencies.
2. **Define the value type** — wrap the target language's JSON value type. Implement predicates, accessors, and conversions.
3. **Implement the frontend interface** — at minimum YAML and JSON. Use the target language's native YAML/JSON parsers.
4. **Implement the schema interface** — use a JSON Schema validator library for the target language.
5. **Implement the VM** — a tree walker with `Host`, `Function`, `AsyncFunction`, `LazyFunction`, and `Operation` traits. Support `eval`, `eval_async`, and `eval_resumable`.
6. **Implement continuations** — serializable frame stack for `eval_resumable`. Each frame records function name, evaluated args, pending expressions, and handler clauses. Serialize to JSON for cross-machine resumability.
7. **Implement field extraction** — single-pass extraction of schemas, functions, data, inheritable, immutable, computed, merge.
8. **Implement interpolation** — recursive `${...}` replacement with path lookup and function call resolution.
9. **Implement the expression parser** — recursive descent for function calls, path lookups, literals, and handle expressions. Handle string literals, nested parens, and comma-separated arguments.
10. **Implement module resolvers** — at minimum a file resolver. Use the module cache pattern for memoization.
11. **Implement the module system** — `Module` trait + `ModuleHost` for composing qualified function namespaces.
12. **Implement platform services** — abstract service traits for `Fs`, `Env`, `Path`, `Time`, `HttpClient`, `Cmd`, `Tmp` with stub and native implementations.
13. **Implement the dialect system** — `Dialect` trait abstracting syntax, vocabulary, and schema. Canonical dialect with `${...}` interpolation and JSON Schema.
14. **Implement the compiler pipeline** — pass interface + pipeline builder. Each pass reads/writes a compilation state.
15. **Source location tracking** — scan source text during parsing to build a path → location map. Resolve error locations after all passes.
16. **Parallel type checking** — if the target language supports parallelism, parallelize independent type checks. The module resolver and module cache must be thread-safe.
