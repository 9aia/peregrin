#![no_main]

use ac_lexer::Lexer;
use ac_lexer_canonical::CanonicalLexer;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Should never panic — only return Err for unrecoverable input.
    let _ = CanonicalLexer::default().lex(data);
});
