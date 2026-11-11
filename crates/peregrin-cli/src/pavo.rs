//! The pavo wire surface of `tamias-driver-peregrin` — the
//! `manifest`/`views`/`query` dispatch behind the driver's `serve`
//! verb (ndjson JSON-RPC on stdio, `pavo_driver_kit::serve_stdio`).
//!
//! This driver is an **engine-ops driver**: it claims no views —
//! `views`/`view`/`render` answer empty/`ok:false` — and `query`
//! resolves `ops` (the [`crate::engine_ops`] manifest) plus
//! `cli.<verb>` engine ops. `submit`/`jobs`/`notifications` are
//! unsupported: peregrin mutations go through `cli.<verb>`/`run`,
//! never `submit`.

use pavo_driver_kit as kit;
use pavo_protocol::DriverManifest;
use pavo_protocol::wire::{self, Request, Response};
use serde_json::{Value, json};

use crate::engine_ops;

/// The driver manifest — no views (engine-ops driver); `ops`/`run`
/// are argv verbs, `serve`/`manifest` are the wire verbs `kit`
/// already declares.
fn manifest() -> DriverManifest {
    kit::Manifest::new("peregrin", "Peregrin")
        .version(env!("CARGO_PKG_VERSION"))
        .icon("🕊️")
        .verb("ops")
        .verb("run")
        .verb("capabilities")
        .build()
}

/// `manifest` verb payload — pretty-printed JSON for stdout.
pub fn manifest_json() -> String {
    kit::manifest_json(&manifest())
}

/// `capabilities` verb payload — merged for both consumers of this
/// binary. pavo's discovery checks `capabilities`/`verbs` for
/// `"pavo"`/`"serve"`; tamias's checks `types` for the content kind
/// and warns on undeclared `verbs`. One JSON answers both.
pub fn capabilities_value() -> Value {
    let m = manifest();
    let mut v = kit::capabilities_value(&m);
    v["views"] = json!(0);
    v["types"] = json!(["peregrin-bundle", "peregrin-node"]);
    // The tamias provider verbs folded into `verbs` alongside the
    // pavo surface verbs the kit already listed.
    let mut verbs: Vec<String> = v["verbs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    for verb in [
        "install",
        "deploy",
        "uninstall",
        "hooks",
        "health",
        "status",
        "logs",
        "config-schema",
        "ops",
        "run",
    ] {
        if !verbs.iter().any(|v| v == verb) {
            verbs.push(verb.to_string());
        }
    }
    v["verbs"] = json!(verbs);
    v
}

/// Dispatch one parsed request to its response.
pub fn dispatch(req: &Request) -> Response {
    match req {
        Request::Manifest => Response::ok_typed(&manifest()),
        Request::Views => Response::ok_typed(&Vec::<wire::ViewSummary>::new()),
        Request::View { view, .. } => Response::err(format!(
            "tamias-driver-peregrin claims no views (view '{view}')"
        )),
        Request::Render { kind, .. } => Response::err(format!(
            "tamias-driver-peregrin claims no renderers (kind '{kind}')"
        )),
        Request::Query { query, params } => {
            // `ops` and `cli.<verb>` are engine ops; there are no
            // view queries — unknown names fail the op.
            if query == "ops" {
                Response::ok_typed(&wire::QueryResult {
                    query: query.clone(),
                    data: engine_ops::manifest(),
                })
            } else if let Some(v) = engine_ops::op_of(query) {
                engine_ops::run_verb(&v, params)
            } else {
                Response::err(format!("unknown query: {query}"))
            }
        }
        // The protocol grew `submit`/`jobs`/`notifications` ops;
        // peregrin mutations go through `cli.<verb>`/`run`, never
        // `submit`, so they answer `ok:false` like any unsupported
        // op.
        Request::Submit { .. } | Request::Jobs | Request::Notifications => {
            Response::err("tamias-driver-peregrin implements no submit/jobs/notifications ops")
        }
    }
}

/// The `serve` verb — ndjson JSON-RPC on stdio until EOF.
pub fn serve() -> i32 {
    match kit::serve_stdio(dispatch) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("tamias-driver-peregrin serve: {e}");
            1
        }
    }
}
