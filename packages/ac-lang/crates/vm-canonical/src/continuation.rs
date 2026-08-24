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
//! let vm = Vm::new(server_host);
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
//! let vm = Vm::new(client_host);
//! let result = vm.eval_resumable(&expr, &mut ctx, Some(cont));
//! ```

use crate::pattern::{define_pattern, pattern_from_json, pattern_to_json};
use crate::vm::{json_kind, qualified_name};
use crate::{Context, Error, Expr, HandlerClause, Host};
use ac_runtime_values::{FunctionValue, NonFunctionValue};
use serde_json::Value as JsonValue;

type RuntimeExpr = Expr<NonFunctionValue>;
type RuntimeHandlerClause = HandlerClause<NonFunctionValue>;

/// A single frame in the evaluation stack.
///
/// Records the state of a `Call` expression that is mid-evaluation:
/// some arguments have been evaluated, others are pending.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The function name being called.
    pub name: String,
    /// Arguments that have already been evaluated.
    pub evaluated: Vec<NonFunctionValue>,
    /// Arguments that still need evaluation (remaining, in order).
    pub pending: Vec<RuntimeExpr>,
    /// Handler clauses active for this frame (from enclosing Handle exprs).
    /// Empty for frames outside any handler. Innermost handler is last.
    pub handlers: Vec<RuntimeHandlerClause>,
}

/// A serializable continuation of a paused evaluation.
///
/// Contains the frame stack at the point where evaluation was
/// paused. All data is JSON-compatible (function names are strings,
/// values are `serde_json::Value`, expressions are `Expr<NonFunctionValue>`).
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
                        "handlers": frame.handlers.iter().map(handler_clause_to_json).collect::<Vec<_>>()})
                })
                .collect(),
        )
    }

    /// Deserializes a continuation from a JSON value.
    pub fn from_json(value: &serde_json::Value) -> Result<Self, Error<NonFunctionValue>> {
        let arr = value.as_array().ok_or_else(|| {
            Error::PrimitiveError("continuation must be a JSON array".to_string())
        })?;

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
            "value": value.clone().into_inner()}),
        Expr::Var(name) => serde_json::json!({
            "type": "var",
            "name": name}),
        Expr::Assign { target, value } => serde_json::json!({
            "type": "assign",
            "target": pattern_to_json(target),
            "value": expr_to_json(value)}),
        Expr::Function {
            type_params,
            params,
            ret,
            body,
        } => serde_json::json!({
            "type": "function",
            "type_params": type_params,
            "params": params.iter().map(|(p, t)| {
                serde_json::json!([pattern_to_json(p), t.as_deref().map(expr_to_json)])
            }).collect::<Vec<_>>(),
            "ret": ret.as_deref().map(expr_to_json),
            "body": body}),
        Expr::Call { name, args, .. } => serde_json::json!({
            "type": "call",
            "name": name,
            "args": args.iter().map(expr_to_json).collect::<Vec<_>>()}),
        Expr::Apply { target, args } => serde_json::json!({
            "type": "apply",
            "target": expr_to_json(target),
            "args": args.iter().map(expr_to_json).collect::<Vec<_>>()}),
        Expr::Index { target, index } => serde_json::json!({
            "type": "index",
            "target": expr_to_json(target),
            "index": expr_to_json(index)}),
        Expr::FieldAccess { target, field } => serde_json::json!({
            "type": "field",
            "target": expr_to_json(target),
            "field": field}),
        Expr::Handle { body, handlers } => serde_json::json!({
            "type": "handle",
            "body": expr_to_json(body),
            "handlers": handlers.iter().map(handler_clause_to_json).collect::<Vec<_>>()}),
        Expr::Array(elements) => serde_json::json!({
            "type": "array",
            "elements": elements.iter().map(expr_to_json).collect::<Vec<_>>()}),
        Expr::Object(entries) => serde_json::json!({
            "type": "object",
            "entries": entries.iter().map(|(k, v)| serde_json::json!([k, expr_to_json(v)])).collect::<Vec<_>>()}),
    }
}

fn expr_from_json(value: &serde_json::Value) -> Result<RuntimeExpr, Error<NonFunctionValue>> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::PrimitiveError("expression must be a JSON object".to_string()))?;

    match obj.get("type").and_then(|v| v.as_str()) {
        Some("value") => {
            let val = obj.get("value").ok_or_else(|| {
                Error::PrimitiveError("value expression missing 'value' field".to_string())
            })?;
            Ok(Expr::Value(NonFunctionValue::new(val.clone())))
        }
        Some("var") => {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::PrimitiveError("var expression missing 'name' field".to_string())
                })?
                .to_string();
            Ok(Expr::Var(name))
        }
        Some("assign") => {
            let target = obj
                .get("target")
                .and_then(pattern_from_json)
                .ok_or_else(|| {
                    Error::PrimitiveError("assign expression missing 'target' field".to_string())
                })?;
            let value = obj
                .get("value")
                .ok_or_else(|| {
                    Error::PrimitiveError("assign expression missing 'value' field".to_string())
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            Ok(Expr::Assign { target, value })
        }
        Some("function") => {
            let type_params = obj
                .get("type_params")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let params = obj
                .get("params")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("function expression missing 'params' array".to_string())
                })?
                .iter()
                .map(|entry| {
                    let arr = entry.as_array().ok_or_else(|| {
                        Error::PrimitiveError(
                            "function param must be a [pattern, schema] array".to_string(),
                        )
                    })?;
                    let pattern = arr.get(0).and_then(pattern_from_json).ok_or_else(|| {
                        Error::PrimitiveError("function param missing pattern".to_string())
                    })?;
                    let ty = arr
                        .get(1)
                        .filter(|v| !v.is_null())
                        .map(expr_from_json)
                        .transpose()?
                        .map(Box::new);
                    Ok((pattern, ty))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let ret = obj
                .get("ret")
                .filter(|v| !v.is_null())
                .map(expr_from_json)
                .transpose()?
                .map(Box::new);

            let body = obj
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::PrimitiveError("function expression missing 'body' string".to_string())
                })?
                .to_string();

            Ok(Expr::Function {
                type_params,
                params,
                ret,
                body,
            })
        }
        Some("call") => {
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::PrimitiveError("call expression missing 'name' field".to_string())
                })?
                .to_string();
            let args = obj
                .get("args")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("call expression missing 'args' array".to_string())
                })?
                .iter()
                .map(expr_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Call {
                name,
                args,
                type_args: Vec::new(),
            })
        }
        Some("apply") => {
            let target = obj
                .get("target")
                .ok_or_else(|| {
                    Error::PrimitiveError("apply expression missing 'target' field".to_string())
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            let args = obj
                .get("args")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("apply expression missing 'args' array".to_string())
                })?
                .iter()
                .map(expr_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Apply { target, args })
        }
        Some("index") => {
            let target = obj
                .get("target")
                .ok_or_else(|| {
                    Error::PrimitiveError("index expression missing 'target' field".to_string())
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            let index = obj
                .get("index")
                .ok_or_else(|| {
                    Error::PrimitiveError("index expression missing 'index' field".to_string())
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            Ok(Expr::Index { target, index })
        }
        Some("field") => {
            let target = obj
                .get("target")
                .ok_or_else(|| {
                    Error::PrimitiveError(
                        "field access expression missing 'target' field".to_string(),
                    )
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            let field = obj
                .get("field")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    Error::PrimitiveError(
                        "field access expression missing 'field' field".to_string(),
                    )
                })?
                .to_string();
            Ok(Expr::FieldAccess { target, field })
        }
        Some("handle") => {
            let body = obj
                .get("body")
                .ok_or_else(|| {
                    Error::PrimitiveError("handle expression missing 'body' field".to_string())
                })
                .and_then(expr_from_json)
                .map(Box::new)?;
            let handlers = obj
                .get("handlers")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("handle expression missing 'handlers' array".to_string())
                })?
                .iter()
                .map(handler_clause_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Handle { body, handlers })
        }
        Some("array") => {
            let elements = obj
                .get("elements")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("array expression missing 'elements' array".to_string())
                })?
                .iter()
                .map(expr_from_json)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Array(elements))
        }
        Some("object") => {
            let entries = obj
                .get("entries")
                .and_then(|v| v.as_array())
                .ok_or_else(|| {
                    Error::PrimitiveError("object expression missing 'entries' array".to_string())
                })?
                .iter()
                .map(|entry| {
                    let arr = entry.as_array().ok_or_else(|| {
                        Error::PrimitiveError(
                            "object entry must be a [key, value] array".to_string(),
                        )
                    })?;
                    let key = arr
                        .get(0)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            Error::PrimitiveError("object entry missing string key".to_string())
                        })?
                        .to_string();
                    let value = expr_from_json(arr.get(1).ok_or_else(|| {
                        Error::PrimitiveError("object entry missing value expression".to_string())
                    })?)?;
                    Ok((key, value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Expr::Object(entries))
        }
        other => Err(Error::PrimitiveError(format!(
            "unknown expression type: {other:?}"
        ))),
    }
}

fn handler_clause_to_json(clause: &RuntimeHandlerClause) -> serde_json::Value {
    serde_json::json!({
        "op": clause.op_name,
        "params": clause.params,
        "resume": clause.continuation_name,
        "body": expr_to_json(&clause.body)})
}

fn handler_clause_from_json(
    value: &serde_json::Value,
) -> Result<RuntimeHandlerClause, Error<NonFunctionValue>> {
    let obj = value
        .as_object()
        .ok_or_else(|| Error::PrimitiveError("handler clause must be a JSON object".to_string()))?;

    let op_name = obj
        .get("op")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::PrimitiveError("handler clause missing 'op' field".to_string()))?
        .to_string();

    let params = obj
        .get("params")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let continuation_name = obj
        .get("resume")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::PrimitiveError("handler clause missing 'resume' field".to_string()))?
        .to_string();

    let body = obj
        .get("body")
        .ok_or_else(|| Error::PrimitiveError("handler clause missing 'body' field".to_string()))
        .and_then(expr_from_json)?;

    Ok(HandlerClause {
        op_name,
        params,
        continuation_name,
        body,
    })
}

fn frame_from_json(value: &serde_json::Value) -> Result<Frame, Error<NonFunctionValue>> {
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
        .map(|v| NonFunctionValue::new(v.clone()))
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
        handlers: obj
            .get("handlers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(handler_clause_from_json)
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default(),
    })
}

/// The result of a resumable evaluation step.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    /// Evaluation completed successfully.
    Done(NonFunctionValue),
    /// Evaluation paused — a host function yielded.
    ///
    /// The [`Continuation`] captures the state needed to resume.
    /// The [`NonFunctionValue`] is the value yielded by the host function.
    Yielded {
        value: NonFunctionValue,
        continuation: Continuation,
    },
    /// Evaluation failed with an error.
    Error(Error<NonFunctionValue>),
}

impl<H: Host<NonFunctionValue>> crate::CanonicalVm<H> {
    /// Evaluates an expression using an explicit frame stack.
    ///
    /// This is the resumable evaluation mode. Unlike [`eval`](crate::CanonicalVm::eval)
    /// and [`eval_async`](crate::CanonicalVm::eval_async), the continuation state
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
    /// new host (e.g. via a cache). See the `cache` capability of `ac-runtime-common`.
    pub fn eval_resumable(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
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

    fn resume(&self, cont: Continuation, ctx: &mut dyn Context<NonFunctionValue>) -> EvalResult {
        let mut frames = cont.frames;

        // The bottom frame (index 0) is the innermost call — the
        // one that yielded. We re-dispatch it on the new host.
        let bottom = match frames.first() {
            Some(f) => f.clone(),
            None => {
                return EvalResult::Error(Error::PrimitiveError(
                    "cannot resume: empty continuation".to_string(),
                ));
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
            Err(Error::Operation { name, args }) => {
                // The resumed operation signaled. Check handler
                // clauses on the bottom frame for a match.
                let matching_clause = bottom.handlers.iter().rev().find(|h| h.op_name == name);

                if let Some(clause) = matching_clause {
                    // Found a matching handler. Bind params and
                    // continuation, then evaluate the handler body.
                    ctx.push_scope();

                    for (i, param) in clause.params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            ctx.define(param, arg.clone());
                        }
                    }

                    // Build continuation from remaining frames (the
                    // parent frames waiting for the result). The
                    // signaling frame (bottom) is NOT included.
                    let cont_value = NonFunctionValue::new(serde_json::json!({
                        "type": "continuation",
                        "frames": frames.iter().map(|f| {
                            serde_json::json!({
                                "name": f.name,
                                "evaluated": f.evaluated.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>(),
                                "pending": f.pending.iter().map(expr_to_json).collect::<Vec<_>>(),
                                "handlers": f.handlers.iter().map(handler_clause_to_json).collect::<Vec<_>>()})
                        }).collect::<Vec<_>>()}));
                    ctx.define(&clause.continuation_name, cont_value);

                    let mut fresh_stack = Vec::new();
                    let remaining_handlers: Vec<RuntimeHandlerClause> = bottom
                        .handlers
                        .iter()
                        .filter(|h| h.op_name != clause.op_name)
                        .cloned()
                        .collect();
                    let step_result = self.eval_step_with_handlers(
                        &clause.body,
                        ctx,
                        &mut fresh_stack,
                        &remaining_handlers,
                    );
                    ctx.pop_scope();

                    match step_result {
                        StepResult::Done(v) => EvalResult::Done(v),
                        StepResult::Yield {
                            value,
                            frames: yield_frames,
                        } => EvalResult::Yielded {
                            value,
                            continuation: Continuation {
                                frames: yield_frames,
                            },
                        },
                        StepResult::Error(err) => EvalResult::Error(err),
                    }
                } else {
                    // No matching handler — propagate.
                    frames.insert(0, bottom);
                    let op_value = NonFunctionValue::new(serde_json::json!({
                        "type": "operation",
                        "name": name,
                        "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                    EvalResult::Yielded {
                        value: op_value,
                        continuation: Continuation { frames },
                    }
                }
            }
            Err(err) => EvalResult::Error(err),
        }
    }

    fn unwind(
        &self,
        mut frames: Vec<Frame>,
        value: NonFunctionValue,
        ctx: &mut dyn Context<NonFunctionValue>,
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
            StepResult::Yield {
                value,
                frames: mut yield_frames,
            } => {
                // yield_frames are innermost-to-current. Append the
                // remaining ancestor frames to get the full stack.
                yield_frames.extend(frames);
                EvalResult::Yielded {
                    value,
                    continuation: Continuation {
                        frames: yield_frames,
                    },
                }
            }
            StepResult::Error(err) => EvalResult::Error(err),
        }
    }

    fn eval_frame(
        &self,
        frame: &mut Frame,
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        // Evaluate remaining pending args one by one.
        // Propagate this frame's handlers to child frames so that
        // operations in nested calls can be intercepted.
        let handlers = frame.handlers.clone();
        while !frame.pending.is_empty() {
            let arg = frame.pending.remove(0);
            match self.eval_step_with_handlers(&arg, ctx, stack, &handlers) {
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

        // All args evaluated — check if this is a continuation call.
        // A continuation is a variable bound by a handler clause that
        // holds captured frames. Calling it resumes the captured
        // evaluation with the first argument as the result.
        if let Ok(var) = ctx.get(&frame.name)
            && var.get("type").and_then(|v| v.as_str()) == Some("continuation")
        {
            // Extract the resume value (first arg).
            let resume_value = frame
                .evaluated
                .first()
                .cloned()
                .unwrap_or(NonFunctionValue::new(serde_json::Value::Null));

            // Deserialize the captured frames.
            let frames_json = var.get("frames").and_then(|v| v.as_array());
            if let Some(frames_json) = frames_json {
                let captured_frames: Result<Vec<Frame>, Error<NonFunctionValue>> =
                    frames_json.iter().map(frame_from_json).collect();

                match captured_frames {
                    Ok(captured_frames) => {
                        if captured_frames.is_empty() {
                            return StepResult::Done(resume_value);
                        }

                        // Resume: feed the resume value to the captured
                        // frames, exactly like `unwind`.
                        let result = self.unwind(captured_frames, resume_value, ctx);
                        return match result {
                            EvalResult::Done(v) => StepResult::Done(v),
                            EvalResult::Yielded {
                                value,
                                continuation,
                            } => StepResult::Yield {
                                value,
                                frames: continuation.frames,
                            },
                            EvalResult::Error(err) => StepResult::Error(err),
                        };
                    }
                    Err(err) => return StepResult::Error(err),
                }
            }
        }

        // Not a continuation call — dispatch normally.
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
            Err(Error::Operation { name, args }) => {
                // A typed operation signaled. Check handler clauses
                // on this frame for a matching operation.
                let matching_clause = frame.handlers.iter().rev().find(|h| h.op_name == name);

                if let Some(clause) = matching_clause {
                    // Found a matching handler. Bind params and
                    // continuation, then evaluate the handler body.
                    ctx.push_scope();

                    // Bind operation args to handler params.
                    for (i, param) in clause.params.iter().enumerate() {
                        if let Some(arg) = args.get(i) {
                            ctx.define(param, arg.clone());
                        }
                    }

                    // Bind the continuation. The continuation contains
                    // the PARENT frames (from the stack) that are waiting
                    // for the result of this call. The signaling frame
                    // itself is NOT included — its operation has been
                    // handled, and the handler provides the result.
                    let cont_frames = std::mem::take(stack);
                    let cont_value = NonFunctionValue::new(serde_json::json!({
                        "type": "continuation",
                        "frames": cont_frames.iter().map(|f| {
                            serde_json::json!({
                                "name": f.name,
                                "evaluated": f.evaluated.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>(),
                                "pending": f.pending.iter().map(expr_to_json).collect::<Vec<_>>(),
                                "handlers": f.handlers.iter().map(handler_clause_to_json).collect::<Vec<_>>()})
                        }).collect::<Vec<_>>()}));
                    ctx.define(&clause.continuation_name, cont_value);

                    // Evaluate the handler body.
                    // Propagate remaining handlers (excluding the matched
                    // clause) so nested operations can be intercepted.
                    let remaining_handlers: Vec<RuntimeHandlerClause> = frame
                        .handlers
                        .iter()
                        .filter(|h| h.op_name != clause.op_name)
                        .cloned()
                        .collect();
                    let result =
                        self.eval_step_with_handlers(&clause.body, ctx, stack, &remaining_handlers);

                    ctx.pop_scope();
                    result
                } else {
                    // No matching handler — propagate the operation
                    // as a structured yield to the caller.
                    stack.push(frame.clone());
                    let op_value = NonFunctionValue::new(serde_json::json!({
                        "type": "operation",
                        "name": name,
                        "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                    StepResult::Yield {
                        value: op_value,
                        frames: std::mem::take(stack),
                    }
                }
            }
            Err(err) => StepResult::Error(err),
        }
    }

    fn eval_step(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        self.eval_step_with_handlers(expr, ctx, stack, &[])
    }

    fn json_kind(v: &JsonValue) -> &'static str {
        match v {
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
            JsonValue::String(_) => "string",
            JsonValue::Number(_) => "number",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Null => "null",
        }
    }

    fn eval_index(
        &self,
        target: &RuntimeExpr,
        index: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
        handlers: &[RuntimeHandlerClause],
    ) -> StepResult {
        let target_value = match self.eval_step_with_handlers(target, ctx, stack, handlers) {
            StepResult::Done(v) => v,
            StepResult::Yield { value, frames } => {
                return StepResult::Yield { value, frames };
            }
            StepResult::Error(err) => return StepResult::Error(err),
        };

        let index_value = match self.eval_step_with_handlers(index, ctx, stack, handlers) {
            StepResult::Done(v) => v,
            StepResult::Yield { value, frames } => {
                return StepResult::Yield { value, frames };
            }
            StepResult::Error(err) => return StepResult::Error(err),
        };

        let (name, got) = match target_value.inner() {
            JsonValue::Array(_) => ("array.at", "array"),
            JsonValue::Object(_) => ("obj.get", "object"),
            JsonValue::String(_) => ("str.at", "string"),
            other => {
                return StepResult::Error(Error::TypeMismatch {
                    expected: "array, object, or string",
                    got: Self::json_kind(other),
                });
            }
        };

        if self.host().function(name).is_none() {
            return StepResult::Error(Error::TypeMismatch {
                expected: "indexable",
                got,
            });
        }

        let mut frame = Frame {
            name: name.to_string(),
            evaluated: vec![target_value, index_value],
            pending: vec![],
            handlers: handlers.to_vec(),
        };
        self.eval_frame(&mut frame, ctx, stack)
    }

    fn eval_field(
        &self,
        target: &RuntimeExpr,
        field: &str,
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
        handlers: &[RuntimeHandlerClause],
    ) -> StepResult {
        let target_value = match self.eval_step_with_handlers(target, ctx, stack, handlers) {
            StepResult::Done(v) => v,
            StepResult::Yield { value, frames } => {
                return StepResult::Yield { value, frames };
            }
            StepResult::Error(err) => return StepResult::Error(err),
        };

        match target_value.inner() {
            JsonValue::Object(map) => {
                let value = map.get(field).cloned().unwrap_or(JsonValue::Null);
                StepResult::Done(NonFunctionValue::new(value))
            }
            _ => {
                if let Some(name) = qualified_name(target) {
                    let qualified = format!("{name}.{field}");
                    StepResult::Done(NonFunctionValue::new(JsonValue::String(qualified)))
                } else {
                    StepResult::Error(Error::TypeMismatch {
                        expected: "object",
                        got: Self::json_kind(target_value.inner()),
                    })
                }
            }
        }
    }

    fn eval_step_with_handlers(
        &self,
        expr: &RuntimeExpr,
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
        inherited_handlers: &[RuntimeHandlerClause],
    ) -> StepResult {
        match expr {
            Expr::Value(value) => StepResult::Done(value.clone()),

            Expr::Var(name) => match ctx.get(name) {
                Ok(value) => StepResult::Done(value),
                Err(_) => {
                    if let Some(function) = self.host().function(name) {
                        match function.call(&[], ctx) {
                            Ok(value) => StepResult::Done(value),
                            Err(Error::Yield(value)) => StepResult::Yield {
                                value,
                                frames: std::mem::take(stack),
                            },
                            Err(Error::Operation { name, args }) => {
                                stack.push(Frame {
                                    name: name.clone(),
                                    evaluated: args.clone(),
                                    pending: vec![],
                                    handlers: inherited_handlers.to_vec(),
                                });
                                let op_value = NonFunctionValue::new(serde_json::json!({
                                    "type": "operation",
                                    "name": name,
                                    "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                                StepResult::Yield {
                                    value: op_value,
                                    frames: std::mem::take(stack),
                                }
                            }
                            Err(err) => StepResult::Error(err),
                        }
                    } else {
                        StepResult::Error(Error::UnknownVariable(name.clone()))
                    }
                }
            },

            Expr::Assign { target, value } => {
                for name in target.bound_names() {
                    if self.host().is_reserved(&name) {
                        return StepResult::Error(Error::ReservedName(name));
                    }
                }
                match self.eval_step_with_handlers(value, ctx, stack, inherited_handlers) {
                    StepResult::Done(evaluated) => {
                        match define_pattern(ctx, target, evaluated.clone()) {
                            Ok(_) => StepResult::Done(evaluated),
                            Err(err) => StepResult::Error(err),
                        }
                    }
                    other => other,
                }
            }

            Expr::Function {
                type_params: _,
                params,
                ret,
                body,
            } => {
                let mut evaluated_params = Vec::new();
                let mut pattern_json = Vec::new();
                for (pattern, ty) in params {
                    let schema = match ty {
                        Some(type_expr) => match self.eval_step_with_handlers(
                            type_expr,
                            ctx,
                            stack,
                            inherited_handlers,
                        ) {
                            StepResult::Done(v) => v.into_inner(),
                            other => return other,
                        },
                        None => JsonValue::Null,
                    };
                    for name in pattern.bound_names() {
                        evaluated_params.push((name, schema.clone()));
                    }
                    pattern_json.push(pattern_to_json(pattern));
                }

                let return_schema = match ret {
                    Some(type_expr) => match self.eval_step_with_handlers(
                        type_expr,
                        ctx,
                        stack,
                        inherited_handlers,
                    ) {
                        StepResult::Done(v) => Some(v.into_inner()),
                        other => return other,
                    },
                    None => None,
                };

                let function_value = FunctionValue {
                    params: evaluated_params,
                    return_schema,
                    body: body.clone(),
                    patterns: pattern_json,
                };

                StepResult::Done(NonFunctionValue::new(function_value.to_json()))
            }

            Expr::Call { name, args, .. } => {
                // Check lazy functions first — they receive unevaluated
                // args and decide which to evaluate (short-circuit).
                if let Some(lazy) = self.host().lazy_function(name) {
                    let handlers = inherited_handlers.to_vec();
                    let mut evaluate =
                        |sub: &RuntimeExpr, c: &mut dyn Context<NonFunctionValue>| match self
                            .eval_step_with_handlers(sub, c, stack, &handlers)
                        {
                            StepResult::Done(v) => Ok(v),
                            StepResult::Yield { value, .. } => Err(Error::Yield(value)),
                            StepResult::Error(err) => Err(err),
                        };
                    match lazy.call(args, ctx, &mut evaluate) {
                        Ok(v) => StepResult::Done(v),
                        Err(Error::Yield(value)) => StepResult::Yield {
                            value,
                            frames: std::mem::take(stack),
                        },
                        Err(Error::Operation { name, args }) => {
                            // Check handler clauses for a match.
                            let matching_clause =
                                inherited_handlers.iter().rev().find(|h| h.op_name == name);
                            if let Some(clause) = matching_clause {
                                self.handle_operation(
                                    name,
                                    args,
                                    clause,
                                    inherited_handlers,
                                    ctx,
                                    stack,
                                )
                            } else {
                                stack.push(Frame {
                                    name: name.clone(),
                                    evaluated: args.clone(),
                                    pending: vec![],
                                    handlers: inherited_handlers.to_vec(),
                                });
                                let op_value = NonFunctionValue::new(serde_json::json!({
                                    "type": "operation",
                                    "name": name,
                                    "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                                StepResult::Yield {
                                    value: op_value,
                                    frames: std::mem::take(stack),
                                }
                            }
                        }
                        Err(err) => StepResult::Error(err),
                    }
                } else {
                    let mut frame = Frame {
                        name: name.clone(),
                        evaluated: Vec::with_capacity(args.len()),
                        pending: args.clone(),
                        handlers: inherited_handlers.to_vec(),
                    };
                    self.eval_frame(&mut frame, ctx, stack)
                }
            }

            Expr::Apply { target, args } => {
                let mut pending = vec![target.as_ref().clone()];
                pending.extend(args.iter().cloned());
                let mut frame = Frame {
                    name: "apply".to_string(),
                    evaluated: Vec::with_capacity(pending.len()),
                    pending,
                    handlers: inherited_handlers.to_vec(),
                };
                self.eval_frame(&mut frame, ctx, stack)
            }

            Expr::Handle { body, handlers } => {
                // Merge inner handlers with inherited (outer) handlers.
                // Inner handlers take precedence (checked first).
                let mut all_handlers = handlers.clone();
                all_handlers.extend(inherited_handlers.iter().cloned());
                self.eval_handle(body, &all_handlers, ctx, stack)
            }

            Expr::Array(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for elem in elements {
                    match self.eval_step_with_handlers(elem, ctx, stack, inherited_handlers) {
                        StepResult::Done(v) => values.push(v.into_inner()),
                        StepResult::Yield { value, frames } => {
                            return StepResult::Yield { value, frames };
                        }
                        StepResult::Error(err) => return StepResult::Error(err),
                    }
                }
                StepResult::Done(NonFunctionValue::new(serde_json::Value::Array(values)))
            }

            Expr::Object(entries) => {
                let mut map = serde_json::Map::with_capacity(entries.len());
                for (key, expr) in entries {
                    match self.eval_step_with_handlers(expr, ctx, stack, inherited_handlers) {
                        StepResult::Done(v) => {
                            map.insert(key.clone(), v.into_inner());
                        }
                        StepResult::Yield { value, frames } => {
                            return StepResult::Yield { value, frames };
                        }
                        StepResult::Error(err) => return StepResult::Error(err),
                    }
                }
                StepResult::Done(NonFunctionValue::new(serde_json::Value::Object(map)))
            }

            Expr::Index { target, index } => {
                self.eval_index(target, index, ctx, stack, inherited_handlers)
            }

            Expr::FieldAccess { target, field } => {
                self.eval_field(target, field, ctx, stack, inherited_handlers)
            }
        }
    }

    /// Evaluate a handle body with handler clauses active.
    ///
    /// The handlers are attached to any frame that yields during
    /// evaluation of body. When an operation signals, `eval_frame`
    /// checks the frame's handlers for a matching clause.
    fn eval_handle(
        &self,
        body: &RuntimeExpr,
        handlers: &[RuntimeHandlerClause],
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        match body {
            Expr::Value(value) => StepResult::Done(value.clone()),

            Expr::Var(name) => match ctx.get(name) {
                Ok(value) => StepResult::Done(value),
                Err(_) => {
                    if let Some(function) = self.host().function(name) {
                        match function.call(&[], ctx) {
                            Ok(value) => StepResult::Done(value),
                            Err(Error::Yield(value)) => StepResult::Yield {
                                value,
                                frames: std::mem::take(stack),
                            },
                            Err(Error::Operation { name, args }) => {
                                let matching_clause =
                                    handlers.iter().rev().find(|h| h.op_name == name);
                                if let Some(clause) = matching_clause {
                                    self.handle_operation(name, args, clause, handlers, ctx, stack)
                                } else {
                                    stack.push(Frame {
                                        name: name.clone(),
                                        evaluated: args.clone(),
                                        pending: vec![],
                                        handlers: handlers.to_vec(),
                                    });
                                    let op_value = NonFunctionValue::new(serde_json::json!({
                                        "type": "operation",
                                        "name": name,
                                        "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                                    StepResult::Yield {
                                        value: op_value,
                                        frames: std::mem::take(stack),
                                    }
                                }
                            }
                            Err(err) => StepResult::Error(err),
                        }
                    } else {
                        StepResult::Error(Error::UnknownVariable(name.clone()))
                    }
                }
            },

            Expr::Assign { target, value } => {
                for name in target.bound_names() {
                    if self.host().is_reserved(&name) {
                        return StepResult::Error(Error::ReservedName(name));
                    }
                }
                match self.eval_handle(value, handlers, ctx, stack) {
                    StepResult::Done(evaluated) => {
                        match define_pattern(ctx, target, evaluated.clone()) {
                            Ok(_) => StepResult::Done(evaluated),
                            Err(err) => StepResult::Error(err),
                        }
                    }
                    other => other,
                }
            }

            Expr::Function {
                type_params: _,
                params,
                ret,
                body,
            } => {
                let mut evaluated_params = Vec::new();
                let mut pattern_json = Vec::new();
                for (pattern, ty) in params {
                    let schema = match ty {
                        Some(type_expr) => {
                            match self.eval_handle(type_expr, handlers, ctx, stack) {
                                StepResult::Done(v) => v.into_inner(),
                                other => return other,
                            }
                        }
                        None => JsonValue::Null,
                    };
                    for name in pattern.bound_names() {
                        evaluated_params.push((name, schema.clone()));
                    }
                    pattern_json.push(pattern_to_json(pattern));
                }

                let return_schema = match ret {
                    Some(type_expr) => match self.eval_handle(type_expr, handlers, ctx, stack) {
                        StepResult::Done(v) => Some(v.into_inner()),
                        other => return other,
                    },
                    None => None,
                };

                let function_value = FunctionValue {
                    params: evaluated_params,
                    return_schema,
                    body: body.clone(),
                    patterns: pattern_json,
                };

                StepResult::Done(NonFunctionValue::new(function_value.to_json()))
            }

            Expr::Call { name, args, .. } => {
                // Check lazy functions first — they receive unevaluated
                // args and decide which to evaluate (short-circuit).
                if let Some(lazy) = self.host().lazy_function(name) {
                    let handlers_vec = handlers.to_vec();
                    let mut evaluate =
                        |sub: &RuntimeExpr, c: &mut dyn Context<NonFunctionValue>| match self
                            .eval_step_with_handlers(sub, c, stack, &handlers_vec)
                        {
                            StepResult::Done(v) => Ok(v),
                            StepResult::Yield { value, .. } => Err(Error::Yield(value)),
                            StepResult::Error(err) => Err(err),
                        };
                    match lazy.call(args, ctx, &mut evaluate) {
                        Ok(v) => StepResult::Done(v),
                        Err(Error::Yield(value)) => StepResult::Yield {
                            value,
                            frames: std::mem::take(stack),
                        },
                        Err(Error::Operation { name, args }) => {
                            let matching_clause = handlers.iter().rev().find(|h| h.op_name == name);
                            if let Some(clause) = matching_clause {
                                self.handle_operation(name, args, clause, handlers, ctx, stack)
                            } else {
                                stack.push(Frame {
                                    name: name.clone(),
                                    evaluated: args.clone(),
                                    pending: vec![],
                                    handlers: handlers.to_vec(),
                                });
                                let op_value = NonFunctionValue::new(serde_json::json!({
                                    "type": "operation",
                                    "name": name,
                                    "args": args.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>()}));
                                StepResult::Yield {
                                    value: op_value,
                                    frames: std::mem::take(stack),
                                }
                            }
                        }
                        Err(err) => StepResult::Error(err),
                    }
                } else {
                    let mut frame = Frame {
                        name: name.clone(),
                        evaluated: Vec::with_capacity(args.len()),
                        pending: args.clone(),
                        handlers: handlers.to_vec(),
                    };
                    self.eval_frame(&mut frame, ctx, stack)
                }
            }

            Expr::Apply { target, args } => {
                let mut pending = vec![target.as_ref().clone()];
                pending.extend(args.iter().cloned());
                let mut frame = Frame {
                    name: "apply".to_string(),
                    evaluated: Vec::with_capacity(pending.len()),
                    pending,
                    handlers: handlers.to_vec(),
                };
                self.eval_frame(&mut frame, ctx, stack)
            }

            Expr::Handle {
                body: inner_body,
                handlers: inner_handlers,
            } => {
                // Nested handle: inner handlers take precedence.
                let mut merged = inner_handlers.clone();
                merged.extend(handlers.iter().cloned());
                self.eval_handle(inner_body, &merged, ctx, stack)
            }

            Expr::Array(elements) => {
                let mut values = Vec::with_capacity(elements.len());
                for elem in elements {
                    match self.eval_handle(elem, handlers, ctx, stack) {
                        StepResult::Done(v) => values.push(v.into_inner()),
                        StepResult::Yield { value, frames } => {
                            return StepResult::Yield { value, frames };
                        }
                        StepResult::Error(err) => return StepResult::Error(err),
                    }
                }
                StepResult::Done(NonFunctionValue::new(serde_json::Value::Array(values)))
            }

            Expr::Object(entries) => {
                let mut map = serde_json::Map::with_capacity(entries.len());
                for (key, expr) in entries {
                    match self.eval_handle(expr, handlers, ctx, stack) {
                        StepResult::Done(v) => {
                            map.insert(key.clone(), v.into_inner());
                        }
                        StepResult::Yield { value, frames } => {
                            return StepResult::Yield { value, frames };
                        }
                        StepResult::Error(err) => return StepResult::Error(err),
                    }
                }
                StepResult::Done(NonFunctionValue::new(serde_json::Value::Object(map)))
            }

            Expr::Index { target, index } => self.eval_index(target, index, ctx, stack, handlers),

            Expr::FieldAccess { target, field } => {
                self.eval_field(target, field, ctx, stack, handlers)
            }
        }
    }

    fn dispatch_call(
        &self,
        name: &str,
        args: &[NonFunctionValue],
        ctx: &mut dyn Context<NonFunctionValue>,
    ) -> Result<NonFunctionValue, Error<NonFunctionValue>> {
        // `apply` is special: the first argument is the function to call.
        // If it is a string, dispatch directly to the named host function.
        if name == "apply" && !args.is_empty() {
            if let Some(fname) = args[0].as_str() {
                if let Some(function) = self.host().function(fname) {
                    return function.call(&args[1..], ctx);
                }
                return Err(Error::UnknownFunction(fname.to_string()));
            }

            if FunctionValue::from_json(args[0].inner()).is_some() {
                if let Some(apply) = self.host().function("apply") {
                    return apply.call(args, ctx);
                }
                return Err(Error::UnknownFunction("apply".to_string()));
            }

            return Err(Error::TypeMismatch {
                expected: "function",
                got: json_kind(args[0].inner()),
            });
        }

        // Check operations first — operations signal (yield) rather
        // than complete. If an operation is found and it signals
        // (returns Err(Error::Operation { ... })), the caller
        // (eval_frame) captures the continuation.
        if let Some(operation) = self.host().operation(name) {
            return operation.perform(args, ctx);
        }

        if let Some(function) = self.host().function(name) {
            return function.call(args, ctx);
        }

        // Allow function values stored in the context to be called by name.
        if let Ok(value) = ctx.get(name)
            && FunctionValue::from_json(value.inner()).is_some()
        {
            let apply = self
                .host()
                .function("apply")
                .ok_or_else(|| Error::UnknownFunction("apply".to_string()))?;
            let mut apply_args = vec![value];
            apply_args.extend_from_slice(args);
            return apply.call(&apply_args, ctx);
        }

        Err(Error::UnknownFunction(name.to_string()))
    }

    /// Handle an operation signal by matching it against handler clauses
    /// and evaluating the handler body.
    ///
    /// Shared logic between the lazy-function path and `eval_frame`.
    fn handle_operation(
        &self,
        _name: String,
        args: Vec<NonFunctionValue>,
        clause: &RuntimeHandlerClause,
        all_handlers: &[RuntimeHandlerClause],
        ctx: &mut dyn Context<NonFunctionValue>,
        stack: &mut Vec<Frame>,
    ) -> StepResult {
        ctx.push_scope();

        for (i, param) in clause.params.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                ctx.define(param, arg.clone());
            }
        }

        let cont_frames = std::mem::take(stack);
        let cont_value = NonFunctionValue::new(serde_json::json!({
            "type": "continuation",
            "frames": cont_frames.iter().map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "evaluated": f.evaluated.iter().map(|v| v.clone().into_inner()).collect::<Vec<_>>(),
                    "pending": f.pending.iter().map(expr_to_json).collect::<Vec<_>>(),
                    "handlers": f.handlers.iter().map(handler_clause_to_json).collect::<Vec<_>>()})
            }).collect::<Vec<_>>()}));
        ctx.define(&clause.continuation_name, cont_value);

        let remaining_handlers: Vec<RuntimeHandlerClause> = all_handlers
            .iter()
            .filter(|h| h.op_name != clause.op_name)
            .cloned()
            .collect();
        let result = self.eval_step_with_handlers(&clause.body, ctx, stack, &remaining_handlers);

        ctx.pop_scope();
        result
    }
}

/// Internal result of a single evaluation step.
enum StepResult {
    /// Evaluation completed with a value.
    Done(NonFunctionValue),
    /// A host function yielded. The `frames` vector is the frame
    /// stack at the point of yield (bottom to top).
    Yield {
        value: NonFunctionValue,
        frames: Vec<Frame>,
    },
    /// An error occurred.
    Error(Error<NonFunctionValue>),
}
