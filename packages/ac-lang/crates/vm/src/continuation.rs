//! Serializable continuation for resumable evaluation.
//!
//! The [`Continuation`] type represents a paused evaluation that can
//! be serialized, sent to another machine, and resumed with a
//! different host — true cross-machine resumability without
//! re-evaluating from scratch.
//!
//! ## How it works
//!
//! Instead of using Rust's call stack (which is not serializable),
//! `eval_resumable` maintains an explicit stack of [`Frame`]s. Each
//! frame records:
//!
//! - The function name being called.
//! - Which arguments have been evaluated so far.
//! - Which arguments still need evaluation.
//!
//! When a host function returns `Err(Error::Yield(value))`, the
//! current frame stack is captured into a [`Continuation`]. The
//! caller can serialize it (it's all JSON internally), ship it to
//! another machine, and resume by calling `eval_resumable` again
//! with a fresh [`Vm`] and [`Context`].
//!
//! ## Example
//!
//! ```ignore
//! // Machine A: start evaluation
//! let mut vm = Vm::new(server_host);
//! let result = vm.eval_resumable(&expr, &mut ctx, None);
//! match result {
//!     EvalResult::Yield(cont) => {
//!         // Serialize and send to Machine B
//!         let json = cont.to_json();
//!         send_to_client(json);
//!     }
//!     EvalResult::Done(value) => { /* finished on server */ }
//!     EvalResult::Error(err) => { /* handle error */ }
//! }
//!
//! // Machine B: resume evaluation
//! let cont = Continuation::from_json(&received_json)?;
//! let mut vm = Vm::new(client_host);
//! let result = vm.eval_resumable(&expr, &mut ctx, Some(cont));
//! ```

use ac_ast::Expr;
use ac_runtime::RuntimeValue;

use crate::Context;
use crate::Error;
use crate::Host;

type RuntimeExpr = Expr<RuntimeValue>;

/// A single frame in the evaluation stack.
///
/// Records the state of a `Call` expression that is mid-evaluation:
/// some arguments have been evaluated, others are pending.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The function name being called.
    pub name: String,
    /// Arguments that have already been evaluated.
    pub evaluated: Vec<RuntimeValue>,
    /// Arguments that still need to be evaluated (remaining, in order).
    pub pending: Vec<RuntimeExpr>,
}

/// A serializable continuation of a paused evaluation.
///
/// Contains the frame stack at the point where evaluation was
/// paused. All data is JSON-compatible (function names are strings,
/// values are `serde_json::Value`, expressions are `Expr<RuntimeValue>`).
#[derive(Debug, Clone, PartialEq)]
pub struct Continuation {
    /// The frame stack, bottom to top.
    pub frames: Vec<Frame>,
}

impl Continuation {
    /// Creates an empty continuation (no frames).
    pub fn empty() -> Self {
        Continuation { frames: Vec::new() }
    }

    /// Returns `true` if there are no frames to resume.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Serializes the continuation to a JSON value.
    ///
    /// The format is an array of frame objects, each with:
    /// - `name` — the function name
    /// - `evaluated` — array of already-evaluated argument values
    /// - `pending` — array of remaining expression trees
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::Value::Array(
            self.frames
                .iter()
                .map(|frame| {
                    serde_json::json!({
                        "name": frame.name,
                        "evaluated": frame.evaluated.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>(),
                        "pending": frame.pending.iter().map(expr_to_json).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        )
    }

    /// Deserializes a continuation from a JSON value.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, Error> {
        let arr = value
            .as_array()
            .ok_or_else(|| Error::PrimitiveError("continuation must be a JSON array".to_string()))?;

        let frames = arr
            .iter()
            .map(frame_from_json)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Continuation { frames })
    }
}

fn expr_to_json(expr: &RuntimeExpr) -> serde_json::Value {
    match expr {
        Expr::Value(value) => serde_json::json!({
            "type": "value",
            "value": value.clone().into_inner(),
        }),
        Expr::Call { name, args } => serde_json::json!({
            "type": "call",
            "name": name,
            "args": args.iter().map(expr_to_json).collect::<Vec<_>>(),
        }),
    }
}

fn expr_from_json(value: &serde_json::Value) -> Result<RuntimeExpr, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::PrimitiveError("expression must be a JSON object".to_string()))?;

    match obj.get("type").and_then(|v| v.as_str()) {
        Some("value") => {
            let val = obj
                .get("value")
                .ok_or_else(|| Error::PrimitiveError("value expression missing 'value' field".to_string()))?;
            Ok(Expr::Value(RuntimeValue::new(val.clone())))
        }
        Some("call") => {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::PrimitiveError("call expression missing 'name' field".to_string()))?
                .to_string();
            let args = obj
                .get("args")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::PrimitiveError("call expression missing 'args' array".to_string()))?
                .iter()
                .map(expr_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Call { name, args })
        }
        other => Err(Error::PrimitiveError(format!(
            "unknown expression type: {other:?}"
        ))),
    }
}

fn frame_from_json(value: &serde_json::Value) -> Result<Frame, Error> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::PrimitiveError("frame must be a JSON object".to_string()))?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::PrimitiveError("frame missing 'name' field".to_string()))?
        .to_string();

    let evaluated = obj
        .get("evaluated")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::PrimitiveError("frame missing 'evaluated' array".to_string()))?
        .iter()
        .map(|v| RuntimeValue::new(v.clone()))
        .collect();

    let pending = obj
        .get("pending")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::PrimitiveError("frame missing 'pending' array".to_string()))?
        .iter()
        .map(expr_from_json)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Frame {
        name,
        evaluated,
        pending,
    })
}

/// The result of a resumable evaluation step.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// Evaluation completed successfully.
    Done(RuntimeValue),
    /// Evaluation paused — a host function yielded.
    ///
    /// The [`Continuation`] captures the state needed to resume.
    /// The [`RuntimeValue`] is the value yielded by the host function.
    Yielded {
        value: RuntimeValue,
        continuation: Continuation,
    },
    /// Evaluation failed with an error.
    Error(Error),
}

impl<H: Host> crate::Vm<H> {
    /// Evaluates an expression using an explicit frame stack.
    ///
    /// This is the resumable evaluation mode. Unlike [`eval`](crate::Vm::eval)
    /// and [`eval_async`](crate::Vm::eval_async), the continuation state
    /// is stored as data (a stack of [`Frame`]s), not as a Rust call
    /// stack or `Future`. This means:
    ///
    /// - A paused evaluation can be serialized via
    ///   [`Continuation::to_json`] and sent to another machine.
    /// - Resumption can use a **different host** — e.g. start on a
    ///   server with `fetch`, yield, then resume on a browser with
    ///   a different `fetch` implementation.
    /// - No re-evaluation of already-completed sub-expressions.
    ///
    /// # Arguments
    ///
    /// - `expr` — the expression to evaluate.
    /// - `ctx` — the execution context.
    /// - `resume` — `None` for a fresh evaluation, or a [`Continuation`]
    ///   from a previous `Yielded` result to resume.
    ///
    /// # How yielding works
    ///
    /// When a host function returns `Err(Error::Yield(value))`, the
    /// current frame stack is captured. The caller receives
    /// [`EvalResult::Yielded`] with the yielded value and the
    /// continuation. To resume, call `eval_resumable` again with
    /// the same expression, context, and the continuation.
    ///
    /// The host function that yielded will **not** be called again
    /// on resume — its result is expected to be provided by the
    /// new host (e.g. via a cache). See the `ac-host-cache` crate.
    pub fn eval_resumable(
        &mut self,
        expr: &RuntimeExpr,
        ctx: &mut Context,
        resume: Option<Continuation>,
    ) -> EvalResult {
        if let Some(cont) = resume {
            return self.resume(cont, ctx);
        }

        match self.eval_step(expr, ctx, &mut Vec::new()) {
            StepResult::Done(value) => EvalResult::Done(value),
            StepResult::Yield { value, frames } => EvalResult::Yielded {
                value,
                continuation: Continuation { frames },
            },
            StepResult::Error(err) => EvalResult::Error(err),
        }
    }

    fn resume(&mut self, cont: Continuation, ctx: &mut Context) -> EvalResult {
        let mut frames = cont.frames;

        // The bottom frame (index 0) is the innermost call — the
        // one that yielded. We re-dispatch it on the new host.
        let bottom = match frames.first() {
            Some(f) => f.clone(),
            None => {
                return EvalResult::Error(Error::PrimitiveError(
                    "cannot resume: empty continuation".to_string(),
                ))
            }
        };
        frames.remove(0);

        // Re-dispatch the yielded call on the new host. If the host
        // has a cached result, it returns Ok. If it yields again,
        // we capture a new continuation.
        let result = self.dispatch_call(&bottom.name, &bottom.evaluated, ctx);

        match result {
            Ok(value) => {
                // The call succeeded. Feed its result to the next
                // frame up (the parent that was waiting for it).
                self.unwind(frames, value, ctx)
            }
            Err(Error::Yield(yielded_value)) => {
                // The resumed call yielded again. Re-insert the
                // bottom frame and return the new continuation.
                frames.insert(0, bottom);
                EvalResult::Yielded {
                    value: yielded_value,
                    continuation: Continuation { frames },
                }
            }
            Err(err) => EvalResult::Error(err),
        }
    }

    fn unwind(
        &mut self,
        mut frames: Vec<Frame>,
        value: RuntimeValue,
        ctx: &mut Context,
    ) -> EvalResult {
        if frames.is_empty() {
            return EvalResult::Done(value);
        }

        // Take the next frame from the front (innermost remaining).
        let mut frame = frames.remove(0);
        frame.evaluated.push(value);

        // Use a fresh stack so yield frames don't get corrupted by
        // ancestor frames still in `frames`.
        let mut fresh_stack = Vec::new();
        match self.eval_frame(&mut frame, ctx, &mut fresh_stack) {
            StepResult::Done(result) => self.unwind(frames, result, ctx),
            StepResult::Yield { value, frames: mut yield_frames } => {
                // yield_frames are innermost-to-current. Append the
                // remaining ancestor frames to get the full stack.
                yield_frames.extend(frames);
                EvalResult::Yielded {
                    value,
                    continuation: Continuation { frames: yield_frames },
                }
            }
            StepResult::Error(err) => EvalResult::Error(err),
        }
    }

    fn eval_frame(
        &mut self,
        frame: &mut Frame,
        ctx: &mut Context,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        // Evaluate remaining pending args one by one.
        while !frame.pending.is_empty() {
            let arg = frame.pending.remove(0);
            match self.eval_step(&arg, ctx, stack) {
                StepResult::Done(value) => frame.evaluated.push(value),
                StepResult::Yield { value, mut frames } => {
                    // Push the current frame (with partial progress) onto
                    // the stack, then push the yield frame.
                    frames.push(frame.clone());
                    return StepResult::Yield { value, frames };
                }
                StepResult::Error(err) => return StepResult::Error(err),
            }
        }

        // All args evaluated — dispatch the call.
        match self.dispatch_call(&frame.name, &frame.evaluated, ctx) {
            Ok(value) => StepResult::Done(value),
            Err(Error::Yield(yielded_value)) => {
                // The call itself yielded. Push this frame onto the stack.
                stack.push(frame.clone());
                StepResult::Yield {
                    value: yielded_value,
                    frames: std::mem::take(stack),
                }
            }
            Err(err) => StepResult::Error(err),
        }
    }

    fn eval_step(
        &mut self,
        expr: &RuntimeExpr,
        ctx: &mut Context,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        match expr {
            Expr::Value(value) => StepResult::Done(value.clone()),

            Expr::Call { name, args } => {
                let mut frame = Frame {
                    name: name.clone(),
                    evaluated: Vec::with_capacity(args.len()),
                    pending: args.clone(),
                };
                self.eval_frame(&mut frame, ctx, stack)
            }
        }
    }

    fn dispatch_call(
        &self,
        name: &str,
        args: &[RuntimeValue],
        ctx: &mut Context,
    ) -> Result<RuntimeValue, Error> {
        if let Some(function) = self.host().function(name) {
            return function.call(args, ctx);
        }

        Err(Error::UnknownFunction(name.to_string()))
    }
}

/// Internal result of a single evaluation step.
enum StepResult {
    /// Evaluation completed with a value.
    Done(RuntimeValue),
    /// A host function yielded. The `frames` vector is the frame
    /// stack at the point of yield (bottom to top).
    Yield {
        value: RuntimeValue,
        frames: Vec<Frame>,
    },
    /// An error occurred.
    Error(Error),
}
