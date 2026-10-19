//! peregrin-control — the policy layer over sepia session planes:
//! who may hold a session (lock registry), when a takeover preempts
//! (arbitration), which node owns a session (routing), and how
//! multi-node lists merge. Mechanism stays in sepia — peregrin
//! decides, sepia executes.
//!
//! Every type here is pure — no I/O — so policy is testable without
//! standing up nodes. Wire calls (`sepia serve`/`/api/*`) happen in
//! `peregrin-node`/`peregrin-hub`, which consult this crate for the
//! decisions.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A node id — the unit routing decides between. (`peregrin-http`
/// uses `/api/node`'s id on the wire.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub String);

/// An actor contending for a session — a client identity on a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    pub id: String,
    pub node: NodeId,
}

/// How a holder attached — mirrors sepia's attach modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    /// Read-write attach — the exclusive lock that prompts go through.
    ReadWrite,
    /// Read-only attach — no contention, never blocks others.
    ReadOnly,
}

/// A lock currently held against a session.
#[derive(Debug, Clone)]
pub struct HeldLock {
    /// The holder's identity.
    pub holder: Actor,
    pub mode: LockMode,
    /// Last time the holder was confirmed live (attach or probe).
    pub held_since: Instant,
}

/// The lock registry — peregrin's record of who holds which session
/// on which node. The hub is the authority; nodes populate it from
/// `sessions.list(withLocks)` + attach outcomes.
#[derive(Default)]
pub struct LockRegistry {
    /// session id → active RW lock. RO attaches don't register —
    /// they never contend.
    locks: HashMap<String, HeldLock>,
}

impl LockRegistry {
    /// Record a successful RW attach.
    pub fn grant(&mut self, session: &str, holder: Actor) {
        self.locks.insert(
            session.to_string(),
            HeldLock {
                holder,
                mode: LockMode::ReadWrite,
                held_since: Instant::now(),
            },
        );
    }

    /// Drop the lock (detach/detach-all/shutdown).
    pub fn release(&mut self, session: &str, holder_id: &str) {
        if self
            .locks
            .get(session)
            .is_some_and(|l| l.holder.id == holder_id)
        {
            self.locks.remove(session);
        }
    }

    /// Who holds `session` (if anyone) — refreshed `held_since` reads
    /// come from `refresh`.
    #[must_use]
    pub fn holder(&self, session: &str) -> Option<&HeldLock> {
        self.locks.get(session)
    }

    /// Mark a holder live — probes/watchdogs keep locks from aging
    /// into takeover territory.
    pub fn refresh(&mut self, session: &str, holder_id: &str) {
        if let Some(lock) = self.locks.get_mut(session)
            && lock.holder.id == holder_id
        {
            lock.held_since = Instant::now();
        }
    }

    /// Locks stale beyond `ttl` — holders presumed dead (node dropped,
    /// agent crashed). Callers release them; policy may preempt.
    pub fn stale(&self, ttl: Duration) -> Vec<String> {
        let now = Instant::now();
        self.locks
            .iter()
            .filter(|(_, l)| now.duration_since(l.held_since) > ttl)
            .map(|(s, _)| s.clone())
            .collect()
    }
}

/// What the policy decides when `actor` wants session `session`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Free (or same-holder re-attach) — attach RW.
    Allow,
    /// Another live actor holds it — attach read-only.
    AttachReadOnly { holder: String },
    /// The holder is stale — preempt (sepia executes the takeover).
    Preempt { stale_holder: String },
    /// The session's owning node is unreachable — queue the op.
    Queue { node: NodeId },
}

/// Takeover policy — pure decisions, no side effects.
#[derive(Debug, Clone)]
pub struct Arbitration {
    /// Locks older than this are preemptable — the node's probe
    /// interval sets the bound.
    pub stale_after: Duration,
    /// Whether a contended session attaches read-only instead of
    /// erroring (false → strict single-writer).
    pub read_only_on_contention: bool,
}

impl Default for Arbitration {
    fn default() -> Self {
        Self {
            stale_after: Duration::from_secs(30),
            read_only_on_contention: true,
        }
    }
}

impl Arbitration {
    /// Decide for an attach request.
    #[must_use]
    pub fn attach(
        &self,
        registry: &LockRegistry,
        session: &str,
        actor: &Actor,
        takeover: bool,
    ) -> Decision {
        let Some(lock) = registry.holder(session) else {
            return Decision::Allow;
        };
        if lock.holder.id == actor.id {
            return Decision::Allow;
        }
        let stale = lock.held_since.elapsed() > self.stale_after;
        if takeover || stale {
            return Decision::Preempt {
                stale_holder: lock.holder.id.clone(),
            };
        }
        if self.read_only_on_contention {
            return Decision::AttachReadOnly {
                holder: lock.holder.id.clone(),
            };
        }
        Decision::Queue {
            node: lock.holder.node.clone(),
        }
    }
}

/// Where a session op should land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// To a specific live node.
    Node(NodeId),
    /// To the local plane (session lives here or is unowned).
    Local,
    /// The owning node is offline — buffer via `peregrin-outbox`.
    Offline(NodeId),
    /// Nothing owns this session.
    Unknown,
}

/// Session→node ownership merged from nodes' indexes (`peregrin-sync`
/// writes it; `peregrin-hub` reads for route + merged views).
#[derive(Default)]
pub struct Ownership {
    map: HashMap<String, NodeId>,
    live: HashMap<NodeId, bool>,
}

impl Ownership {
    /// Record that `session` is indexed on `node`.
    pub fn index(&mut self, session: &str, node: NodeId) {
        self.map.insert(session.to_string(), node);
    }

    /// Liveness a node last reported (heartbeat/`/api/node` poll).
    pub fn set_live(&mut self, node: NodeId, live: bool) {
        self.live.insert(node, live);
    }

    /// Route an op for `session`.
    #[must_use]
    pub fn route(&self, session: &str, local: &NodeId) -> Route {
        let Some(node) = self.map.get(session) else {
            return Route::Unknown;
        };
        if node == local {
            return Route::Local;
        }
        match self.live.get(node) {
            Some(true) => Route::Node(node.clone()),
            _ => Route::Offline(node.clone()),
        }
    }

    /// Sessions a node owns — the hub's merged list groups by these.
    #[must_use]
    pub fn sessions_on(&self, node: &NodeId) -> Vec<String> {
        self.map
            .iter()
            .filter(|(_, n)| *n == node)
            .map(|(s, _)| s.clone())
            .collect()
    }
}

/// A session-index row from the merged multi-node view.
#[derive(Debug, Clone)]
pub struct OwnershipRow {
    /// The session id as indexed on `node_id`.
    pub session_id: String,
    /// The node that indexed it.
    pub node_id: String,
    /// The agent that owns the session on that node.
    pub agent: String,
}

/// Resolve which node owns `session` over merged projection rows —
/// `agent` narrows when ids collide across nodes; the first match wins
/// otherwise. Pure policy: callers do the node lookup.
///
/// # Errors
/// `"Session not found"` when no row matches.
pub fn pick_owner<'a, I>(rows: I, session: &str, agent: Option<&str>) -> Result<NodeId, String>
where
    I: Iterator<Item = &'a OwnershipRow>,
{
    let mut found: Option<NodeId> = None;
    for row in rows {
        if row.session_id != session {
            continue;
        }
        if let Some(want) = agent {
            if row.agent == want {
                return Ok(NodeId(row.node_id.clone()));
            }
        } else if found.is_none() {
            found = Some(NodeId(row.node_id.clone()));
        }
    }
    found.ok_or_else(|| format!("Session not found: {session}"))
}
