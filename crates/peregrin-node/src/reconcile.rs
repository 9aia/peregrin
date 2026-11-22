//! Campaign session reconcile — the orphan/retention sweeper that
//! runs inside `campaign serve` (P-054).
//!
//! Campaign cleanup nodes own the happy path: they record the agent
//! session id under `<worktree_base>/sessions/<key>` and append every
//! outcome to `<worktree_base>/cleanup-ledger.jsonl`. This pass covers
//! what nodes can't:
//!
//! - `retained` ledger entries past `retain_until` (failure-path
//!   debugging window, 7d by campaign convention) → delete.
//! - `failed` ledger entries (session delete errored at cleanup) →
//!   retry delete.
//! - Orphans: sessions whose cwd sits under a campaign's
//!   `worktree_base` with no ledger/registry record — the workflow
//!   died before its cleanup node ran → delete once older than
//!   `PEREGRIN_RECONCILE_GRACE_SECS` (default 1h).
//!
//! Safety: `locked`/`busy` summaries are never touched; sessions
//! outside every `worktree_base` are never candidates (human sessions
//! are invisible to this pass). Deletes go through the session plane
//! (`sessions.delete` semantics — store fallback for agents without
//! ACP `session/delete`), never raw store surgery.
//!
//! Cadence: one pass at boot, then every
//! `PEREGRIN_RECONCILE_INTERVAL_SECS` (default 900). The pass spawns
//! a fresh `sepia serve` child each run — same resolution as the node
//! daemon — so a wedged child can't poison later passes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use peregrin_session::{SepiaService, ServicePlane, SessionPlane, sepia_serve};
use sepia_session::ControlErrorCode;

/// Ledger file campaigns append cleanup outcomes to.
const LEDGER_NAME: &str = "cleanup-ledger.jsonl";
/// Per-key session-id registry dir under the worktree base.
const REGISTRY_DIR: &str = "sessions";
/// Default pass cadence.
const DEFAULT_INTERVAL: Duration = Duration::from_secs(900);
/// Default orphan grace — a session younger than this may belong to a
/// run still in flight.
const DEFAULT_GRACE: Duration = Duration::from_secs(3600);
/// Sleep slice while waiting out the interval — bounds shutdown
/// latency without a watch channel.
const SHUTDOWN_POLL: Duration = Duration::from_secs(5);

/// A campaign's reconcile roots — display name + worktree base.
pub struct ReconcileRoot {
    /// Campaign name (ledger `campaign` field on reconcile writes).
    pub name: String,
    /// The campaign's `config.worktree_base`.
    pub worktree_base: PathBuf,
}

/// One parsed `cleanup-ledger.jsonl` line — only the fields the pass
/// reads; unknown keys are ignored (campaigns own the schema).
#[derive(Debug, Default)]
struct LedgerEntry {
    session_id: Option<String>,
    outcome: String,
    retain_until: Option<i64>,
    ts: i64,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Read + parse a base's ledger — bad lines are skipped (the file is
/// campaign-owned; a torn write must not kill the pass).
fn read_ledger(base: &Path) -> Vec<LedgerEntry> {
    let path = base.join(LEDGER_NAME);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            Some(LedgerEntry {
                session_id: v
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                outcome: v
                    .get("outcome")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                retain_until: v.get("retain_until").and_then(Value::as_i64),
                ts: v.get("ts").and_then(Value::as_i64).unwrap_or(0),
            })
        })
        .collect()
}

/// Registry ids under `<base>/sessions/` → (id, file mtime secs).
fn read_registry(base: &Path) -> Vec<(String, i64)> {
    let dir = base.join(REGISTRY_DIR);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    rd.filter_map(|ent| {
        let ent = ent.ok()?;
        if !ent.file_type().ok()?.is_file() {
            return None;
        }
        let id = std::fs::read_to_string(ent.path()).ok()?.trim().to_string();
        if id.is_empty() {
            return None;
        }
        let mtime = ent
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        Some((id, mtime))
    })
    .collect()
}

/// `cwd` lives somewhere under `base` (the worktree root) — the
/// orphan-discovery rule. Component-wise so `base2` can't false-match.
fn under_base(cwd: &str, base: &Path) -> bool {
    Path::new(cwd)
        .strip_prefix(base)
        .is_ok_and(|rel| rel.components().next().is_some())
}

/// Parse a summary's `updated_at` (ISO-8601) to epoch secs; `None`
/// when absent/unparseable — callers treat that as "can't age, skip".
fn updated_at_secs(updated_at: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(updated_at)
        .ok()
        .map(|d| d.timestamp())
}

/// What the pass decided for one session.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Delete now — expired retention or failed cleanup retry.
    Delete(&'static str),
    /// Orphan candidate — only deleted when older than grace.
    Orphan,
    /// Leave alone.
    Keep,
}

/// Classify one session summary against its base's ledger + registry.
fn classify(
    cwd_under_base: bool,
    ledger_latest: Option<&LedgerEntry>,
    registered: bool,
    now: i64,
) -> Verdict {
    match ledger_latest {
        // Latest word is "retained": alive until retain_until, then
        // the debugging window closed → delete.
        Some(e) if e.outcome == "retained" => {
            if e.retain_until.is_some_and(|ru| ru <= now) {
                Verdict::Delete("retention expired")
            } else {
                Verdict::Keep
            }
        }
        // Cleanup attempted and failed → retry.
        Some(e) if e.outcome == "failed" => Verdict::Delete("cleanup retry"),
        // deleted/deleted-cwd/none/unknown → the ledger already had
        // its say; don't second-guess it.
        Some(_) => Verdict::Keep,
        // No ledger entry: a registry record or an ephemeral cwd makes
        // it an orphan candidate — age decides.
        None => {
            if registered || cwd_under_base {
                Verdict::Orphan
            } else {
                Verdict::Keep
            }
        }
    }
}

/// Append a reconcile outcome to the base's ledger — same schema the
/// campaign cleanup nodes write, so the file stays the single audit
/// trail.
fn append_ledger(base: &Path, campaign: &str, id: &str, cwd: &str, reason: &str) {
    let line = serde_json::json!({
        "ts": now_secs(),
        "campaign": campaign,
        "key": id,
        "session_id": id,
        "worktree": cwd,
        "outcome": "deleted",
        "retain_until": null,
        "detail": format!("reconcile: {reason}"),
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(base.join(LEDGER_NAME))
    {
        use std::io::Write;
        let _ = writeln!(f, "{line}");
    }
}

/// Per-root state for one pass — latest ledger entry per session id
/// plus the registry (id → mtime).
struct RootState {
    root: ReconcileRoot,
    latest: HashMap<String, LedgerEntry>,
    registry: HashMap<String, i64>,
}

#[derive(Default)]
struct PassStats {
    listed: usize,
    deleted: usize,
    kept: usize,
    errors: usize,
}

/// One reconcile pass — list sessions, classify, delete candidates.
async fn pass(plane: &ServicePlane, roots: &[ReconcileRoot], grace_secs: i64) -> PassStats {
    let mut stats = PassStats::default();
    let sessions = match plane.list_sessions(true).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("session reconcile: list failed: {e}");
            stats.errors += 1;
            return stats;
        }
    };
    stats.listed = sessions.len();
    let now = now_secs();

    // Per-root state: latest ledger entry per session id + registry
    // (id → mtime).
    let states: Vec<RootState> = roots
        .iter()
        .map(|root| {
            let mut latest: HashMap<String, LedgerEntry> = HashMap::new();
            for e in read_ledger(&root.worktree_base) {
                if let Some(id) = e.session_id.clone() {
                    latest.insert(
                        id,
                        LedgerEntry {
                            session_id: e.session_id,
                            outcome: e.outcome,
                            retain_until: e.retain_until,
                            ts: e.ts,
                        },
                    );
                }
            }
            RootState {
                root: ReconcileRoot {
                    name: root.name.clone(),
                    worktree_base: root.worktree_base.clone(),
                },
                latest,
                registry: read_registry(&root.worktree_base).into_iter().collect(),
            }
        })
        .collect();

    for summary in &sessions {
        // Find the root whose worktree base contains this cwd — a
        // session can only be ephemeral under one.
        let Some(state) = states
            .iter()
            .find(|s| under_base(&summary.cwd, &s.root.worktree_base))
        else {
            stats.kept += 1;
            continue;
        };
        if summary.locked || summary.busy {
            stats.kept += 1;
            continue;
        }
        let ledger_latest = state.latest.get(&summary.id);
        let registered = state.registry.contains_key(&summary.id);
        let verdict = classify(
            registered || under_base(&summary.cwd, &state.root.worktree_base),
            ledger_latest,
            registered,
            now,
        );
        let reason = match verdict {
            Verdict::Keep => {
                stats.kept += 1;
                continue;
            }
            Verdict::Delete(r) => r,
            Verdict::Orphan => {
                // Age gate: registry mtime first, then updated_at. No
                // age evidence → can't prove it's old → keep.
                let epoch = state
                    .registry
                    .get(&summary.id)
                    .copied()
                    .filter(|t| *t > 0)
                    .or_else(|| updated_at_secs(&summary.updated_at));
                match epoch {
                    Some(t) if now - t >= grace_secs => "orphan",
                    _ => {
                        stats.kept += 1;
                        continue;
                    }
                }
            }
        };
        match plane
            .delete_session(&summary.id, Some(summary.agent.as_str()))
            .await
        {
            Ok(()) => {}
            Err(e) if e.code == ControlErrorCode::UnknownAgent => {
                if let Err(e2) = plane
                    .delete_session_store(summary.agent.as_str(), &summary.id)
                    .await
                {
                    tracing::warn!(
                        session = %summary.id,
                        agent = %summary.agent,
                        "reconcile delete failed: {e2}"
                    );
                    stats.errors += 1;
                    continue;
                }
            }
            Err(e) => {
                tracing::warn!(
                    session = %summary.id,
                    agent = %summary.agent,
                    "reconcile delete failed: {e}"
                );
                stats.errors += 1;
                continue;
            }
        }
        append_ledger(
            &state.root.worktree_base,
            &state.root.name,
            &summary.id,
            &summary.cwd,
            reason,
        );
        tracing::info!(
            session = %summary.id,
            agent = %summary.agent,
            campaign = %state.root.name,
            reason,
            "reconcile deleted ephemeral session"
        );
        stats.deleted += 1;
    }
    stats
}

/// One pass end-to-end: resolve + spawn `sepia serve`, run
/// [`pass`]. A fresh child per pass keeps a wedged serve from
/// poisoning later runs.
async fn run_pass(roots: &[ReconcileRoot], grace: i64) -> PassStats {
    let Some(target) = sepia_serve() else {
        tracing::debug!("session reconcile: no sepia binary resolved");
        return PassStats::default();
    };
    let spec = peregrin_session::Spec {
        binary: target.binary,
        args: target.args,
        env: Vec::new(),
    };
    match SepiaService::spawn(spec).await {
        Ok(service) => pass(&ServicePlane(service), roots, grace).await,
        Err(e) => {
            tracing::warn!("session reconcile: sepia serve spawn failed: {e}");
            PassStats {
                errors: 1,
                ..PassStats::default()
            }
        }
    }
}

/// Sleep in shutdown-poll slices until the interval elapses or
/// shutdown is requested.
fn sleep_or_shutdown(shutdown: &AtomicBool, interval: Duration) {
    let mut remaining = interval;
    while remaining > Duration::ZERO && !shutdown.load(Ordering::Relaxed) {
        let slice = remaining.min(SHUTDOWN_POLL);
        std::thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
}

/// The reconcile loop — spawns a fresh `sepia serve` plane per pass.
/// Returns immediately when reconcile is disabled
/// (`PEREGRIN_RECONCILE_DISABLE=1` or interval 0) or no sepia binary
/// resolves.
///
/// # Panics
/// Only if the thread's tokio runtime fails to build (fatal anyway).
pub fn spawn(roots: Vec<ReconcileRoot>, shutdown: Arc<AtomicBool>) -> Option<JoinHandle<()>> {
    if roots.is_empty() || std::env::var("PEREGRIN_RECONCILE_DISABLE").is_ok_and(|v| v == "1") {
        return None;
    }
    let interval = std::env::var("PEREGRIN_RECONCILE_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(DEFAULT_INTERVAL, Duration::from_secs);
    if interval.is_zero() {
        return None;
    }
    let grace = std::env::var("PEREGRIN_RECONCILE_GRACE_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or_else(|| i64::try_from(DEFAULT_GRACE.as_secs()).unwrap_or(i64::MAX));
    std::thread::Builder::new()
        .name("session-reconcile".to_string())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::warn!("session reconcile: runtime build failed: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                loop {
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }
                    let stats = run_pass(&roots, grace).await;
                    if stats.deleted > 0 || stats.errors > 0 {
                        tracing::info!(
                            listed = stats.listed,
                            deleted = stats.deleted,
                            kept = stats.kept,
                            errors = stats.errors,
                            "session reconcile pass"
                        );
                    }
                    // Sleep on the blocking pool so the async sleep
                    // doesn't pin the single-thread runtime.
                    let flag = Arc::clone(&shutdown);
                    let _ = tokio::task::spawn_blocking(move || {
                        sleep_or_shutdown(&flag, interval);
                    })
                    .await;
                }
            });
        })
        .ok()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn ledger_parses_and_skips_bad_lines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(LEDGER_NAME),
            concat!(
                "{\"ts\":1,\"session_id\":\"s1\",\"outcome\":\"retained\",\"retain_until\":100}\n",
                "not json\n",
                "{\"ts\":2,\"session_id\":\"s2\",\"outcome\":\"failed\"}\n",
            ),
        )
        .unwrap();
        let entries = read_ledger(dir.path());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id.as_deref(), Some("s1"));
        assert_eq!(entries[0].retain_until, Some(100));
        assert_eq!(entries[1].outcome, "failed");
    }

    #[test]
    fn registry_reads_ids_and_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let sdir = dir.path().join(REGISTRY_DIR);
        std::fs::create_dir_all(&sdir).unwrap();
        std::fs::write(sdir.join("k1"), "sess-1\n").unwrap();
        std::fs::write(sdir.join("empty"), "  \n").unwrap();
        let reg = read_registry(dir.path());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg[0].0, "sess-1");
        assert!(reg[0].1 > 0);
    }

    #[test]
    fn under_base_matches_only_real_descendants() {
        let base = Path::new("/data/wts");
        assert!(under_base("/data/wts/abc123", base));
        assert!(under_base("/data/wts/abc123/pkg/sub", base));
        assert!(!under_base("/data/wts", base));
        assert!(!under_base("/data/wts2/abc", base));
        assert!(!under_base("/home/user/proj", base));
    }

    #[test]
    fn classify_retention_expiry() {
        let now = 1000;
        let retained_live = LedgerEntry {
            session_id: Some("s".into()),
            outcome: "retained".into(),
            retain_until: Some(now + 10),
            ts: 1,
        };
        assert_eq!(
            classify(false, Some(&retained_live), false, now),
            Verdict::Keep
        );
        let retained_expired = LedgerEntry {
            retain_until: Some(now - 1),
            ..retained_live
        };
        assert_eq!(
            classify(false, Some(&retained_expired), false, now),
            Verdict::Delete("retention expired")
        );
    }

    #[test]
    fn classify_failed_retries_and_deleted_keeps() {
        let now = 1000;
        let failed = LedgerEntry {
            session_id: Some("s".into()),
            outcome: "failed".into(),
            retain_until: None,
            ts: 1,
        };
        assert_eq!(
            classify(false, Some(&failed), false, now),
            Verdict::Delete("cleanup retry")
        );
        let deleted = LedgerEntry {
            outcome: "deleted".into(),
            ..failed
        };
        assert_eq!(classify(false, Some(&deleted), false, now), Verdict::Keep);
    }

    #[test]
    fn classify_orphan_requires_evidence() {
        let now = 1000;
        // No ledger, registered → orphan.
        assert_eq!(classify(false, None, true, now), Verdict::Orphan);
        // No ledger, no registry, no base match → keep.
        assert_eq!(classify(false, None, false, now), Verdict::Keep);
        // No ledger but cwd under a base → orphan.
        assert_eq!(classify(true, None, false, now), Verdict::Orphan);
    }

    #[test]
    fn updated_at_parses_iso() {
        let ts = updated_at_secs("2026-11-22T22:00:00Z").unwrap();
        assert!(ts > 1_700_000_000 && ts < 1_900_000_000);
        assert_eq!(updated_at_secs("not-a-date"), None);
        assert_eq!(updated_at_secs(""), None);
    }

    #[test]
    fn append_ledger_writes_parseable_line() {
        let dir = tempfile::tempdir().unwrap();
        append_ledger(dir.path(), "camp", "s1", "/wts/x", "orphan");
        let entries = read_ledger(dir.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id.as_deref(), Some("s1"));
        assert_eq!(entries[0].outcome, "deleted");
    }

    #[test]
    fn disabled_without_roots_or_flag() {
        // spawn returns None on empty roots — can't test env flags
        // safely under parallel tests (process env is shared).
        assert!(spawn(Vec::new(), Arc::new(AtomicBool::new(false))).is_none());
    }
}
