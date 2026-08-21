use crate::Context;
use crate::Error;
use crate::Host;
use ac_ast::Expr;
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

    /// Evaluates an expression asynchronously.
    ///
    /// Like [`eval`](Vm::eval), but supports async host functions.
    /// The VM checks [`Host::function`] first (sync), then falls back
    /// to [`Host::async_function`] (async). This means a host can
    /// provide both sync and async functions, and the VM will use
    /// whichever is available.
    ///
    /// # Yielding
    ///
    /// Async host functions can yield control to the executor by
    /// awaiting a pending future. The entire evaluation suspends
    /// until the future is ready.
    ///
    /// To explicitly yield a value to the caller, a host function
    /// returns `Err(Error::Yield(value))`. The caller receives
    /// `Err(Error::Yield(value))` and can inspect the yielded value.
    pub async fn eval_async(
        &mut self,
        expr: &RuntimeExpr,
        ctx: &mut Context,
    ) -> Result<RuntimeValue, Error> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),

            Expr::Call { name, args } => {
                let mut evaluated_args = Vec::with_capacity(args.len());
                for arg in args {
                    let value = Box::pin(self.eval_async(arg, ctx)).await?;
                    evaluated_args.push(value);
                }

                if let Some(function) = self.host.function(name) {
                    return function.call(&evaluated_args, ctx);
                }

                if let Some(async_function) = self.host.async_function(name) {
                    return async_function.call(&evaluated_args, ctx).await;
                }

                Err(Error::UnknownFunction(name.clone()))
            }
        }
    }
}
