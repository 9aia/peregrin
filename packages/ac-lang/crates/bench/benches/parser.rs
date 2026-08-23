//! Parser benchmarks.
//!
//! Measures the canonical AC parser on a representative expression.

use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;
use ac_parser_canonical::CanonicalParser;
use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};

const SAMPLE: &str = r#"handle foo as bar(handle.baz(1, -2.5, "hello"), { a: true, b: null, c: [1, 2, 3] }) // line comment
/* block comment */
qux.handle.nested.call(123.456)"#;

fn bench_canonical_parse(c: &mut Criterion) {
    let lexer = CanonicalLexer::default();
    let tokens = lexer.lex(SAMPLE).unwrap();
    c.bench_function("parser/canonical_parse", |b| {
        b.iter_batched(
            || tokens.clone(),
            |tokens| CanonicalParser.parse(black_box(tokens)).unwrap(),
            BatchSize::SmallInput,
        );
    });
}

fn bench_canonical_lex_and_parse(c: &mut Criterion) {
    let lexer = CanonicalLexer::default();
    c.bench_function("parser/canonical_lex_and_parse", |b| {
        b.iter(|| {
            let tokens = lexer.lex(black_box(SAMPLE)).unwrap();
            CanonicalParser.parse(black_box(tokens)).unwrap()
        });
    });
}

criterion_group!(
    parser_benches,
    bench_canonical_parse,
    bench_canonical_lex_and_parse
);
criterion_main!(parser_benches);
