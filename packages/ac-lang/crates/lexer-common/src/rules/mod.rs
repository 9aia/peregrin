//! Rule kinds and rule mock types.

mod block_comment;
mod delimiter;
mod ident;
mod keyword;
mod line_comment;
mod mock;
mod number;
mod operator;
mod string;

mod config;

#[cfg(test)]
mod block_comment_test;
#[cfg(test)]
mod delimiter_test;
#[cfg(test)]
mod ident_test;
#[cfg(test)]
mod keyword_test;
#[cfg(test)]
mod line_comment_test;
#[cfg(test)]
mod number_test;
#[cfg(test)]
mod operator_test;
#[cfg(test)]
mod string_test;

pub use block_comment::BlockCommentRule;
pub use config::{empty_config, test_config};
pub use delimiter::DelimiterRule;
pub use ident::IdentRule;
pub use keyword::KeywordRule;
pub use line_comment::LineCommentRule;
pub use mock::{AtRule, BangRule, HashRule, RuleMock};
pub use number::NumberRule;
pub use operator::OperatorRule;
pub use string::StringRule;
