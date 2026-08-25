use crate::document_vocabulary::DocumentVocabulary;

use crate::dialect::dialect_stub::StubDialect;

impl DocumentVocabulary for StubDialect {
    fn fields_key(&self) -> &str {
        "$"
    }

    fn expr_key(&self) -> &str {
        "expr"
    }

    fn arg_key(&self) -> &str {
        "arg"
    }

    fn merge_key(&self) -> &str {
        "merge"
    }

    fn inheritable_key(&self) -> &str {
        "inheritable"
    }

    fn mutable_key(&self) -> &str {
        "mutable"
    }

    fn params_key(&self) -> &str {
        "params"
    }

    fn return_key(&self) -> &str {
        "return"
    }

    fn body_key(&self) -> &str {
        "body"
    }
}
