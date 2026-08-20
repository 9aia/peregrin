use crate::Context;
use crate::Error;
use crate::Host;
use ac_core::Expr;
use ac_runtime::RuntimeValue;

type RuntimeExpr = Expr<RuntimeValue>;

/// The language virtual machine.
///
/// The VM evaluates expressions by walking the expression tree.
/// Values are returned as-is, calls are dispatched to host-provided
/// functions via the [`Host`] trait.
///
/// # Type parameters
///
/// - `H` — the host type, which must implement [`Host`].
///
/// # Architecture
///
/// ```text
///               VM
///                │
///                │ trait
///                ▼
///              Host
///                │
///        ┌───────┼────────┐
///        ▼       ▼        ▼
///       now()   env()    custom()
/// ```
///
/// The VM doesn't know what any of those functions mean.
pub struct Vm<H: Host> {
    host: H,
}

impl<H: Host> Vm<H> {
    /// Creates a new VM with the given host.
    pub fn new(host: H) -> Self {
        Vm { host }
    }

    /// Returns a reference to the host.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Evaluates an expression in the given context.
    ///
    /// This is the core evaluation loop. It recursively evaluates
    /// sub-expressions, dispatches calls to host functions, and
    /// returns the resulting [`RuntimeValue`].
    pub fn eval(&mut self, expr: &RuntimeExpr, ctx: &mut Context) -> Result<RuntimeValue, Error> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),

            Expr::Call { name, args } => {
                let evaluated_args = args
                    .iter()
                    .map(|arg| self.eval(arg, ctx))
                    .collect::<Result<Vec<RuntimeValue>, Error>>()?;

                let function = self
                    .host
                    .function(name)
                    .ok_or_else(|| Error::UnknownFunction(name.clone()))?;

                function.call(&evaluated_args, ctx)
            }
        }
    }
}
