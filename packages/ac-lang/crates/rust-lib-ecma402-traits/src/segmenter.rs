/// Segmenter granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Granularity {
    /// Grapheme cluster boundaries.
    Grapheme,
    /// Word boundaries.
    Word,
    /// Sentence boundaries.
    Sentence,
    /// Line break boundaries.
    Line,
}

/// Line-break strictness.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LineBreakStyle {
    /// Strict line breaking.
    #[default]
    Strict,
    /// Normal line breaking.
    Normal,
    /// Loose line breaking.
    Loose,
    /// Break anywhere, disregarding any prohibition against line breaks.
    Anywhere,
}

/// Line-break word handling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LineBreakWord {
    /// Customary word-breaking rules.
    #[default]
    Normal,
    /// Breaking is allowed within "words".
    BreakAll,
    /// Breaking is forbidden within "words".
    KeepAll,
}

/// Options used to construct a `Segmenter` instance.
#[derive(Debug, Clone)]
pub struct SegmenterOptions {
    /// Desired segmentation level.
    pub granularity: Granularity,
    /// Line break style, relevant when `granularity` is `Line`.
    pub line_break_style: LineBreakStyle,
    /// Line break word handling, relevant when `granularity` is `Line`.
    pub line_break_word: LineBreakWord,
}

/// A single segment.
#[derive(Debug, Clone)]
pub struct Segment {
    /// The text of the segment.
    pub text: String,
    /// Byte index of the segment in the original string.
    pub index: usize,
    /// `true` if the segment is word-like, when known.
    pub is_word_like: Option<bool>,
}

/// ECMA-402 `Intl.Segmenter` behavior, backend-agnostic.
pub trait Segmenter {
    /// Error type returned when segmentation fails.
    type Error;

    /// Segment `text` into a sequence of `Segment`s.
    fn segment(&self, text: &str) -> Result<Vec<Segment>, Self::Error>;
}