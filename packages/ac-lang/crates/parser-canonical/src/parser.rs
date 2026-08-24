use ac_ast_canonical::{Expr, HandlerClause, Pattern};
use ac_lexer::{DefaultLexError, Span, Token};
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
        let tokens = lexer.lex(input).map_err(|e| AcParseError::LexError {
            message: e.to_string(),
        })?;
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

    fn parse_postfix(
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
            } else if matches!(
                kind,
                CanonicalKind::Is
                    | CanonicalKind::Must
                    | CanonicalKind::May
                    | CanonicalKind::Not
                    | CanonicalKind::When
            ) {
                context.advance()?;
                let target_kind = self.parse_kind(context)?;
                let mood_fn = match &kind {
                    CanonicalKind::Is => "mood.int",
                    CanonicalKind::Must => "mood.norm",
                    CanonicalKind::May => "mood.pot",
                    CanonicalKind::Not => "mood.neg",
                    CanonicalKind::When => "mood.cnd",
                    _ => unreachable!(),
                };
                let mut args = vec![
                    primary,
                    Expr::Value(NonFunctionValue::new(json!(target_kind))),
                ];
                if matches!(kind, CanonicalKind::When) {
                    context.expect(CanonicalKind::Else, "`else`")?;
                    let default = self.parse_expr(context)?;
                    args.push(default);
                }
                primary = Expr::call(mood_fn, args);
            } else {
                break;
            }
        }
        Ok(primary)
    }

    fn parse_expr(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        self.parse_expr_bp(context, 0)
    }

    fn parse_expr_bp(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
        min_bp: u32,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut left = self.parse_postfix(context)?;

        while let Some(kind) = context.peek_kind() {
            let (lbp, rbp) = match self.infix_binding(&kind) {
                Some(bp) => bp,
                None => break,
            };

            if lbp < min_bp {
                break;
            }

            context.advance()?;

            left = match kind {
                CanonicalKind::Pipe => {
                    let right = self.parse_expr_bp(context, rbp)?;
                    Expr::call("fn.pipe", vec![left, right])
                }
                CanonicalKind::BackwardPipe => {
                    let right = self.parse_expr_bp(context, rbp)?;
                    Expr::call("fn.apply", vec![left, Expr::array(vec![right])])
                }
                CanonicalKind::Flow => {
                    let right = self.parse_expr_bp(context, rbp)?;
                    Expr::call("fn.flow", vec![left, right])
                }
                CanonicalKind::Compose => {
                    let right = self.parse_expr_bp(context, rbp)?;
                    Expr::call("fn.compose", vec![left, right])
                }
                CanonicalKind::Fanout => {
                    let right = self.parse_expr_bp(context, rbp)?;
                    let left_src = self.expr_to_canonical(&left)?;
                    let right_src = self.expr_to_canonical(&right)?;
                    let body = format!(
                        "[fn.apply({}, [__x]), fn.apply({}, [__x])]",
                        left_src, right_src
                    );
                    Expr::function(
                        Vec::new(),
                        vec![(Pattern::Identifier("__x".into()), None)],
                        None,
                        body,
                    )
                }
                CanonicalKind::Question => {
                    let then = self.parse_expr_bp(context, rbp)?;
                    context.expect(CanonicalKind::Colon, "`:` in ternary")?;
                    let else_ = self.parse_expr_bp(context, rbp)?;
                    Expr::call("if", vec![left, then, else_])
                }
                _ => break,
            };
        }

        Ok(left)
    }

    fn infix_binding(&self, kind: &CanonicalKind) -> Option<(u32, u32)> {
        match kind {
            CanonicalKind::Pipe => Some((40, 41)),
            CanonicalKind::BackwardPipe => Some((40, 39)),
            CanonicalKind::Fanout => Some((55, 56)),
            CanonicalKind::Flow => Some((60, 61)),
            CanonicalKind::Compose => Some((60, 61)),
            CanonicalKind::Question => Some((30, 29)),
            _ => None,
        }
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
            Some(CanonicalKind::Match) => {
                context.advance()?;
                self.parse_match(context)
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
            Some(CanonicalKind::Not) => {
                context.advance()?;
                let operand = self.parse_primary(context)?;
                Ok(Expr::call("bool.not", vec![operand]))
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

    fn parse_kind(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<String, DefaultParseError> {
        let mut parts = Vec::new();
        let first = match context.peek_kind() {
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                name
            }
            _ => {
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "kind name after mood keyword",
                ));
            }
        };
        parts.push(first);
        while context.peek_kind() == Some(CanonicalKind::Dot) {
            context.advance()?;
            match context.peek_kind() {
                Some(CanonicalKind::Identifier(name)) => {
                    context.advance()?;
                    parts.push(name);
                }
                _ => {
                    return Err(DefaultParseError::unexpected_token(
                        context.peek_span(),
                        "identifier after `.` in kind",
                    ));
                }
            }
        }
        Ok(parts.join("."))
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
            if matches!(
                after,
                Some(CanonicalKind::Arrow) | Some(CanonicalKind::Colon)
            ) {
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
                    ));
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
            let call = Expr::call(name, args);
            return if self.has_placeholder(&call) {
                self.make_partial(call)
            } else {
                Ok(call)
            };
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
                ));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "object key (string or identifier)",
                ));
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
                    ));
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
                        ));
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
                ));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "object key (string or identifier)",
                ));
            }
        };

        let value = if context.peek_kind() == Some(CanonicalKind::Colon) {
            context.advance()?;
            self.parse_expr(context)?
        } else {
            Expr::var(key.clone())
        };
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
                ));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "operation name string",
                ));
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
                    ));
                }
                None => {
                    return Err(DefaultParseError::unexpected_eof(
                        context.peek_span(),
                        "parameter name",
                    ));
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
                        ));
                    }
                    None => {
                        return Err(DefaultParseError::unexpected_eof(
                            context.peek_span(),
                            "parameter name",
                        ));
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
                ));
            }
            None => {
                return Err(DefaultParseError::unexpected_eof(
                    context.peek_span(),
                    "continuation name",
                ));
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

    fn parse_match(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let value = self.parse_expr(context)?;
        context.expect(CanonicalKind::BlockOpen, "`{`")?;

        let mut branches = Vec::new();
        if context.peek_kind() != Some(CanonicalKind::BlockClose) {
            loop {
                let (pattern, bindings) = self.parse_match_pattern(context)?;
                context.expect(CanonicalKind::Arrow, "`->`")?;
                let body = self.capture_body(context)?;

                let params = bindings
                    .into_iter()
                    .map(|name| (Pattern::Identifier(name), None))
                    .collect();
                let handler = Expr::function(Vec::new(), params, None, body);

                branches.push(Expr::array(vec![Expr::Value(pattern), handler]));

                if context.peek_kind() == Some(CanonicalKind::Comma) {
                    context.advance()?;
                } else {
                    break;
                }
            }
        }

        context.expect(CanonicalKind::BlockClose, "`}`")?;
        let mut args = vec![value];
        args.extend(branches);
        Ok(Expr::call("fn.match", args))
    }

    fn parse_match_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(NonFunctionValue, Vec<String>), DefaultParseError> {
        let (pattern, mut bindings) = self.parse_match_pattern_core(context)?;
        let json = match pattern {
            MatchPatternShape::Json(v) => v,
            MatchPatternShape::Binding(name) => {
                bindings.push(name.clone());
                NonFunctionValue::new(json!({"type": "identifier", "name": name}))
            }
            MatchPatternShape::Wildcard => NonFunctionValue::new(json!({"type": "wildcard"})),
        };
        Ok((json, bindings))
    }

    fn parse_match_pattern_core(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(MatchPatternShape, Vec<String>), DefaultParseError> {
        match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                Ok((MatchPatternShape::Json(NonFunctionValue::new(json!(s))), Vec::new()))
            }
            Some(CanonicalKind::Number(n)) => {
                context.advance()?;
                Ok((MatchPatternShape::Json(NonFunctionValue::new(json!(n))), Vec::new()))
            }
            Some(CanonicalKind::Bool(b)) => {
                context.advance()?;
                Ok((MatchPatternShape::Json(NonFunctionValue::new(json!(b))), Vec::new()))
            }
            Some(CanonicalKind::Null) => {
                context.advance()?;
                Ok((MatchPatternShape::Json(NonFunctionValue::new(json!(null))), Vec::new()))
            }
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                if name == "_" {
                    Ok((MatchPatternShape::Wildcard, Vec::new()))
                } else {
                    Ok((MatchPatternShape::Binding(name), Vec::new()))
                }
            }
            Some(CanonicalKind::BracketOpen) => self.parse_match_array_pattern(context),
            Some(CanonicalKind::BlockOpen) => self.parse_match_object_pattern(context),
            _ => Err(DefaultParseError::unexpected_token(
                context.peek_span(),
                "match pattern",
            )),
        }
    }

    fn parse_match_array_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(MatchPatternShape, Vec<String>), DefaultParseError> {
        context.expect(CanonicalKind::BracketOpen, "`[`")?;
        let mut patterns = Vec::new();
        let mut bindings = Vec::new();

        if context.peek_kind() != Some(CanonicalKind::BracketClose) {
            let (p, mut b) = self.parse_match_pattern(context)?;
            patterns.push(p);
            bindings.append(&mut b);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                let (p, mut b) = self.parse_match_pattern(context)?;
                patterns.push(p);
                bindings.append(&mut b);
            }
        }

        context.expect(CanonicalKind::BracketClose, "`]`")?;
        let arr = patterns.iter().map(|p| p.inner().clone()).collect::<Vec<_>>();
        let json = NonFunctionValue::new(json!({"type": "array", "patterns": arr}));
        Ok((MatchPatternShape::Json(json), bindings))
    }

    fn parse_match_object_pattern(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(MatchPatternShape, Vec<String>), DefaultParseError> {
        context.expect(CanonicalKind::BlockOpen, "`{`")?;
        let mut entries = Vec::new();
        let mut bindings = Vec::new();

        if context.peek_kind() != Some(CanonicalKind::BlockClose) {
            let (key, p, mut b) = self.parse_match_object_pattern_entry(context)?;
            entries.push((key, p));
            bindings.append(&mut b);
            while context.peek_kind() == Some(CanonicalKind::Comma) {
                context.advance()?;
                let (key, p, mut b) = self.parse_match_object_pattern_entry(context)?;
                entries.push((key, p));
                bindings.append(&mut b);
            }
        }

        context.expect(CanonicalKind::BlockClose, "`}`")?;

        let entries_json = entries
            .iter()
            .map(|(k, p)| json!([k, p.inner()]))
            .collect::<Vec<_>>();
        let json = NonFunctionValue::new(json!({"type": "object", "entries": entries_json}));
        Ok((MatchPatternShape::Json(json), bindings))
    }

    fn parse_match_object_pattern_entry(
        &self,
        context: &mut ParserContext<CanonicalKind, Expr<NonFunctionValue>, DefaultParseError>,
    ) -> Result<(String, NonFunctionValue, Vec<String>), DefaultParseError> {
        let key = match context.peek_kind() {
            Some(CanonicalKind::String(s)) => {
                context.advance()?;
                s
            }
            Some(CanonicalKind::Identifier(name)) => {
                context.advance()?;
                name
            }
            _ => {
                return Err(DefaultParseError::unexpected_token(
                    context.peek_span(),
                    "object pattern key",
                ));
            }
        };

        let (pattern, bindings) = if context.peek_kind() == Some(CanonicalKind::Colon) {
            context.advance()?;
            self.parse_match_pattern(context)?
        } else {
            let (shape, _) = self.parse_match_pattern_core(context)?;
            let mut b = Vec::new();
            let p = match shape {
                MatchPatternShape::Binding(_) | MatchPatternShape::Wildcard => {
                    self.shape_to_json(shape, &mut b)
                }
                MatchPatternShape::Json(v) => v,
            };
            (p, b)
        };
        Ok((key, pattern, bindings))
    }

    fn shape_to_json(
        &self,
        shape: MatchPatternShape,
        bindings: &mut Vec<String>,
    ) -> NonFunctionValue {
        match shape {
            MatchPatternShape::Json(v) => v,
            MatchPatternShape::Binding(name) => {
                bindings.push(name.clone());
                NonFunctionValue::new(json!({"type": "identifier", "name": name}))
            }
            MatchPatternShape::Wildcard => NonFunctionValue::new(json!({"type": "wildcard"})),
        }
    }

    fn make_partial(
        &self,
        mut expr: Expr<NonFunctionValue>,
    ) -> Result<Expr<NonFunctionValue>, DefaultParseError> {
        let mut params = Vec::new();
        self.rename_placeholders(&mut expr, &mut params);
        let body = self.expr_to_canonical(&expr)?;
        let patterns = params
            .into_iter()
            .map(|name| (Pattern::Identifier(name), None))
            .collect();
        Ok(Expr::function(Vec::new(), patterns, None, body))
    }

    fn has_placeholder(&self, expr: &Expr<NonFunctionValue>) -> bool {
        match expr {
            Expr::Var(name) if name == "_" => true,
            Expr::Call { args, .. } | Expr::Apply { args, .. } | Expr::Array { 0: args } => {
                args.iter().any(|a| self.has_placeholder(a))
            }
            Expr::FieldAccess { target, .. } | Expr::Index { target, .. } => {
                self.has_placeholder(target)
            }
            Expr::Object(entries) => entries.iter().any(|(_, v)| self.has_placeholder(v)),
            _ => false,
        }
    }

    fn rename_placeholders(
        &self,
        expr: &mut Expr<NonFunctionValue>,
        params: &mut Vec<String>,
    ) {
        match expr {
            Expr::Var(name) if name == "_" => {
                let new_name = format!("__p{}", params.len());
                *name = new_name.clone();
                params.push(new_name);
            }
            Expr::Call { args, .. } | Expr::Apply { args, .. } | Expr::Array { 0: args } => {
                for a in args.iter_mut() {
                    self.rename_placeholders(a, params);
                }
            }
            Expr::FieldAccess { target, .. } | Expr::Index { target, .. } => {
                self.rename_placeholders(target, params);
            }
            Expr::Object(entries) => {
                for (_, v) in entries.iter_mut() {
                    self.rename_placeholders(v, params);
                }
            }
            _ => {}
        }
    }

    fn expr_to_canonical(
        &self,
        expr: &Expr<NonFunctionValue>,
    ) -> Result<String, DefaultParseError> {
        match expr {
            Expr::Value(v) => {
                Ok(serde_json::to_string(v.inner()).unwrap_or_else(|_| "null".to_string()))
            }
            Expr::Var(name) => Ok(name.clone()),
            Expr::Call { name, args, .. } => {
                let rendered = args
                    .iter()
                    .map(|a| self.expr_to_canonical(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{}({})", name, rendered))
            }
            Expr::Apply { target, args } => {
                let target = self.expr_to_canonical(target)?;
                let rendered = args
                    .iter()
                    .map(|a| self.expr_to_canonical(a))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("fn.apply({}, [{}])", target, rendered))
            }
            Expr::FieldAccess { target, field } => {
                let target = self.expr_to_canonical(target)?;
                Ok(format!("{}.{}", target, field))
            }
            Expr::Index { target, index } => {
                let target = self.expr_to_canonical(target)?;
                let index = self.expr_to_canonical(index)?;
                Ok(format!("{}[{}]", target, index))
            }
            Expr::Array(elements) => {
                let rendered = elements
                    .iter()
                    .map(|e| self.expr_to_canonical(e))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("[{}]", rendered))
            }
            Expr::Object(entries) => {
                let rendered = entries
                    .iter()
                    .map(|(k, v)| {
                        let key = serde_json::to_string(k).unwrap_or_else(|_| "\"\"".to_string());
                        let value = self.expr_to_canonical(v)?;
                        Ok(format!("{}: {}", key, value))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{{{}}}", rendered))
            }
            Expr::Function {
                type_params,
                params,
                ret,
                body,
            } => {
                let params = params
                    .iter()
                    .map(|(p, _)| self.pattern_to_canonical(p))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let ret = match ret {
                    Some(r) => {
                        let r = self.expr_to_canonical(r)?;
                        format!(": {}", r)
                    }
                    None => String::new(),
                };
                let type_params = if type_params.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", type_params.join(", "))
                };
                Ok(format!("{}({}){} -> {}", type_params, params, ret, body))
            }
            _ => Err(DefaultParseError::message(
                Span::at(0),
                "unsupported expression in partial application",
            )),
        }
    }

    fn pattern_to_canonical(
        &self,
        pattern: &Pattern,
    ) -> Result<String, DefaultParseError> {
        match pattern {
            Pattern::Identifier(name) => Ok(name.clone()),
            Pattern::Array(elements) => {
                let inner = elements
                    .iter()
                    .map(|p| self.pattern_to_canonical(p))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("[{}]", inner))
            }
            Pattern::Object(entries) => {
                let inner = entries
                    .iter()
                    .map(|(k, p)| {
                        let p = self.pattern_to_canonical(p)?;
                        Ok(format!("{}: {}", k, p))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                Ok(format!("{{{}}}", inner))
            }
            Pattern::Rest(name) => Ok(format!("...{}", name)),
        }
    }
}

enum MatchPatternShape {
    Json(NonFunctionValue),
    Binding(String),
    Wildcard,
}
