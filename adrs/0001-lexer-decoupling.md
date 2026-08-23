# ADR 0001: Generic, Composable Lexer Architecture

## Status

Accepted

## Context

The `ac-lexer` crate was created to tokenize AC (Astral Chronicle) expression
syntax. Over time it became clear that the lexer would need to support multiple
dialects, parser-adjacent tooling (formatters, LSPs), and possibly syntax
variations (different quotes, comment delimiters, number formats, identifier
rules). The original design coupled every token category to a single
`TokenKind`/`KindFactory` pair and a single closed `LexError` enum, which made
these variations expensive to add.

Goals for the refactor:

1. Allow dialects to define their own `TokenKind` and factory types.
2. Keep the core crate from forcing every dialect to implement every token
   category (e.g. a JSON-like dialect should not need comment support).
3. Make primitive scanning (strings, numbers, identifiers, comments, operators)
   configurable without writing an entirely new rule.
4. Allow dialects to use their own error type.
5. Preserve the ability to compose, merge, and extend lexer configurations at
   runtime.

## Decision

We split the lexer into three layers:

1. **Token kind layer (`TokenKind`, `CommentTokenKind`)**

   Dialects define their own enum implementing `TokenKind`. `TokenKind::Factory`
   is only required to implement `EofFactory`. Other factory traits are opt-in.

2. **Factory layer (`EofFactory`, `StringFactory`, `NumberFactory`,
   `IdentFactory`, `KeywordFactory`, `OperatorFactory`, `CommentFactory`)**

   Each factory is a small trait with the canonical default behavior. Rules
   require only the factories they actually use, so `TokenKind::Factory` can be
   a partial implementation. `KindFactory` remains as a convenience marker that
   rolls the common factories together; blanket `impl`s for each sub-factory
   delegate to `KindFactory`, so full-dialect factories can still implement one
   trait.

3. **Rule layer (`LexRule<K, E>`, `LexerConfig<K, E>`, `Lexer<K, E>`)**

   `LexRule`, `LexerConfig`, and `Lexer` are generic over `E: LexError`.
   `DefaultLexError` is the built-in concrete error type, but dialects can
   define their own `E` if they need additional error cases or different error
   shaping.

### Why `LexError` as a trait?

A closed `LexError` enum forced all dialects to either shoehorn their errors
into our variants or perform lossy conversion. By making `LexError` a blanket
trait (`Clone + Debug + Display + Error + Send + Sync + 'static`) and the
lexer/rules generic over `E`, dialects keep full control of their error types.
Rules that rely on the default primitives only require `E: From<DefaultLexError>`,
so built-in rules still work with custom errors.

### Why primitive methods in the factory?

Initially `StringRule`, `NumberRule`, `IdentRule`, etc. called free functions in
`ac_lexer::primitives`. That made the scanning syntax hard-coded. By moving
`lex_string`, `lex_number`, `lex_ident`, `is_ident_start`, `is_string_start`,
etc. into the factories with default implementations, the canonical rules remain
usable while any factory method can be overridden. The rules became thin
dispatchers; the factories own the dialect-specific syntax.

### Why `CommentFactory` and `OperatorFactory` primitives?

Comments and operators had the same hard-coding problem as strings/numbers. We
added `CommentFactory::is_line_comment_start` / `lex_line_comment` /
`is_block_comment_start` / `lex_block_comment` and `OperatorFactory::operator_chars`
/ `lex_operator` so dialects can customize delimiters and operator sets without
replacing the rules.

### `DefaultLexError` shape

The default error type intentionally has three small, `Clone`-able variants:

- `UnterminatedString { span }`
- `InvalidNumber { span }`
- `UnexpectedChar { span, char }`

We deliberately removed per-error `String` messages (e.g. a formatted parse
message) to avoid allocation on the error path. The `Display` implementation
produces a short, static message.

## Consequences

### Positive

- Dialects can now change string quotes, number formats, comment delimiters,
  operator characters, and identifier rules by overriding factory methods.
- Dialects can omit token categories they do not need (e.g. no `CommentFactory`
  for a data-only language).
- Dialects can define their own `LexError` types without forking the lexer.
- `LexerConfig<K, E>` still supports `merge`, `with_rule`, `without_rule_by_name`,
  and runtime composition.
- The rule list stays `Vec<Box<dyn LexRule<K, E>>>`, keeping object-safe,
  runtime-polymorphic composition.

### Trade-offs

- `Vec<Box<dyn LexRule<K, E>>>` means every token position dispatches through
  several trait objects. This is fine for parser-adjacent use, but a
  hand-optimized, table-driven lexer would be faster for pure tokenization
  throughput.
- `NumberFactory::lex_number` builds a `String` before calling `f64::parse`. To
  avoid this we would need to either change the rule to know the input `&str`
  slice or introduce a no-alloc float parser. This was left in scope for a later
  performance pass.
- Adding `K` and `E` type parameters to `Lexer`, `LexRule`, and `LexerConfig`
  makes signatures more verbose. Using `DefaultLexError` as a default in most
  places keeps the canonical dialect ergonomic.

## References

- `crates/lexer/src/token.rs`
- `crates/lexer/src/error.rs`
- `crates/lexer/src/rule/mod.rs`
- `crates/lexer/src/lexer.rs`
- `crates/lexer/src/primitives.rs`
- `crates/lexer-common/src/rules/`
