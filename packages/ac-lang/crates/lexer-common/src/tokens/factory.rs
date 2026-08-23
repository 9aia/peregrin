//! [`MockFactory`] — a configurable [`KindFactory`] for test token kinds.

use ac_lexer::{CommentFactory, KindFactory};
use super::MockKind;
use std::sync::Arc;

pub struct MockFactory {
    string_fn: Arc<dyn Fn(String) -> MockKind + Send + Sync>,
    number_fn: Arc<dyn Fn(f64) -> MockKind + Send + Sync>,
    ident_fn: Arc<dyn Fn(String) -> MockKind + Send + Sync>,
    comment_fn: Arc<dyn Fn(String) -> MockKind + Send + Sync>,
    keyword_fn: Arc<dyn Fn(String) -> Option<MockKind> + Send + Sync>,
    operator_fn: Arc<dyn Fn(String) -> Option<MockKind> + Send + Sync>}

impl Default for MockFactory {
    fn default() -> Self {
        MockFactory::new()
    }
}

impl MockFactory {
    pub fn new() -> Self {
        MockFactory {
            string_fn: Arc::new(|v| MockKind::Str(v)),
            number_fn: Arc::new(|v| MockKind::Num(v)),
            ident_fn: Arc::new(|t| MockKind::Ident(t)),
            comment_fn: Arc::new(|t| MockKind::Comment(t)),
            keyword_fn: Arc::new(|_| None),
            operator_fn: Arc::new(|_| None)}
    }

    pub fn with_string_kind<F>(self, f: F) -> Self
    where F: Fn(String) -> MockKind + Send + Sync + 'static
    {
        MockFactory { string_fn: Arc::new(f), ..self }
    }

    pub fn with_number_kind<F>(self, f: F) -> Self
    where F: Fn(f64) -> MockKind + Send + Sync + 'static
    {
        MockFactory { number_fn: Arc::new(f), ..self }
    }

    pub fn with_ident_kind<F>(self, f: F) -> Self
    where F: Fn(String) -> MockKind + Send + Sync + 'static
    {
        MockFactory { ident_fn: Arc::new(f), ..self }
    }

    pub fn with_comment_kind<F>(self, f: F) -> Self
    where F: Fn(String) -> MockKind + Send + Sync + 'static
    {
        MockFactory { comment_fn: Arc::new(f), ..self }
    }

    pub fn with_keyword_kind<F>(self, f: F) -> Self
    where F: Fn(String) -> Option<MockKind> + Send + Sync + 'static
    {
        MockFactory { keyword_fn: Arc::new(f), ..self }
    }

    pub fn with_operator_kind<F>(self, f: F) -> Self
    where F: Fn(String) -> Option<MockKind> + Send + Sync + 'static
    {
        MockFactory { operator_fn: Arc::new(f), ..self }
    }
}

impl Clone for MockFactory {
    fn clone(&self) -> Self {
        MockFactory {
            string_fn: Arc::clone(&self.string_fn),
            number_fn: Arc::clone(&self.number_fn),
            ident_fn: Arc::clone(&self.ident_fn),
            comment_fn: Arc::clone(&self.comment_fn),
            keyword_fn: Arc::clone(&self.keyword_fn),
            operator_fn: Arc::clone(&self.operator_fn)}
    }
}

impl KindFactory<MockKind> for MockFactory {
    fn string_kind(&self, value: String) -> MockKind { (self.string_fn)(value) }
    fn number_kind(&self, value: f64) -> MockKind { (self.number_fn)(value) }
    fn ident_kind(&self, text: String) -> MockKind { (self.ident_fn)(text) }
    fn eof_kind(&self) -> MockKind { MockKind::Eof }
    fn keyword_kind(&self, text: &str) -> Option<MockKind> { (self.keyword_fn)(text.into()) }
    fn operator_kind(&self, text: &str) -> Option<MockKind> { (self.operator_fn)(text.into()) }
}

impl CommentFactory<MockKind> for MockFactory {
    fn comment_kind(&self, text: String) -> MockKind { (self.comment_fn)(text) }
}
