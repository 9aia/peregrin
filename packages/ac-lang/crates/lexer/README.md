# ac-lexer

Generic, composable lexing abstraction for AC expressions. Defines the `Lexer`
trait, `LexerConfig`, `LexRule`, `Token<K>`, `TokenKind`, the factory traits,
`Span`, and the `LexError` / `DefaultLexError` error types — everything needed
to turn raw text into a token stream.

The canonical implementation lives in [`ac-lexer-canonical`](../lexer-canonical).

## Architecture

> **The lexer produces tokens; the parser consumes them.**

The lexer is deliberately separate from the parser so that tooling (LSP,
formatters, linters) can depend on tokens without pulling in the full parser
and AST.

## Generic design

The lexer is generic over `K: TokenKind` and `E: LexError`. Each dialect defines
its own `TokenKind` enum, an associated factory, and optionally its own `LexError`
type. Dialects only implement the factories they need.

```rust
use ac_lexer::{DefaultLexError, EofFactory, Lexer, LexRule, LexerConfig, Token, TokenKind};

pub trait TokenKind: Clone + PartialEq + std::fmt::Debug + std::fmt::Display + Send + Sync + 'static {
    type Factory: EofFactory<Self>;
    fn is_eof(&self) -> bool;
    fn is_error(&self) -> bool;
}

pub trait LexError:
    Clone + std::fmt::Debug + std::fmt::Display + std::error::Error + Send + Sync + 'static
{
}

pub trait Lexer<K: TokenKind, E: LexError>: Send + Sync {
    fn lex(&self, input: &str) -> Result<Vec<Token<K>>, E>;
    fn lex_iter<'a>(&'a self, input: &'a str)
        -> Box<dyn Iterator<Item = Result<Token<K>, E>> + 'a>;
}
```

`TokenKind::Factory` only has to implement `EofFactory`. Rule-specific factories
(`StringFactory`, `NumberFactory`, `IdentFactory`, `KeywordFactory`,
`OperatorFactory`, `CommentFactory`) are required by the rules that use them,
so a dialect can omit categories it does not support.

`KindFactory` is a convenience trait that rolls the common factories into one.
Blanket impls ensure that implementing `KindFactory` gives you every sub-factory.

## Token factories

Each factory is both a constructor and, where relevant, a primitive lexer. The
rule dispatches to the factory; the factory owns the dialect-specific syntax.

```rust
pub trait StringFactory<K: TokenKind>: Clone + Send + Sync + 'static {
    fn string_kind(&self, value: String) -> K;
    fn is_string_start(&self, ch: char) -> bool;
    fn lex_string(&self, chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>)
        -> Result<(String, Span), DefaultLexError>;
}
```

Default implementations use the canonical syntax (`"..."`, `'...'` for strings,
`0-9` and `-` for numbers, `//` and `/* ... */` for comments, etc.). Dialects
override only the methods that differ.

## Lexing rules

Rules implement the `LexRule<K, E>` trait. Rules are tried in order by
`LexerConfig`; the first that returns `Some` wins.

```rust
pub trait LexRule<K: TokenKind, E: LexError>: Send + Sync + 'static {
    fn try_lex(
        &self,
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<K, E>,
    ) -> Option<Result<Token<K>, E>>;
    fn name(&self) -> &'static str;
    fn clone_box(&self) -> Box<dyn LexRule<K, E>>;
}
```

### Built-in rule structs

| Struct             | Matches                                          | Name            | Factory needed                  |
| ------------------ | ------------------------------------------------ | --------------- | ------------------------------- |
| `StringRule`       | String literals (factory-decides quote chars)    | `string`        | `StringFactory`                 |
| `NumberRule`       | Numbers including negatives                      | `number`        | `NumberFactory`                 |
| `IdentRule`        | Identifiers and keywords                         | `ident`         | `IdentFactory`                  |
| `KeywordRule`      | Keywords before `IdentRule` runs                 | `keyword`       | `KeywordFactory + IdentFactory` |
| `DelimiterRule`    | Single-character delimiters from config          | `delimiter`     | `TokenKind`                     |
| `LineCommentRule`  | Line comments (factory-decides delimiter)        | `line_comment`  | `CommentFactory`                |
| `BlockCommentRule` | Block comments (factory-decides delimiters)      | `block_comment` | `CommentFactory`                |
| `OperatorRule`     | Operators (factory-decides chars and multi-char) | `operator`      | `OperatorFactory`               |

### Custom rules

For simple cases, wrap a function pointer with `FnRule`:

```rust
use ac_lexer::{DefaultLexError, FnRule, LexerConfig};

fn hash_rule(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    _config: &LexerConfig<MyKind, DefaultLexError>,
) -> Option<Result<Token<MyKind>, DefaultLexError>> {
    // ...
    None
}

let config = config.with_rule(FnRule::new("hash", hash_rule));
```

For stateful or composable rules, implement `LexRule<K, E>` directly on a struct.

## LexerConfig

`LexerConfig<K, E>` holds keywords, delimiters, and an ordered list of
`Box<dyn LexRule<K, E>>`. Builder methods:

- `with_keyword(text, kind)` / `without_keyword(text)`
- `with_delimiter(ch, kind)` / `without_delimiter(ch)`
- `with_rule(rule)` / `insert_rule(idx, rule)` / `without_rule(&rule)` / `without_rule_by_name(name)`
- `merge(other)` — combines keywords, delimiters, and rules (deduplicates by name)

## Usage

```rust
use ac_lexer::Lexer;
use ac_lexer_canonical::{CanonicalLexer, CanonicalKind};

let tokens = CanonicalLexer::default().lex("add(1, 2)")?;
assert_eq!(tokens[0].kind, CanonicalKind::Identifier("add".into()));
```

### Custom dialect

```rust
use ac_lexer_canonical::{CanonicalLexer, CanonicalKind, canonical_config};
use ac_lexer::DefaultLexError;

let config = canonical_config()
    .with_keyword("await", CanonicalKind::Handle)
    .with_delimiter(';', CanonicalKind::Comma)
    .without_rule_by_name("string");

let lexer = CanonicalLexer::with_config(config);
let tokens = lexer.lex("await(foo; 1)")?;
```

### Composition

```rust
use ac_lexer_canonical::{CanonicalKind, canonical_config};

let base = canonical_config();
let extension = canonical_config()
    .with_keyword("await", CanonicalKind::Handle);

let merged = base.merge(extension);
```

## Streaming

`lex_iter` yields tokens one at a time. The canonical implementation is truly
streaming — it processes char-by-char and returns each token as soon as it's
complete:

```rust
use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;

let lexer = CanonicalLexer::default();
for result in lexer.lex_iter("add(1, 2)") {
    let token = result?;
    println!("{} at {}", token.kind, token.span);
}
```

The iterator always emits `Eof` as the final token, then returns `None`. On
unrecoverable input it yields `Err(E)` and stops.

## Error handling

`LexError` is a trait. Dialects can use the built-in `DefaultLexError` or define
their own `E: LexError`. `DefaultLexError` has three small, `Clone`-able variants:

- `UnterminatedString { span }`
- `InvalidNumber { span }`
- `UnexpectedChar { span, char }`

All are unrecoverable: the lexer returns `Err(E)` and stops.
