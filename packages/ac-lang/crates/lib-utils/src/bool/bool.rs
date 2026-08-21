use ac_ast::Expr;
use ac_runtime::NonFunctionValue;
use ac_runtime_core::Module;
use ac_vm::{Context, Error, Function, LazyFunction};

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

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "and" => Some(&And),
            "or" => Some(&Or),
            "not" => Some(&Not),
            "xor" => Some(&Xor),
            "nand" => Some(&Nand),
            "nor" => Some(&Nor),
            "if" => Some(&If),
            "eq" => Some(&Eq),
            "from" => Some(&From),
            _ => None,
        }
    }

    fn lazy_function(&self, name: &str) -> Option<&dyn LazyFunction> {
        match name {
            "if" => Some(&IfLazy),
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
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
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
        impl Function for $struct_name {
            fn call(
                &self,
                args: &[NonFunctionValue],
                _ctx: &mut Context,
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
bool_binary_fn!(If, |a: bool, b: bool| !a || b);
bool_binary_fn!(Eq, |a: bool, b: bool| a == b);

// ── lazy functions (short-circuit evaluation) ──────────────────────

/// Lazy `if(cond, then, else)` — only evaluates the chosen branch.
#[derive(Clone, Copy)]
pub struct IfLazy;

impl LazyFunction for IfLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut Context,
        evaluate: &mut dyn FnMut(&Expr<NonFunctionValue>, &mut Context) -> Result<NonFunctionValue, Error>,
    ) -> Result<NonFunctionValue, Error> {
        if args.len() < 3 {
            return Err(Error::ArgumentCount {
                expected: 3,
                got: args.len(),
            });
        }
        let cond = evaluate(&args[0], ctx)?;
        let cond = cond.as_bool().ok_or(Error::TypeMismatch {
            expected: "boolean",
            got: "non-boolean",
        })?;
        if cond {
            evaluate(&args[1], ctx)
        } else {
            evaluate(&args[2], ctx)
        }
    }
}

/// Lazy `and(a, b)` — stops at the first `false`.
#[derive(Clone, Copy)]
pub struct AndLazy;

impl LazyFunction for AndLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut Context,
        evaluate: &mut dyn FnMut(&Expr<NonFunctionValue>, &mut Context) -> Result<NonFunctionValue, Error>,
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

impl LazyFunction for OrLazy {
    fn call(
        &self,
        args: &[Expr<NonFunctionValue>],
        ctx: &mut Context,
        evaluate: &mut dyn FnMut(&Expr<NonFunctionValue>, &mut Context) -> Result<NonFunctionValue, Error>,
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

#[derive(Clone, Copy)]
pub struct From;
impl Function for From {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut Context) -> Result<NonFunctionValue, Error> {
        let arg = args.first().ok_or(Error::ArgumentCount {
            expected: 1,
            got: 0,
        })?;
        if let Some(b) = arg.as_bool() {
            return Ok(NonFunctionValue::new(b));
        }
        if let Some(n) = arg.as_number() {
            return Ok(NonFunctionValue::new(n != 0.0));
        }
        if let Some(s) = arg.as_str() {
            return Ok(NonFunctionValue::new(!s.is_empty()));
        }
        if arg.is_null() {
            return Ok(NonFunctionValue::new(false));
        }
        if let Some(arr) = arg.as_array() {
            return Ok(NonFunctionValue::new(!arr.is_empty()));
        }
        if let Some(obj) = arg.as_object() {
            return Ok(NonFunctionValue::new(!obj.is_empty()));
        }
        Ok(NonFunctionValue::new(true))
    }
}
