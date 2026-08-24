use ac_ast_canonical::{Expr, HandlerClause, Pattern};
use ac_lexer::{DefaultLexError, Token};
use ac_lexer_canonical::CanonicalKind;
use ac_parser::{ParseError as AcParseError, Parser as ParserTrait};
use ac_parser_common::{DefaultParseError, Grammar, ParseError, Parser, ParserContext};
use ac_runtime_values::NonFunctionValue;

use serde_json::json;

fn to_ac_parse_error(e: DefaultParseError) -> AcParseError {
    match e {
        DefaultParseError::UnexpectedEof { span, expected } => {
            AcParseError::UnexpectedEof { span, expected }
        }
        DefaultParseError::UnexpectedToken { span, expected } => {
            AcParseError::UnexpectedToken { span, expected }
        }
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
    fn parse(
        &self,
        tokens: Vec<Token<CanonicalKind>>,
    ) -> Result<Expr<NonFunctionValue>, AcParseError> {
        Parser::new(CanonicalGrammar)
            .parse(tokens, "")
            .map_err(to_ac_parse_error)
            .map(|ast| ast.root)
    }

    fn parse_text(
        &self,
        input: &str,
        lexer: &dyn ac_lexer::Lexer<CanonicalKind, DefaultLexError>,
    ) -> Result<Expr<NonFunctionValue>, AcParseError> {
        let tokens = lexer
            .lex(input)
            .map_err(|e| AcParseError::LexError { message: e.to_string() })?;
        Parser::new(CanonicalGrammar)
            .parse(tokens, input)
            .map_err(to_ac_parse_error)
            .map(|ast| ast.root)
    }
}

#[derive(Clone, Copy, Default)]
struct CanonicalGrammar;

impl Grammar<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError> for CanonicalGrammar {
    fn rule_for(
        &self,
        _kind: &CanonicalKind,
    ) -> Option<ac_parser_common::Rule<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>>
    {
        None
    }

    fn eof(&self) -> Option<CanonicalKind> {
        Some(CanonicalKind::Eof)
    }

    fn parse_root(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        if self.has_top_level_assignment(context) {
            let target = self.parse_pattern(context)?;
            context.expect(CanonicalKind::Equal, "`=`")?;
            let value = self.parse_expr(context)?;
            Ok(Expr::Assign {
                target,
                value: Box::new(value),
            })
        } else {
            self.parse_expr(context)
        }
    }
}

impl CanonicalGrammar {
    /// Returns `true` if there is a top-level `=` in the remaining token
    /// stream, ignoring `=` that appear inside delimited groups.
    fn has_top_level_assignment(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> bool {
        let mut depth: i32 = 0;
        let mut bracket: i32 = 0;
        let mut block: i32 = 0;
        for i in context.pos()..context.tokens().len() {
            match context.tokens()[i].kind {
                CanonicalKind::CallOpen => depth += 1,
                CanonicalKind::CallClose => depth -= 1,
                CanonicalKind::BracketOpen => bracket += 1,
                CanonicalKind::BracketClose => bracket -= 1,
                CanonicalKind::BlockOpen => block += 1,
                CanonicalKind::BlockClose => block -= 1,
                CanonicalKind::Equal if depth == 0 && bracket == 0 && block == 0 => return true,
                _ => {}
            }
            if depth < 0 || bracket < 0 || block < 0 {
                break;
            }
        }
        false
    }

    fn parse_expr(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut primary = self.parse_primary(context)?;
        while let Some(kind) = context.peek_kind() {
            if kind == CanonicalKind::CallOpen {
                let args = self.parse_call_args(context)?;
                primary = Expr::Apply {
                    target: Box::new(primary),
                    args,
                };
            } else if kind == CanonicalKind::BracketOpen {
                context.advance()?;
                let index = self.parse_expr(context)?;
                context.expect(CanonicalKind::BracketClose, "`]`")?;
                primary = Expr::Index {
                    target: Box::new(primary),
                    index: Box::new(index),
                };
            } else if kind == CanonicalKind::Dot {
                context.advance()?;
                let field = match context.peek_kind() {
                    Some(CanonicalKind::Identifier(name)) => {
                        context.advance()?;
                        name
                    }
                    _ => {
                        return Err(DefaultParseError::unexpected_token(
                            context.peek_span(),
                            "field name after `.`",
                        ));
                    }
                };
                primary = Expr::field_access(primary, field);
            } else {
                break;
            }
        }
        Ok(primary)
    }

    fn parse_primary(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
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
                self.parse_after_identifier(name, context)
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
            Some(CanonicalKind::CallOpen) => {
                context.advance()?;
                self.parse_parenthesized(context)
            }
            Some(CanonicalKind::Eof) => Err(DefaultParseError::unexpected_eof(
                context.peek_span(),
                "an expression",
            )),
            Some(_) => Err(DefaultParseError::unexpected_token(
                context.peek_span(),
                "an expression",
            )),
            None => Err(DefaultParseError::unexpected_eof(
                context.peek_span(),
                "an expression",
            )),
        }
    }

    fn find_matching_call_close(
        &self,
        context: &ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
        start: usize,
    ) -> Option<usize> {
        let tokens = context.tokens();
        let mut depth = 1;
        for i in start..tokens.len() {
            match tokens[i].kind {
                CanonicalKind::CallOpen => depth += 1,
                CanonicalKind::CallClose => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return Some(i);
            }
            if depth < 0 {
                break;
            }
        }
        None
    }

    fn parse_parenthesized(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let start = context.pos();
        if let Some(close) = self.find_matching_call_close(context, start) {
            let after = context.tokens().get(close + 1).map(|t| t.kind.clone());
            if matches!(after, Some(CanonicalKind::Arrow) | Some(CanonicalKind::Colon)) {
                return self.parse_function_literal(Vec::new(), context);
            }
        }
        let expr = self.parse_expr(context)?;
        context.expect(CanonicalKind::CallClose, "`)`")?;
        Ok(expr)
    }

    fn parse_call_args(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Vec<Expr<NonFunctionValue>>, DefaultParseError> {
        context.expect(CanonicalKind::CallOpen, "`(")?;
        let mut args = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::CallClose) {
            args.push(self.parse_expr(context)?);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                args.push(self.parse_expr(context)?);
            }
        }
        context.expect(CanonicalKind::CallClose, "`)")?;
        Ok(args)
    }

    fn parse_after_identifier(
        &self,
        name: String,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut full_name = name;
        while let Some(CanonicalKind::Dot) = context.peek_kind() {
            context.advance()?;
            let part = match context.peek_kind() {
                Some(CanonicalKind::Identifier(part)) => part,
                _ => {
                    return Err(DefaultParseError::unexpected_token(
                        context.peek_span(),
                        "identifier after `.`",
                    ))
                }
            };
            context.advance()?;
            full_name = format!("{full_name}.{part}");
        }

        match context.peek_kind() {
            Some(CanonicalKind::LessThan) => {
                let type_params = self.parse_generic_params(context)?;
                context.expect(CanonicalKind::CallOpen, "`(` after generic params")?;
                let (params, ret, body) = self.parse_function_parts(context)?;
                Ok(Expr::Assign {
                    target: Pattern::Identifier(full_name),
                    value: Box::new(Expr::Function {
                        type_params,
                        params,
                        ret,
                        body,
                    }),
                })
            }
            Some(CanonicalKind::CallOpen) => self.parse_call_or_named_function(full_name, context),
            _ => Ok(Expr::Var(full_name)),
        }
    }

    fn parse_call_or_named_function(
        &self,
        name: String,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        context.expect(CanonicalKind::CallOpen, "`(`")?;

        // An `Identifier(...)` is a named function only if the matching `)` is
        // followed by `:` (typed return) or `->` (body). Otherwise it is a call.
        let start = context.pos();
        let is_function = if let Some(close) = self.find_matching_call_close(context, start) {
            matches!(
                context.tokens().get(close + 1).map(|t| t.kind.clone()),
                Some(CanonicalKind::Arrow) | Some(CanonicalKind::Colon)
            )
        } else {
            false
        };

        if !is_function {
            let mut args = Vec::new();
            if context.peek_kind() != Some(CanonicalKind::CallClose) {
                args.push(self.parse_expr(context)?);
                while context.peek_kind() == Some(CanonicalKind::Comma) {
                    context.advance()?;
                    args.push(self.parse_expr(context)?);
                }
            }
            context.expect(CanonicalKind::CallClose, "`)`")?;
            return Ok(Expr::call(name, args));
        }

        let params = self.parse_param_list(context)?;
        context.expect(CanonicalKind::CallClose, "`)`")?;

        let has_annotations = params.iter().any(|(_, t)| t.is_some());

        match context.peek_kind() {
            Some(CanonicalKind::Colon) => {
                context.advance()?;
                let ret = Some(Box::new(self.parse_expr(context)?));
                context.expect(CanonicalKind::Arrow, "`->`")?;
                let body = self.capture_body(context)?;
                Ok(Expr::Assign {
                    target: Pattern::Identifier(name),
                    value: Box::new(Expr::Function {
                        type_params: Vec::new(),
                        params,
                        ret,
                        body,
                    }),
                })
            }
            Some(CanonicalKind::Arrow) => {
                context.advance()?;
                let body = self.capture_body(context)?;
                Ok(Expr::Assign {
                    target: Pattern::Identifier(name),
                    value: Box::new(Expr::Function {
                        type_params: Vec::new(),
                        params,
                        ret: None,
                        body,
                    }),
                })
            }
            _ if has_annotations => Err(DefaultParseError::message(
                context.peek_span(),
                "function with parameter types must have a `->` body",
            )),
            _ => {
                // Treat as a call only if every param is a simple identifier.
                let args: Vec<Expr<NonFunctionValue>> = params
                    .into_iter()
                    .map(|(pat, _)| match pat {
                        Pattern::Identifier(n) => Ok(Expr::Var(n)),
                        _ => Err(DefaultParseError::message(
                            context.peek_span(),
                            "expected function arrow `->` after pattern parameter list",
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Expr::call(name, args))
            }
        }
    }

    fn parse_function_literal(
        &self,
        type_params: Vec<String>,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let (params, ret, body) = self.parse_function_parts(context)?;
        Ok(Expr::Function {
            type_params,
            params,
            ret,
            body,
        })
    }

    fn parse_function_parts(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<
        (
            Vec<(Pattern, Option<Box<Expr<NonFunctionValue>>>)>,
            Option<Box<Expr<NonFunctionValue>>>,
            String,
        ),
        DefaultParseError,
    > {
        let params = self.parse_param_list(context)?;
        context.expect(CanonicalKind::CallClose, "`)`")?;

        let ret = if context.peek_kind() == Some(CanonicalKind::Colon) {
            context.advance()?;
            Some(Box::new(self.parse_expr(context)?))
        } else {
            None
        };

        context.expect(CanonicalKind::Arrow, "`->`")?;
        let body = self.capture_body(context)?;
        Ok((params, ret, body))
    }

    fn parse_param_list(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Vec<(Pattern, Option<Box<Expr<NonFunctionValue>>>)>, DefaultParseError> {
        let mut params = Vec::new();

        if context.peek_kind() == Some(CanonicalKind::CallClose) {
            return Ok(params);
        }

        let pattern = self.parse_pattern(context)?;
        let ty = if context.peek_kind() == Some(CanonicalKind::Colon) {
            context.advance()?;
            Some(Box::new(self.parse_expr(context)?))
        } else {
            None
        };
        params.push((pattern, ty));

        while context.peek_kind() == Some(CanonicalKind::Comma) {
            context.advance()?;
            let pattern = self.parse_pattern(context)?;
            let ty = if context.peek_kind() == Some(CanonicalKind::Colon) {
                context.advance()?;
                Some(Box::new(self.parse_expr(context)?))
            } else {
                None
            };
            params.push((pattern, ty));
        }

        Ok(params)
    }

    fn parse_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Pattern, DefaultParseError> {
        match context.peek_kind() {
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                Ok(Pattern::Identifier(name))
            }
            Some(CanonicalKind::BracketOpen) => {
                context.advance()?;
                self.parse_array_pattern(context)
            }
            Some(CanonicalKind::BlockOpen) => {
                context.advance()?;
                self.parse_object_pattern(context)
            }
            _ => Err(DefaultParseError::unexpected_token(
                context.peek_span(),
                "pattern (identifier, `[`, or `{`)",
            )),
        }
    }

    fn parse_array_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Pattern, DefaultParseError> {
        let mut elements = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::BracketClose) {
            elements.push(self.parse_pattern(context)?);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                elements.push(self.parse_pattern(context)?);
            }
        }
        context.expect(CanonicalKind::BracketClose, "`]`")?;
        Ok(Pattern::Array(elements))
    }

    fn parse_object_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Pattern, DefaultParseError> {
        let mut entries = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::BlockClose) {
            let (key, value) = self.parse_object_pattern_entry(context)?;
            entries.push((key, value));
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                let (key, value) = self.parse_object_pattern_entry(context)?;
                entries.push((key, value));
            }
        }
        context.expect(CanonicalKind::BlockClose, "`}`")?;
        Ok(Pattern::Object(entries))
    }

    fn parse_object_pattern_entry(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(String, Pattern), DefaultParseError> {
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
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "object key (string or identifier)",
                ))
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "object key (string or identifier)",
                ))
            }
        };

        if context.peek_kind() == Some(CanonicalKind::Colon) {
            context.advance()?;
            let pattern = self.parse_pattern(context)?;
            Ok((key, pattern))
        } else {
            Ok((key.clone(), Pattern::Identifier(key)))
        }
    }

    fn parse_generic_params(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Vec<String>, DefaultParseError> {
        context.expect(CanonicalKind::LessThan, "`<`")?;

        let mut params = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::GreaterThan) {
            let name = match context.peek_kind() {
                Some(CanonicalKind::Identifier(n)) => {
                    context.advance()?;
                    n
                }
                _ => {
                    return Err(DefaultParseError::unexpected_token(
                        context.peek_span(),
                        "generic parameter name",
                    ))
                }
            };
            params.push(name);

            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                let name = match context.peek_kind() {
                    Some(CanonicalKind::Identifier(n)) => {
                        context.advance()?;
                        n
                    }
                    _ => {
                        return Err(DefaultParseError::unexpected_token(
                            context.peek_span(),
                            "generic parameter name",
                        ))
                    }
                };
                params.push(name);
            }
        }

        context.expect(CanonicalKind::GreaterThan, "`>`")?;
        Ok(params)
    }

    fn capture_body(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<String, DefaultParseError> {
        let start = context.peek_span().start;
        self.parse_expr(context)?;
        let end = context.peek_span().start;
        Ok(context.source()[start..end].to_string())
    }

    fn parse_array(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
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

    fn parse_object(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
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

    fn parse_object_entry(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(String, Expr<NonFunctionValue>), DefaultParseError> {
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
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "object key (string or identifier)",
                ))
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "object key (string or identifier)",
                ))
            }
        };

        context.expect(CanonicalKind::Colon, "`:` after object key")?;
        let value = self.parse_expr(context)?;
        Ok((key, value))
    }

    fn parse_handle(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
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

    fn parse_handler_clause(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<HandlerClause<NonFunctionValue>, DefaultParseError> {
        let op_name = match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                s
            }
            Some(_) => {
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "operation name string",
                ))
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "operation name string",
                ))
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
                    return Err(DefaultParseError::unexpected_token(
                        context.peek_span(),
                        "parameter name",
                    ))
                }
                None => {
                    return Err(DefaultParseError::unexpected_eof(
                        context.peek_span(),
                        "parameter name",
                    ))
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
                        return Err(DefaultParseError::unexpected_token(
                            context.peek_span(),
                            "parameter name",
                    ))
                    }
                    None => {
                        return Err(DefaultParseError::unexpected_eof(
                            context.peek_span(),
                            "parameter name",
                        ))
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
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "continuation name",
                ))
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "continuation name",
                ))
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
