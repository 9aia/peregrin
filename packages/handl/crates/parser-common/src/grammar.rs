use std::collections::HashMap;
use std::mem::Discriminant;
use std::sync::Arc;

use crate::error::ParseError;
use crate::parser::ParserContext;
use handl_lexer::{Span, Token, TokenKind};

pub struct Rule<K: TokenKind, N, E: ParseError> {
    pub left_bp: u32,
    pub right_bp: Option<u32>,
    pub nud: Option<Arc<dyn Fn(Token<K>, &mut ParserContext<K, N, E>) -> Result<N, E> + 'static>>,
    pub led: Option<
        Arc<dyn Fn(N, Token<K>, &mut ParserContext<K, N, E>, u32) -> Result<N, E> + 'static>,
    >,
}

impl<K: TokenKind, N, E: ParseError> Rule<K, N, E> {
    pub fn new(left_bp: u32) -> Self {
        Self {
            left_bp,
            right_bp: None,
            nud: None,
            led: None,
        }
    }

    pub fn with_nud<F>(mut self, f: F) -> Self
    where
        F: Fn(Token<K>, &mut ParserContext<K, N, E>) -> Result<N, E> + 'static,
    {
        let f: Arc<dyn Fn(Token<K>, &mut ParserContext<K, N, E>) -> Result<N, E> + 'static> =
            Arc::new(f);
        self.nud = Some(f);
        self
    }

    pub fn with_led<F>(mut self, f: F) -> Self
    where
        F: Fn(N, Token<K>, &mut ParserContext<K, N, E>, u32) -> Result<N, E> + 'static,
    {
        let f: Arc<
            dyn Fn(N, Token<K>, &mut ParserContext<K, N, E>, u32) -> Result<N, E> + 'static,
        > = Arc::new(f);
        self.led = Some(f);
        self
    }

    pub fn with_right_bp(mut self, right_bp: u32) -> Self {
        self.right_bp = Some(right_bp);
        self
    }
}

impl<K: TokenKind, N, E: ParseError> Clone for Rule<K, N, E> {
    fn clone(&self) -> Self {
        Self {
            left_bp: self.left_bp,
            right_bp: self.right_bp,
            nud: self.nud.clone(),
            led: self.led.clone(),
        }
    }
}

pub trait Grammar<K: TokenKind, N, E: ParseError> {
    fn rule_for(&self, kind: &K) -> Option<Rule<K, N, E>>;
    fn eof(&self) -> Option<K>;
    fn parse_root(&self, context: &mut ParserContext<K, N, E>) -> Result<N, E>;
}

pub struct SimpleGrammar<K: TokenKind, N, E: ParseError> {
    rules: HashMap<Discriminant<K>, Rule<K, N, E>>,
    eof: Option<K>,
    parse_root: Arc<dyn Fn(&mut ParserContext<K, N, E>) -> Result<N, E> + 'static>,
}

impl<K: TokenKind, N, E: ParseError> SimpleGrammar<K, N, E> {
    pub fn new() -> Self {
        let parse_root: Arc<dyn Fn(&mut ParserContext<K, N, E>) -> Result<N, E> + 'static> =
            Arc::new(|_| Err(E::unexpected_eof(Span::at(0), "a root parser")));
        Self {
            rules: HashMap::new(),
            eof: None,
            parse_root,
        }
    }

    pub fn rule(mut self, kind: K, rule: Rule<K, N, E>) -> Self {
        self.rules.insert(std::mem::discriminant(&kind), rule);
        self
    }

    pub fn eof(mut self, kind: K) -> Self {
        self.eof = Some(kind);
        self
    }

    pub fn parse_root<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut ParserContext<K, N, E>) -> Result<N, E> + 'static,
    {
        let f: Arc<dyn Fn(&mut ParserContext<K, N, E>) -> Result<N, E> + 'static> = Arc::new(f);
        self.parse_root = f;
        self
    }
}

impl<K: TokenKind, N, E: ParseError> Grammar<K, N, E> for SimpleGrammar<K, N, E> {
    fn rule_for(&self, kind: &K) -> Option<Rule<K, N, E>> {
        self.rules.get(&std::mem::discriminant(kind)).cloned()
    }

    fn eof(&self) -> Option<K> {
        self.eof.clone()
    }

    fn parse_root(&self, context: &mut ParserContext<K, N, E>) -> Result<N, E> {
        self.parse_root.as_ref()(context)
    }
}
