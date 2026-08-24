use ac_runtime::{Context, Error, Expr, Host, VirtualMachine};
use ac_runtime_values::NonFunctionValue;

type RuntimeExpr = Expr<NonFunctionValue>;

/// A single, pluggable evaluator for one expression shape or language capability.
pub trait ExprHandler: Send + Sync + 'static {
    /// Returns `true` if this handler should evaluate `expr`.
    fn can_handle(&self, expr: &RuntimeExpr) -> bool;

    /// Evaluates `expr` using the host and the given dispatch closure for sub-expressions.
    fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        host: &dyn Host<NonFunctionValue>,
        dispatch: &mut dyn FnMut(&RuntimeExpr, &mut dyn Context<NonFunctionValue>) -> Result<NonFunctionValue, Error<NonFunctionValue>>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>>;
}

/// A virtual machine assembled from an ordered list of expression handlers.
pub struct LegoVm<H: Host<NonFunctionValue>> {
    host: H,
    handlers: Vec<Box<dyn ExprHandler>>,
}

impl<H: Host<NonFunctionValue>> LegoVm<H> {
    /// Creates a new `LegoVm` with the given host and no handlers.
    pub fn new(host: H) -> Self {
        LegoVm {
            host,
            handlers: Vec::new(),
        }
    }

    /// Adds a handler to the end of the dispatch list.
    pub fn with(mut self, handler: impl ExprHandler) -> Self {
        self.handlers.push(Box::new(handler));
        self
    }

    /// Returns a reference to the host.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Evaluates an expression by dispatching to the first matching handler.
    pub fn eval(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let mut dispatch = |sub: &RuntimeExpr, c: &mut dyn Context<NonFunctionValue>| self.eval(sub, c);

        for handler in &self.handlers {
            if handler.can_handle(expr) {
                return handler.eval(expr, ctx, &self.host, &mut dispatch);
            }
        }

        Err(Error::PrimitiveError(
            "no handler for expression".to_string(),
        ))
    }
}

impl<H: Host<NonFunctionValue>> VirtualMachine<NonFunctionValue> for LegoVm<H> {
    fn eval(&self, expr: &RuntimeExpr, ctx: &mut dyn Context<NonFunctionValue>) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        Self::eval(self, expr, ctx)
    }

    fn eval_async(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> impl std::future::Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> {
        std::future::ready(self.eval(expr, ctx))
    }
}
