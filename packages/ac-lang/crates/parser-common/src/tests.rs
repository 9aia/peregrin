use std::marker::PhantomData;

use ac_ast_common::{Node as AstNode, Span as AstSpan};
use ac_lexer::{KindFactory, Span, Token, TokenKind};

use crate::{DefaultParseError, ParseError, Parser, Rule, SimpleGrammar};
use proptest::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum MyKind {
    Number(f64),
    Plus,
    Eof,
}

impl std::fmt::Display for MyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyKind::Number(n) => write!(f, "number({n})"),
            MyKind::Plus => write!(f, "+"),
            MyKind::Eof => write!(f, "<eof>"),
        }
    }
}

#[derive(Clone)]
struct MyFactory;

impl KindFactory<MyKind> for MyFactory {
    fn string_kind(&self, _value: String) -> MyKind {
        MyKind::Eof
    }
    fn number_kind(&self, value: f64) -> MyKind {
        MyKind::Number(value)
    }
    fn ident_kind(&self, _text: String) -> MyKind {
        MyKind::Eof
    }
    fn eof_kind(&self) -> MyKind {
        MyKind::Eof
    }
}

impl TokenKind for MyKind {
    type Factory = MyFactory;

    fn is_eof(&self) -> bool {
        matches!(self, MyKind::Eof)
    }

    fn is_error(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NodeKind {
    Number(f64),
    Binary {
        op: String,
        left: Box<AstNode<NodeKind, (), AstSpan>>,
        right: Box<AstNode<NodeKind, (), AstSpan>>,
    },
}

type Node = AstNode<NodeKind, (), AstSpan>;

fn arithmetic_grammar() -> SimpleGrammar<MyKind, Node, DefaultParseError> {
    SimpleGrammar::<MyKind, Node, DefaultParseError>::new()
        .rule(
            MyKind::Number(0.0),
            Rule::new(0).with_nud(|token, _context| {
                if let MyKind::Number(n) = &token.kind {
                    Ok(Node {
                        kind: NodeKind::Number(*n),
                        metadata: AstSpan {
                            start: token.span.start,
                            end: token.span.end,
                        },
                        _value: PhantomData,
                    })
                } else {
                    Err(DefaultParseError::unexpected_token(token.span, "a number"))
                }
            }),
        )
        .rule(
            MyKind::Plus,
            Rule::new(10).with_led(|left, token, context, rbp| {
                let right = context.expression(rbp)?;
                Ok(Node {
                    kind: NodeKind::Binary {
                        op: "+".into(),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    metadata: AstSpan {
                        start: token.span.start,
                        end: token.span.end,
                    },
                    _value: PhantomData,
                })
            }),
        )
        .eof(MyKind::Eof)
        .parse_root(|context| context.expression(0))
}

#[test]
fn pratt_arithmetic() {
    let tokens = vec![
        Token::new(MyKind::Number(1.0), Span::new(0, 1)),
        Token::new(MyKind::Plus, Span::new(2, 3)),
        Token::new(MyKind::Number(2.0), Span::new(4, 5)),
        Token::new(MyKind::Eof, Span::new(5, 5)),
    ];

    let grammar = arithmetic_grammar();

    let ast = Parser::new(grammar).parse(tokens, "1 + 2").unwrap();

    assert!(matches!(ast.root.kind, NodeKind::Binary { .. }));
    if let NodeKind::Binary { op, left, right } = &ast.root.kind {
        assert_eq!(op, "+");
        assert!(matches!(left.kind, NodeKind::Number(1.0)));
        assert!(matches!(right.kind, NodeKind::Number(2.0)));
    } else {
        panic!("expected binary node");
    }
}

#[test]
fn unexpected_token_error() {
    let tokens = vec![Token::new(MyKind::Plus, Span::new(0, 1))];
    let grammar = SimpleGrammar::<MyKind, Node, DefaultParseError>::new().parse_root(
        |context: &mut crate::parser::ParserContext<MyKind, Node, DefaultParseError>| {
            context.expression(0)
        },
    );
    let result = Parser::new(grammar).parse(tokens, "+");
    assert!(result.is_err());
}

proptest! {
    #[test]
    fn pratt_arithmetic_chain(values in prop::collection::vec(
        any::<f64>().prop_filter("finite", |v| v.is_finite()),
        1..50,
    )) {
        let mut tokens: Vec<Token<MyKind>> = Vec::with_capacity(values.len() * 2 + 1);
        for (i, v) in values.iter().enumerate() {
            tokens.push(Token::new(MyKind::Number(*v), Span::new(i * 2, i * 2 + 1)));
            if i < values.len() - 1 {
                tokens.push(Token::new(MyKind::Plus, Span::new(i * 2 + 1, i * 2 + 2)));
            }
        }
        tokens.push(Token::new(MyKind::Eof, Span::new(values.len() * 2, values.len() * 2)));

        let grammar = arithmetic_grammar();
        let result = Parser::new(grammar).parse(tokens, "");
        prop_assert!(result.is_ok());
    }

    #[test]
    fn pratt_arithmetic_invalid(values in prop::collection::vec(
        any::<f64>().prop_filter("finite", |v| v.is_finite()),
        1..50,
    )) {
        let mut tokens: Vec<Token<MyKind>> = Vec::with_capacity(values.len() * 2 + 2);
        tokens.push(Token::new(MyKind::Plus, Span::new(0, 1)));
        for (i, v) in values.iter().enumerate() {
            tokens.push(Token::new(MyKind::Number(*v), Span::new(i * 2 + 1, i * 2 + 2)));
            if i < values.len() - 1 {
                tokens.push(Token::new(MyKind::Plus, Span::new(i * 2 + 2, i * 2 + 3)));
            }
        }
        tokens.push(Token::new(MyKind::Eof, Span::new(values.len() * 2 + 1, values.len() * 2 + 1)));

        let grammar = arithmetic_grammar();
        let result = Parser::new(grammar).parse(tokens, "");
        prop_assert!(result.is_err());
    }
}
