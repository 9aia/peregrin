/// CLDR plural categories.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
        }
    }
}

/// A single condition for a plural rule: match on `n` or `n % modulo`
/// against a set of values, optionally with a range constraint.
#[derive(Clone, Copy)]
struct RuleCond {
    modulo: Option<u64>,
    values: &'static [u64],
    range: Option<(u64, u64)>,
}

impl RuleCond {
    fn matches(&self, n: f64) -> bool {
        let int_n = n.abs() as u64;
        let operand = match self.modulo {
            Some(m) => int_n % m,
            None => int_n,
        };
        if self.values.contains(&operand) {
            return true;
        }
        if let Some((lo, hi)) = self.range {
            if operand >= lo && operand <= hi {
                return true;
            }
        }
        false
    }
}

/// CLDR plural rule for a language.
#[derive(Clone, Copy)]
pub struct PluralRule {
    zero: Option<RuleCond>,
    one: Option<RuleCond>,
    two: Option<RuleCond>,
    few: Option<RuleCond>,
    many: Option<RuleCond>,
    // `other` is the fallback
}

impl PluralRule {
    /// English-like: one = n == 1, other for rest.
    pub const fn simple_one() -> Self {
        PluralRule {
            zero: None,
            one: Some(RuleCond { modulo: None, values: &[1], range: None }),
            two: None,
            few: None,
            many: None,
        }
    }

    /// No special rules — everything is `other`.
    pub const fn other_only() -> Self {
        PluralRule {
            zero: None,
            one: None,
            two: None,
            few: None,
            many: None,
        }
    }

    /// Arabic plural rules (full 6-category system).
    pub const fn arabic() -> Self {
        PluralRule {
            zero: Some(RuleCond { modulo: None, values: &[0], range: None }),
            one: Some(RuleCond { modulo: None, values: &[1], range: None }),
            two: Some(RuleCond { modulo: None, values: &[2], range: None }),
            few: Some(RuleCond { modulo: Some(100), values: &[3, 4, 5, 6, 7, 8, 9, 10], range: None }),
            many: Some(RuleCond { modulo: Some(100), values: &[11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99], range: None }),
            // many: n % 100 in 11..99
        }
    }

    /// Russian/Polish-like: one, few, many, other.
    /// few = n % 10 in 2..4 and n % 100 not in 12..14
    /// many = n % 10 == 0 or n % 10 in 5..9 or n % 100 in 11..14
    pub const fn east_slavic() -> Self {
        PluralRule {
            zero: None,
            one: Some(RuleCond { modulo: Some(10), values: &[1], range: None }),
            // Simplified: one = n % 10 == 1 and n % 100 != 11
            // For a static table we approximate; full CLDR would need
            // compound conditions. This covers the common cases.
            two: None,
            few: Some(RuleCond { modulo: Some(10), values: &[2, 3, 4], range: None }),
            many: Some(RuleCond { modulo: Some(10), values: &[0, 5, 6, 7, 8, 9], range: None }),
        }
    }

    /// Select the plural category for a given number.
    pub fn category(&self, n: f64) -> PluralCategory {
        if let Some(ref cond) = self.zero {
            if cond.matches(n) {
                return PluralCategory::Zero;
            }
        }
        if let Some(ref cond) = self.one {
            if cond.matches(n) {
                return PluralCategory::One;
            }
        }
        if let Some(ref cond) = self.two {
            if cond.matches(n) {
                return PluralCategory::Two;
            }
        }
        if let Some(ref cond) = self.few {
            if cond.matches(n) {
                return PluralCategory::Few;
            }
        }
        if let Some(ref cond) = self.many {
            if cond.matches(n) {
                return PluralCategory::Many;
            }
        }
        PluralCategory::Other
    }
}

/// Select the plural category for a locale and number.
pub fn plural_category(locale: &crate::locale::Locale, n: f64) -> PluralCategory {
    locale.data().plural_rule.category(n)
}
