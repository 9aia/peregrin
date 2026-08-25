#![no_main]

use handl_lexer::Lexer;
use handl_lexer_canonical::CanonicalLexer;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    // Should never panic — only return Err for unrecoverable input.
    let _ = CanonicalLexer::default().lex(data);
});
