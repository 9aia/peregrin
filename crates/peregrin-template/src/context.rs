//! Evaluation context — the template-relevant slice of the TS
//! `ExecutionContext` plus the optional resolver inputs.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::{Map, Value};

use crate::effects::Effects;
use crate::function::TemplateFunction;

/// Context handed to the resolvers and every function — carries the
/// strict flag, the capability map, the missing-secret list, and the
/// ambient [`Effects`] backing `env`/`file`/`shell`/`now`/`random`/`uuid`.
pub struct Context<'a> {
    /// Workflow `strict` flag. Off: anything unresolvable passes through
    /// and capability *checks* are skipped. On: unresolved placeholders
    /// and missing secrets error, and `env`/`file`/`shell` arguments are
    /// matched against `capabilities`.
    pub strict: bool,
    /// Secret names the host could not provide — strict mode fails with
    /// `missing secrets: …` after rendering. Shared (`Rc<RefCell>`) so
    /// driver-backed custom functions writing to the run's ledger are
    /// visible to the strict check, as the TS shared `ctx.missingSecrets`
    /// array was.
    pub missing_secrets: Rc<RefCell<Vec<String>>>,
    /// Per-function grants. Presence of *any* entry whitelists the
    /// function for resolution (`undefined`/`false` preserves the
    /// placeholder); in strict mode the entry — `true`, a glob list, or
    /// `{allow, deny}` — must also match the argument value.
    pub capabilities: Map<String, Value>,
    /// Ambient effects backing the I/O and entropy builtins.
    pub effects: &'a dyn Effects,
}

impl<'a> Context<'a> {
    /// Permissive context: non-strict, nothing missing, no capabilities
    /// — every placeholder passes through verbatim.
    pub fn new(effects: &'a dyn Effects) -> Self {
        Self {
            strict: false,
            missing_secrets: Rc::new(RefCell::new(Vec::new())),
            capabilities: Map::new(),
            effects,
        }
    }
}

/// Optional resolver inputs — the TS `node?` / `customFunctions?` params.
#[derive(Default)]
pub struct Options<'a> {
    /// Node whose params are being resolved — forwarded to functions
    /// (builtins ignore it; driver-backed custom functions use it).
    pub node: Option<&'a peregrin_core::Node>,
    /// Custom functions layered over [`builtins`](crate::builtins) —
    /// same-name entries replace builtins (`{...builtins, ...custom}`).
    pub functions: &'a [TemplateFunction],
}
