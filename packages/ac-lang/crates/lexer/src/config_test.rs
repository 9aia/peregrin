use crate::{
    DefaultLexError, FnRule, LexRule, LexerConfig, Token,
};
use crate::rule::test_utils::{BangRule, TestKind, cfg};

#[test]
fn new_is_empty() {
    let config = cfg();
    assert_eq!(config.rules().len(), 0);
    assert_eq!(config.lookup_keyword("true"), None);
    assert_eq!(config.lookup_delimiter('('), None);
}

#[test]
fn with_keyword_adds_new() {
    let config = cfg().with_keyword("await", TestKind::Null);
    assert_eq!(config.lookup_keyword("await"), Some(&TestKind::Null));
}

#[test]
fn with_keyword_replaces_existing() {
    let config = cfg().with_keyword("true", TestKind::Null);
    let config = config.with_keyword("true", TestKind::Bool(true));
    assert_eq!(config.lookup_keyword("true"), Some(&TestKind::Bool(true)));
}

#[test]
fn without_keyword_removes() {
    let config = cfg().with_keyword("handle", TestKind::Null);
    let config = config.without_keyword("handle");
    assert_eq!(config.lookup_keyword("handle"), None);
}

#[test]
fn with_delimiter_adds_new() {
    let config = cfg().with_delimiter(';', TestKind::Comma);
    assert_eq!(config.lookup_delimiter(';'), Some(&TestKind::Comma));
}

#[test]
fn without_delimiter_removes() {
    let config = cfg().with_delimiter(':', TestKind::Colon);
    let config = config.without_delimiter(':');
    assert_eq!(config.lookup_delimiter(':'), None);
}

#[test]
fn with_rule_appends() {
    let config = cfg();
    assert_eq!(config.rules().len(), 0);
    let config = config.with_rule(BangRule);
    assert_eq!(config.rules().len(), 1);
}

#[test]
fn without_rule_removes() {
    let config = cfg().with_rule(BangRule);
    let config = config.without_rule(&BangRule as &dyn LexRule<TestKind, DefaultLexError>);
    assert_eq!(config.rules().len(), 0);
}

#[test]
fn without_rule_by_name_removes() {
    let config = cfg().with_rule(BangRule);
    let config = config.without_rule_by_name("bang");
    assert_eq!(config.rules().len(), 0);
}

#[test]
fn insert_rule_at_index() {
    fn rule_a(
        _: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _: &LexerConfig<TestKind, DefaultLexError>,
    ) -> Option<Result<Token<TestKind>, DefaultLexError>> {
        None
    }
    fn rule_b(
        _: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _: &LexerConfig<TestKind, DefaultLexError>,
    ) -> Option<Result<Token<TestKind>, DefaultLexError>> {
        None
    }

    let config = cfg()
        .with_rule(FnRule::new("a", rule_a))
        .with_rule(FnRule::new("b", rule_b));
    let config = config.insert_rule(0, BangRule);
    assert_eq!(config.rules()[0].name(), "bang");
    assert_eq!(config.rules()[1].name(), "a");
    assert_eq!(config.rules()[2].name(), "b");
}

#[test]
fn merge_combines_keywords() {
    let base = cfg().with_keyword("true", TestKind::Bool(true));
    let extension = cfg()
        .with_keyword("await", TestKind::Null)
        .with_keyword("yield", TestKind::Comma);

    let merged = base.merge(extension);
    assert_eq!(merged.lookup_keyword("await"), Some(&TestKind::Null));
    assert_eq!(merged.lookup_keyword("yield"), Some(&TestKind::Comma));
    assert_eq!(merged.lookup_keyword("true"), Some(&TestKind::Bool(true)));
}

#[test]
fn merge_overrides_keywords() {
    let base = cfg().with_keyword("true", TestKind::Bool(true));
    let extension = cfg().with_keyword("true", TestKind::Null);

    let merged = base.merge(extension);
    assert_eq!(merged.lookup_keyword("true"), Some(&TestKind::Null));
}

#[test]
fn merge_combines_delimiters() {
    let base = cfg().with_delimiter('(', TestKind::Comma);
    let extension = cfg().with_delimiter(';', TestKind::Colon);

    let merged = base.merge(extension);
    assert_eq!(merged.lookup_delimiter(';'), Some(&TestKind::Colon));
    assert_eq!(merged.lookup_delimiter('('), Some(&TestKind::Comma));
}

#[test]
fn merge_dedups_rules() {
    let base = cfg().with_rule(BangRule);
    let extension = cfg().with_rule(BangRule);

    let merged = base.merge(extension);
    assert_eq!(merged.rules().len(), 1);
}

#[test]
fn merge_appends_new_rules() {
    fn custom_fn(
        _: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
        _: &LexerConfig<TestKind, DefaultLexError>,
    ) -> Option<Result<Token<TestKind>, DefaultLexError>> {
        None
    }

    let base = cfg().with_rule(BangRule);
    let extension = cfg().with_rule(FnRule::new("custom", custom_fn));

    let merged = base.merge(extension);
    assert_eq!(merged.rules().len(), 2);
    assert_eq!(merged.rules()[1].name(), "custom");
}

#[test]
fn custom_stateful_rule() {
    let config = cfg().with_rule(BangRule);
    assert_eq!(config.rules().len(), 1);
    assert_eq!(config.rules()[0].name(), "bang");
}
