use ecma402_traits::segmenter::{Granularity, LineBreakStyle, Segmenter as _};

use super::{Options, Segmenter};

#[test]
fn grapheme() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Grapheme,
            line_break_style: LineBreakStyle::Strict,
        },
    )
    .unwrap();
    let segments = segmenter.segment("abc").unwrap();
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].text, "a");
    assert_eq!(segments[0].index, 0);
    assert_eq!(segments[0].is_word_like, None);
    assert_eq!(segments[1].text, "b");
    assert_eq!(segments[1].index, 1);
    assert_eq!(segments[1].is_word_like, None);
    assert_eq!(segments[2].text, "c");
    assert_eq!(segments[2].index, 2);
    assert_eq!(segments[2].is_word_like, None);
}

#[test]
fn word() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Word,
            line_break_style: LineBreakStyle::Strict,
        },
    )
    .unwrap();
    let segments = segmenter.segment("Hello, world!").unwrap();

    let hello = segments.iter().find(|s| s.text == "Hello");
    assert!(hello.is_some());
    assert_eq!(hello.unwrap().is_word_like, Some(true));

    assert!(segments.iter().any(|s| s.is_word_like == Some(false)));
}

#[test]
fn sentence() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Sentence,
            line_break_style: LineBreakStyle::Strict,
        },
    )
    .unwrap();
    let segments = segmenter.segment("One. Two!").unwrap();
    assert!(segments.len() >= 2);
    for segment in &segments {
        assert_eq!(segment.is_word_like, None);
    }
}

#[test]
fn line_strict() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Line,
            line_break_style: LineBreakStyle::Strict,
        },
    )
    .unwrap();
    let segments = segmenter.segment("Hello world").unwrap();
    assert!(!segments.is_empty());
    for segment in &segments {
        assert_eq!(segment.is_word_like, None);
    }
}

#[test]
fn line_normal() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Line,
            line_break_style: LineBreakStyle::Normal,
        },
    )
    .unwrap();
    let segments = segmenter.segment("Hello world").unwrap();
    assert!(!segments.is_empty());
    for segment in &segments {
        assert_eq!(segment.is_word_like, None);
    }
}

#[test]
fn line_loose() {
    let segmenter = Segmenter::new(
        "en",
        Options {
            granularity: Granularity::Line,
            line_break_style: LineBreakStyle::Loose,
        },
    )
    .unwrap();
    let segments = segmenter.segment("Hello world").unwrap();
    assert!(!segments.is_empty());
    for segment in &segments {
        assert_eq!(segment.is_word_like, None);
    }
}

#[test]
fn empty_string() {
    let cases = [
        (Granularity::Grapheme, LineBreakStyle::Strict),
        (Granularity::Word, LineBreakStyle::Strict),
        (Granularity::Sentence, LineBreakStyle::Strict),
        (Granularity::Line, LineBreakStyle::Strict),
    ];
    for (granularity, line_break_style) in cases {
        let segmenter = Segmenter::new(
            "en",
            Options {
                granularity,
                line_break_style,
            },
        )
        .unwrap();
        let segments = segmenter.segment("").unwrap();
        assert!(
            segments.is_empty(),
            "{:?} should produce no segments",
            granularity
        );
    }
}
