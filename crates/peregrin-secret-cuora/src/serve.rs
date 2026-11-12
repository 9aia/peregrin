//! Persistent `cuora serve` client — the ndjson JSON-RPC transport
//! alternative to shelling the `cuora` CLI per call.
//!
//! `cuora serve` is one JSON request per stdin line → one JSON
//! response per stdout line (protocol `cuora-serve/1`); audit ndjson
//! stays on stderr, which this client discards (`Stdio::null`) so it
//! never pollutes the parent's stream. The child spawns lazily on
//! the first request and is reused for the client's lifetime; a
//! dead child, a closed stdout (EOF), a malformed reply line, or a
//! request timeout drops the session and the next call respawns.
//!
//! Requests serialize on a mutex — the wire is one-request-at-a-time
//! by design, and the timeout applies to the whole
//! write+read roundtrip while the lock is held.
//!
//! Value handling mirrors the CLI surface deliberately: [`CuoraServe::get`]
//! answers `Option<String>` (`None` on *any* failure — missing binary,
//! transport error, `not_found`, `denied` — so callers' fallthrough
//! chains behave identically), [`CuoraServe::put`] answers
//! `io::Result<()>`. `put` carries the value in the request's
//! `value` field — never stdin of a side channel, never argv.

use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Default per-request deadline — generous because a `[serve.pdp]`
/// consult alone may take its 5s default before the vault is even
/// touched. Override with [`CuoraServe::with_timeout`].
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long [`Drop`] waits for a clean `shutdown`-less exit (stdin
/// EOF) before killing the child.
const DROP_GRACE: Duration = Duration::from_millis(500);

/// Which cuora transport [`crate::CuoraSecrets`] uses — selected by
/// `PEREGRIN_CUORA_TRANSPORT` (`cli` default, `serve` for the
/// persistent ndjson session).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CuoraTransport {
    /// Shell `cuora get`/`put` per call — the historical behavior.
    #[default]
    Cli,
    /// Persistent `cuora serve` child speaking ndjson on stdio.
    Serve,
}

impl CuoraTransport {
    /// Read `PEREGRIN_CUORA_TRANSPORT` — only `serve` (any case)
    /// selects the serve transport; unset, `cli`, or anything else
    /// stays on the CLI so a typo can't silently change the wire.
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var("PEREGRIN_CUORA_TRANSPORT") {
            Ok(v) if v.eq_ignore_ascii_case("serve") => Self::Serve,
            _ => Self::Cli,
        }
    }
}

/// A live `cuora serve` child plus the plumbing to talk to it.
struct Session {
    child: Child,
    stdin: ChildStdin,
    /// Reply lines from the stdout reader thread — `Err` carries the
    /// reader's io failure; channel close means stdout hit EOF.
    lines: Receiver<io::Result<String>>,
}

/// Why a roundtrip failed at the transport layer (as opposed to a
/// well-formed `ok:false` reply, which is a cuora answer, not a
/// transport failure).
enum Fail {
    /// Write/read/EOF/protocol desync — the session is dead.
    Dead,
    /// The deadline passed — session dropped, caller sees the error.
    Timeout,
}

/// Persistent `cuora serve` client. Spawn is lazy — construction is
/// free; the child appears on the first request. Cloning shares the
/// child via [`Arc`] at the [`crate::CuoraSecrets`] layer; `CuoraServe`
/// itself is not `Clone`.
pub struct CuoraServe {
    /// The `cuora` binary — `PEREGRIN_CUORA_BIN` or `cuora` on PATH,
    /// same resolution as the CLI transport.
    bin: PathBuf,
    timeout: Duration,
    next_id: AtomicU64,
    session: Mutex<Option<Session>>,
}

impl CuoraServe {
    /// A client over `bin` with [`DEFAULT_TIMEOUT`].
    #[must_use]
    pub fn new(bin: PathBuf) -> Self {
        Self {
            bin,
            timeout: DEFAULT_TIMEOUT,
            next_id: AtomicU64::new(1),
            session: Mutex::new(None),
        }
    }

    /// Override the per-request deadline (tests use tiny values).
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// `get` — the secret value, or `None` on *any* failure.
    ///
    /// `Ok` replies yield `result.value` (a JSON string); empty
    /// values answer `None`, matching the CLI transport's
    /// empty-stdout rule. `not_found`, `denied`, transport errors,
    /// and a missing binary all collapse to `None` — "not here" —
    /// so secret-chain fallthrough is identical across transports.
    pub fn get(&self, rref: &str) -> Option<String> {
        let reply = self.call(&json!({
            "op": "get",
            "ref": rref,
            "actor": "peregrin",
        }));
        let value = reply
            .ok()?
            .get("result")?
            .get("value")?
            .as_str()?
            .to_string();
        (!value.is_empty()).then_some(value)
    }

    /// `put` — store `value` under `rref`. The value rides the
    /// request's `value` field (the wire's designed channel — the
    /// parent owns the pipe), capped at 64 KiB by cuora itself.
    pub fn put(&self, rref: &str, value: &str) -> io::Result<()> {
        let reply = self
            .call(&json!({
                "op": "put",
                "ref": rref,
                "value": value,
                "actor": "peregrin",
            }))
            .map_err(io::Error::other)?;
        if reply.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "cuora serve put {rref}: {}",
                reply_error(&reply)
            )))
        }
    }

    /// `check` — the config + declared-surface report, `result`
    /// verbatim. Useful for diagnostics; not on the secret path.
    pub fn check(&self) -> io::Result<Value> {
        let reply = self
            .call(&json!({"op": "check", "actor": "peregrin"}))
            .map_err(io::Error::other)?;
        if reply.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(reply.get("result").cloned().unwrap_or(Value::Null))
        } else {
            Err(io::Error::other(format!(
                "cuora serve check: {}",
                reply_error(&reply)
            )))
        }
    }

    /// One request → one reply. Transport failures retry **once** on
    /// a freshly spawned child (a request that never got answered is
    /// safe to replay — `get` is idempotent and `put`/`rotate`
    /// re-putting the same value is a no-op semantically); a parsed
    /// reply — `ok` or `ok:false` — returns as-is.
    fn call(&self, request: &Value) -> Result<Value, String> {
        let mut req = request.clone();
        // `id` is echoed verbatim for correlation; absent → null is
        // legal, but a real id makes a desynced wire diagnosable.
        req["id"] = json!(self.next_id.fetch_add(1, Ordering::Relaxed));
        let line = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        // Three attempts covers detect-dead → respawn-dies-mid-
        // request → respawn-healthy; a request that keeps killing
        // children is a broken `cuora`, not a transient.
        let mut last_err = String::new();
        for _ in 0..3 {
            match self.roundtrip(&line) {
                Ok(reply) => return Ok(reply),
                Err(Fail::Dead) => {
                    self.drop_session();
                    last_err = "cuora serve child died".to_string();
                }
                Err(Fail::Timeout) => {
                    self.drop_session();
                    return Err(format!(
                        "cuora serve {} timed out after {:?}",
                        request.get("op").and_then(Value::as_str).unwrap_or("?"),
                        self.timeout
                    ));
                }
            }
        }
        Err(last_err)
    }

    /// Write `line` + `\n`, flush, read one reply line, parse it.
    /// Any transport-level failure marks the session dead.
    fn roundtrip(&self, line: &str) -> Result<Value, Fail> {
        let mut guard = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_none() {
            *guard = Some(self.spawn().map_err(|_| Fail::Dead)?);
        }
        let Some(session) = guard.as_mut() else {
            return Err(Fail::Dead);
        };
        session.exchange(line, self.timeout)
    }

    /// Spawn `cuora serve` — stdin/stdout piped, stderr discarded
    /// (audit ndjson is cuora's own stream; inheriting it would
    /// interleave with the parent's). A stdout reader thread feeds
    /// reply lines into a channel so [`Session::exchange`] can apply
    /// the timeout without blocking the read itself.
    fn spawn(&self) -> io::Result<Session> {
        let mut child = Command::new(&self.bin)
            // `--actor` seeds the audit default; each request also
            // carries `actor` explicitly (the serve wire's field).
            .args(["--actor", "peregrin", "serve"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("cuora serve spawned without stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("cuora serve spawned without stdout"))?;
        let (tx, lines) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut buf = String::new();
            loop {
                buf.clear();
                match reader.read_line(&mut buf) {
                    // EOF or a read error — the child's stdout is
                    // gone; channel close tells the client.
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(Ok(buf.clone())).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Ok(Session {
            child,
            stdin,
            lines,
        })
    }

    /// Kill + reap the current session, if any. Dropping the
    /// `ChildStdin` first gives cuora its EOF → clean exit; we then
    /// wait briefly before killing so `Drop` stays bounded.
    fn drop_session(&self) {
        let mut guard = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(mut session) = guard.take() {
            reap(&mut session.child);
        }
    }
}

impl Session {
    /// One request/response on the live child. `try_wait` catches an
    /// already-exited child before we write into a dead pipe; the
    /// channel states (timeout / disconnect / reader error) catch
    /// everything else.
    fn exchange(&mut self, line: &str, timeout: Duration) -> Result<Value, Fail> {
        // Already reaped-exited — dead before we even write.
        if let Ok(Some(_)) = self.child.try_wait() {
            return Err(Fail::Dead);
        }
        if self
            .stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush())
            .is_err()
        {
            return Err(Fail::Dead);
        }
        match self.lines.recv_timeout(timeout) {
            Ok(Ok(reply)) => serde_json::from_str(&reply).map_err(|_| Fail::Dead),
            // Reader thread hit an io error or stdout EOF.
            Ok(Err(_)) | Err(RecvTimeoutError::Disconnected) => Err(Fail::Dead),
            Err(RecvTimeoutError::Timeout) => Err(Fail::Timeout),
        }
    }
}

/// Close stdin (EOF → cuora exits cleanly), wait up to [`DROP_GRACE`],
/// then kill + reap. Bounded — a wedged child can't hang drop.
fn reap(child: &mut Child) {
    // `stdin` was moved into the Session; dropping the Session
    // closes it. Here we only own `child` — close is implicit when
    // the Session dropped. Wait briefly, then kill.
    let deadline = Instant::now() + DROP_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
}

impl Drop for CuoraServe {
    fn drop(&mut self) {
        // Take the session out so its `ChildStdin` closes (EOF) —
        // `cuora serve` exits cleanly on stdin EOF — then reap.
        if let Ok(mut guard) = self.session.lock() {
            if let Some(mut session) = guard.take() {
                drop(session.stdin);
                reap(&mut session.child);
            }
        }
    }
}

impl fmt::Debug for CuoraServe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CuoraServe")
            .field("bin", &self.bin)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
}

/// `"code: message"` for an `ok:false` reply — diagnostics only.
fn reply_error(reply: &Value) -> String {
    reply.get("error").map_or_else(
        || "no error detail".to_string(),
        |e| {
            format!(
                "{}: {}",
                e.get("code").and_then(Value::as_str).unwrap_or("?"),
                e.get("message").and_then(Value::as_str).unwrap_or("?")
            )
        },
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// A fake `cuora` speaking the ndjson serve wire. Requests are
    /// compact single-line JSON we generate, so `grep -o` field
    /// extraction is exact. State lives in `$DIR/db-<key>` files;
    /// `$DIR/spawns` counts child invocations (spawn-on-demand and
    /// respawn tests read it). `$DIR/die-once` makes the *next*
    /// spawn consume one line then exit without replying — EOF
    /// mid-request. `$DIR/die-after` makes the *running* child exit
    /// right after its next reply — death between requests.
    /// `$DIR/hang` makes every spawn sleep forever — the timeout
    /// trigger.
    fn stub(dir: &std::path::Path, extra: &str) -> PathBuf {
        let script = format!(
            "#!/bin/sh\n\
             dir='{dir}'\n\
             echo x >> \"$dir/spawns\"\n\
             {extra}\n\
             if [ -f \"$dir/die-once\" ]; then\n\
               rm \"$dir/die-once\"\n\
               IFS= read -r _line\n\
               exit 0\n\
             fi\n\
             if [ -f \"$dir/hang\" ]; then exec sleep 60; fi\n\
             while IFS= read -r line; do\n\
               op=$(printf '%s' \"$line\" | grep -o '\"op\":\"[a-z]*\"' | cut -d'\"' -f4)\n\
               ref=$(printf '%s' \"$line\" | grep -o '\"ref\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               val=$(printf '%s' \"$line\" | grep -o '\"value\":\"[^\"]*\"' | cut -d'\"' -f4)\n\
               key=${{ref##*/}}\n\
               case \"$op\" in\n\
                 get)\n\
                   f=\"$dir/db-$key\"\n\
                   if [ -f \"$f\" ]; then\n\
                     v=$(cat \"$f\")\n\
                     printf '{{\"id\":1,\"ok\":true,\"result\":{{\"ref\":\"%s\",\"value\":\"%s\"}}}}\\n' \"$ref\" \"$v\"\n\
                   else\n\
                     printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"not_found\",\"message\":\"absent\"}}}}\\n'\n\
                   fi ;;\n\
                 put)\n\
                   printf '%s' \"$val\" > \"$dir/db-$key\"\n\
                   printf '{{\"id\":1,\"ok\":true,\"result\":{{\"ref\":\"%s\",\"version\":1}}}}\\n' \"$ref\" ;;\n\
                 check)\n\
                   printf '{{\"id\":1,\"ok\":true,\"result\":{{\"ok\":true,\"vaults\":[]}}}}\\n' ;;\n\
                 shutdown)\n\
                   printf '{{\"id\":1,\"ok\":true,\"result\":{{\"shutdown\":true}}}}\\n'\n\
                   exit 0 ;;\n\
                 *) printf '{{\"id\":1,\"ok\":false,\"error\":{{\"code\":\"unknown_op\",\"message\":\"x\"}}}}\\n' ;;\n\
               esac\n\
               if [ -f \"$dir/die-after\" ]; then\n\
                 rm \"$dir/die-after\"\n\
                 exit 0\n\
               fi\n\
             done\n",
            dir = dir.display(),
            extra = extra,
        );
        let bin = dir.join("cuora-stub.sh");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    fn spawns(dir: &std::path::Path) -> usize {
        std::fs::read_to_string(dir.join("spawns")).map_or(0, |s| s.lines().count())
    }

    #[test]
    fn transport_defaults_to_cli() {
        // The env var can't be set in edition-2024 tests — assert the
        // parse rule on the value space instead: only "serve" flips.
        assert_eq!(CuoraTransport::default(), CuoraTransport::Cli);
        for v in ["cli", "", "SERVE-ish", "bogus"] {
            // eq_ignore_ascii_case check, mirrored here so a refactor
            // of from_env can't silently widen the match.
            assert_ne!(v.to_ascii_lowercase(), "serve", "{v}");
        }
    }

    #[test]
    fn spawn_is_lazy_and_get_put_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "");
        let serve = CuoraServe::new(bin);
        // Construction spawned nothing.
        assert_eq!(spawns(dir.path()), 0);
        // Miss → None, and only now does the child exist.
        assert_eq!(serve.get("cuora://peregrin/MISSING"), None);
        assert_eq!(spawns(dir.path()), 1);
        // put then get — one child serves both.
        serve.put("cuora://peregrin/NAME", "s3cret").unwrap();
        assert_eq!(
            serve.get("cuora://peregrin/NAME").as_deref(),
            Some("s3cret")
        );
        assert_eq!(spawns(dir.path()), 1, "the child is reused");
        // check rides the same session.
        let report = serve.check().unwrap();
        assert_eq!(report["ok"], true);
        assert_eq!(spawns(dir.path()), 1);
    }

    #[test]
    fn respawns_after_child_death() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "");
        let serve = CuoraServe::new(bin);
        serve.put("cuora://peregrin/A", "one").unwrap();
        assert_eq!(spawns(dir.path()), 1);
        // `die-after`: the running child answers this get, then
        // exits — death *between* requests.
        std::fs::write(dir.path().join("die-after"), "").unwrap();
        assert_eq!(serve.get("cuora://peregrin/A").as_deref(), Some("one"));
        assert_eq!(spawns(dir.path()), 1, "the reply landed before exit");
        // Next call finds the child dead → respawn. `die-once` arms
        // that respawn to eat the retried line and die *mid-request*
        // (EOF, no reply) → a second respawn serves it.
        std::fs::write(dir.path().join("die-once"), "").unwrap();
        assert_eq!(serve.get("cuora://peregrin/A").as_deref(), Some("one"));
        assert_eq!(spawns(dir.path()), 3, "dead-detect + EOF-die + serve");
        // And the session is healthy again — no more spawns.
        serve.put("cuora://peregrin/B", "two").unwrap();
        assert_eq!(serve.get("cuora://peregrin/B").as_deref(), Some("two"));
        assert_eq!(spawns(dir.path()), 3);
    }

    #[test]
    fn timeout_maps_to_none_and_err() {
        let dir = tempfile::tempdir().unwrap();
        let bin = stub(dir.path(), "");
        std::fs::write(dir.path().join("hang"), "").unwrap();
        let serve = CuoraServe::new(bin).with_timeout(Duration::from_millis(100));
        assert_eq!(serve.get("cuora://peregrin/X"), None);
        assert!(serve.put("cuora://peregrin/X", "v").is_err());
        assert!(serve.check().is_err());
    }

    #[test]
    fn missing_binary_is_none_and_err_not_panic() {
        let serve = CuoraServe::new(PathBuf::from("/nonexistent/cuora"));
        assert_eq!(serve.get("cuora://peregrin/X"), None);
        assert!(serve.put("cuora://peregrin/X", "v").is_err());
    }
}
