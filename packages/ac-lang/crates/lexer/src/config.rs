//! Configuration for the lexing engine.
//!
//! [`LexerConfig`] holds three composable pieces:
//!
//! - **Keywords** — identifier text mapped to a `TokenKind` (e.g. `true` → `Bool(true)`)
//! - **Delimiters** — single characters mapped to a `TokenKind` (e.g. `(` → `CallOpen`)
//! - **Rules** — ordered lexing functions that try to produce a token from the current
//!   position in the input
//!
//! Configs can be merged with [`LexerConfig::merge`], enabling composition of dialects.
//! A dialect crate (e.g. `ac-lexer-canonical`) provides a preset function that populates
//! the config with the dialect's keywords, delimiters, and rules.

use crate::{LexError, LexRule, TokenKind};

/// Configuration for the lexing engine.
///
/// Holds keywords, delimiters, and an ordered list of [`LexRule`] trait objects.
/// Start with an empty config ([`LexerConfig::new`]) or a dialect preset,
/// then customize with the builder methods.
///
/// # Examples
///
/// ```ignore
/// use ac_lexer::{TokenKind, LexerConfig, StringRule, DefaultLexError};
///
/// let config = LexerConfig::new(MyFactory)
///     .with_keyword("true", MyKind::Bool(true))
///     .with_delimiter('(', MyKind::CallOpen)
///     .with_rule(StringRule);
/// ```
pub struct LexerConfig<K: TokenKind, E: LexError> {
    keywords: Vec<(String, K)>,
    delimiters: Vec<(char, K)>,
    factory: K::Factory,
    rules: Vec<Box<dyn LexRule<K, E>>>,
}

impl<K: TokenKind, E: LexError> Clone for LexerConfig<K, E> {
    fn clone(&self) -> Self {
        LexerConfig {
            keywords: self.keywords.clone(),
            delimiters: self.delimiters.clone(),
            factory: self.factory.clone(),
            rules: self.rules.iter().map(|r| r.clone_box()).collect(),
        }
    }
}

impl<K: TokenKind, E: LexError> LexerConfig<K, E> {
    /// Creates an empty configuration with the given kind factory.
    pub fn new(factory: K::Factory) -> Self {
        LexerConfig {
            keywords: Vec::new(),
            delimiters: Vec::new(),
            factory,
            rules: Vec::new(),
        }
    }

    /// Returns the kind factory.
    pub fn factory(&self) -> &K::Factory {
        &self.factory
    }

    // --- Keywords ---

    /// Adds or replaces a keyword mapping.
    pub fn with_keyword(mut self, text: &str, kind: K) -> Self {
        if let Some(entry) = self.keywords.iter_mut().find(|(k, _)| k == text) {
            entry.1 = kind;
        } else {
            self.keywords.push((text.into(), kind));
        }
        self
    }

    /// Removes a keyword mapping.
    pub fn without_keyword(mut self, text: &str) -> Self {
        self.keywords.retain(|(k, _)| k != text);
        self
    }

    /// Looks up a keyword by identifier text.
    pub fn lookup_keyword(&self, text: &str) -> Option<&K> {
        self.keywords
            .iter()
            .find(|(k, _)| k == text)
            .map(|(_, v)| v)
    }

    // --- Delimiters ---

    /// Adds or replaces a delimiter mapping.
    pub fn with_delimiter(mut self, ch: char, kind: K) -> Self {
        if let Some(entry) = self.delimiters.iter_mut().find(|(c, _)| *c == ch) {
            entry.1 = kind;
        } else {
            self.delimiters.push((ch, kind));
        }
        self
    }

    /// Removes a delimiter mapping.
    pub fn without_delimiter(mut self, ch: char) -> Self {
        self.delimiters.retain(|(c, _)| *c != ch);
        self
    }

    /// Looks up a delimiter by character.
    pub fn lookup_delimiter(&self, ch: char) -> Option<&K> {
        self.delimiters
            .iter()
            .find(|(c, _)| *c == ch)
            .map(|(_, v)| v)
    }

    // --- Rules ---

    /// Appends a lexing rule to the end of the rule list.
    ///
    /// Rules are tried in order; the first that returns `Some` wins.
    pub fn with_rule(mut self, rule: impl LexRule<K, E> + 'static) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    /// Inserts a lexing rule at the given index, shifting existing rules.
    pub fn insert_rule(mut self, index: usize, rule: impl LexRule<K, E> + 'static) -> Self {
        self.rules
            .insert(index.min(self.rules.len()), Box::new(rule));
        self
    }

    /// Removes all rules with the same name as the given rule.
    pub fn without_rule(mut self, rule: &dyn LexRule<K, E>) -> Self {
        let name = rule.name();
        self.rules.retain(|r| r.name() != name);
        self
    }

    /// Removes all rules with the given name.
    pub fn without_rule_by_name(mut self, name: &str) -> Self {
        self.rules.retain(|r| r.name() != name);
        self
    }

    /// Replaces the entire rule list.
    pub fn with_rules(mut self, rules: Vec<Box<dyn LexRule<K, E>>>) -> Self {
        self.rules = rules;
        self
    }

    /// Returns the ordered list of lexing rules.
    pub fn rules(&self) -> &[Box<dyn LexRule<K, E>>] {
        &self.rules
    }

    // --- Composition ---

    /// Merges another config into this one.
    ///
    /// - **Keywords**: `other`'s keywords are added or replace existing ones.
    /// - **Delimiters**: `other`'s delimiters are added or replace existing ones.
    /// - **Rules**: `other`'s rules are appended (duplicates by name are skipped).
    pub fn merge(mut self, other: LexerConfig<K, E>) -> Self {
        for (k, v) in other.keywords {
            if let Some(entry) = self.keywords.iter_mut().find(|(ek, _)| ek == &k) {
                entry.1 = v;
            } else {
                self.keywords.push((k, v));
            }
        }
        for (c, v) in other.delimiters {
            if let Some(entry) = self.delimiters.iter_mut().find(|(ec, _)| *ec == c) {
                entry.1 = v;
            } else {
                self.delimiters.push((c, v));
            }
        }
        for rule in other.rules {
            let name = rule.name();
            if !self.rules.iter().any(|r| r.name() == name) {
                self.rules.push(rule);
            }
        }
        self
    }
}