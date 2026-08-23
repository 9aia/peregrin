//! Lexer benchmarks.
//!
//! Measures canonical AC expression tokenization in batch and streaming modes.

use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

const SAMPLE: &str = r#"handle foo as bar(handle.baz(1, -2.5, "hello"), { a: true, b: null, c: [1, 2, 3] }) // line comment
/* block comment */
qux.handle.nested.call(123.456)"#;

fn bench_lex_batch(c: &mut Criterion) {
    let lexer = CanonicalLexer::default();
    c.bench_function("lexer/batch", |b| {
        b.iter(|| {
            black_box(lexer.lex(black_box(SAMPLE)).unwrap());
        });
    });
}

fn bench_lex_streaming(c: &mut Criterion) {
    let lexer = CanonicalLexer::default();
    c.bench_function("lexer/streaming", |b| {
        b.iter(|| {
            black_box(lexer.lex_iter(black_box(SAMPLE)).collect::<Vec<_>>());
        });
    });
}

criterion_group!(lexer_benches, bench_lex_batch, bench_lex_streaming);
criterion_main!(lexer_benches);
