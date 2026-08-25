# Dialect Removal & Architecture Refactoring Plan

## Goal

Eliminate `handl-dialect` and `handl-dialect-canonical` by distributing their
concerns into the existing toolkit crates and creating new abstractions
where genuinely needed.

## Current State

```
handl-dialect (4 traits)
├── Syntax              → interpolation markers, path sep, delimiters
├── ExprParser          → string-level expression parsing (DUPLICATES lexer/parser)
├── DocumentVocabulary  → reserved key names ($, expr, arg, merge, ...)
└── SchemaVocabulary    → schema compile/validate

handl-dialect-canonical (4 impls)
├── HandlDialect           → facade struct
├── ExprParser impl     → hand-rolled string scanner
├── Syntax impl         → returns "${", "}", '.', '(', ')'
├── DocumentVocabulary impl → returns "$", "expr", "arg", ...
└── SchemaVocabulary impl → JSON Schema draft 2020-12

30 crates depend on handl-dialect
```

## Target State

```
DELETED: handl-dialect, handl-dialect-canonical

ExprParser → DELETED
  Consumers switch to handl-lexer-canonical + handl-parser-canonical.

Syntax → handl-syntax (new, tiny crate)
  Const struct with interpolation markers, path separator, call delimiters.
  No trait — just data. Custom formats override the const.

DocumentVocabulary → handl-syntax (same crate)
  Const struct with reserved key names.
  No trait — just data.

SchemaVocabulary → handl-schema (new crate)
  Trait + JSON Schema impl (the one piece that genuinely needs polymorphism).

handl-document (new crate) — Document/Source abstraction
  Trait for loading documents from any source (file, HTTP, in-memory, custom).
  Unifies compiler::Source + module-resolver + frontend selection.
```

## Phases

### Phase 1: handl-syntax crate (absorb Syntax + DocumentVocabulary)

**New crate: `handl-syntax`**

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

1. Create `handl-syntax` crate with `SyntaxConfig`, `DocumentConfig`, and their `const` defaults
2. Update all consumers to use `CANONICAL_SYNTAX` / `CANONICAL_DOCUMENT` instead of `dialect.syntax()` / `dialect.document_vocabulary()`
3. Remove `Syntax` and `DocumentVocabulary` traits from `handl-dialect`

**Affected crates (read syntax/vocabulary from dialect):**

- `handl-compiler` (config.rs, pipeline.rs)
- `handl-compiler-pass-extract`
- `handl-compiler-pass-typecheck`
- `handl-compiler-pass-generate`
- `handl-compiler-pass-resolve`
- `handl-compiler-pass-parse`
- `handl-fields`
- `handl-typecheck-infer`
- `handl-typecheck-keys` modules
- `handl-typegen-core`
- `handl-typegen-key-set-special`
- `handl-typegen-import`
- `handl-typegen-backend-json-schema`
- `handl-meta`
- `handl-host-type`

### Phase 2: handl-schema crate (absorb SchemaVocabulary)

**New crate: `handl-schema`**

Move `SchemaVocabulary` trait, `Schema`, `ValidationError` from `handl-dialect`.
Move `JsonSchemaVocabulary`, `JsonSchema` from `handl-dialect-canonical`.

```rust
pub trait SchemaVocabulary<V>: Send + Sync {
    fn id(&self) -> &'static str;
    fn compile(&self, raw: &serde_json::Value) -> Result<Box<dyn Schema<V>>, ValidationError>;
}

pub struct JsonSchemaVocabulary;
impl SchemaVocabulary<NonFunctionValue> for JsonSchemaVocabulary { ... }
```

**Steps:**

1. Create `handl-schema` crate, move schema types there
2. Update `handl-typecheck-schema-json` to depend on `handl-schema` instead of `handl-dialect`
3. Update `handl-compiler` config to use `handl-schema` types
4. Remove `SchemaVocabulary` from `handl-dialect`

### Phase 3: Remove ExprParser (migrate to lexer/parser)

**Delete `ExprParser` trait and `ParsedHandlerClause` from `handl-dialect`.**

**Consumers to migrate:**

1. **`handl-fields/src/parser.rs`** — currently recursive descent using `ExprParser` methods
   - Replace with: `CanonicalLexer::default().lex(input)` → `CanonicalParser.parse(&tokens)`
   - Convert `Expr` (parser AST) → `Expr<NonFunctionValue>` (runtime AST)
   - Add `handl-lexer-canonical` + `handl-parser-canonical` deps to `handl-fields`

2. **`handl-typecheck-infer/src/parser.rs`** — currently recursive descent using `ExprParser`
   - Replace with: lex + parse → convert `Expr` → `ExprAst`
   - Add `handl-lexer-canonical` + `handl-parser-canonical` deps to `handl-typecheck-infer`

3. **`handl-meta/src/functions.rs`** — uses `ExprParser` for literal detection
   - Replace with: lex → check token kinds (simpler than full parse)

4. **`handl-host-type/src/eval/evaluator.rs`** — uses `ExprParser` for literal detection
   - Replace with: lex → check token kinds

**Steps:**

1. Write `Expr` → `Expr<NonFunctionValue>` conversion in `handl-fields`
2. Write `Expr` → `ExprAst` conversion in `handl-typecheck-infer`
3. Update `handl-meta` and `handl-host-type` to use lexer directly
4. Remove `ExprParser` from `handl-dialect`
5. Run all tests

### Phase 4: Delete handl-dialect + handl-dialect-canonical

**Steps:**

1. Remove `handl-dialect` and `handl-dialect-canonical` from workspace `Cargo.toml`
2. Remove both crate directories
3. Update all remaining `Cargo.toml` files that still reference `handl-dialect`
4. `cargo build` — fix any remaining references
5. `cargo test` — all tests pass

### Phase 5: handl-document crate (Document/Source abstraction)

**New crate: `handl-document`**

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

- `compiler::Source` → `handl-document::Document`
- `module-resolver-core::ModuleResolver` → `handl-document::DocumentLoader`
- `runtime::Frontend` → `handl-document::DocumentFormat` (relocated)

**Existing impls migrate:**

- `handl-module-resolver-file` → impl `DocumentLoader`
- `handl-module-resolver-http` → impl `DocumentLoader`
- `handl-module-resolver-git` → impl `DocumentLoader`
- `handl-module-resolver-virtual` → impl `DocumentLoader`
- `handl-frontend-json` → impl `DocumentFormat`
- `handl-frontend-yaml` → impl `DocumentFormat`
- `handl-frontend-toml` → impl `DocumentFormat`

**Steps:**

1. Create `handl-document` crate with `Document`, `DocumentSource`, `DocumentLoader`, `DocumentFormat`
2. Implement conversions from old types
3. Update `handl-compiler` to use `Document` instead of `Source`
4. Update module-resolver crates to impl `DocumentLoader`
5. Update frontend crates to impl `DocumentFormat`
6. Deprecate old traits (or remove if no external consumers)
7. Run all tests

## Dependency Graph (Target)

```
handl-lexer ← handl-lexer-canonical ← handl-parser-canonical
                                    ↓
handl-ast ← handl-parser ← handl-fields, handl-typecheck-infer, handl-meta, handl-host-type
                                    ↓
handl-syntax (const config)    handl-schema (trait + JSON impl)
        ↓                           ↓
    handl-compiler ← handl-compiler-pass-*
        ↓
handl-document (Document/Loader/Format)
        ↓
    handl-frontend-{json,yaml,toml}, handl-module-resolver-{file,http,git,virtual}
```

## Risk Assessment

| Phase                  | Risk                                      | Mitigation                                             |
| ---------------------- | ----------------------------------------- | ------------------------------------------------------ |
| 1 (handl-syntax)       | Low — const structs replacing trait calls | Mechanical replacement                                 |
| 2 (handl-schema)       | Low — trait moves unchanged               | Move + re-export                                       |
| 3 (ExprParser removal) | Medium — 4 consumers need AST conversion  | Write conversion fns, test each consumer               |
| 4 (delete dialect)     | Low — cleanup after 1-3                   | Build + test                                           |
| 5 (handl-document)     | Medium — unifies 3 abstractions           | Phase after dialect removal, can be done independently |

## Ordering

Phases 1-4 must be sequential (each depends on the previous).
Phase 5 can be done independently before or after, but is cleaner after
the dialect crates are gone.
