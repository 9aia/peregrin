//! Embedded surface dispatch — the driver serving its own engine's
//! ops through `pavo-driver-kit`'s surface layer, no second binary
//! needed. `cli`/`mcp` argv verbs (feature-gated) plug this
//! [`Surface`] impl into `surface::cli::run` /
//! `surface::mcp::serve_stdio`.
//!
//! `op` reuses the same `cli.*` path wire queries take
//! (`engine_ops::run_verb` re-execs `run` in a child — output stays
//! captured off the serve stream). `surface` dispatches in-process
//! through `cli::run_from`, which owns exit codes exactly like the
//! `peregrin` bin — that's where the unbounded (`serve`,
//! `campaign serve`/`run`, `sessions attach`/`stream`, `events`)
//! verbs live, since they need inherited stdio a captured op child
//! can't provide.

use pavo_driver_kit::surface::{OpOutcome, OpSpec, Surface};
use serde_json::{Map, Value};

use crate::engine_ops;

/// peregrin's [`Surface`] — engine dispatch, already linked.
pub struct Engine;

impl Surface for Engine {
    /// Same path as the `cli.<verb>` wire query: re-exec `run` in
    /// a child, captured output → `OpOutcome`.
    fn op(&mut self, op: &OpSpec, params: &Map<String, Value>) -> OpOutcome {
        OpOutcome::from_response(&engine_ops::run_op(&op.name, params))
    }

    /// `peregrin <name> <args>` in-process — for the surfaces
    /// (`serve`, `campaign serve`/`run`, `sessions attach`/`stream`,
    /// `events`): inherited stdio. Bounded ops never reach here.
    fn surface(&mut self, name: &str, args: &[String]) -> i32 {
        let argv = std::iter::once("peregrin".to_string())
            .chain(std::iter::once(name.to_string()))
            .chain(args.iter().cloned());
        crate::cli::run_from(argv)
    }
}
