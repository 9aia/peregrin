//! The [`EntityApi`] port — the node daemon's entity-plane surface —
//! plus [`HttpEntityApi`], the ureq-backed client impl. `peregrin-hub`'s
//! `SyncEntityApi` is the sync-engine impl; integrations and clients
//! call through whichever `Arc<dyn EntityApi>` the host provides.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::dto::{
    CreateResultDto, EntityDto, EntityIndexDto, EntityOp, NodeInfoDto, NodeStatusDto, PendingOpDto,
    PushSubscriptionDto,
};

/// Port to the node's entity plane — implemented by [`HttpEntityApi`]
/// for direct connections and by the hub's `SyncEntityApi` (merged
/// index + durable outbox) for multi-node deployments. Deliberately
/// kind-agnostic: `Op.kind` and `EntityDto.index_doc` are owned by the
/// integration, never by this crate (ADR-0091).
#[async_trait::async_trait]
pub trait EntityApi: Send + Sync + 'static {
    /* ---- entities --------------------------------------------------*/
    /// `GET /api/entities` — every entity the node can see; `kind`
    /// filters by entity kind (`sepia-session`, `campaign`, …).
    async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String>;
    /// `GET /api/entities/{id}`.
    async fn entity(&self, id: &str) -> Result<EntityDto, String>;
    /// `POST /api/entities` — `{kind, indexDoc}` creates an entity.
    /// `node` pins the create on a multi-node hub; `None` = primary.
    async fn create_entity(
        &self,
        kind: &str,
        init: &Value,
        node: Option<&str>,
    ) -> Result<CreateResultDto, String>;
    /// `POST /api/entities/{id}/ops` — submit an op; queues in the
    /// outbox when the owning node is down.
    async fn submit_op(&self, id: &str, op: &EntityOp) -> Result<(), String>;
    /// `DELETE /api/entities/{id}`.
    async fn delete_entity(&self, id: &str) -> Result<(), String>;
    /// `GET /api/entities/{id}/snapshot` — the entity's snapshot bytes
    /// (integration-defined IR).
    async fn snapshot_export(&self, id: &str) -> Result<Value, String>;
    /// `POST /api/entities/snapshot` — resume a snapshot on a node
    /// (`node` picks the target; `None` = primary).
    async fn snapshot_import(&self, body: &Value, node: Option<&str>) -> Result<EntityDto, String>;

    /* ---- index + outbox --------------------------------------------*/
    /// `GET /api/entity-index` — per-node entity indexes (the merged
    /// projection on a hub, one row on a direct connection).
    async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String>;
    /// `GET /api/pending-ops` — queued and dead-lettered ops in the
    /// hub's durable outbox. Direct connections have no outbox and
    /// return `[]`.
    async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String>;

    /* ---- node-level (substrate-generic) ----------------------------*/
    /// `GET /api/node` — the node descriptor (primary on a hub).
    async fn node_info(&self) -> Result<NodeInfoDto, String>;
    /// `PATCH /api/node` — `{name}` renames a node; returns the
    /// updated descriptor. `node` picks the target on a multi-node hub.
    async fn rename_node(&self, name: &str, node: Option<&str>) -> Result<NodeInfoDto, String>;
    /// Per-node health rows — the sync projection on a hub, a one-row
    /// probe on a direct connection.
    async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String>;
    /// `POST /api/pair` — redeem a one-time pairing code for a
    /// long-lived node credential (`{code}` → `{token}`).
    async fn pair(&self, code: &str, node: Option<&str>) -> Result<String, String>;
    /// `GET /api/config` — the node's config document.
    async fn get_config(&self) -> Result<BTreeMap<String, Value>, String>;
    /// `PATCH /api/config/{key}` — `{value}` stored verbatim.
    async fn set_config(&self, key: &str, value: &Value) -> Result<(), String>;
    /// `GET /api/fs?path=` — direct subdirectories of an absolute
    /// path. `node` routes on a multi-node hub; `None` = primary.
    async fn fs_dirs(&self, path: &str, node: Option<String>) -> Result<Vec<String>, String>;
    /// `GET /api/push/vapid` — `{publicKey}`.
    async fn push_vapid(&self) -> Result<String, String>;
    /// `POST /api/push/subscribe` — `{endpoint, keys}`.
    async fn push_subscribe(&self, subscription: &PushSubscriptionDto) -> Result<(), String>;
    /// `DELETE /api/push/subscribe` — `{endpoint}`.
    async fn push_unsubscribe(&self, endpoint: &str) -> Result<(), String>;

    /// The entity kind a kind-less create defaults to — the first
    /// registered kind on a policy-driven api (`sepia-session` on the
    /// stock hub), `None` when the backend has no default (the node
    /// applies its own).
    fn default_entity_kind(&self) -> Option<&'static str> {
        None
    }
}

/* ---- HTTP client ------------------------------------------------*/

/// `EntityApi` backed by the node daemon's HTTP API (`ureq` — blocking
/// I/O, so each trait method hops onto `spawn_blocking`).
#[derive(Clone)]
pub struct HttpEntityApi {
    base: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl HttpEntityApi {
    /// `base` is the node's origin, e.g. `http://127.0.0.1:18787`.
    pub fn new(base: impl Into<String>, token: Option<String>) -> Self {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(15)))
            .build();
        Self {
            base: base.into().trim_end_matches('/').to_string(),
            token,
            agent: ureq::Agent::new_with_config(config),
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
    ) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        let url = format!("{}{}", self.base, path);
        // Bodiless verbs only — PATCH/POST carry JSON via `send_json`.
        let mut req = match method {
            "DELETE" => self.agent.delete(&url),
            _ => self.agent.get(&url),
        };
        if let Some(t) = &self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        req
    }

    fn send_json(
        &self,
        method: &str,
        path: &str,
        body: &Value,
    ) -> Result<ureq::http::Response<ureq::Body>, String> {
        // DELETE with a JSON body: ureq's typestate has no bodied
        // delete verb — build the raw `http::Request` and `agent.run`.
        if method == "DELETE" {
            let url = format!("{}{}", self.base, path);
            let bytes = serde_json::to_vec(body).map_err(err_msg)?;
            let mut builder = ureq::http::Request::builder().method("DELETE").uri(&url);
            if let Some(t) = &self.token {
                builder = builder.header("Authorization", format!("Bearer {t}"));
            }
            let req = builder
                .header("Content-Type", "application/json")
                .body(bytes)
                .map_err(err_msg)?;
            return self.agent.run(req).map_err(err_msg);
        }
        let mut req = match method {
            "POST" => self.agent.post(format!("{}{}", self.base, path)),
            "PATCH" => self.agent.patch(format!("{}{}", self.base, path)),
            _ => self.agent.put(format!("{}{}", self.base, path)),
        };
        if let Some(t) = &self.token {
            req = req.header("Authorization", &format!("Bearer {t}"));
        }
        req.send_json(body).map_err(err_msg)
    }

    fn get(&self, path: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
        self.request("GET", path).call().map_err(err_msg)
    }
}

fn err_msg(e: impl std::fmt::Display) -> String {
    format!("{e}")
}

fn error_of(mut res: ureq::http::Response<ureq::Body>, what: &str) -> String {
    let status = res.status().as_u16();
    let body = res.body_mut().read_to_string().unwrap_or_default();
    if body.is_empty() {
        format!("{what}: HTTP {status}")
    } else {
        format!("{what}: HTTP {status} — {body}")
    }
}

fn read_json<T: serde::de::DeserializeOwned>(
    mut res: ureq::http::Response<ureq::Body>,
    what: &str,
) -> Result<T, String> {
    if !res.status().is_success() {
        return Err(error_of(res, what));
    }
    res.body_mut()
        .read_json()
        .map_err(|e| format!("{what}: decode {e}"))
}

fn url_encode(s: &str) -> String {
    use std::fmt::Write;
    s.chars().fold(String::new(), |mut out, c| {
        if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
            out.push(c);
        } else {
            let _ = write!(out, "%{:02X}", c as u32);
        }
        out
    })
}

fn node_query(node: Option<&str>) -> String {
    node.filter(|n| !n.is_empty())
        .map_or_else(String::new, |n| format!("?node={}", url_encode(n)))
}

fn kind_query(kind: Option<&str>) -> String {
    kind.filter(|k| !k.is_empty())
        .map_or_else(String::new, |k| format!("?kind={}", url_encode(k)))
}

#[async_trait::async_trait]
impl EntityApi for HttpEntityApi {
    async fn list_entities(&self, kind: Option<&str>) -> Result<Vec<EntityDto>, String> {
        let api = self.clone();
        let kind = kind.map(str::to_string);
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct EntitiesBody {
                #[serde(default)]
                entities: Vec<EntityDto>,
            }
            let body: EntitiesBody = read_json(
                api.get(&format!("/api/entities{}", kind_query(kind.as_deref())))?,
                "list entities",
            )?;
            Ok(body.entities)
        })
        .await
        .map_err(err_msg)?
    }

    async fn entity(&self, id: &str) -> Result<EntityDto, String> {
        let api = self.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            read_json(api.get(&format!("/api/entities/{id}"))?, "get entity")
        })
        .await
        .map_err(err_msg)?
    }

    async fn create_entity(
        &self,
        kind: &str,
        init: &Value,
        node: Option<&str>,
    ) -> Result<CreateResultDto, String> {
        let api = self.clone();
        let body = serde_json::json!({ "kind": kind, "indexDoc": init });
        let q = node_query(node);
        tokio::task::spawn_blocking(move || {
            read_json(
                api.send_json("POST", &format!("/api/entities{q}"), &body)?,
                "create entity",
            )
        })
        .await
        .map_err(err_msg)?
    }

    async fn submit_op(&self, id: &str, op: &EntityOp) -> Result<(), String> {
        let api = self.clone();
        let id = id.to_string();
        let body = serde_json::to_value(op).map_err(err_msg)?;
        tokio::task::spawn_blocking(move || {
            let res = api.send_json("POST", &format!("/api/entities/{id}/ops"), &body)?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(error_of(res, "submit op"))
            }
        })
        .await
        .map_err(err_msg)?
    }

    async fn delete_entity(&self, id: &str) -> Result<(), String> {
        let api = self.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let res = api
                .request("DELETE", &format!("/api/entities/{id}"))
                .call()
                .map_err(err_msg)?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(error_of(res, "delete entity"))
            }
        })
        .await
        .map_err(err_msg)?
    }

    async fn snapshot_export(&self, id: &str) -> Result<Value, String> {
        let api = self.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            read_json(
                api.get(&format!("/api/entities/{id}/snapshot"))?,
                "snapshot export",
            )
        })
        .await
        .map_err(err_msg)?
    }

    async fn snapshot_import(&self, body: &Value, node: Option<&str>) -> Result<EntityDto, String> {
        let api = self.clone();
        let body = body.clone();
        let q = node_query(node);
        tokio::task::spawn_blocking(move || {
            read_json(
                api.send_json("POST", &format!("/api/entities/snapshot{q}"), &body)?,
                "snapshot import",
            )
        })
        .await
        .map_err(err_msg)?
    }

    async fn entity_index(&self) -> Result<Vec<EntityIndexDto>, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct IndexBody {
                #[serde(default)]
                index: Vec<EntityIndexDto>,
            }
            let body: IndexBody = read_json(api.get("/api/entity-index")?, "entity index")?;
            Ok(body.index)
        })
        .await
        .map_err(err_msg)?
    }

    async fn pending_ops(&self) -> Result<Vec<PendingOpDto>, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct OutboxBody {
                #[serde(default)]
                ops: Vec<PendingOpDto>,
            }
            let body: OutboxBody = read_json(api.get("/api/pending-ops")?, "pending ops")?;
            Ok(body.ops)
        })
        .await
        .map_err(err_msg)?
    }

    async fn node_info(&self) -> Result<NodeInfoDto, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || read_json(api.get("/api/node")?, "node info"))
            .await
            .map_err(err_msg)?
    }

    async fn rename_node(&self, name: &str, node: Option<&str>) -> Result<NodeInfoDto, String> {
        let api = self.clone();
        let body = serde_json::json!({ "name": name });
        let q = node_query(node);
        tokio::task::spawn_blocking(move || {
            read_json(
                api.send_json("PATCH", &format!("/api/node{q}"), &body)?,
                "rename node",
            )
        })
        .await
        .map_err(err_msg)?
    }

    async fn node_status(&self) -> Result<Vec<NodeStatusDto>, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || {
            // Direct connection: synthesize the one-row probe — a
            // reachable `/api/node` means the node is up.
            match api.get("/api/node") {
                Ok(mut res) if res.status().is_success() => {
                    let _ = res.body_mut().read_to_string();
                    Ok(vec![NodeStatusDto {
                        id: "node".into(),
                        url: api.base.clone(),
                        label: "node".into(),
                        status: "up".into(),
                        last_seen_at: None,
                    }])
                }
                Ok(res) => Err(error_of(res, "node status")),
                Err(e) => Ok(vec![NodeStatusDto {
                    id: "node".into(),
                    url: api.base.clone(),
                    label: "node".into(),
                    status: "down".into(),
                    last_seen_at: Some(e),
                }]),
            }
        })
        .await
        .map_err(err_msg)?
    }

    async fn pair(&self, code: &str, node: Option<&str>) -> Result<String, String> {
        let api = self.clone();
        let body = serde_json::json!({ "code": code });
        let q = node_query(node);
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct PairBody {
                token: String,
            }
            let body: PairBody = read_json(
                api.send_json("POST", &format!("/api/pair{q}"), &body)?,
                "pair",
            )?;
            Ok(body.token)
        })
        .await
        .map_err(err_msg)?
    }

    async fn get_config(&self) -> Result<BTreeMap<String, Value>, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || read_json(api.get("/api/config")?, "get config"))
            .await
            .map_err(err_msg)?
    }

    async fn set_config(&self, key: &str, value: &Value) -> Result<(), String> {
        let api = self.clone();
        let path = format!("/api/config/{}", url_encode(key));
        let body = serde_json::json!({ "value": value });
        tokio::task::spawn_blocking(move || {
            let res = api.send_json("PATCH", &path, &body)?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(error_of(res, "set config"))
            }
        })
        .await
        .map_err(err_msg)?
    }

    async fn fs_dirs(&self, path: &str, node: Option<String>) -> Result<Vec<String>, String> {
        use std::fmt::Write as _;
        let api = self.clone();
        let mut p = format!("/api/fs?path={}", url_encode(path));
        if let Some(n) = node.filter(|n| !n.is_empty()) {
            let _ = write!(p, "&node={}", url_encode(&n));
        }
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct DirsBody {
                #[serde(default)]
                dirs: Vec<String>,
            }
            let body: DirsBody = read_json(api.get(&p)?, "fs dirs")?;
            Ok(body.dirs)
        })
        .await
        .map_err(err_msg)?
    }

    async fn push_vapid(&self) -> Result<String, String> {
        let api = self.clone();
        tokio::task::spawn_blocking(move || {
            #[derive(serde::Deserialize)]
            struct VapidBody {
                #[serde(rename = "publicKey")]
                public_key: String,
            }
            let body: VapidBody = read_json(api.get("/api/push/vapid")?, "push vapid")?;
            Ok(body.public_key)
        })
        .await
        .map_err(err_msg)?
    }

    async fn push_subscribe(&self, subscription: &PushSubscriptionDto) -> Result<(), String> {
        let api = self.clone();
        let body = serde_json::to_value(subscription).map_err(err_msg)?;
        tokio::task::spawn_blocking(move || {
            let res = api.send_json("POST", "/api/push/subscribe", &body)?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(error_of(res, "push subscribe"))
            }
        })
        .await
        .map_err(err_msg)?
    }

    async fn push_unsubscribe(&self, endpoint: &str) -> Result<(), String> {
        let api = self.clone();
        let body = serde_json::json!({ "endpoint": endpoint });
        tokio::task::spawn_blocking(move || {
            let res = api.send_json("DELETE", "/api/push/subscribe", &body)?;
            if res.status().is_success() {
                Ok(())
            } else {
                Err(error_of(res, "push unsubscribe"))
            }
        })
        .await
        .map_err(err_msg)?
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn request_method_arms() {
        let api = HttpEntityApi::new("http://127.0.0.1:1/", Some("t".into()));
        let _ = api.request("GET", "/x");
        let _ = api.request("PATCH", "/x");
        let _ = api.request("DELETE", "/x");
        let _ = api.request("OTHER", "/x");
        let api = HttpEntityApi::new("http://127.0.0.1:1", None);
        let _ = api.request("GET", "/x");
    }

    #[test]
    fn encoders() {
        assert_eq!(url_encode("a b/c~d"), "a%20b%2Fc~d");
        assert_eq!(node_query(None), "");
        assert_eq!(node_query(Some("tower")), "?node=tower");
        assert_eq!(kind_query(None), "");
        assert_eq!(kind_query(Some("sepia-session")), "?kind=sepia-session");
    }

    /// The client's entity routes must match what the servers actually
    /// mount (`/api/entity-index`, `/api/pending-ops`,
    /// `/api/entities/{id}/snapshot`, `/api/entities/snapshot`) — this
    /// canned node records every request line so a drifted path fails
    /// here instead of 404ing in production.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn http_client_hits_the_served_entity_routes() {
        use std::io::{BufRead, BufReader, Write};
        use std::sync::{Arc, Mutex};

        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        {
            let calls = Arc::clone(&calls);
            std::thread::spawn(move || {
                while let Ok((mut conn, _)) = listener.accept() {
                    let calls = Arc::clone(&calls);
                    std::thread::spawn(move || {
                        let mut reader = BufReader::new(conn.try_clone().unwrap());
                        let mut request_line = String::new();
                        let _ = reader.read_line(&mut request_line);
                        calls.lock().unwrap().push(request_line.trim().to_string());
                        // Drain headers so ureq sees a clean close.
                        let mut line = String::new();
                        loop {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) | Err(_) => break,
                                Ok(_) if line.trim().is_empty() => break,
                                Ok(_) => {}
                            }
                        }
                        let body = if request_line.contains("entity-index") {
                            r#"{"index":[]}"#
                        } else if request_line.contains("pending-ops") {
                            r#"{"ops":[]}"#
                        } else if request_line.starts_with("POST") {
                            r#"{"id":"i1","kind":"sepia-session"}"#
                        } else if request_line.contains("/api/entities/")
                            && !request_line.contains("snapshot")
                        {
                            r#"{"id":"s1","kind":"sepia-session"}"#
                        } else {
                            "{}"
                        };
                        let res = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = conn.write_all(res.as_bytes());
                    });
                }
            });
        }

        let api = HttpEntityApi::new(format!("http://127.0.0.1:{port}"), None);
        api.entity_index().await.unwrap();
        api.pending_ops().await.unwrap();
        api.snapshot_export("s1").await.unwrap();
        api.snapshot_import(&serde_json::json!({}), None)
            .await
            .unwrap();
        api.entity("s1").await.unwrap();

        let calls = calls.lock().unwrap();
        let paths: Vec<&str> = calls.iter().map(|c| c.split(' ').nth(1).unwrap()).collect();
        assert_eq!(
            paths,
            [
                "/api/entity-index",
                "/api/pending-ops",
                "/api/entities/s1/snapshot",
                "/api/entities/snapshot",
                "/api/entities/s1",
            ]
        );
    }
}
