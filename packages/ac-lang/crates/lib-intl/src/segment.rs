use ac_runtime::NonFunctionValue;
use ac_vm::{Context, Error, Function};
use icu::segmenter::{GraphemeClusterSegmenter, WordSegmenter, SentenceSegmenter};

use super::common::{optional_locale, optional_object, require_string};

/// `intl.segment(text: string, granularity: string, locale?: string, opts?: object) → array`
///
/// Segments text into an array of string segments using ICU4X.
///
/// `granularity` must be one of: `"grapheme"`, `"word"`, `"sentence"`.
///
/// Examples:
/// - `intl.segment("Hello World", "word")` → `["Hello", " ", "World"]`
/// - `intl.segment("Hello", "grapheme")` → `["H", "e", "l", "l", "o"]`
/// - `intl.segment("Hello. World.", "sentence")` → `["Hello. ", "World."]`
#[derive(Clone, Copy)]
pub struct Segment;

impl Function for Segment {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let text = require_string(args, 0)?;
        let granularity = require_string(args, 1)?;
        let _locale = optional_locale(args, 2)?;
        let _opts = optional_object(args, 3)?;

        let segments: Vec<String> = match granularity {
            "grapheme" => {
                let segmenter = GraphemeClusterSegmenter::new();
                let breakpoints: Vec<usize> = segmenter.segment_str(text).collect();
                breakpoints_to_segments(text, &breakpoints)
            }
            "word" => {
                let segmenter = WordSegmenter::new_auto(Default::default());
                let breakpoints: Vec<usize> = segmenter.segment_str(text).collect();
                breakpoints_to_segments(text, &breakpoints)
            }
            "sentence" => {
                let segmenter = SentenceSegmenter::new(Default::default());
                let breakpoints: Vec<usize> = segmenter.segment_str(text).collect();
                breakpoints_to_segments(text, &breakpoints)
            }
            _ => return Err(Error::PrimitiveError(format!("invalid granularity: {granularity}"))),
        };

        let json_segments: Vec<serde_json::Value> = segments.iter().map(|s| serde_json::json!(s)).collect();
        Ok(NonFunctionValue::new(serde_json::Value::Array(json_segments)))
    }
}

fn breakpoints_to_segments(text: &str, breakpoints: &[usize]) -> Vec<String> {
    breakpoints
        .windows(2)
        .filter_map(|w| {
            let start = w[0];
            let end = w[1];
            if end > start {
                Some(text[start..end].to_string())
            } else {
                None
            }
        })
        .collect()
}
