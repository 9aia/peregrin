use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::segmenter::{
    GraphemeClusterSegmenter, LineSegmenter, SentenceSegmenter, WordSegmenter,
};
use icu::segmenter::options::{LineBreakOptions, LineBreakStrictness, LineBreakWordOption};

use super::common::{make_provider, opt_str, optional_locale, optional_object, require_string};
use super::error::IntlError;
use super::provider::IntlProvider;

/// `intl.segment(text: string, granularity: string, locale?: string, opts?: object) → array`
///
/// Segments text using ICU4X.
///
/// `granularity` must be one of: `"grapheme"`, `"word"`, `"sentence"`, or `"line"`.
///
/// Returns an array of segment objects with `segment` (string), `index` (number),
/// and `isWordLike` (boolean, only for word granularity).
///
/// Options (for `granularity: "line"`):
/// - `lineBreakStyle`: `"strict"`, `"normal"`, `"loose"`, or `"anywhere"` (default: ICU4X default)
/// - `lineBreakWord`: `"normal"`, `"break-all"`, or `"keep-all"` (default: `"normal"`)
///
/// Examples:
/// - `intl.segment("hello world", "word")` → `[{segment: "hello", index: 0, isWordLike: true}, ...]`
/// - `intl.segment("こんにちは", "grapheme", "ja")` → 5 segments
/// - `intl.segment("Hello World", "line", "ja", {lineBreakStyle: "loose"})` → line segments with loose breaking
#[derive(Clone)]
pub struct Segment {
    provider: Option<IntlProvider>,
}

impl Segment {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: IntlProvider) -> Self {
        Self { provider: Some(provider) }
    }
}

impl Default for Segment {
    fn default() -> Self {
        Self::new()
    }
}

impl Function for Segment {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let text = require_string(args, 0)?;
        let granularity = require_string(args, 1)?;
        let locale = optional_locale(args, 2)?;
        let opts = optional_object(args, 3)?;
        let dp = make_provider(&self.provider)?;

        let segments: Vec<serde_json::Value> = match granularity {
            "grapheme" => {
                if let Some(p) = dp.as_buffer_provider() {
                    let segmenter = GraphemeClusterSegmenter::try_new_with_buffer_provider(p)
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                    collect_segments(text, segmenter.as_borrowed().segment_str(text))
                } else {
                    let segmenter = GraphemeClusterSegmenter::new();
                    collect_segments(text, segmenter.segment_str(text))
                }
            }
            "word" => {
                if let Some(p) = dp.as_buffer_provider() {
                    let segmenter = WordSegmenter::try_new_auto_with_buffer_provider(p, Default::default())
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                    collect_word_segments(text, segmenter.as_borrowed().segment_str(text))
                } else {
                    let segmenter = WordSegmenter::new_auto(Default::default());
                    collect_word_segments(text, segmenter.segment_str(text))
                }
            }
            "sentence" => {
                if let Some(p) = dp.as_buffer_provider() {
                    let segmenter = SentenceSegmenter::try_new_with_buffer_provider(p, Default::default())
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                    collect_segments(text, segmenter.as_borrowed().segment_str(text))
                } else {
                    let segmenter = SentenceSegmenter::new(Default::default());
                    collect_segments(text, segmenter.segment_str(text))
                }
            }
            "line" => {
                let mut line_opts = LineBreakOptions::default();
                line_opts.content_locale = Some(&locale.id);
                if let Some(style) = opt_str(opts, "lineBreakStyle") {
                    line_opts.strictness = Some(match style {
                        "strict" => LineBreakStrictness::Strict,
                        "loose" => LineBreakStrictness::Loose,
                        "anywhere" => LineBreakStrictness::Anywhere,
                        _ => LineBreakStrictness::Normal,
                    });
                }
                if let Some(word) = opt_str(opts, "lineBreakWord") {
                    line_opts.word_option = Some(match word {
                        "break-all" => LineBreakWordOption::BreakAll,
                        "keep-all" => LineBreakWordOption::KeepAll,
                        _ => LineBreakWordOption::Normal,
                    });
                }
                if let Some(p) = dp.as_buffer_provider() {
                    let segmenter = LineSegmenter::try_new_auto_with_buffer_provider(p, line_opts)
                        .map_err(|e| IntlError::Formatting { message: e.to_string() }.into_vm_error())?;
                    collect_segments(text, segmenter.as_borrowed().segment_str(text))
                } else {
                    let segmenter = LineSegmenter::new_auto(line_opts);
                    collect_segments(text, segmenter.segment_str(text))
                }
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
            result.push(serde_json::json!({
                "segment": segment,
                "index": start,
            }));
        }
        start = breakpoint;
    }
    if start < text.len() {
        result.push(serde_json::json!({
            "segment": &text[start..],
            "index": start,
        }));
    }
    result
}

fn collect_word_segments<I: Iterator<Item = usize>>(text: &str, breakpoints: I) -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for word_break in breakpoints {
        if word_break > start {
            let segment = &text[start..word_break];
            let is_word_like = segment.chars().any(|c| c.is_alphanumeric());
            result.push(serde_json::json!({
                "segment": segment,
                "index": start,
                "isWordLike": is_word_like,
            }));
        }
        start = word_break;
    }
    if start < text.len() {
        let segment = &text[start..];
        let is_word_like = segment.chars().any(|c| c.is_alphanumeric());
        result.push(serde_json::json!({
            "segment": segment,
            "index": start,
            "isWordLike": is_word_like,
        }));
    }
    result
}
