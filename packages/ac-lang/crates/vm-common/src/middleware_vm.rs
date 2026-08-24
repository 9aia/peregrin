use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ac_runtime::{Context, Error, Expr, VirtualMachine};
use ac_runtime_values::NonFunctionValue;

/// Whether an evaluation is synchronous or asynchronous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalKind {
    /// A synchronous evaluation.
    Sync,
    /// An asynchronous evaluation.
    Async,
}

/// A hook that can observe, short-circuit, or transform VM evaluations.
///
/// Middlewares are `Send + Sync + 'static` so they can travel across
/// threads and be held in `Chain` and `Box<dyn VmMiddleware>`.
pub trait VmMiddleware: Send + Sync + 'static {
    /// Called before the inner VM. Returning `Some(result)` short-circuits
    /// the evaluation; the inner VM is not called.
    fn before_eval(
        &self,
        _kind: EvalKind,
        _expr: &Expr<NonFunctionValue>,
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Option<Result<NonFunctionValue, Error<NonFunctionValue>>> {
        None
    }

    /// Called after the inner VM (or a short-circuited `before_eval`).
    ///
    /// The middleware may inspect or transform the result, including turning
    /// an `Ok` into an `Err` or vice versa.
    fn after_eval(
        &self,
        _kind: EvalKind,
        _expr: &Expr<NonFunctionValue>,
        result: Result<NonFunctionValue, Error<NonFunctionValue>>,
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        result
    }
}

/// A `VirtualMachine` wrapper that applies one or more `VmMiddleware`s.
///
/// `MiddlewareVm` is generic over the inner VM and the middleware, so any
/// combination can be assembled without modifying the VM trait.
pub struct MiddlewareVm<V, M> {
    inner: V,
    middleware: M,
}

impl<V, M> MiddlewareVm<V, M> {
    /// Creates a new wrapper around `inner` with the given `middleware`.
    pub fn new(inner: V, middleware: M) -> Self {
        MiddlewareVm { inner, middleware }
    }

    /// Returns a reference to the inner VM.
    pub fn inner(&self) -> &V {
        &self.inner
    }

    /// Returns a reference to the middleware.
    pub fn middleware(&self) -> &M {
        &self.middleware
    }
}

impl<V: VirtualMachine<NonFunctionValue>, M: VmMiddleware> VirtualMachine<NonFunctionValue>
    for MiddlewareVm<V, M>
{
    fn eval(
        &self,
        expr: &Expr<NonFunctionValue>,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let result = match self.middleware.before_eval(EvalKind::Sync, expr, ctx) {
            Some(cached) => cached,
            None => self.inner.eval(expr, ctx),
        };
        self.middleware
            .after_eval(EvalKind::Sync, expr, result, ctx)
    }

    fn eval_async(
        &self,
        expr: &Expr<NonFunctionValue>,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> impl Future<Output = Result<NonFunctionValue, Error<NonFunctionValue>>> {
        async move {
            let result = match self.middleware.before_eval(EvalKind::Async, expr, ctx) {
                Some(cached) => cached,
                None => self.inner.eval_async(expr, ctx).await,
            };
            self.middleware
                .after_eval(EvalKind::Async, expr, result, ctx)
        }
    }
}

/// A middleware that does nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopMiddleware;

impl VmMiddleware for NoopMiddleware {}

/// A chain of middlewares applied in order.
///
/// `before_eval` runs left-to-right; the first `Some` short-circuits the rest.
/// `after_eval` runs right-to-left so nested middlewares close like a stack
/// and each can transform the result produced by the inner/earlier layers.
pub struct Chain {
    middlewares: Vec<Box<dyn VmMiddleware>>,
}

impl Chain {
    /// Creates an empty chain.
    pub fn new() -> Self {
        Chain {
            middlewares: Vec::new(),
        }
    }

    /// Adds a middleware to the chain.
    pub fn with(mut self, middleware: Box<dyn VmMiddleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Returns the number of middlewares in the chain.
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Returns `true` if the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for Chain {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chain")
            .field("len", &self.middlewares.len())
            .finish()
    }
}

impl VmMiddleware for Chain {
    fn before_eval(
        &self,
        kind: EvalKind,
        expr: &Expr<NonFunctionValue>,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Option<Result<NonFunctionValue, Error<NonFunctionValue>>> {
        for middleware in &self.middlewares {
            if let Some(result) = middleware.before_eval(kind, expr, ctx) {
                return Some(result);
            }
        }
        None
    }

    fn after_eval(
        &self,
        kind: EvalKind,
        expr: &Expr<NonFunctionValue>,
        result: Result<NonFunctionValue, Error<NonFunctionValue>>,
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        let mut result = result;
        for middleware in self.middlewares.iter().rev() {
            result = middleware.after_eval(kind, expr, result, ctx);
        }
        result
    }
}

/// A recorded VM evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceEvent {
    /// Whether the evaluation was sync or async.
    pub kind: EvalKind,
    /// A short summary of the evaluated expression.
    pub summary: String,
    /// The result produced by the wrapped VM.
    pub result: Result<NonFunctionValue, Error<NonFunctionValue>>,
}

fn summarize(expr: &Expr<NonFunctionValue>) -> String {
    match expr {
        Expr::Value(_) => "Value".to_string(),
        Expr::Var(name) => format!("Var({})", name),
        Expr::Assign { target, .. } => format!("Assign({})", target.bound_names().join(",")),
        Expr::Function {
            type_params,
            params,
            ..
        } => {
            let count: usize = params.iter().map(|(p, _)| p.bound_names().len()).sum();
            format!("Function<{}>({})", type_params.join(","), count)
        }
        Expr::Call { name, args, .. } => format!("Call({}, {})", name, args.len()),
        Expr::Apply { args, .. } => format!("Apply({})", args.len()),
        Expr::Index { .. } => "Index".to_string(),
        Expr::FieldAccess { field, .. } => format!("FieldAccess({})", field),
        Expr::Handle { .. } => "Handle".to_string(),
        Expr::Array(elements) => format!("Array({})", elements.len()),
        Expr::Object(entries) => format!("Object({})", entries.len()),
    }
}

/// A middleware that records every evaluation.
#[derive(Clone, Default)]
pub struct Trace {
    events: Arc<Mutex<Vec<TraceEvent>>>,
}

impl Trace {
    /// Creates a new trace recorder.
    pub fn new() -> Self {
        Trace {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the recorded trace events.
    ///
    /// This does not panic if the underlying `Mutex` is poisoned.
    pub fn events(&self) -> Vec<TraceEvent> {
        let guard = self.events.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    /// Clears the recorded trace events.
    ///
    /// This does not panic if the underlying `Mutex` is poisoned.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.events.lock() {
            guard.clear();
        }
    }
}

impl std::fmt::Debug for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let events = self.events();
        f.debug_struct("Trace").field("events", &events).finish()
    }
}

impl VmMiddleware for Trace {
    fn after_eval(
        &self,
        kind: EvalKind,
        expr: &Expr<NonFunctionValue>,
        result: Result<NonFunctionValue, Error<NonFunctionValue>>,
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        if let Ok(mut guard) = self.events.lock() {
            guard.push(TraceEvent {
                kind,
                summary: summarize(expr),
                result: result.clone(),
            });
        }
        result
    }
}

/// A middleware that counts completed evaluations.
#[derive(Debug, Clone, Default)]
pub struct Counter {
    count: Arc<AtomicUsize>,
}

impl Counter {
    /// Creates a new counter.
    pub fn new() -> Self {
        Counter {
            count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the number of evaluations that have completed.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
}

impl VmMiddleware for Counter {
    fn after_eval(
        &self,
        _kind: EvalKind,
        _expr: &Expr<NonFunctionValue>,
        result: Result<NonFunctionValue, Error<NonFunctionValue>>,
        _ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        self.count.fetch_add(1, Ordering::Relaxed);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockHost, NoopHost, fn_function};
    use ac_runtime::{CanonicalContext, Expr, NonFunctionValue};
    use ac_runtime_canonical::CanonicalVm;
    use std::future::Future;
    use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

    fn block_on<F: Future>(future: F) -> F::Output {
        fn noop_clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        fn noop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);

        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
        let mut cx = TaskContext::from_waker(&waker);

        let mut future = std::pin::pin!(future);
        loop {
            if let Poll::Ready(val) = future.as_mut().poll(&mut cx) {
                return val;
            }
            std::thread::yield_now();
        }
    }

    fn inc_host() -> MockHost {
        MockHost::new().with_function(
            "inc",
            fn_function(|args, _| {
                let n = args[0]
                    .inner()
                    .as_f64()
                    .ok_or(Error::PrimitiveError("not a number".to_string()))?
                    + 1.0;
                Ok(NonFunctionValue::new(serde_json::json!(n)))
            }),
        )
    }

    fn inc_expr() -> Expr<NonFunctionValue> {
        Expr::Call {
            name: "inc".to_string(),
            args: vec![Expr::Value(NonFunctionValue::new(serde_json::json!(1.0)))],
            type_args: Vec::new(),
        }
    }

    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    #[test]
    fn middleware_records_sync_trace() {
        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), Trace::new());
        let mut ctx = CanonicalContext::new();
        let result = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(result, NonFunctionValue::new(serde_json::json!(2.0)));
        assert_eq!(vm.middleware().events().len(), 1);
        assert_eq!(vm.middleware().events()[0].kind, EvalKind::Sync);
        assert_eq!(vm.middleware().events()[0].summary, "Call(inc, 1)");
    }

    #[test]
    fn middleware_records_async_trace() {
        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), Trace::new());
        let mut ctx = CanonicalContext::new();
        let result = block_on(vm.eval_async(&inc_expr(), &mut ctx)).unwrap();

        assert_eq!(result, NonFunctionValue::new(serde_json::json!(2.0)));
        assert_eq!(vm.middleware().events().len(), 1);
        assert_eq!(vm.middleware().events()[0].kind, EvalKind::Async);
    }

    #[test]
    fn chain_composes_middlewares() {
        let trace = Trace::new();
        let counter = Counter::new();

        let chain = Chain::new()
            .with(Box::new(trace.clone()))
            .with(Box::new(counter.clone()));

        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), chain);
        let mut ctx = CanonicalContext::new();

        let _ = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(trace.events().len(), 1);
        assert_eq!(counter.count(), 1);
    }

    #[test]
    fn custom_middleware_is_extensible() {
        #[derive(Debug, Clone, Default)]
        struct Log(Arc<Mutex<Vec<String>>>);

        impl Log {
            fn new() -> Self {
                Log(Arc::new(Mutex::new(Vec::new())))
            }
            fn entries(&self) -> Vec<String> {
                let guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
                guard.clone()
            }
        }

        impl VmMiddleware for Log {
            fn before_eval(
                &self,
                kind: EvalKind,
                expr: &Expr<NonFunctionValue>,
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Option<Result<NonFunctionValue, Error<NonFunctionValue>>> {
                if let Ok(mut guard) = self.0.lock() {
                    guard.push(format!("{:?} {}", kind, summarize(expr)));
                }
                None
            }
        }

        let log = Log::new();
        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), log.clone());
        let mut ctx = CanonicalContext::new();

        let _ = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(log.entries(), vec!["Sync Call(inc, 1)".to_string()]);
    }

    #[test]
    fn before_eval_can_short_circuit() {
        #[derive(Debug, Clone, Default)]
        struct AlwaysAnswer;

        impl VmMiddleware for AlwaysAnswer {
            fn before_eval(
                &self,
                _kind: EvalKind,
                _expr: &Expr<NonFunctionValue>,
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Option<Result<NonFunctionValue, Error<NonFunctionValue>>> {
                Some(Ok(NonFunctionValue::new(serde_json::json!(42.0))))
            }
        }

        // Host that would fail; middleware bypasses it.
        let vm = MiddlewareVm::new(CanonicalVm::new(NoopHost), AlwaysAnswer);
        let mut ctx = CanonicalContext::new();
        let result = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(result, NonFunctionValue::new(serde_json::json!(42.0)));
    }

    #[test]
    fn after_eval_can_rewrite_result() {
        #[derive(Debug, Clone, Default)]
        struct AddOne;

        impl VmMiddleware for AddOne {
            fn after_eval(
                &self,
                _kind: EvalKind,
                _expr: &Expr<NonFunctionValue>,
                result: Result<NonFunctionValue, Error<NonFunctionValue>>,
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
                let n = result?
                    .inner()
                    .as_f64()
                    .ok_or(Error::PrimitiveError("not a number".to_string()))?
                    + 1.0;
                Ok(NonFunctionValue::new(serde_json::json!(n)))
            }
        }

        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), AddOne);
        let mut ctx = CanonicalContext::new();
        let result = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(result, NonFunctionValue::new(serde_json::json!(3.0)));
    }

    #[test]
    fn after_eval_can_fail() {
        #[derive(Debug, Clone, Default)]
        struct Reject;

        impl VmMiddleware for Reject {
            fn after_eval(
                &self,
                _kind: EvalKind,
                _expr: &Expr<NonFunctionValue>,
                _result: Result<NonFunctionValue, Error<NonFunctionValue>>,
                _ctx: &mut dyn Context<NonFunctionValue>,
            ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
                Err(Error::PrimitiveError("rejected".to_string()))
            }
        }

        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), Reject);
        let mut ctx = CanonicalContext::new();
        let result = vm.eval(&inc_expr(), &mut ctx);

        assert!(matches!(result, Err(Error::PrimitiveError(_))));
    }

    #[test]
    fn noop_does_not_affect_evaluation() {
        let vm = MiddlewareVm::new(CanonicalVm::new(inc_host()), NoopMiddleware);
        let mut ctx = CanonicalContext::new();
        let result = vm.eval(&inc_expr(), &mut ctx).unwrap();

        assert_eq!(result, NonFunctionValue::new(serde_json::json!(2.0)));
    }

    #[test]
    fn middleware_vm_and_middlewares_are_send_and_sync() {
        assert_send::<MiddlewareVm<CanonicalVm<NoopHost>, Trace>>();
        assert_sync::<MiddlewareVm<CanonicalVm<NoopHost>, Trace>>();
        assert_send::<Chain>();
        assert_sync::<Chain>();
    }
}
