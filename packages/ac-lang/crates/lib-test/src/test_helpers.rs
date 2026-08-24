use ac_runtime::Eval;
use ac_runtime_values::{Error, NonFunctionValue};

/// An [`Eval`] that always returns `true`.
pub struct TrueEval;

impl Eval<NonFunctionValue> for TrueEval {
    fn eval_expr(
        &self,
        _expr: &str,
        _vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        Ok(NonFunctionValue::new(true))
    }
}

/// An [`Eval`] that always fails.
pub struct ErrorEval;

impl Eval<NonFunctionValue> for ErrorEval {
    fn eval_expr(
        &self,
        _expr: &str,
        _vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        Err(Error::PrimitiveError("eval error".to_string()))
    }
}

/// An [`Eval`] that returns the value bound to the first variable.
pub struct VarEval;

impl Eval<NonFunctionValue> for VarEval {
    fn eval_expr(
        &self,
        _expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        Ok(vars
            .first()
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| NonFunctionValue::new(serde_json::Value::Null)))
    }
}
