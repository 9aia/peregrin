# Dialect Removal & Architecture Refactoring Plan

## Goal

Eliminate `ac-dialect` and `ac-dialect-canonical` by distributing their
concerns into the existing toolkit crates and creating new abstractions
where genuinely needed.

## Current State

```
ac-dialect (4 traits)
├── Syntax              → interpolation markers, path sep, delimiters
├── ExprParser          → string-level expression parsing (DUPLICATES lexer/parser)
├── DocumentVocabulary  → reserved key names ($, expr, arg, merge, ...)
└── SchemaVocabulary    → schema compile/validate

ac-dialect-canonical (4 impls)
├── AcDialect           → facade struct
├── ExprParser impl     → hand-rolled string scanner
├── Syntax impl         → returns "${", "}", '.', '(', ')'
├── DocumentVocabulary impl → returns "$", "expr", "arg", ...
└── SchemaVocabulary impl → JSON Schema draft 2020-12

30 crates depend on ac-dialect
```

## Target State

```
DELETED: ac-dialect, ac-dialect-canonical

ExprParser → DELETED
  Consumers switch to ac-lexer-canonical + ac-parser-canonical.

Syntax → ac-syntax (new, tiny crate)
  Const struct with interpolation markers, path separator, call delimiters.
  No trait — just data. Custom formats override the const.

DocumentVocabulary → ac-syntax (same crate)
  Const struct with reserved key names.
  No trait — just data.

SchemaVocabulary → ac-schema (new crate)
  Trait + JSON Schema impl (the one piece that genuinely needs polymorphism).

ac-document (new crate) — Document/Source abstraction
  Trait for loading documents from any source (file, HTTP, in-memory, custom).
  Unifies compiler::Source + module-resolver + frontend selection.
```

## Phases

### Phase 1: ac-syntax crate (absorb Syntax + DocumentVocabulary)

**New crate: `ac-syntax`**

```rust
// Interpolation and path syntax configuration
pub struct SyntaxConfig {
    pub interpolation_prefix: &'static str,  // "${"
    pub interpolation_suffix: &'static str,  // "}"
    pub path_separator: char,                 // '.'
    pub call_open: char,                      // '('
    pub call_close: char,                     // ')'
}

pub const CANONICAL_SYNTAX: SyntaxConfig = SyntaxConfig { ... };

// Reserved document key names
pub struct DocumentConfig {
    pub fields_key: &'static str,       // "$"
    pub expr_key: &'static str,         // "expr"
    pub arg_key: &'static str,          // "arg"
    pub merge_key: &'static str,        // "merge"
    pub inheritable_key: &'static str,  // "inheritable"
    pub mutable_key: &'static str,      // "mutable"
    pub params_key: &'static str,       // "params"
    pub return_key: &'static str,       // "return"
    pub body_key: &'static str,         // "body"
}

pub const CANONICAL_DOCUMENT: DocumentConfig = DocumentConfig { ... };
```

**Steps:**

1. Create `ac-syntax` crate with `SyntaxConfig`, `DocumentConfig`, and their `const` defaults
2. Update all consumers to use `CANONICAL_SYNTAX` / `CANONICAL_DOCUMENT` instead of `dialect.syntax()` / `dialect.document_vocabulary()`
3. Remove `Syntax` and `DocumentVocabulary` traits from `ac-dialect`

**Affected crates (read syntax/vocabulary from dialect):**

- `ac-compiler` (config.rs, pipeline.rs)
- `ac-compiler-pass-extract`
- `ac-compiler-pass-typecheck`
- `ac-compiler-pass-generate`
- `ac-compiler-pass-resolve`
- `ac-compiler-pass-parse`
- `ac-fields`
- `ac-typecheck-infer`
- `ac-typecheck-key-*`
- `ac-typegen-core`
- `ac-typegen-key-set-special`
- `ac-typegen-import`
- `ac-typegen-backend-json-schema`
- `ac-meta`
- `ac-host-type`

### Phase 2: ac-schema crate (absorb SchemaVocabulary)

**New crate: `ac-schema`**

Move `SchemaVocabulary` trait, `Schema`, `ValidationError` from `ac-dialect`.
Move `JsonSchemaVocabulary`, `JsonSchema` from `ac-dialect-canonical`.

```rust
pub trait SchemaVocabulary<V>: Send + Sync {
    fn id(&self) -> &'static str;
    fn compile(&self, raw: &serde_json::Value) -> Result<Box<dyn Schema<V>>, ValidationError>;
}

pub struct JsonSchemaVocabulary;
impl SchemaVocabulary<NonFunctionValue> for JsonSchemaVocabulary { ... }
```

**Steps:**

1. Create `ac-schema` crate, move schema types there
2. Update `ac-typecheck-schema-json` to depend on `ac-schema` instead of `ac-dialect`
3. Update `ac-compiler` config to use `ac-schema` types
4. Remove `SchemaVocabulary` from `ac-dialect`

### Phase 3: Remove ExprParser (migrate to lexer/parser)

**Delete `ExprParser` trait and `ParsedHandlerClause` from `ac-dialect`.**

**Consumers to migrate:**

1. **`ac-fields/src/parser.rs`** — currently recursive descent using `ExprParser` methods
   - Replace with: `CanonicalLexer::default().lex(input)` → `CanonicalParser.parse(&tokens)`
   - Convert `Expr` (parser AST) → `Expr<NonFunctionValue>` (runtime AST)
   - Add `ac-lexer-canonical` + `ac-parser-canonical` deps to `ac-fields`

2. **`ac-typecheck-infer/src/parser.rs`** — currently recursive descent using `ExprParser`
   - Replace with: lex + parse → convert `Expr` → `ExprAst`
   - Add `ac-lexer-canonical` + `ac-parser-canonical` deps to `ac-typecheck-infer`

3. **`ac-meta/src/functions.rs`** — uses `ExprParser` for literal detection
   - Replace with: lex → check token kinds (simpler than full parse)

4. **`ac-host-type/src/eval/evaluator.rs`** — uses `ExprParser` for literal detection
   - Replace with: lex → check token kinds

**Steps:**

1. Write `Expr` → `Expr<NonFunctionValue>` conversion in `ac-fields`
2. Write `Expr` → `ExprAst` conversion in `ac-typecheck-infer`
3. Update `ac-meta` and `ac-host-type` to use lexer directly
4. Remove `ExprParser` from `ac-dialect`
5. Run all tests

### Phase 4: Delete ac-dialect + ac-dialect-canonical

**Steps:**

1. Remove `ac-dialect` and `ac-dialect-canonical` from workspace `Cargo.toml`
2. Remove both crate directories
3. Update all remaining `Cargo.toml` files that still reference `ac-dialect`
4. `cargo build` — fix any remaining references
5. `cargo test` — all tests pass

### Phase 5: ac-document crate (Document/Source abstraction)

**New crate: `ac-document`**

Unifies the three separate concepts that currently exist:

- `compiler::Source` (raw text + format + path)
- `module-resolver-core::ModuleResolver` (load from file/http/git/memory)
- `frontend::Frontend` (parse text → value, serialize value → text)

```rust
/// A loaded document — raw text + metadata about where it came from.
pub struct Document {
    pub content: String,
    pub format: DocumentFormat,   // Yaml, Json, Toml, Custom(String)
    pub source: DocumentSource,   // where it came from
}

/// Where a document was loaded from.
#[derive(Clone)]
pub enum DocumentSource {
    File(PathBuf),
    Url(String),
    Memory(String),  // identifier for in-memory docs
    Custom(String),
}

/// A loader — abstracts how documents are fetched.
/// Implementations: file, HTTP, git, in-memory, custom.
pub trait DocumentLoader: Send + Sync {
    fn load(&self, path: &str, base: Option<&Path>) -> Option<Document>;
}

/// A document format — parse text to value, serialize value to text.
/// (This is the existing Frontend trait, relocated.)
pub trait DocumentFormat {
    type Error;
    fn parse(&self, input: &str) -> Result<NonFunctionValue, Self::Error>;
    fn serialize(&self, value: &NonFunctionValue) -> Result<String, Self::Error>;
    fn parse_with_locations(&self, input: &str) -> Result<(NonFunctionValue, SourceMap), Self::Error>;
}
```

**This replaces:**

- `compiler::Source` → `ac-document::Document`
- `module-resolver-core::ModuleResolver` → `ac-document::DocumentLoader`
- `runtime::Frontend` → `ac-document::DocumentFormat` (relocated)

**Existing impls migrate:**

- `ac-module-resolver-file` → impl `DocumentLoader`
- `ac-module-resolver-http` → impl `DocumentLoader`
- `ac-module-resolver-git` → impl `DocumentLoader`
- `ac-module-resolver-virtual` → impl `DocumentLoader`
- `ac-frontend-json` → impl `DocumentFormat`
- `ac-frontend-yaml` → impl `DocumentFormat`
- `ac-frontend-toml` → impl `DocumentFormat`

**Steps:**

1. Create `ac-document` crate with `Document`, `DocumentSource`, `DocumentLoader`, `DocumentFormat`
2. Implement conversions from old types
3. Update `ac-compiler` to use `Document` instead of `Source`
4. Update module-resolver crates to impl `DocumentLoader`
5. Update frontend crates to impl `DocumentFormat`
6. Deprecate old traits (or remove if no external consumers)
7. Run all tests

## Dependency Graph (Target)

```
ac-lexer ← ac-lexer-canonical ← ac-parser-canonical
                                    ↓
ac-ast ← ac-parser ← ac-fields, ac-typecheck-infer, ac-meta, ac-host-type
                                    ↓
ac-syntax (const config)    ac-schema (trait + JSON impl)
        ↓                           ↓
    ac-compiler ← ac-compiler-pass-*
        ↓
ac-document (Document/Loader/Format)
        ↓
    ac-frontend-{json,yaml,toml}, ac-module-resolver-{file,http,git,virtual}
```

## Risk Assessment

| Phase                  | Risk                                      | Mitigation                                             |
| ---------------------- | ----------------------------------------- | ------------------------------------------------------ |
| 1 (ac-syntax)          | Low — const structs replacing trait calls | Mechanical replacement                                 |
| 2 (ac-schema)          | Low — trait moves unchanged               | Move + re-export                                       |
| 3 (ExprParser removal) | Medium — 4 consumers need AST conversion  | Write conversion fns, test each consumer               |
| 4 (delete dialect)     | Low — cleanup after 1-3                   | Build + test                                           |
| 5 (ac-document)        | Medium — unifies 3 abstractions           | Phase after dialect removal, can be done independently |

## Ordering

Phases 1-4 must be sequential (each depends on the previous).
Phase 5 can be done independently before or after, but is cleaner after
the dialect crates are gone.
