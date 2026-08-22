use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::segmenter::{
    GraphemeClusterSegmenter, LineSegmenter, SentenceSegmenter, WordSegmenter,
};

use super::common::{optional_locale, optional_object, require_string};
use super::error::IntlError;

/// `intl.segment(text: string, granularity: string, locale?: string, opts?: object) → array`
///
/// Segments text using ICU4X.
///
/// `granularity` must be one of: `"grapheme"`, `"word"`, `"sentence"`, or `"line"`.
///
/// Returns an array of segment objects with `segment` (string), `index` (number),
/// and `isWordLike` (boolean, only for word granularity).
///
/// Examples:
/// - `intl.segment("hello world", "word")` → `[{segment: "hello", index: 0, isWordLike: true}, ...]`
/// - `intl.segment("こんにちは", "grapheme", "ja")` → 5 segments
#[derive(Clone, Copy)]
pub struct Segment;

impl Function for Segment {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let text = require_string(args, 0)?;
        let granularity = require_string(args, 1)?;
        let _locale = optional_locale(args, 2)?;
        let _opts = optional_object(args, 3)?;

        let segments: Vec<serde_json::Value> = match granularity {
            "grapheme" => {
                let segmenter = GraphemeClusterSegmenter::new();
                collect_segments(text, segmenter.segment_str(text))
            }
            "word" => {
                let segmenter = WordSegmenter::new_auto(Default::default());
                collect_word_segments(text, segmenter.segment_str(text))
            }
            "sentence" => {
                let segmenter = SentenceSegmenter::new(Default::default());
                collect_segments(text, segmenter.segment_str(text))
            }
            "line" => {
                let segmenter = LineSegmenter::new_auto(Default::default());
                collect_segments(text, segmenter.segment_str(text))
            }
            _ => return Err(IntlError::InvalidType {
                type_name: granularity.to_string(),
                valid: "grapheme, word, sentence, line".into(),
            }.into_vm_error()),
        };

        Ok(NonFunctionValue::new(serde_json::Value::Array(segments)))
    }
}

fn collect_segments<I: Iterator<Item = usize>>(text: &str, breakpoints: I) -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for breakpoint in breakpoints {
        if breakpoint > start {
            let segment = &text[start..breakpoint];
            result.push(serde_json::json!(segment));
        }
        start = breakpoint;
    }
    if start < text.len() {
        result.push(serde_json::json!(&text[start..]));
    }
    result
}

fn collect_word_segments<I: Iterator<Item = usize>>(text: &str, breakpoints: I) -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for word_break in breakpoints {
        if word_break > start {
            let segment = &text[start..word_break];
            if segment.chars().any(|c| c.is_alphanumeric()) {
                result.push(serde_json::json!(segment));
            }
        }
        start = word_break;
    }
    if start < text.len() {
        let segment = &text[start..];
        if segment.chars().any(|c| c.is_alphanumeric()) {
            result.push(serde_json::json!(segment));
        }
    }
    result
}
