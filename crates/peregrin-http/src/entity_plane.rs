//! The kind-plane port + registry — how `/api/entities*` stops being
//! session-shaped (ADR-0091, PL-004). Each resident entity kind
//! registers one [`EntityPlane`] impl that owns its slice of the
//! surface: entity docs, its `<ns>.<verb>` op table, and snapshot
//! export/import. The substrate routes parse just enough to pick a
//! plane (`body.kind`, the op namespace) and hand over verbatim —
//! payloads stay opaque and no global verb table grows here.
//!
//! `SessionEntityPlane` (`crate::session_entity`) is the built-in
//! `kind="sepia-session"` impl — it wraps the `/api/sessions*`
//! handlers the same way the old hard-coded dispatch did. Future
//! integrations register their own planes at daemon composition via
//! [`crate::AppState::with_kind`] / `state.kinds.register(...)`.

use std::sync::Arc;

use axum::response::Response;
use serde_json::Value;

use crate::AppState;

/// One registered entity kind's serving plane. Responses are the
/// kind's own wire surface under `/api/entities*` — the substrate
/// forwards them untouched, so an impl controls its status codes,
/// error shapes, and payload translation entirely.
///
/// `query` parameters are the raw `?…` string of the incoming
/// request: kind-specific params (the session plane's `?agent=`
/// disambiguator, `withLocks`) are each plane's business, not the
/// substrate's.
#[async_trait::async_trait]
pub trait EntityPlane: Send + Sync {
    /// The entity kind tag this plane serves (`"sepia-session"`).
    fn kind(&self) -> &'static str;

    /// The op namespace this plane owns for `<ns>.<verb>` ops —
    /// defaults to `kind()`; `sepia-session` serves `sepia.*`. The
    /// substrate namespace `entity.*` is reserved and never reaches a
    /// plane.
    fn op_namespace(&self) -> &'static str {
        self.kind()
    }

    /// Additional `<ns>` prefixes this plane also owns — e.g.
    /// `sepia-session` serves `meta.*` (the node-local meta overlay)
    /// alongside `sepia.*`. The substrate namespace `entity.*` is
    /// reserved and never reaches a plane.
    fn extra_op_namespaces(&self) -> &'static [&'static str] {
        &[]
    }

    /// `GET /api/entities` contribution — this kind's entity docs
    /// (`{id,kind,indexDoc,updatedAt,locked,live}` rows). Errors
    /// degrade to an empty contribution, matching the list route's
    /// long-standing silent-200.
    async fn list(&self, state: &AppState, query: Option<&str>) -> Vec<Value>;

    /// `GET /api/entities/{id}` — the entity doc, or the kind's own
    /// error response. A 404 tells the substrate the id isn't this
    /// plane's — the next registered plane gets a shot.
    async fn get(&self, state: &AppState, id: &str, query: Option<&str>) -> Response;

    /// `POST /api/entities` — `init` is the body's `indexDoc` (the
    /// whole body when `indexDoc` is absent); the response is the
    /// kind's own create payload.
    async fn create(&self, state: &AppState, init: Value) -> Response;

    /// `DELETE /api/entities/{id}` / the `entity.delete` op — only
    /// invoked after [`Self::get`] has claimed the id for this plane,
    /// so impls may assume ownership.
    async fn delete(&self, state: &AppState, id: &str, query: Option<&str>) -> Response;

    /// `POST /api/entities/{id}/ops` — `op` is the op kind verbatim
    /// (`<ns>.<verb>`, or a bare verb routed to the default plane);
    /// the plane strips its [`Self::op_namespace`] and dispatches its
    /// own verb table.
    async fn op(
        &self,
        state: &AppState,
        id: &str,
        op: &str,
        payload: Value,
        query: Option<&str>,
    ) -> Response;

    /// `GET /api/entities/{id}/snapshot` — kind-shaped snapshot bytes
    /// the substrate moves opaquely. 404 = not this plane's id.
    async fn export(&self, state: &AppState, id: &str, query: Option<&str>) -> Response;

    /// `POST /api/entities/snapshot` — `body` is the import body
    /// verbatim (kind-defined shape).
    async fn import(&self, state: &AppState, body: Value) -> Response;
}

/// `kind` → [`EntityPlane`] registry carried on [`AppState::kinds`].
/// Registration order matters for two fallbacks: the first registered
/// plane is the default for un-namespaced ops and for creates/imports
/// that omit `kind` — keeping `sepia-session` the implicit target
/// every legacy caller already assumes.
#[derive(Clone, Default)]
pub struct KindRegistry {
    planes: Vec<Arc<dyn EntityPlane>>,
}

impl KindRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plane; a second plane for the same `kind` replaces
    /// the first (recomposition is idempotent).
    pub fn register(&mut self, plane: Arc<dyn EntityPlane>) {
        if let Some(slot) = self.planes.iter_mut().find(|p| p.kind() == plane.kind()) {
            *slot = plane;
        } else {
            self.planes.push(plane);
        }
    }

    /// The plane serving `kind`, if registered.
    pub fn plane(&self, kind: &str) -> Option<Arc<dyn EntityPlane>> {
        self.planes.iter().find(|p| p.kind() == kind).cloned()
    }

    /// The first registered plane — the implicit target for
    /// un-namespaced ops and `kind`-less creates/imports.
    pub fn default_plane(&self) -> Option<Arc<dyn EntityPlane>> {
        self.planes.first().cloned()
    }

    /// Resolve an op kind to its plane: `<ns>.<verb>` looks the
    /// namespace up across planes; a bare verb falls back to the
    /// default plane (legacy un-namespaced outbox rows read as the
    /// session plane's verbs). `entity.*` never reaches this —
    /// the substrate handles that namespace itself.
    pub fn plane_for_op(&self, op: &str) -> Option<Arc<dyn EntityPlane>> {
        match op.split_once('.') {
            Some((namespace, _)) => self
                .planes
                .iter()
                .find(|p| {
                    p.op_namespace() == namespace || p.extra_op_namespaces().contains(&namespace)
                })
                .cloned(),
            None => self.default_plane(),
        }
    }

    /// Every registered plane, in registration order.
    pub fn planes(&self) -> &[Arc<dyn EntityPlane>] {
        &self.planes
    }

    pub fn is_empty(&self) -> bool {
        self.planes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    struct Stub(&'static str, &'static str);

    #[async_trait::async_trait]
    impl EntityPlane for Stub {
        fn kind(&self) -> &'static str {
            self.0
        }
        fn op_namespace(&self) -> &'static str {
            self.1
        }
        async fn list(&self, _: &AppState, _: Option<&str>) -> Vec<Value> {
            Vec::new()
        }
        async fn get(&self, _: &AppState, _: &str, _: Option<&str>) -> Response {
            unimplemented!()
        }
        async fn create(&self, _: &AppState, _: Value) -> Response {
            unimplemented!()
        }
        async fn delete(&self, _: &AppState, _: &str, _: Option<&str>) -> Response {
            unimplemented!()
        }
        async fn op(&self, _: &AppState, _: &str, _: &str, _: Value, _: Option<&str>) -> Response {
            unimplemented!()
        }
        async fn export(&self, _: &AppState, _: &str, _: Option<&str>) -> Response {
            unimplemented!()
        }
        async fn import(&self, _: &AppState, _: Value) -> Response {
            unimplemented!()
        }
    }

    #[test]
    fn registry_resolves_kinds_namespaces_and_defaults() {
        let mut registry = KindRegistry::new();
        let sessions = Arc::new(Stub("sepia-session", "sepia"));
        let widgets = Arc::new(Stub("test-widget", "test-widget"));
        registry.register(sessions);
        registry.register(widgets);

        assert_eq!(
            registry.plane("test-widget").map(|p| p.kind()),
            Some("test-widget")
        );
        assert!(registry.plane("ghost").is_none());
        assert_eq!(
            registry.plane_for_op("sepia.prompt").map(|p| p.kind()),
            Some("sepia-session")
        );
        assert_eq!(
            registry.plane_for_op("test-widget.poke").map(|p| p.kind()),
            Some("test-widget")
        );
        assert!(registry.plane_for_op("ghost.verb").is_none());
        // Bare verbs and a kind-less create land on the first plane.
        assert_eq!(
            registry.plane_for_op("prompt").map(|p| p.kind()),
            Some("sepia-session")
        );
        // Re-registering a kind replaces in place — order holds.
        registry.register(Arc::new(Stub("test-widget", "test-widget")));
        assert_eq!(registry.planes().len(), 2);
    }
}
