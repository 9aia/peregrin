use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use ac_runtime::{
    AsyncEval, CanonicalContext, Error, Eval, Expr, NonFunctionValue, VirtualMachine,
};
use serde_json::Value as JsonValue;

/// Builds a canonical VM context from small, composable decisions.
///
/// You can chain variable namespaces, resource limits, and eval
/// callbacks without touching `CanonicalContext` directly.
#[derive(Default)]
pub struct ContextBuilder {
    namespaces: HashMap<String, JsonValue>,
    max_depth: Option<usize>,
    max_calls: Option<usize>,
    eval: Option<Rc<dyn Eval>>,
    async_eval: Option<Rc<dyn AsyncEval>>,
}

struct EvalClone(Rc<dyn Eval>);

impl EvalClone {
    fn new(inner: Rc<dyn Eval>) -> Self {
        EvalClone(inner)
    }
}

impl Eval for EvalClone {
    fn eval_expr(
        &self,
        expr: &str,
        vars: &[(String, NonFunctionValue)],
    ) -> Result<NonFunctionValue, Error> {
        self.0.eval_expr(expr, vars)
    }
}

struct AsyncEvalClone(Rc<dyn AsyncEval>);

impl AsyncEvalClone {
    fn new(inner: Rc<dyn AsyncEval>) -> Self {
        AsyncEvalClone(inner)
    }
}

impl AsyncEval for AsyncEvalClone {
    fn eval_expr_async<'a>(
        &'a self,
        expr: &'a str,
        vars: &'a [(String, NonFunctionValue)],
    ) -> Pin<Box<dyn Future<Output = Result<NonFunctionValue, Error>> + 'a>> {
        AsyncEval::eval_expr_async(&*self.0, expr, vars)
    }
}

impl ContextBuilder {
    /// Creates a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single variable to the top-level namespace.
    pub fn with_namespace(mut self, name: impl Into<String>, value: NonFunctionValue) -> Self {
        self.namespaces.insert(name.into(), value.inner().clone());
        self
    }

    /// Adds many top-level variables at once.
    pub fn with_namespaces<I, K>(mut self, namespaces: I) -> Self
    where
        I: IntoIterator<Item = (K, NonFunctionValue)>,
        K: Into<String>,
    {
        for (name, value) in namespaces {
            self.namespaces.insert(name.into(), value.inner().clone());
        }
        self
    }

    /// Sets the maximum scope depth.
    pub fn with_max_depth(mut self, max: usize) -> Self {
        self.max_depth = Some(max);
        self
    }

    /// Sets the maximum number of function calls.
    pub fn with_max_calls(mut self, max: usize) -> Self {
        self.max_calls = Some(max);
        self
    }

    /// Sets the synchronous expression-evaluation callback.
    pub fn with_eval(mut self, eval: impl Eval + 'static) -> Self {
        self.eval = Some(Rc::new(eval) as Rc<dyn Eval>);
        self
    }

    /// Sets the asynchronous expression-evaluation callback.
    pub fn with_async_eval(mut self, eval: impl AsyncEval + 'static) -> Self {
        self.async_eval = Some(Rc::new(eval) as Rc<dyn AsyncEval>);
        self
    }

    /// Builds the configured `CanonicalContext`.
    pub fn build(&self) -> CanonicalContext {
        let mut ctx = CanonicalContext::from_namespaces(&self.namespaces);

        if let Some(max) = self.max_depth {
            ctx.set_max_depth(max);
        }
        if let Some(max) = self.max_calls {
            ctx.set_max_calls(max);
        }
        if let Some(eval) = &self.eval {
            ctx.set_eval(Box::new(EvalClone::new(eval.clone())));
        }
        if let Some(eval) = &self.async_eval {
            ctx.set_async_eval(Box::new(AsyncEvalClone::new(eval.clone())));
        }

        ctx
    }
}

/// A reusable, pre-configured VM instance.
///
/// `Machine` is generic over any [`VirtualMachine`], so it works with
/// the canonical tree-walking VM or any future alternative implementation.
/// The context is built fresh on each evaluation from a [`ContextBuilder`],
/// keeping the VM stateless and concurrency-friendly.
pub struct Machine<V: VirtualMachine> {
    vm: V,
    context: ContextBuilder,
}

impl<V: VirtualMachine> Machine<V> {
    /// Creates a machine from a virtual-machine implementation.
    pub fn new(vm: V) -> Self {
        Machine {
            vm,
            context: ContextBuilder::new(),
        }
    }

    /// Configures the context builder for every evaluation.
    pub fn with_context(mut self, f: impl FnOnce(ContextBuilder) -> ContextBuilder) -> Self {
        self.context = f(self.context);
        self
    }

    /// Sets a fresh context builder.
    pub fn set_context(mut self, context: ContextBuilder) -> Self {
        self.context = context;
        self
    }

    /// Evaluates an expression synchronously.
    pub fn eval(&self, expr: &Expr<NonFunctionValue>) -> Result<NonFunctionValue, Error> {
        let mut ctx = self.context.build();
        self.vm.eval(expr, &mut ctx)
    }

    /// Evaluates an expression asynchronously.
    pub async fn eval_async(
        &self,
        expr: &Expr<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error> {
        let mut ctx = self.context.build();
        self.vm.eval_async(expr, &mut ctx).await
    }
}
