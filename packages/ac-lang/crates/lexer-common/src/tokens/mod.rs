//! Token mock types — [`MockKind`], [`MockFactory`], [`MockLexer`].

mod factory;
mod kind;
mod mock_lexer;

pub use factory::MockFactory;
pub use kind::MockKind;
pub use mock_lexer::MockLexer;