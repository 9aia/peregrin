//! peregrin-hub — the `peregrin-hub` gateway's Peregrin half (PL-005
//! P4b). The gateway machinery — sync engine, merged projection,
//! outbox, entity API, node credentials, push store, HTTP/SSE
//! bridges — lives in `hypha-hub`; this crate keeps what is
//! Peregrin's:
//!
//! - [`kind_policies`] — the `sepia-session`/`campaign`
//!   [`hypha_sync::KindPolicy`] registry the sync engine, feed hook,
//!   and entity API consume.
//! - the `PEREGRIN_*` env contract — [`hub_config`] maps
//!   `PEREGRIN_HUB_*`/`PEREGRIN_NODE_*`/`PEREGRIN_NODES`/
//!   `PEREGRIN_HOME` onto [`hypha_hub::HubConfig`] and the env lookup
//!   [`hub_state`] hands to `hypha_hub::hub_state_from_with`, so no
//!   `HYPHA_*` variable is ever required.
//! - [`router`]/[`hub_state`] — thin re-shapes over `hypha_hub`'s.

pub mod kind_policies;

pub use kind_policies::default_kind_policies;

use std::collections::HashMap;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::PathBuf;

use hypha_hub::{HubConfig as HyphaHubConfig, HubState};

/// The hub's runtime configuration — the `PEREGRIN_*` env contract,
/// field-for-field identical to [`hypha_hub::HubConfig`]'s `HYPHA_*`
/// names. Kept as a distinct type so parse errors and docs name the
/// variables operators actually set.
#[derive(Clone, Debug)]
pub struct HubConfig {
    /// `PEREGRIN_HUB_HOST` — default `127.0.0.1`.
    pub host: String,
    /// `PEREGRIN_HUB_PORT` — default `13000`.
    pub port: u16,
    /// `PEREGRIN_NODE_URL` — default `http://127.0.0.1:18787`.
    pub node_url: String,
    /// `PEREGRIN_NODE_TOKEN` — bearer token forwarded hub → node.
    pub node_token: Option<String>,
    /// `PEREGRIN_HUB_TOKEN` — client→hub auth; required on non-loopback
    /// binds, honored as bearer header or `?token=` query.
    pub hub_token: Option<String>,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 13000,
            node_url: "http://127.0.0.1:18787".into(),
            node_token: None,
            hub_token: None,
        }
    }
}

/// `PEREGRIN_HUB_TOKEN` required on non-loopback binds — mirrors
/// `hypha_http::env::is_loopback_host` and the old hub's guard.
fn is_loopback_host(host: &str) -> bool {
    hypha_http::env::is_loopback_host(host)
}

impl HubConfig {
    /// Parse from the process env.
    ///
    /// # Errors
    /// `String` on a malformed port/address or a non-loopback bind
    /// without `PEREGRIN_HUB_TOKEN` — fail fast.
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Test seam — parse from a key/value map instead of `std::env`.
    pub fn from_map(vars: &HashMap<String, String>) -> Result<Self, String> {
        Self::from_lookup(|key| vars.get(key).cloned())
    }

    /// Shared parser behind [`from_env`](Self::from_env) — reads the
    /// `PEREGRIN_*` names directly (`PEREGRIN_HUB_PORT` falls back to
    /// `PEREGRIN_PORT`, like the node).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut cfg = Self::default();
        if let Some(v) = get("PEREGRIN_HUB_HOST") {
            cfg.host = v;
        }
        if let Some(v) = get("PEREGRIN_HUB_PORT").or_else(|| get("PEREGRIN_PORT")) {
            cfg.port = v.parse::<u16>().map_err(|_| {
                format!("PEREGRIN_HUB_PORT/PEREGRIN_PORT must be a port number, got {v:?}")
            })?;
        }
        if let Some(v) = get("PEREGRIN_NODE_URL") {
            cfg.node_url = v.trim_end_matches('/').to_string();
        }
        cfg.node_token = get("PEREGRIN_NODE_TOKEN").filter(|t| !t.is_empty());
        cfg.hub_token = get("PEREGRIN_HUB_TOKEN").filter(|t| !t.is_empty());
        // The client-surface bind guard: a wildcard/LAN bind with no
        // hub token would expose the API.
        if !is_loopback_host(&cfg.host) && cfg.hub_token.is_none() {
            return Err(format!(
                "peregrin-hub refuses to bind {} without PEREGRIN_HUB_TOKEN. Set PEREGRIN_HUB_TOKEN or bind a loopback address (PEREGRIN_HUB_HOST=127.0.0.1).",
                cfg.host
            ));
        }
        Ok(cfg)
    }

    /// The `hypha_hub::HubConfig` view — field-for-field.
    #[must_use]
    pub fn to_hypha(&self) -> HyphaHubConfig {
        HyphaHubConfig {
            host: self.host.clone(),
            port: self.port,
            node_url: self.node_url.clone(),
            node_token: self.node_token.clone(),
            hub_token: self.hub_token.clone(),
        }
    }

    pub fn socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|_| format!("invalid hub address {}:{}", self.host, self.port))
    }
}

/// Boot the gateway state — `hypha_hub::hub_state_from_with` under
/// the hood with [`default_kind_policies`] and the `PEREGRIN_*` env
/// mapping above. Projection + outbox + push live under
/// `$PEREGRIN_HOME/hub/` exactly as before.
///
/// # Errors
/// `String` on a missing/empty node list or a projection/outbox open
/// failure — see `hypha_hub`.
pub fn hub_state(config: &HubConfig) -> Result<HubState, String> {
    hub_state_from(|k| std::env::var_os(k), config)
}

/// [`hub_state`] with the process env injected — the test seam.
#[doc(hidden)]
pub fn hub_state_from(
    get: impl Fn(&str) -> Option<OsString>,
    config: &HubConfig,
) -> Result<HubState, String> {
    // The injected getter answers the PEREGRIN_* names; translate to
    // the HYPHA_* names hypha_hub reads, resolving HYPHA_HOME through
    // the same fallback chain the node uses.
    let lookup = |k: &str| -> Option<OsString> {
        match k {
            "HYPHA_HOME" => get("PEREGRIN_HOME")
                .or_else(|| {
                    get("HOME").map(|h| {
                        PathBuf::from(h)
                            .join(".local/share/peregrin")
                            .into_os_string()
                    })
                })
                .or_else(|| Some(OsString::from(".peregrin"))),
            "HYPHA_NODES" => get("PEREGRIN_NODES"),
            "HYPHA_CREDENTIAL_STORE" => get("PEREGRIN_CREDENTIAL_STORE"),
            _ => None,
        }
    };
    hypha_hub::hub_state_from_with(lookup, &config.to_hypha(), default_kind_policies())
}

/// Build the axum router — `hypha_hub::router` verbatim.
pub fn router(state: HubState) -> axum::Router {
    hypha_hub::router(state)
}
