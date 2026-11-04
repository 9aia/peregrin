//! Scope providers — the bindings the scope-resolution pass
//! (`docs/reference/scope-resolution.md`) resolves `${event.*}`,
//! `${config.*}`, `${params.*}`, `${state.*}`, `${node.*}`, and
//! `${secret.*}` against. The runner (or a driver re-resolving
//! templates) binds them per resolve call; unbound scopes miss and
//! placeholders pass through verbatim (or fail the strict audit).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::rc::Rc;

use serde_json::{Map, Value};

/// The whole scope binding for one resolve call — every field optional;
/// an unbound namespace makes its lookups miss.
#[derive(Default)]
pub struct Scope<'a> {
    /// `${event.*}` — `triggers[0]` of the current fire plus the node's
    /// `on` patterns (wildcard captures are resolved lazily).
    pub event: Option<EventScope<'a>>,
    /// `${config.*}` — the campaign `config:` map.
    pub config: Option<&'a Map<String, Value>>,
    /// `${params.*}` — the node's declared `params:` mid-resolution
    /// (lazy, memoized, cycle-guarded) or the already-resolved params
    /// object everywhere else.
    pub params: ParamsScope<'a>,
    /// `${state.*}` — the ctx's state store (scoped when `state.key`
    /// is declared, else the shared run store).
    pub state: Option<&'a dyn StateScope>,
    /// `${node.*}` — the firing node.
    pub node: Option<NodeScope<'a>>,
    /// `${secret.*}` — the run's secrets record (capability-gated).
    pub secrets: Option<&'a BTreeMap<String, String>>,
}

/// The bound trigger for `${event.*}` — `triggers[0]` plus the node's
/// `on` patterns; `match.<N>`/`shard` capture from the first pattern
/// that matched the event.
pub struct EventScope<'a> {
    /// The event the current fire bound (`triggers[0]`).
    pub event: &'a peregrin_core::Event,
    /// The node's `on` patterns — wildcard captures come from the
    /// first pattern matching `event.kind`.
    pub patterns: &'a [String],
}

/// The firing node for `${node.*}` — `id` is a string, `fires` the
/// fire count including this fire (`ctx.fires`).
pub struct NodeScope<'a> {
    /// `node.id`.
    pub id: &'a str,
    /// `ctx.fires` — the fire count including the current fire.
    pub fires: u64,
}

/// How `${params.*}` binds for this resolve call.
#[derive(Default)]
pub enum ParamsScope<'a> {
    /// Unbound — every `params.*` lookup misses.
    #[default]
    Unbound,
    /// The fire's already-resolved params object — bound by every
    /// resolution outside the params pass itself (`state.key`,
    /// `state.skip_if`, driver-side re-resolves).
    Resolved(&'a Map<String, Value>),
    /// `node.params` mid-resolution — referenced keys resolve lazily
    /// through the full pipeline on first reference, memoize for the
    /// rest of the pass, and circular references miss (the
    /// in-progress key is never visible).
    Declared(Rc<RefCell<ParamScope>>),
}

impl ParamsScope<'_> {
    /// Fresh lazy state for a declared-params pass — one per pass so
    /// memoized keys live for exactly that pass.
    #[must_use]
    pub fn declared() -> Self {
        Self::Declared(Rc::new(RefCell::new(ParamScope::default())))
    }
}

/// Shared lazy-resolution state behind [`ParamsScope::Declared`] —
/// `resolving` is the in-progress key set (cycle guard), `memo` the
/// per-pass resolved-key cache.
#[derive(Default)]
pub struct ParamScope {
    /// Keys whose resolution is currently in flight — a lookup that
    /// lands here is a circular reference and misses.
    pub(crate) resolving: HashSet<String>,
    /// Resolved values by key — populated on first reference so
    /// `${params.*}` and the params pass agree on one value per key.
    pub(crate) memo: Map<String, Value>,
}

/// Read-side provider for `${state.*}` — the ctx's state store (scoped
/// or shared), adapted by the caller so the resolver needs no store
/// dependency. `state.<key>` calls [`StateScope::get`], then remaining
/// path segments walk the stored record.
pub trait StateScope {
    /// `store.get(key)` — `None` for absent keys.
    fn get(&self, key: &str) -> Option<Value>;
}

impl<F> StateScope for F
where
    F: Fn(&str) -> Option<Value>,
{
    fn get(&self, key: &str) -> Option<Value> {
        self(key)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_scope_is_fully_unbound() {
        let scope = Scope::default();
        assert!(scope.event.is_none());
        assert!(scope.config.is_none());
        assert!(scope.state.is_none());
        assert!(scope.node.is_none());
        assert!(scope.secrets.is_none());
        assert!(matches!(scope.params, ParamsScope::Unbound));
        assert!(matches!(ParamsScope::default(), ParamsScope::Unbound));
    }

    #[test]
    fn declared_params_scope_carries_shared_state() {
        let ParamsScope::Declared(state) = ParamsScope::declared() else {
            panic!("declared() must build the Declared variant");
        };
        state
            .borrow_mut()
            .memo
            .insert("k".to_string(), Value::from(1));
        assert_eq!(state.borrow().memo.get("k"), Some(&Value::from(1)));
    }

    #[test]
    fn state_scope_reads_through_a_closure() {
        let data = Map::from_iter([("k".to_string(), Value::from("v"))]);
        let view = |key: &str| data.get(key).cloned();
        assert_eq!(StateScope::get(&view, "k"), Some(Value::from("v")));
        assert_eq!(StateScope::get(&view, "absent"), None);
    }
}
