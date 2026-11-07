//! Capability checks — port of `functions/capability.ts`.
//!
//! A `capabilities` map gates functions twice: presence of any entry
//! whitelists the call at all (checked in `engine`), and in strict mode
//! the argument value must match the grant — `true` allows everything,
//! a list is glob alternatives, `{allow, deny}` is a glob policy. The
//! grant matching itself lives in [`peregrin_core::capability_matches`]
//! — the driver's `declared_only` secret-source gate matches the same
//! way, so a `capabilities.secrets` entry means one thing everywhere.

use serde_json::Value;

use crate::context::Context;
use crate::error::TemplateError;

/// `checkCapability` — only enforced under `ctx.strict`; denies when the
/// grant is missing/false or the value doesn't match the grant.
pub(crate) fn check_capability(
    name: &str,
    value: &str,
    ctx: &Context<'_>,
) -> Result<(), TemplateError> {
    if !ctx.strict {
        return Ok(());
    }
    let granted = ctx
        .capabilities
        .get(name)
        .is_some_and(|cap| capability_matches(value, cap));
    if granted {
        return Ok(());
    }
    Err(TemplateError::new(format!(
        "capability '{name}' not granted for '{value}'"
    )))
}

/// `matches` — delegates to [`peregrin_core::capability_matches`] so
/// every consumer of a `capabilities` grant evaluates it identically.
fn capability_matches(value: &str, caps: &Value) -> bool {
    peregrin_core::capability_matches(value, caps)
}
