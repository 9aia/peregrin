use crate::{CanonicalKind, CanonicalLexer, canonical_config};
use handl_lexer::{Lexer, Span, Token};

type CanonToken = Token<CanonicalKind>;

fn lex(input: &str) -> Vec<CanonicalKind> {
    CanonicalLexer::default()
        .lex(input)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

fn lex_tokens(input: &str) -> Vec<CanonToken> {
    CanonicalLexer::default().lex(input).unwrap()
}

#[test]
fn lex_string_double_quoted() {
    let tokens = lex(r#""hello""#);
    assert_eq!(
        tokens,
        vec![CanonicalKind::String("hello".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_string_single_quoted() {
    let tokens = lex("'world'");
    assert_eq!(
        tokens,
        vec![CanonicalKind::String("world".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_string_with_escapes() {
    let tokens = lex(r#""a\nb""#);
    assert_eq!(
        tokens,
        vec![CanonicalKind::String("a\nb".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_number_integer() {
    let tokens = lex("42");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_number_decimal() {
    let tokens = lex("1.5");
    assert_eq!(tokens, vec![CanonicalKind::Number(1.5), CanonicalKind::Eof]);
}

#[test]
fn lex_number_negative() {
    let tokens = lex("-3");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(-3.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_bool_true() {
    let tokens = lex("true");
    assert_eq!(tokens, vec![CanonicalKind::Bool(true), CanonicalKind::Eof]);
}

#[test]
fn lex_bool_false() {
    let tokens = lex("false");
    assert_eq!(tokens, vec![CanonicalKind::Bool(false), CanonicalKind::Eof]);
}

#[test]
fn lex_null() {
    let tokens = lex("null");
    assert_eq!(tokens, vec![CanonicalKind::Null, CanonicalKind::Eof]);
}

#[test]
fn lex_identifier() {
    let tokens = lex("add");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Identifier("add".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_namespaced_identifier() {
    let tokens = lex("math.sqrt");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("math.sqrt".into()),
            CanonicalKind::Eof
        ]
    );
}

#[test]
fn lex_keywords() {
    let tokens = lex("handle as");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Handle, CanonicalKind::As, CanonicalKind::Eof]
    );
}

#[test]
fn lex_call_expression() {
    let tokens = lex("add(1, 2)");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("add".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_handle_expression() {
    let tokens = lex(r#"handle("fetch"(url) as k: k(42))"#);
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Handle,
            CanonicalKind::CallOpen,
            CanonicalKind::String("fetch".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Identifier("url".into()),
            CanonicalKind::CallClose,
            CanonicalKind::As,
            CanonicalKind::Identifier("k".into()),
            CanonicalKind::Colon,
            CanonicalKind::Identifier("k".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Number(42.0),
            CanonicalKind::CallClose,
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_unterminated_string_errors() {
    let result = CanonicalLexer::default().lex(r#""unterminated"#);
    assert!(result.is_err());
}

#[test]
fn lex_empty_input() {
    let tokens = lex("");
    assert_eq!(tokens, vec![CanonicalKind::Eof]);
}

#[test]
fn lex_whitespace_only() {
    let tokens = lex("   \n\t  ");
    assert_eq!(tokens, vec![CanonicalKind::Eof]);
}

// --- Edge cases: string escapes ---

#[test]
fn lex_string_unterminated_after_escape() {
    let result = CanonicalLexer::default().lex(r#""hello\"#);
    assert!(result.is_err());
}

#[test]
fn lex_string_unknown_escape_passes_through() {
    let tokens = lex(r#""a\xb""#);
    assert_eq!(
        tokens,
        vec![CanonicalKind::String("axb".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_string_all_escapes() {
    let tokens = lex(r#""\n\t\r\\\"\'""#);
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::String("\n\t\r\\\"\'".into()),
            CanonicalKind::Eof
        ]
    );
}

#[test]
fn lex_string_empty() {
    let tokens = lex(r#""""#);
    assert_eq!(
        tokens,
        vec![CanonicalKind::String("".into()), CanonicalKind::Eof]
    );
}

// --- Edge cases: numbers ---

#[test]
fn lex_number_trailing_dot() {
    // "1." parses as f64 1.0, then "foo" is a separate identifier.
    let tokens = lex("1.foo");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Number(1.0),
            CanonicalKind::Identifier("foo".into()),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_number_multiple_dots_errors() {
    // The lexer greedily consumes digits and dots, so "1.2.3" is lexed
    // as a single number token — which then fails to parse as f64.
    let result = CanonicalLexer::default().lex("1.2.3");
    assert!(result.is_err());
}

#[test]
fn lex_dash_at_eof() {
    let result = CanonicalLexer::default().lex("-");
    assert!(result.is_err());
}

#[test]
fn lex_dash_not_followed_by_digit() {
    let result = CanonicalLexer::default().lex("- abc");
    assert!(result.is_err());
}

#[test]
fn lex_number_zero() {
    let tokens = lex("0");
    assert_eq!(tokens, vec![CanonicalKind::Number(0.0), CanonicalKind::Eof]);
}

// --- Edge cases: identifiers ---

#[test]
fn lex_unicode_identifier() {
    let tokens = lex("café");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Identifier("café".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_identifier_starting_with_dot() {
    let tokens = lex(".foo");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Identifier(".foo".into()), CanonicalKind::Eof]
    );
}

#[test]
fn lex_identifier_with_underscore() {
    let tokens = lex("my_var");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("my_var".into()),
            CanonicalKind::Eof
        ]
    );
}

// --- Edge cases: span correctness ---

#[test]
fn span_of_string_literal() {
    let tokens = lex_tokens(r#""hello""#);
    assert_eq!(tokens[0].span, Span::new(0, 7));
}

#[test]
fn span_of_number_literal() {
    let tokens = lex_tokens("42");
    assert_eq!(tokens[0].span, Span::new(0, 2));
}

#[test]
fn span_of_identifier() {
    let tokens = lex_tokens("add");
    assert_eq!(tokens[0].span, Span::new(0, 3));
}

#[test]
fn span_of_call_expression() {
    let tokens = lex_tokens("add(1, 2)");
    // add: 0..3, (: 3..4, 1: 4..5, ,: 5..6, space, 2: 7..8, ): 8..9
    assert_eq!(tokens[0].span, Span::new(0, 3));
    assert_eq!(tokens[1].span, Span::at(3)); // CallOpen — zero-length span
    assert_eq!(tokens[2].span, Span::new(4, 5));
    assert_eq!(tokens[3].span, Span::at(5)); // Comma
    assert_eq!(tokens[4].span, Span::new(7, 8));
    assert_eq!(tokens[5].span, Span::at(8)); // CallClose
}

#[test]
fn span_of_eof() {
    let tokens = lex_tokens("42");
    assert_eq!(tokens[1].span, Span::at(2));
}

// --- Edge cases: delimiters ---

#[test]
fn lex_block_delimiters() {
    let tokens = lex("{}");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BlockOpen,
            CanonicalKind::BlockClose,
            CanonicalKind::Eof
        ]
    );
}

#[test]
fn lex_colon() {
    let tokens = lex(":");
    assert_eq!(tokens, vec![CanonicalKind::Colon, CanonicalKind::Eof]);
}

#[test]
fn lex_bracket_delimiters() {
    let tokens = lex("[]");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BracketOpen,
            CanonicalKind::BracketClose,
            CanonicalKind::Eof
        ]
    );
}

#[test]
fn lex_array_literal() {
    let tokens = lex("[1, 2, 3]");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BracketOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(3.0),
            CanonicalKind::BracketClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_object_literal() {
    let tokens = lex("{a: 1, b: 2}");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BlockOpen,
            CanonicalKind::Identifier("a".into()),
            CanonicalKind::Colon,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Identifier("b".into()),
            CanonicalKind::Colon,
            CanonicalKind::Number(2.0),
            CanonicalKind::BlockClose,
            CanonicalKind::Eof,
        ]
    );
}

// --- Edge cases: unexpected characters ---

#[test]
fn lex_unexpected_char_errors() {
    let result = CanonicalLexer::default().lex("@#$");
    assert!(result.is_err());
}

#[test]
fn lex_unexpected_char_after_valid_token() {
    let result = CanonicalLexer::default().lex("add @");
    assert!(result.is_err());
}

// --- Config: custom keywords ---

#[test]
fn config_custom_keyword() {
    let lexer = CanonicalLexer::with_config(
        canonical_config().with_keyword("await", CanonicalKind::Handle),
    );
    let tokens = lexer.lex("await").unwrap();
    assert_eq!(tokens[0].kind, CanonicalKind::Handle);
}

#[test]
fn config_replace_keyword() {
    let lexer =
        CanonicalLexer::with_config(canonical_config().with_keyword("true", CanonicalKind::Null));
    let tokens = lexer.lex("true").unwrap();
    assert_eq!(tokens[0].kind, CanonicalKind::Null);
}

#[test]
fn config_without_keyword_falls_back_to_identifier() {
    let lexer = CanonicalLexer::with_config(canonical_config().without_keyword("handle"));
    let tokens = lexer.lex("handle").unwrap();
    assert_eq!(tokens[0].kind, CanonicalKind::Identifier("handle".into()));
}

// --- Config: custom delimiters ---

#[test]
fn config_custom_delimiter() {
    let lexer =
        CanonicalLexer::with_config(canonical_config().with_delimiter(';', CanonicalKind::Comma));
    let tokens = lexer.lex("a;b").unwrap();
    assert_eq!(
        tokens.iter().map(|t| t.kind.clone()).collect::<Vec<_>>(),
        vec![
            CanonicalKind::Identifier("a".into()),
            CanonicalKind::Comma,
            CanonicalKind::Identifier("b".into()),
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn config_without_delimiter_errors() {
    let lexer = CanonicalLexer::with_config(canonical_config().without_delimiter(':'));
    let result = lexer.lex(":");
    assert!(result.is_err());
}

// --- Config: default is canonical ---

#[test]
fn config_default_matches_canonical() {
    let default = CanonicalLexer::default();
    let canonical = CanonicalLexer::with_config(canonical_config());

    let input = "handle(\"fetch\"(url) as k: k(42))";
    let default_tokens: Vec<_> = default
        .lex(input)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    let canonical_tokens: Vec<_> = canonical
        .lex(input)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();

    assert_eq!(default_tokens, canonical_tokens);
}

// --- Streaming (lex_iter) ---

fn lex_iter_kinds(input: &str) -> Vec<CanonicalKind> {
    CanonicalLexer::default()
        .lex_iter(input)
        .map(|r| r.unwrap().kind)
        .collect()
}

#[test]
fn lex_iter_matches_batch() {
    let inputs = [
        "42",
        r#""hello""#,
        "true false null",
        "add(1, 2)",
        "handle(\"fetch\"(url) as k: k(42))",
        "[1, 2, 3]",
        "{a: 1, b: 2}",
        "math.sqrt(16)",
        "",
        "   ",
    ];

    for input in &inputs {
        let batch: Vec<CanonicalKind> = lex(input);
        let stream: Vec<CanonicalKind> = lex_iter_kinds(input);
        assert_eq!(batch, stream, "mismatch for input: {input:?}");
    }
}

#[test]
fn lex_iter_yields_one_at_a_time() {
    let lexer = CanonicalLexer::default();
    let mut iter = lexer.lex_iter("add(1, 2)");

    assert_eq!(
        iter.next().unwrap().unwrap().kind,
        CanonicalKind::Identifier("add".into())
    );
    assert_eq!(iter.next().unwrap().unwrap().kind, CanonicalKind::CallOpen);
    assert_eq!(
        iter.next().unwrap().unwrap().kind,
        CanonicalKind::Number(1.0)
    );
    assert_eq!(iter.next().unwrap().unwrap().kind, CanonicalKind::Comma);
    assert_eq!(
        iter.next().unwrap().unwrap().kind,
        CanonicalKind::Number(2.0)
    );
    assert_eq!(iter.next().unwrap().unwrap().kind, CanonicalKind::CallClose);
    assert_eq!(iter.next().unwrap().unwrap().kind, CanonicalKind::Eof);
    assert!(iter.next().is_none());
}

#[test]
fn lex_iter_empty_input() {
    let tokens: Vec<_> = CanonicalLexer::default().lex_iter("").collect();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].as_ref().unwrap().kind, CanonicalKind::Eof);
}

#[test]
fn lex_iter_whitespace_only() {
    let tokens: Vec<_> = CanonicalLexer::default().lex_iter("   \n\t  ").collect();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].as_ref().unwrap().kind, CanonicalKind::Eof);
}

#[test]
fn lex_iter_error_stops_iteration() {
    let lexer = CanonicalLexer::default();
    let mut iter = lexer.lex_iter("@");

    let first = iter.next();
    assert!(first.unwrap().is_err());

    assert!(iter.next().is_none());
}

#[test]
fn lex_iter_error_after_valid_tokens() {
    let lexer = CanonicalLexer::default();
    let mut iter = lexer.lex_iter("42 @");

    assert_eq!(
        iter.next().unwrap().unwrap().kind,
        CanonicalKind::Number(42.0)
    );
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn lex_iter_array_literal() {
    let tokens = lex_iter_kinds("[1, 2, 3]");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BracketOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(3.0),
            CanonicalKind::BracketClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_iter_object_literal() {
    let tokens = lex_iter_kinds("{a: 1, b: 2}");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::BlockOpen,
            CanonicalKind::Identifier("a".into()),
            CanonicalKind::Colon,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Identifier("b".into()),
            CanonicalKind::Colon,
            CanonicalKind::Number(2.0),
            CanonicalKind::BlockClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_iter_with_custom_config() {
    let lexer = CanonicalLexer::with_config(
        canonical_config().with_keyword("await", CanonicalKind::Handle),
    );

    let tokens: Vec<_> = lexer
        .lex_iter("await(42)")
        .map(|r| r.unwrap().kind)
        .collect();
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Handle,
            CanonicalKind::CallOpen,
            CanonicalKind::Number(42.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof
        ]
    );
}

// --- Composability: custom rules ---

#[test]
fn custom_rule_handles_comments() {
    use crate::CanonicalKind;
    use handl_lexer::{DefaultLexError, FnRule, LexerConfig, Span, Token};

    fn comment_rule(
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        match chars.peek() {
            Some(&(_, '#')) => {
                let (start, _) = chars.next().unwrap();
                let mut end = start + 1;
                while let Some(&(pos, ch)) = chars.peek() {
                    if ch == '\n' {
                        end = pos;
                        break;
                    }
                    end = pos + ch.len_utf8();
                    chars.next();
                }
                Some(Ok(Token::new(
                    CanonicalKind::Comment("# comment".into()),
                    Span::new(start, end),
                )))
            }
            _ => None,
        }
    }

    let config = canonical_config().insert_rule(0, FnRule::new("hash_comment", comment_rule));
    let lexer = CanonicalLexer::with_config(config);

    let tokens: Vec<_> = lexer
        .lex("# this is a comment\n42")
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], CanonicalKind::Number(42.0));
    assert_eq!(tokens[1], CanonicalKind::Eof);
}

#[test]
fn merge_configs_and_lex() {
    let base = canonical_config();
    let extension = canonical_config()
        .with_keyword("await", CanonicalKind::Handle)
        .with_delimiter(';', CanonicalKind::Comma);

    let merged = base.merge(extension);
    let lexer = CanonicalLexer::with_config(merged);

    let tokens: Vec<_> = lexer
        .lex("await(1; 2)")
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Handle,
            CanonicalKind::CallOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn rule_ordering_matters() {
    use crate::CanonicalKind;
    use handl_lexer::{DefaultLexError, FnRule, LexerConfig, Span, Token};

    fn custom_ident_rule(
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        config: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        match chars.peek() {
            Some(&(_, '$')) => {
                let (start, _) = chars.next().unwrap();
                let mut text = String::from("$");
                let mut end = start + 1;
                while let Some(&(pos, ch)) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        text.push(ch);
                        end = pos + ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                let kind = config
                    .lookup_keyword(&text)
                    .cloned()
                    .unwrap_or_else(|| CanonicalKind::Identifier(text));
                Some(Ok(Token::new(kind, Span::new(start, end))))
            }
            _ => None,
        }
    }

    let config = canonical_config().insert_rule(0, FnRule::new("dollar_ident", custom_ident_rule));
    let lexer = CanonicalLexer::with_config(config);

    let tokens: Vec<_> = lexer
        .lex("$foo")
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(
        tokens,
        vec![CanonicalKind::Identifier("$foo".into()), CanonicalKind::Eof]
    );
}

#[test]
fn without_rule_disables_feature() {
    let config = canonical_config().without_rule_by_name("string");
    let lexer = CanonicalLexer::with_config(config);

    let result = lexer.lex("\"hello\"");
    assert!(result.is_err());
}

#[test]
fn merge_preserves_rule_order() {
    use handl_lexer::{DefaultLexError, FnRule, LexerConfig, Token};

    fn rule_a(
        _: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        None
    }
    fn rule_b(
        _: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        None
    }

    let base = canonical_config().with_rule(FnRule::new("rule_a", rule_a));
    let extension = canonical_config().with_rule(FnRule::new("rule_b", rule_b));

    let merged = base.merge(extension);
    let rules = merged.rules();

    assert_eq!(rules.len(), 10);
    assert_eq!(rules[8].name(), "rule_a");
    assert_eq!(rules[9].name(), "rule_b");
}

#[test]
fn custom_rule_emits_extension_token() {
    use crate::CanonicalKind;
    use handl_lexer::{DefaultLexError, FnRule, LexerConfig, Span, Token};

    fn regex_rule(
        chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _config: &LexerConfig<CanonicalKind, DefaultLexError>,
    ) -> Option<Result<Token<CanonicalKind>, DefaultLexError>> {
        match chars.peek() {
            Some(&(_, '/')) => {
                let (start, _) = chars.next().unwrap();
                let mut value = String::new();
                let mut end = start + 1;
                while let Some(&(pos, ch)) = chars.peek() {
                    if ch == '/' {
                        end = pos + 1;
                        chars.next();
                        break;
                    }
                    value.push(ch);
                    end = pos + ch.len_utf8();
                    chars.next();
                }
                Some(Ok(Token::new(
                    CanonicalKind::Comment(format!("regex:{value}")),
                    Span::new(start, end),
                )))
            }
            _ => None,
        }
    }

    let config = canonical_config().insert_rule(0, FnRule::new("regex", regex_rule));
    let lexer = CanonicalLexer::with_config(config);

    let tokens: Vec<_> = lexer
        .lex("/[a-z]+/")
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], CanonicalKind::Eof);
}

// --- Comments ---

#[test]
fn lex_line_comment_at_eof() {
    let tokens = lex("// hello world");
    assert_eq!(tokens, vec![CanonicalKind::Eof]);
}

#[test]
fn lex_line_comment_before_expression() {
    let tokens = lex("// comment\n42");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_line_comment_after_expression() {
    let tokens = lex("42 // trailing comment");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_line_comment_between_tokens() {
    let tokens = lex("add // first\n// second\n(1)");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("add".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_block_comment_at_eof() {
    let tokens = lex("/* hello */");
    assert_eq!(tokens, vec![CanonicalKind::Eof]);
}

#[test]
fn lex_block_comment_before_expression() {
    let tokens = lex("/* comment */ 42");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_block_comment_multiline() {
    let tokens = lex("/* line one\nline two */ 42");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_block_comment_nested_stars() {
    let tokens = lex("/* a * b * c */ 42");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_block_comment_inside_call() {
    let tokens = lex("add(/* x */ 1, /* y */ 2)");
    assert_eq!(
        tokens,
        vec![
            CanonicalKind::Identifier("add".into()),
            CanonicalKind::CallOpen,
            CanonicalKind::Number(1.0),
            CanonicalKind::Comma,
            CanonicalKind::Number(2.0),
            CanonicalKind::CallClose,
            CanonicalKind::Eof,
        ]
    );
}

#[test]
fn lex_mixed_comments() {
    let tokens = lex("// line\n/* block */ 42 // trailing");
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_unterminated_block_comment() {
    let result = CanonicalLexer::default().lex("/* never closed");
    assert!(result.is_err());
}

#[test]
fn lex_iter_skips_comments() {
    let lexer = CanonicalLexer::default();
    let tokens: Vec<_> = lexer
        .lex_iter("// comment\n42 /* block */")
        .map(|r| r.unwrap().kind)
        .collect();
    assert_eq!(
        tokens,
        vec![CanonicalKind::Number(42.0), CanonicalKind::Eof]
    );
}

#[test]
fn lex_iter_line_comment_at_eof() {
    let lexer = CanonicalLexer::default();
    let tokens: Vec<_> = lexer
        .lex_iter("// just a comment")
        .map(|r| r.unwrap().kind)
        .collect();
    assert_eq!(tokens, vec![CanonicalKind::Eof]);
}
