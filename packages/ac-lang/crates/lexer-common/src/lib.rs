//! # ac-lexer-common
//!
//! Shared test infrastructure and built-in rule kinds for the lexer crates.
//!
//! Provides:
//! - **Token mocks**: [`MockKind`], [`MockFactory`] (configurable), [`MockLexer`].
//! - **Rule kinds**: [`StringRule`], [`NumberRule`], [`DelimiterRule`],
//!   [`IdentRule`], [`KeywordRule`], [`OperatorRule`], [`LineCommentRule`],
//!   [`BlockCommentRule`].
//! - **Rule mocks**: [`RuleMock`], [`BangRule`], [`HashRule`], [`AtRule`].
//! - **Config builders**: [`empty_config`], [`test_config`].
//!
//! Depends on `ac-lexer` for core types (`LexRule`, `LexerConfig`, `Token`, etc.).

mod rules;
mod tokens;

pub use rules::{
    AtRule, BangRule, BlockCommentRule, DelimiterRule, HashRule, IdentRule, KeywordRule,
    LineCommentRule, NumberRule, OperatorRule, RuleMock, StringRule, empty_config, test_config,
};
pub use tokens::{MockFactory, MockKind, MockLexer};
