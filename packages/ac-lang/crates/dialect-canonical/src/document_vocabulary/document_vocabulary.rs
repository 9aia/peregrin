use ac_dialect::DocumentVocabulary;

use crate::AcDialect;

impl DocumentVocabulary for AcDialect {
    fn fields_key(&self) -> &str {
        "fields"
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
