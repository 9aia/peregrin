mod basic;
mod case;
mod common;
mod search;
mod transform;

pub use basic::{
    Concat, Equals, IsEmpty, Length, Lower, Reverse, Trim, TrimEnd, TrimMatches, TrimStart, Upper,
};
pub use case::{CamelCase, Capitalize, KebabCase, PascalCase, SnakeCase, TitleCase};
pub use search::{Contains, Count, EndsWith, Find, RFind, StartsWith};
pub use transform::{
    CharAt, Join, Lines, PadLeft, PadRight, Repeat, Replace, ReplaceFirst, Slice, Split, SplitAt,
    StripPrefix, StripSuffix,
};

use ac_runtime_core::Module;
use ac_vm::Function;

/// The `str` module — string utility functions.
///
/// Provides functions callable as `str.upper(s)`, `str.split(s, sep)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct StrModule;

impl Module for StrModule {
    fn name(&self) -> &str {
        "str"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            // basic
            "upper" => Some(&Upper),
            "lower" => Some(&Lower),
            "trim" => Some(&Trim),
            "trim_start" => Some(&TrimStart),
            "trim_end" => Some(&TrimEnd),
            "trim_matches" => Some(&TrimMatches),
            "reverse" => Some(&Reverse),
            "length" => Some(&Length),
            "concat" => Some(&Concat),
            "is_empty" => Some(&IsEmpty),
            "equals" => Some(&Equals),
            // search
            "contains" => Some(&Contains),
            "starts_with" => Some(&StartsWith),
            "ends_with" => Some(&EndsWith),
            "count" => Some(&Count),
            "find" => Some(&Find),
            "rfind" => Some(&RFind),
            // transform
            "split" => Some(&Split),
            "join" => Some(&Join),
            "replace" => Some(&Replace),
            "replace_first" => Some(&ReplaceFirst),
            "repeat" => Some(&Repeat),
            "char_at" => Some(&CharAt),
            "slice" => Some(&Slice),
            "pad_left" => Some(&PadLeft),
            "pad_right" => Some(&PadRight),
            "strip_prefix" => Some(&StripPrefix),
            "strip_suffix" => Some(&StripSuffix),
            "lines" => Some(&Lines),
            "split_at" => Some(&SplitAt),
            // case
            "capitalize" => Some(&Capitalize),
            "title_case" => Some(&TitleCase),
            "snake_case" => Some(&SnakeCase),
            "camel_case" => Some(&CamelCase),
            "pascal_case" => Some(&PascalCase),
            "kebab_case" => Some(&KebabCase),
            _ => None,
        }
    }
}

#[cfg(test)]
mod basic_tests;
#[cfg(test)]
mod case_tests;
#[cfg(test)]
mod search_tests;
#[cfg(test)]
mod transform_tests;
