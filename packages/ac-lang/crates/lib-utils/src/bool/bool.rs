use ac_ast_canonical::Expr;
use ac_runtime_values::NonFunctionValue;

use ac_runtime::{Context, Function, LazyFunction};
use ac_runtime_values::Error;

use ac_runtime_modules::Module;

/// The `bool` module — boolean utility functions.
///
/// Provides functions callable as `bool.and(a, b)`, `bool.or(a, b)`,
/// etc. in AC expressions.
#[derive(Clone, Copy)]
pub struct BoolModule;

impl Module for BoolModule {
    fn name(&self) -> &str {
        "bool"
    }

    fn function(&self, name: &str) -> Option<&dyn Function<NonFunctionValue>> {
        match name {
            "and" => Some(&And),
            "or" => Some(&Or),
            "not" => Some(&Not),
            "xor" => Some(&Xor),
            "nand" => Some(&Nand),
            "nor" => Some(&Nor),
            "implies" => Some(&Implies),
            "eq" => Some(&Eq),
            _ => None,
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction<NonFunctionValue>> {
        match name {
            "and" => Some(&AndLazy),
            "or" => Some(&OrLazy),
            _ => None,
        }
    }
}

fn require_bool(arg: &NonFunctionValue) -> Result<bool, Error> {
    arg.as_bool().ok_or(Error::TypeMismatch {
        expected: "boolean",
        got: "non-boolean",
    })
}

fn require_one_bool(args: &[NonFunctionValue]) -> Result<bool, Error> {
    let arg = args.first().ok_or(Error::ArgumentCount {
        expected: 1,
        got: 0,
    })?;
    require_bool(arg)
}

fn require_two_bools(args: &[NonFunctionValue]) -> Result<(bool, bool), Error> {
    if args.len() < 2 {
        return Err(Error::ArgumentCount {
            expected: 2,
            got: args.len(),
        });
    }
    let a = require_bool(&args[0])?;
    let b = require_bool(&args[1])?;
    Ok((a, b))
}

fn to_bool(value: bool) -> NonFunctionValue {
    NonFunctionValue::new(value)
}

macro_rules! bool_unary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function<NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error> {
                let input = require_one_bool(args)?;
                Ok(to_bool($body(input)))
            }
        }
    };
}

macro_rules! bool_binary_fn {
    ($struct_name:ident, $body:expr) => {
        #[derive(Clone, Copy)]
        pub struct $struct_name;
        impl Function<NonFunctionValue> for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error> {
                let (a, b) = require_two_bools(args)?;
                Ok(to_bool($body(a, b)))
            }
        }
    };
}

bool_unary_fn!(Not, |a: bool| !a);
bool_binary_fn!(And, |a: bool, b: bool| a && b);
bool_binary_fn!(Or, |a: bool, b: bool| a || b);
bool_binary_fn!(Xor, |a: bool, b: bool| a ^ b);
bool_binary_fn!(Nand, |a: bool, b: bool| !(a && b));
bool_binary_fn!(Nor, |a: bool, b: bool| !(a || b));
bool_binary_fn!(Implies, |a: bool, b: bool| !a || b);
bool_binary_fn!(Eq, |a: bool, b: bool| a == b);

// ── lazy functions (short-circuit evaluation) ──────────────────────

/// Lazy `and(a, b)` — stops at the first `false`.
#[derive(Clone, Copy)]
pub struct AndLazy;

impl LazyFunction<NonFunctionValue> for AndLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut dyn Context<NonFunctionValue>,
        evaluate: &mut dyn FnMut(
            &Expr<NonFunctionValue>,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let a = evaluate(&args[0], ctx)?;
        let a = a.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        if !a {
            return Ok(NonFunctionValue::new(false));
        }
        let b = evaluate(&args[1], ctx)?;
        let b = b.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        Ok(NonFunctionValue::new(b))
    }
}

/// Lazy `or(a, b)` — stops at the first `true`.
#[derive(Clone, Copy)]
pub struct OrLazy;

impl LazyFunction<NonFunctionValue> for OrLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut dyn Context<NonFunctionValue>,
        evaluate: &mut dyn FnMut(
            &Expr<NonFunctionValue>,
            &mut dyn Context<NonFunctionValue>,
        ) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 2 {
            return Err(Error::ArgumentCount {
                expected: 2,
                got: args.len(),
            });
        }
        let a = evaluate(&args[0], ctx)?;
        let a = a.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        if a {
            return Ok(NonFunctionValue::new(true));
        }
        let b = evaluate(&args[1], ctx)?;
        let b = b.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        Ok(NonFunctionValue::new(b))
    }
}
