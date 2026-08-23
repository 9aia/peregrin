use std::sync::Arc;

use ac_lexer::{Span, Token, TokenKind};
use crate::error::ParseError;
use crate::grammar::Grammar;

#[derive(Debug, Clone)]
pub struct Ast<N> {
    pub source: String,
    pub root: N}

pub struct ParserContext<K: TokenKind, N, E: ParseError> {
    pub(crate) tokens: Vec<Token<K>>,
    pub(crate) source: String,
    pub(crate) pos: usize,
    pub(crate) grammar: Arc<dyn Grammar<K, N, E>>}


impl<K: TokenKind, N, E: ParseError> ParserContext<K, N, E> {
    pub fn tokens(&self) -> &[Token<K>] {
        &self.tokens
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub fn peek(&self) -> Option<&Token<K>> {
        self.tokens.get(self.pos)
    }

    pub fn peek_kind(&self) -> Option<K> {
        self.peek().map(|t| t.kind.clone())
    }

    pub fn peek_span(&self) -> Span {
        self.peek()
            .map(|t| t.span)
            .unwrap_or_else(|| self.tokens.last().map(|t| Span::at(t.span.end)).unwrap_or(Span::at(0)))
    }

    pub fn advance(&mut self) -> Result<&Token<K>, E> {
        if self.tokens.get(self.pos).is_some() {
            let pos = self.pos;
            self.pos += 1;
            return Ok(&self.tokens[pos]);
        }
        Err(E::unexpected_eof(self.peek_span(), "a token"))
    }

    pub fn match_kind(&mut self, kind: K) -> Option<&Token<K>> {
        let matches = match self.tokens.get(self.pos) {
            Some(token) => std::mem::discriminant(&token.kind) == std::mem::discriminant(&kind),
            None => false};
        if matches {
            let pos = self.pos;
            self.pos += 1;
            return self.tokens.get(pos);
        }
        None
    }

    pub fn expect(&mut self, kind: K, label: &str) -> Result<&Token<K>, E> {
        let matches = match self.peek() {
            Some(token) => std::mem::discriminant(&token.kind) == std::mem::discriminant(&kind),
            None => false};
        if matches {
            let pos = self.pos;
            self.pos += 1;
            return Ok(&self.tokens[pos]);
        }
        Err(E::unexpected_token(self.peek_span(), label))
    }

    pub fn expression(&mut self, min_bp: u32) -> Result<N, E> {
        let token = self.advance()?.clone();
        let rule = self.grammar.rule_for(&token.kind).ok_or_else(|| {
            E::unexpected_token(token.span, "a prefix expression")
        })?;
        let nud = rule.nud.as_ref().ok_or_else(|| {
            E::unexpected_token(token.span, "a prefix expression")
        })?;
        let mut left = nud.as_ref()(token, self)?;

        while let Some(kind) = self.peek_kind() {
            let rule = match self.grammar.rule_for(&kind) {
                Some(rule) => rule,
                None => break};
            if rule.led.is_none() || rule.left_bp <= min_bp {
                break;
            }
            let operator = self.advance()?.clone();
            let rbp = rule.right_bp.unwrap_or(rule.left_bp);
            let led = rule.led.as_ref().unwrap();
            left = led.as_ref()(left, operator, self, rbp)?;
        }

        Ok(left)
    }
}

pub struct Parser<K: TokenKind, N, E: ParseError> {
    grammar: Arc<dyn Grammar<K, N, E>>}

impl<K: TokenKind, N, E: ParseError> Parser<K, N, E> {
    pub fn new<G>(grammar: G) -> Self
    where
        G: Grammar<K, N, E> + 'static,
    {
        Self {
            grammar: Arc::new(grammar)}
    }

    pub fn parse(&self, tokens: Vec<Token<K>>, source: &str) -> Result<Ast<N>, E> {
        let mut context = ParserContext {
            tokens,
            source: source.to_string(),
            pos: 0,
            grammar: self.grammar.clone()};
        let root = self.grammar.parse_root(&mut context)?;

        if let Some(eof) = self.grammar.eof() {
            context.expect(eof, "end of input")?;
        } else if !context.is_at_end() {
            return Err(E::unexpected_token(context.peek_span(), "end of input"));
        }

        Ok(Ast {
            source: source.to_string(),
            root})
    }
}
