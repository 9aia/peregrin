# handl-lexer-canonical

Canonical lexer for handl expressions. Implements `Lexer<CanonicalKind>` using the canonical handl dialect syntax.

Built on [`handl-lexer`](../lexer) — defines `CanonicalKind`, `CanonicalFactory`, and `canonical_config()`.

## Syntax

| Category    | Examples                                                |
| ----------- | ------------------------------------------------------- |
| Strings     | `"hello"`, `'world'` (with `\n \t \r \\ \" \'` escapes) |
| Numbers     | `42`, `1.5`, `-3`                                       |
| Booleans    | `true`, `false`                                         |
| Null        | `null`                                                  |
| Keywords    | `handle`, `as`                                          |
| Delimiters  | `(` `)` `{` `}` `[` `]` `,` `:`                         |
| Comments    | `// line`, `/* block */` (filtered from token stream)   |
| Identifiers | `add`, `math.sqrt` (dots allowed for namespace paths)   |

## Usage

```rust
use handl_lexer::Lexer;
use handl_lexer_canonical::{CanonicalLexer, CanonicalKind};

let tokens = CanonicalLexer::default().lex("add(1, 2)")?;
assert_eq!(tokens[0].kind, CanonicalKind::Identifier("add".into()));
```

## Streaming

```rust
use handl_lexer::Lexer;
use handl_lexer_canonical::CanonicalLexer;

for result in CanonicalLexer::default().lex_iter("add(1, 2)") {
    let token = result?;
    println!("{} at {}", token.kind, token.span);
}
```

## Customization

Start from `canonical_config()` and modify keywords, delimiters, and rules:

```rust
use handl_lexer_canonical::{CanonicalLexer, CanonicalKind, canonical_config};

let config = canonical_config()
    .with_keyword("await", CanonicalKind::Handle)
    .with_delimiter(';', CanonicalKind::Comma)
    .without_rule_by_name("string");

let lexer = CanonicalLexer::with_config(config);
```

## CanonicalKind variants

`Identifier(String)`, `String(String)`, `Number(f64)`, `Bool(bool)`, `Null`, `Handle`, `As`, `CallOpen`, `CallClose`, `BlockOpen`, `BlockClose`, `BracketOpen`, `BracketClose`, `Comma`, `Colon`, `Comment(String)`, `Eof`.
