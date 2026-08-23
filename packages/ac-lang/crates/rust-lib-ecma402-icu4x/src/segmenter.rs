use std::fmt;

use ecma402_traits::segmenter::{
    Granularity, LineBreakStyle, LineBreakWord, Segment as SegmentOutput,
    Segmenter as SegmenterTrait,
};
use icu::locale::Locale;
use icu::segmenter::{
    GraphemeClusterSegmenter, LineSegmenter, SentenceSegmenter, WordSegmenter,
    options::{LineBreakOptions, LineBreakStrictness, LineBreakWordOption},
};
use icu_provider::DataError;
use icu_provider_blob::BlobDataProvider;

pub use ecma402_traits::segmenter::{Segment, SegmenterOptions as Options};

type BreakFn = Box<dyn Fn(&str) -> Vec<usize>>;

/// ICU4X-backed ECMA-402 `Intl.Segmenter`.
pub struct Segmenter {
    break_fn: BreakFn,
    is_word: bool,
}

/// Errors that can occur when constructing or using a `Segmenter`.
#[derive(Debug)]
pub enum Error {
    InvalidLocale(String),
    Data(DataError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLocale(s) => write!(f, "invalid locale: {s}"),
            Self::Data(e) => write!(f, "data provider error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<DataError> for Error {
    fn from(e: DataError) -> Self {
        Self::Data(e)
    }
}

impl Segmenter {
    fn parse_locale(locale: &str) -> Result<Locale, Error> {
        Locale::try_from_str(locale).map_err(|_| Error::InvalidLocale(locale.to_string()))
    }

    fn line_options(
        style: LineBreakStyle,
        word: LineBreakWord,
        locale: &Locale,
    ) -> LineBreakOptions<'_> {
        let mut opts = LineBreakOptions::default();
        opts.content_locale = Some(&locale.id);
        opts.strictness = Some(match style {
            LineBreakStyle::Strict => LineBreakStrictness::Strict,
            LineBreakStyle::Normal => LineBreakStrictness::Normal,
            LineBreakStyle::Loose => LineBreakStrictness::Loose,
            LineBreakStyle::Anywhere => LineBreakStrictness::Anywhere,
        });
        opts.word_option = Some(match word {
            LineBreakWord::Normal => LineBreakWordOption::Normal,
            LineBreakWord::BreakAll => LineBreakWordOption::BreakAll,
            LineBreakWord::KeepAll => LineBreakWordOption::KeepAll,
        });
        opts
    }

    fn make_segmenter(
        options: &Options,
        locale: &Locale,
        provider: Option<&BlobDataProvider>,
    ) -> Result<(BreakFn, bool), Error> {
        let is_word = options.granularity == Granularity::Word;

        let break_fn: BreakFn = match options.granularity {
            Granularity::Grapheme => match provider {
                Some(p) => {
                    let seg = GraphemeClusterSegmenter::try_new_with_buffer_provider(p)?;
                    Box::new(move |text| seg.as_borrowed().segment_str(text).collect())
                }
                None => {
                    let seg = GraphemeClusterSegmenter::new();
                    Box::new(move |text| seg.segment_str(text).collect())
                }
            },
            Granularity::Word => match provider {
                Some(p) => {
                    let seg =
                        WordSegmenter::try_new_auto_with_buffer_provider(p, Default::default())?;
                    Box::new(move |text| seg.as_borrowed().segment_str(text).collect())
                }
                None => {
                    let seg = WordSegmenter::new_auto(Default::default());
                    Box::new(move |text| seg.segment_str(text).collect())
                }
            },
            Granularity::Sentence => match provider {
                Some(p) => {
                    let seg =
                        SentenceSegmenter::try_new_with_buffer_provider(p, Default::default())?;
                    Box::new(move |text| seg.as_borrowed().segment_str(text).collect())
                }
                None => {
                    let seg = SentenceSegmenter::new(Default::default());
                    Box::new(move |text| seg.segment_str(text).collect())
                }
            },
            Granularity::Line => {
                let line_opts =
                    Self::line_options(options.line_break_style, options.line_break_word, locale);
                match provider {
                    Some(p) => {
                        let seg = LineSegmenter::try_new_auto_with_buffer_provider(p, line_opts)?;
                        Box::new(move |text| seg.as_borrowed().segment_str(text).collect())
                    }
                    None => {
                        let seg = LineSegmenter::new_auto(line_opts);
                        Box::new(move |text| seg.segment_str(text).collect())
                    }
                }
            }
        };

        Ok((break_fn, is_word))
    }

    /// Create a new `Segmenter` for the given BCP 47 locale tag using compiled CLDR data.
    pub fn new(locale: &str, options: Options) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let (break_fn, is_word) = Self::make_segmenter(&options, &locale, None)?;
        Ok(Self { break_fn, is_word })
    }

    /// Create a new `Segmenter` using an ICU4X blob data provider.
    pub fn with_blob(
        locale: &str,
        options: Options,
        provider: &BlobDataProvider,
    ) -> Result<Self, Error> {
        let locale: Locale = Self::parse_locale(locale)?;
        let (break_fn, is_word) = Self::make_segmenter(&options, &locale, Some(provider))?;
        Ok(Self { break_fn, is_word })
    }
}

impl SegmenterTrait for Segmenter {
    type Error = Error;

    fn segment(&self, text: &str) -> Result<Vec<SegmentOutput>, Self::Error> {
        let breakpoints = (self.break_fn)(text);

        let mut result = Vec::new();
        let mut start = 0usize;

        for breakpoint in breakpoints {
            if breakpoint > start {
                let slice = &text[start..breakpoint];
                let is_word_like = if self.is_word {
                    Some(slice.chars().any(|c| c.is_alphanumeric()))
                } else {
                    None
                };
                result.push(SegmentOutput {
                    text: slice.to_string(),
                    index: start,
                    is_word_like,
                });
            }
            start = breakpoint;
        }

        if start < text.len() {
            let slice = &text[start..];
            let is_word_like = if self.is_word {
                Some(slice.chars().any(|c| c.is_alphanumeric()))
            } else {
                None
            };
            result.push(SegmentOutput {
                text: slice.to_string(),
                index: start,
                is_word_like,
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
#[path = "segmenter_tests.rs"]
mod segmenter_tests;
