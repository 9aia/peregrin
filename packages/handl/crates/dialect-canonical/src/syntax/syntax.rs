use handl_dialect::Syntax;

use crate::HandlDialect;

impl Syntax for HandlDialect {
    fn interpolation_prefix(&self) -> &str {
        "${"
    }

    fn interpolation_suffix(&self) -> &str {
        "}"
    }

    fn path_separator(&self) -> char {
        '.'
    }

    fn call_open(&self) -> char {
        '('
    }

    fn call_close(&self) -> char {
        ')'
    }
}
