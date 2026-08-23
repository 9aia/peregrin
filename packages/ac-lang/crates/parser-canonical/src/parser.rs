use ac_ast::{Expr, HandlerClause};
use ac_lexer::{DefaultLexError, Token};
use ac_lexer_canonical::CanonicalKind;
use ac_parser::{ParseError as AcParseError, Parser as ParserTrait};
use ac_parser_common::{DefaultParseError, Grammar, ParseError, Parser, ParserContext};
use ac_runtime::NonFunctionValue;

use serde_json::json;

fn to_ac_parse_error(e: DefaultParseError) -> AcParseError {
    match e {
        DefaultParseError::UnexpectedEof { span, expected } => AcParseError::UnexpectedEof { span, expected },
        DefaultParseError::UnexpectedToken { span, expected } => AcParseError::UnexpectedToken { span, expected },
        DefaultParseError::Message { span, message } => AcParseError::UnexpectedToken {
            span,
            expected: message,
        },
    }
}

/// The canonical AC expression parser.
///
/// Recursive-descent parser that consumes a flat token list and
/// builds an [`Expr`] tree. Stateless — safe to reuse across inputs.
#[derive(Clone, Copy, Default)]
pub struct CanonicalParser;

impl ParserTrait<CanonicalKind, DefaultLexError> for CanonicalParser {
    fn parse(&self, tokens: Vec<Token<CanonicalKind>>) -> Result<Expr<NonFunctionValue>, AcParseError> {
        Parser::new(CanonicalGrammar)
            .parse(tokens, "")
            .map_err(to_ac_parse_error)
            .map(|ast| ast.root)
    }
}

#[derive(Clone, Copy, Default)]
struct CanonicalGrammar;

impl Grammar<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError> for CanonicalGrammar {
    fn rule_for(&self, _kind: &CanonicalKind) -> Option<ac_parser_common::Rule<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>> {
        None
    }

    fn eof(&self) -> Option<CanonicalKind> {
        Some(CanonicalKind::Eof)
    }

    fn parse_root(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let expr = self.parse_expr(context)?;
        Ok(expr)
    }
}

impl CanonicalGrammar {
    fn parse_expr(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                Ok(Expr::Value(NonFunctionValue::new(json!(s))))
            }
            Some(CanonicalKind::Number(n)) => {
                context.advance()?;
                Ok(Expr::Value(NonFunctionValue::new(json!(n))))
            }
            Some(CanonicalKind::Bool(b)) => {
                context.advance()?;
                Ok(Expr::Value(NonFunctionValue::new(json!(b))))
            }
            Some(CanonicalKind::Null) => {
                context.advance()?;
                Ok(Expr::Value(NonFunctionValue::new(json!(null))))
            }
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                if context.peek_kind() == Some(CanonicalKind::CallOpen) {
                    self.parse_call(name, context)
                } else {
                    Err(DefaultParseError::unexpected_token(context.peek_span(), "`(` to start a call"))
                }
            }
            Some(CanonicalKind::Handle) => {
                context.advance()?;
                self.parse_handle(context)
            }
            Some(CanonicalKind::BracketOpen) => {
                context.advance()?;
                self.parse_array(context)
            }
            Some(CanonicalKind::BlockOpen) => {
                context.advance()?;
                self.parse_object(context)
            }
            Some(CanonicalKind::Eof) => Err(DefaultParseError::unexpected_eof(context.peek_span(), "an expression")),
            Some(_) => Err(DefaultParseError::unexpected_token(context.peek_span(), "an expression")),
            None => Err(DefaultParseError::unexpected_eof(context.peek_span(), "an expression")),
        }
    }

    fn parse_call(&self, name: String, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        context.expect(CanonicalKind::CallOpen, "`(`")?;

        let mut args = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::CallClose) {
            args.push(self.parse_expr(context)?);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                args.push(self.parse_expr(context)?);
            }
        }

        context.expect(CanonicalKind::CallClose, "`)`")?;
        Ok(Expr::call(name, args))
    }

    fn parse_array(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut elements = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::BracketClose) {
            elements.push(self.parse_expr(context)?);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                elements.push(self.parse_expr(context)?);
            }
        }
        context.expect(CanonicalKind::BracketClose, "`]`")?;
        Ok(Expr::array(elements))
    }

    fn parse_object(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut entries = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::BlockClose) {
            entries.push(self.parse_object_entry(context)?);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                entries.push(self.parse_object_entry(context)?);
            }
        }
        context.expect(CanonicalKind::BlockClose, "`}`")?;
        Ok(Expr::object(entries))
    }

    fn parse_object_entry(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<(String, Expr<NonFunctionValue>), DefaultParseError> {
        let key = match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                s
            }
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                name
            }
            Some(_) => {
                return Err(DefaultParseError::unexpected_token(context.peek_span(), "object key (string or identifier)"));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(context.peek_span(), "object key (string or identifier)"));
            }
        };

        context.expect(CanonicalKind::Colon, "`:` after object key")?;
        let value = self.parse_expr(context)?;
        Ok((key, value))
    }

    fn parse_handle(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        context.expect(CanonicalKind::CallOpen, "`(` after `handle`")?;

        let mut handlers = Vec::new();
        handlers.push(self.parse_handler_clause(context)?);
        while context.peek_kind() == Some(CanonicalKind::Comma) {
            context.advance()?;
            handlers.push(self.parse_handler_clause(context)?);
        }

        context.expect(CanonicalKind::CallClose, "`)` to close `handle`")?;

        let body = if handlers.len() == 1 {
            handlers[0].body.clone()
        } else {
            handlers.last().unwrap().body.clone()
        };

        Ok(Expr::handle(body, handlers))
    }

    fn parse_handler_clause(&self, context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>) -> Result<HandlerClause<NonFunctionValue>, DefaultParseError> {
        let op_name = match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                s
            }
            Some(_) => {
                return Err(DefaultParseError::unexpected_token(context.peek_span(), "operation name string"));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(context.peek_span(), "operation name string"));
            }
        };

        context.expect(CanonicalKind::CallOpen, "`(` for handler params")?;

        let mut params = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::CallClose) {
            match context.peek_kind() {
                Some(CanonicalKind::Identifier(name)) => {
                    context.advance()?;
                    params.push(name);
                }
                Some(_) => {
                    return Err(DefaultParseError::unexpected_token(context.peek_span(), "parameter name"));
                }
                None => {
                    return Err(DefaultParseError::unexpected_eof(context.peek_span(), "parameter name"));
                }
            }
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                match context.peek_kind() {
                    Some(CanonicalKind::Identifier(name)) => {
                        context.advance()?;
                        params.push(name);
                    }
                    Some(_) => {
                        return Err(DefaultParseError::unexpected_token(context.peek_span(), "parameter name"));
                    }
                    None => {
                        return Err(DefaultParseError::unexpected_eof(context.peek_span(), "parameter name"));
                    }
                }
            }
        }

        context.expect(CanonicalKind::CallClose, "`)` after handler params")?;
        context.expect(CanonicalKind::As, "`as`")?;

        let continuation_name = match context.peek_kind() {
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                name
            }
            Some(_) => {
                return Err(DefaultParseError::unexpected_token(context.peek_span(), "continuation name"));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(context.peek_span(), "continuation name"));
            }
        };

        context.expect(CanonicalKind::Colon, "`:`")?;

        let body = self.parse_expr(context)?;

        Ok(HandlerClause {
            op_name,
            params,
            continuation_name,
            body,
        })
    }
}
