//! `driver: git` — lifecycle tests over real temp repos, matching
//! `peregrin-git`'s suite style (the crate exists to wrap process git,
//! so tests shell out rather than fake the seam). Every invoke gets
//! an explicit `params.base` — the default points at the real
//! `~/.local/share/peregrin/worktrees`.
#![allow(clippy::unwrap_used)]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use peregrin_core::{Event, Node};
use peregrin_driver::{Driver, DriverError, ExecutionContext, NoopDriver};
use peregrin_driver_git::{DriverLookup, GitDriver, cache_key};
use peregrin_driver_shell::ShellDriver;
use peregrin_template::Effects;
use serde_json::{Value, json};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------------

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repo with two commits and a `sub/` dir — `(repo, first_sha, head_sha)`.
fn fixture_repo(root: &Path) -> (PathBuf, String, String) {
    let repo = root.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-b", "main"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    fs::write(repo.join("f"), "one").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "one"]);
    let first = git(&repo, &["rev-parse", "HEAD"]);
    fs::create_dir_all(repo.join("sub")).unwrap();
    fs::write(repo.join("sub").join("g"), "two").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "two"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    (repo, first, head)
}

fn node(params: Value) -> Node {
    Node {
        id: "n".into(),
        driver: "git".into(),
        params,
        on: vec![],
        ..Default::default()
    }
}

fn shell_lookup() -> HashMap<String, Arc<dyn Driver>> {
    HashMap::from_iter([(
        "shell".to_string(),
        Arc::new(ShellDriver::new()) as Arc<dyn Driver>,
    )])
}

fn ctx() -> ExecutionContext {
    ExecutionContext::default()
}

fn base(tmp: &TempDir) -> String {
    tmp.path().join("worktrees").display().to_string()
}

/// Standard params over a local repo: url-less `repo` source.
fn params(repo: &Path, base: &str, sha: &str, inner: &Value) -> Value {
    json!({
        "repo": repo.display().to_string(),
        "sha": sha,
        "base": base,
        "inner": inner,
    })
}

fn shell_inner(cmd: &str) -> Value {
    json!({"driver": "shell", "params": {"cmd": cmd}})
}

// ---------------------------------------------------------------------------
// api surface + param validation
// ---------------------------------------------------------------------------

#[test]
fn api_surface() {
    let d = GitDriver::new(shell_lookup());
    assert_eq!(d.name(), "git");
    let input = d.input().unwrap();
    let compiled = input.compiled_schema().unwrap();
    assert_eq!(compiled["required"], json!(["sha", "inner"]));
    assert_eq!(d.output().len(), 2);
    assert_eq!(d.output()[0].kind, "*.done");
    assert_eq!(d.output()[1].kind, "node.failed");

    // The other constructors and the public cache key.
    let _d = GitDriver::with_git(peregrin_git::SystemGit::new(), shell_lookup());
    let map: HashMap<String, Arc<dyn Driver>> = shell_lookup();
    let lookup: Arc<dyn DriverLookup> = Arc::new(map);
    let _d = GitDriver::with_parts(
        peregrin_git::SystemGit::new(),
        lookup,
        Arc::new(peregrin_template::SystemEffects),
    );
    let key = cache_key("ssh://git@example.com/repo.git");
    assert!(key.starts_with("ssh---git-example-com-r"));
    assert_ne!(cache_key("a"), cache_key("b"));
    assert_eq!(cache_key("same"), cache_key("same"));
}

#[test]
fn missing_params_are_driver_errors() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);

    let err = d.invoke(&node(json!({})), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("missing 'params.url' or 'params.repo'"));

    let err = d
        .invoke(
            &node(json!({"repo": repo.display().to_string()})),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("missing 'params.sha'"));

    let err = d
        .invoke(&node(params(&repo, &b, &first, &json!(5))), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("params.inner"));

    let err = d
        .invoke(&node(params(&repo, &b, &first, &json!({}))), &[], &ctx)
        .unwrap_err();
    assert!(err.reason.contains("params.inner"));

    let err = d
        .invoke(
            &node(params(
                &repo,
                &b,
                &first,
                &json!({"params": {"cmd": "true"}}),
            )),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(err.reason.contains("missing 'driver'"));

    let err = d
        .invoke(
            &node(params(&repo, &b, &first, &json!({"driver": "bogus"}))),
            &[],
            &ctx,
        )
        .unwrap_err();
    assert!(
        err.reason
            .contains("no driver registered for inner 'bogus'")
    );
}

#[test]
fn dry_run_emits_without_side_effects() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let dry = ExecutionContext {
        dry_run: true,
        ..ExecutionContext::default()
    };
    let b = base(&tmp);
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &shell_inner("exit 1"))),
            &[],
            &dry,
        )
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["dry_run"], true);
    assert_eq!(out[0].payload["sha"], first);
    // Nothing materialized — no base dir, no manifest.
    assert!(!tmp.path().join("worktrees").exists());
}

// ---------------------------------------------------------------------------
// happy paths
// ---------------------------------------------------------------------------

#[test]
fn local_repo_runs_inner_shell_in_worktree() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);
    // pwd is the worktree; HEAD is detached at the requested sha.
    let inner = shell_inner(
        "test \"$(pwd)\" = \"$PERE_WORKTREE\" && test \"$(git rev-parse HEAD)\" = \"$PERE_SHA\"",
    );
    let out = d
        .invoke(&node(params(&repo, &b, &first, &inner)), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["sha"], first);
    assert_eq!(out[0].payload["inner_rc"], 0);
    assert_ne!(out[0].payload["sha"], head);
    // Destroyed on completion — the reported path no longer exists.
    let worktree = out[0].payload["worktree"].as_str().unwrap();
    assert!(!Path::new(worktree).exists());
}

#[test]
fn emits_first_declares_done_type() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let mut n = node(params(&repo, &base(&tmp), &first, &shell_inner("true")));
    n.emits = vec!["gate.done".to_string()];
    let out = d.invoke(&n, &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "gate.done");
}

#[test]
fn inner_rc_maps_through_emit() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);

    // Exact rc arm.
    let mut p = params(&repo, &b, &first, &shell_inner("exit 3"));
    p["emit"] = json!({"3": "three.seen", "default": "d.seen"});
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "three.seen");
    assert_eq!(out[0].payload["inner_rc"], 3);

    // `default` arm.
    let mut p = params(&repo, &b, &first, &shell_inner("exit 9"));
    p["emit"] = json!({"3": "three.seen", "default": "d.seen"});
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "d.seen");

    // No emit → rc != 0 is `node.failed` with inner_rc carried.
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &shell_inner("exit 3"))),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["inner_rc"], 3);
    assert_eq!(out[0].payload["node"], "n");
    assert!(out[0].payload["error"].as_str().unwrap().contains("shell"));
}

#[test]
fn string_inner_and_noop() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let mut lookup = shell_lookup();
    lookup.insert("noop".to_string(), Arc::new(NoopDriver));
    let d = GitDriver::new(lookup);
    let out = d
        .invoke(
            &node(params(&repo, &base(&tmp), &first, &json!("noop"))),
            &[],
            &ctx(),
        )
        .unwrap();
    // noop's `node.done` carries no rc — inner_rc collapses to 0.
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["inner_rc"], 0);
}

/// A canned inner driver — returns a fixed event list or an error.
struct StubInner {
    name: &'static str,
    result: Result<Vec<Event>, String>,
}

impl Driver for StubInner {
    fn name(&self) -> &'static str {
        self.name
    }

    fn invoke(
        &self,
        _node: &Node,
        _triggers: &[Event],
        _ctx: &ExecutionContext,
    ) -> Result<Vec<Event>, DriverError> {
        self.result.clone().map_err(DriverError::new)
    }
}

fn stub_lookup(name: &'static str, result: Result<Vec<Event>, String>) -> impl DriverLookup {
    let driver: Arc<dyn Driver> = Arc::new(StubInner { name, result });
    move |n: &str| (n == name).then(|| driver.clone())
}

#[test]
fn inner_event_kinds_collapse_to_rc() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let b = base(&tmp);

    // `.failed`-kinded last event without an rc → inner_rc 1.
    let d = GitDriver::new(stub_lookup(
        "stub",
        Ok(vec![Event::new("x.failed", json!({}), "s")]),
    ));
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &json!("stub"))),
            &[],
            &ctx(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["inner_rc"], 1);

    // A no-rc, non-failed last event → rc 0 → done.
    let d = GitDriver::new(stub_lookup(
        "stub",
        Ok(vec![Event::new("x.done", json!({"note": "hi"}), "s")]),
    ));
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &json!("stub"))),
            &[],
            &ctx(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["inner_rc"], 0);

    // No events at all → rc 0.
    let d = GitDriver::new(stub_lookup("stub", Ok(Vec::new())));
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &json!("stub"))),
            &[],
            &ctx(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["inner_rc"], 0);
}

#[test]
fn inner_error_propagates_and_worktree_is_destroyed() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let b = base(&tmp);
    let d = GitDriver::new(stub_lookup("stub", Err("inner blew up".to_string())));
    let err = d
        .invoke(
            &node(params(&repo, &b, &first, &json!("stub"))),
            &[],
            &ctx(),
        )
        .unwrap_err();
    assert!(err.reason.contains("inner blew up"));
    // The worktree came and went — nothing tracked remains.
    let w = peregrin_git::Worktrees::new(&b);
    assert_eq!(w.list().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// inner params: cwd + env injection
// ---------------------------------------------------------------------------

#[test]
fn relative_inner_cwd_joins_the_worktree() {
    let tmp = TempDir::new().unwrap();
    // `sub/` lands in the second commit — check out `head`.
    let (repo, _, head) = fixture_repo(tmp.path());
    let inner = json!({"driver": "shell", "params": {
        "cmd": "test \"$(pwd)\" = \"$PERE_WORKTREE/sub\" && test -f g",
        "cwd": "sub",
    }});
    let d = GitDriver::new(shell_lookup());
    let out = d
        .invoke(
            &node(params(&repo, &base(&tmp), &head, &inner)),
            &[],
            &ctx(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
}

#[test]
fn inner_env_declared_wins_and_non_object_is_replaced() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let b = base(&tmp);
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();

    // Inner-declared env keys win over the injected bindings.
    let inner = json!({"driver": "shell", "params": {
        "cmd": "test \"$PERE_SHA\" = mine",
        "env": {"PERE_SHA": "mine"},
    }});
    let out = d
        .invoke(&node(params(&repo, &b, &first, &inner)), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "n.done");

    // A non-object `env` is replaced, not silently dropped.
    let inner = json!({"driver": "shell", "params": {
        "cmd": "test -n \"$PERE_WORKTREE\" && test -n \"$PERE_REPO\" && test -n \"$PERE_CLONE_URL\"",
        "env": "bogus",
    }});
    let out = d
        .invoke(&node(params(&repo, &b, &first, &inner)), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
}

#[test]
fn absolute_inner_cwd_stays_in_the_worktree() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let inner = json!({"driver": "shell", "handler": "h", "emits": ["inner.done"], "params": {
        "cmd": "test \"$(pwd)\" = \"$PERE_WORKTREE\"",
        "cwd": "/etc",
    }});
    let d = GitDriver::new(shell_lookup());
    let out = d
        .invoke(
            &node(params(&repo, &base(&tmp), &first, &inner)),
            &[],
            &ctx(),
        )
        .unwrap();
    assert_eq!(out[0].kind, "n.done");
}

// ---------------------------------------------------------------------------
// clone source: local dir vs clone cache
// ---------------------------------------------------------------------------

#[test]
fn url_source_clones_into_cache_and_reuses_it() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);
    let url = format!("file://{}", repo.display());

    let p = |inner: Value| json!({"url": url, "sha": first, "base": b, "inner": inner});
    let out = d.invoke(&node(p(shell_inner("true"))), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
    // The bare clone cached under `<base>/.repos/<key>`.
    let cache = Path::new(&b).join(".repos").join(cache_key(&url));
    assert!(cache.is_dir());
    assert_eq!(git(&cache, &["rev-parse", "--is-bare-repository"]), "true");

    // Second run hits the cache — fetch refreshes, new sha resolves.
    let p2 = json!({"url": url, "sha": head, "base": b, "inner": shell_inner(
        "test \"$(git rev-parse HEAD)\" = \"$PERE_SHA\"")});
    let out = d.invoke(&node(p2), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["sha"], head);
}

#[test]
fn repo_dir_that_is_not_a_repo_fails() {
    let tmp = TempDir::new().unwrap();
    let plain = tmp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    let d = GitDriver::new(shell_lookup());
    let p = json!({
        "repo": plain, "sha": "x", "base": base(&tmp), "inner": shell_inner("true"),
    });
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("not a git repository")
    );
}

#[test]
fn clone_failure_and_bad_sha_are_node_failed() {
    let tmp = TempDir::new().unwrap();
    let (repo, _, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);

    let p = json!({
        "url": "file:///definitely/not/a/repo",
        "sha": "x", "base": b, "inner": shell_inner("true"),
    });
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(out[0].payload["error"].as_str().unwrap().contains("clone"));

    let p = json!({
        "repo": repo, "sha": "0000000000000000000000000000000000000000",
        "base": b, "inner": shell_inner("true"),
    });
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("not resolvable")
    );
}

#[test]
fn clone_cache_failures() {
    let tmp = TempDir::new().unwrap();
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);
    let url = "file:///definitely/not/a/repo";
    let p =
        |base: &str| json!({"url": url, "sha": "x", "base": base, "inner": shell_inner("true")});

    // `.repos` exists as a file — the cache dir can't be created.
    let wt = tmp.path().join("wt-file");
    fs::create_dir_all(&wt).unwrap();
    fs::write(wt.join(".repos"), "x").unwrap();
    let out = d
        .invoke(&node(p(&wt.display().to_string())), &[], &ctx)
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("clone cache")
    );

    // The cache slot itself is occupied by a non-repo path.
    fs::create_dir_all(Path::new(&b).join(".repos")).unwrap();
    fs::write(Path::new(&b).join(".repos").join(cache_key(url)), "x").unwrap();
    let out = d.invoke(&node(p(&b)), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("occupied")
    );
}

// ---------------------------------------------------------------------------
// worktree dest: naming, occupied, adopted
// ---------------------------------------------------------------------------

#[test]
fn dest_naming() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let ctx = ExecutionContext {
        idempotency_key: "n:1".to_string(),
        ..ExecutionContext::default()
    };

    // `<idempotency>-<sha12>` with `:` sanitized.
    let p = params(&repo, &b, &first, &shell_inner("true"));
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
    let worktree = out[0].payload["worktree"].as_str().unwrap();
    assert!(
        worktree.ends_with(&format!("n-1-{}", &first[..12])),
        "dest: {worktree}"
    );

    // `params.dest` wins verbatim.
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["dest"] = json!("custom-dest");
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert!(
        out[0].payload["worktree"]
            .as_str()
            .unwrap()
            .ends_with("custom-dest")
    );

    // No idempotency key and no node id → `run-<sha12>`.
    let plain = ExecutionContext::default();
    let mut n = node(params(&repo, &b, &first, &shell_inner("true")));
    n.id = String::new();
    let out = d.invoke(&n, &[], &plain).unwrap();
    assert!(
        out[0].payload["worktree"]
            .as_str()
            .unwrap()
            .contains(&format!("run-{}", &first[..12]))
    );
}

#[test]
fn occupied_dest_is_node_failed() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    // A plain file sits where the worktree would land.
    let dest = Path::new(&b).join(format!("n-1-{}", &first[..12]));
    fs::create_dir_all(&b).unwrap();
    fs::write(&dest, "occupied").unwrap();
    let ctx = ExecutionContext {
        idempotency_key: "n:1".to_string(),
        ..ExecutionContext::default()
    };
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &shell_inner("true"))),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("occupied")
    );
    // The foreign file is untouched.
    assert_eq!(fs::read_to_string(&dest).unwrap(), "occupied");
}

#[test]
fn adopted_checkout_at_wrong_sha_fails() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    // A foreign checkout at the dest — `create` adopts it, but its
    // head is `head`, not the requested `first`.
    let dest = Path::new(&b).join(format!("n-1-{}", &first[..12]));
    fs::create_dir_all(&b).unwrap();
    git(
        &repo,
        &["worktree", "add", "--detach", dest.to_str().unwrap(), &head],
    );
    let ctx = ExecutionContext {
        idempotency_key: "n:1".to_string(),
        ..ExecutionContext::default()
    };
    let out = d
        .invoke(
            &node(params(&repo, &b, &first, &shell_inner("true"))),
            &[],
            &ctx,
        )
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("detached at")
    );
    // Left for `Worktrees::cleanup`, not destroyed out from under
    // whoever made it.
    assert!(dest.is_dir());
    git(
        &repo,
        &["worktree", "remove", "--force", dest.to_str().unwrap()],
    );
}

// ---------------------------------------------------------------------------
// keep vs cleanup
// ---------------------------------------------------------------------------

#[test]
fn keep_retains_the_worktree() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["keep"] = json!(true);
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "n.done");
    let worktree = PathBuf::from(out[0].payload["worktree"].as_str().unwrap());
    assert!(worktree.is_dir());
    // Marked kept so `cleanup()` won't take it.
    let w = peregrin_git::Worktrees::new(&b);
    assert!(w.kept(&worktree));
    assert_eq!(w.cleanup().unwrap().len(), 0);
}

#[test]
fn cleanup_destroy_failure_is_node_failed() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    // The inner script deletes the worktree dir and leaves a plain
    // file in its place — `destroy` then fails on every arm.
    let inner = shell_inner(
        "cd \"$PERE_WORKTREE/..\" && rm -rf \"$PERE_WORKTREE\" && printf x > \"$PERE_WORKTREE\"",
    );
    let out = d
        .invoke(&node(params(&repo, &b, &first, &inner)), &[], &ctx())
        .unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("cleanup failed")
    );
}

#[test]
fn keep_failure_is_node_failed() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    // Corrupt the manifest from inside the run — `keep` then fails
    // loading it, so the worktree is left behind for debugging.
    let inner = shell_inner("printf '{' > \"$PERE_WORKTREE/../.peregrin-worktrees.json\"");
    let mut p = params(&repo, &b, &first, &inner);
    p["keep"] = json!("yes");
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("cleanup failed")
    );
}

// ---------------------------------------------------------------------------
// credential binding
// ---------------------------------------------------------------------------

/// Effects with one ambient var — the env-name fallback path.
struct AmbientEffects;

impl Effects for AmbientEffects {
    fn env(&self, name: &str) -> Option<String> {
        (name == "AMBIENT_TOKEN").then(|| "ambient-value".to_string())
    }
    fn cwd(&self) -> PathBuf {
        PathBuf::from("/")
    }
    fn read_file(&self, _path: &Path) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
    fn shell(&self, _command: &str) -> Result<String, String> {
        Ok(String::new())
    }
    fn now_unix(&self) -> i64 {
        0
    }
    fn now_iso(&self) -> String {
        String::new()
    }
    fn random(&self) -> f64 {
        0.0
    }
    fn uuid(&self) -> String {
        String::new()
    }
}

#[test]
fn credential_names_resolve_to_git_config_args() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let b = base(&tmp);
    let url = format!("file://{}", repo.display());

    // Secrets record + ambient env both resolve; the resolved `-c`
    // args ride the clone call (file:// ignores them — the arg
    // building is what's under test).
    let secrets: BTreeMap<String, String> = [("SSH_CMD".to_string(), "ssh -i /key".to_string())]
        .into_iter()
        .collect();
    let ctx = ExecutionContext {
        secrets: std::rc::Rc::new(secrets),
        ..ExecutionContext::default()
    };
    let d = GitDriver::with_parts(
        peregrin_git::SystemGit::new(),
        Arc::new(shell_lookup()),
        Arc::new(AmbientEffects),
    );
    let p = json!({
        "url": url, "sha": first, "base": b,
        "ssh_env": "SSH_CMD",
        "token_env": ["AMBIENT_TOKEN"],
        "inner": shell_inner("true"),
    });
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
}

#[test]
fn unresolved_credential_names_are_audited_and_skipped() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let b = base(&tmp);
    let d = GitDriver::new(shell_lookup());

    // Non-strict: the miss is skipped and the run proceeds.
    let ctx = ctx();
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["ssh_env"] = json!("NO_SUCH_VAR_OR_SECRET");
    p["token_env"] = json!(42); // wrong shape — ignored
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");

    // Strict: the audited miss lands in the shared ledger and fails
    // the run at the inner driver's param resolve — the same
    // missing-secret semantics the engine enforces on `${secret.*}`.
    let missing: std::rc::Rc<std::cell::RefCell<Vec<String>>> = std::rc::Rc::default();
    let ctx = ExecutionContext {
        strict: true,
        missing_secrets: missing.clone(),
        ..ExecutionContext::default()
    };
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["ssh_env"] = json!("NO_SUCH_VAR_OR_SECRET");
    let err = d.invoke(&node(p), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("NO_SUCH_VAR_OR_SECRET"));
    assert_eq!(
        missing.borrow().as_slice(),
        &["NO_SUCH_VAR_OR_SECRET".to_string()]
    );

    // An empty-string secret resolves to nothing — same skip path.
    let secrets: BTreeMap<String, String> =
        [("EMPTY".to_string(), String::new())].into_iter().collect();
    let ctx = ExecutionContext {
        secrets: std::rc::Rc::new(secrets),
        ..ExecutionContext::default()
    };
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["token_env"] = json!("EMPTY");
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
}

// ---------------------------------------------------------------------------
// templating: sha from the trigger event
// ---------------------------------------------------------------------------

#[test]
fn template_resolution_failures_are_driver_errors() {
    let tmp = TempDir::new().unwrap();
    let d = GitDriver::new(shell_lookup());
    // `${event.*}` with no trigger in scope stays literal under
    // non-strict; strict turns the leftover placeholder into a
    // resolve failure — before any git work happens.
    let ctx = ExecutionContext {
        strict: true,
        ..ExecutionContext::default()
    };
    let p = json!({
        "url": "${event.payload.clone_url}",
        "sha": "x",
        "base": base(&tmp),
        "inner": shell_inner("true"),
    });
    let err = d.invoke(&node(p), &[], &ctx).unwrap_err();
    assert!(err.reason.contains("unresolved placeholders"));
    assert!(!tmp.path().join("worktrees").exists());
}

#[test]
fn state_scope_resolves_in_params() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    ctx.state.borrow_mut().set("sha", json!(first));
    let p = json!({
        "repo": repo.display().to_string(),
        "sha": "${state.sha}",
        "base": base(&tmp),
        "inner": shell_inner("test \"$(git rev-parse HEAD)\" = \"$PERE_SHA\""),
    });
    let out = d.invoke(&node(p), &[], &ctx).unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["sha"], first);
}

#[test]
fn sha_templates_from_the_trigger_event() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let trigger = Event::new(
        "push",
        json!({"origin": {"clone_url": repo.display().to_string(), "sha": first}}),
        "webhook",
    );
    let p = json!({
        "url": "${event.payload.origin.clone_url}",
        "sha": "${event.payload.origin.sha}",
        "base": base(&tmp),
        "inner": shell_inner("test \"$(git rev-parse HEAD)\" = \"$PERE_SHA\""),
    });
    let out = d.invoke(&node(p), &[trigger], &ctx()).unwrap();
    assert_eq!(out[0].kind, "n.done");
    assert_eq!(out[0].payload["sha"], first);
}

// ---------------------------------------------------------------------------
// extra_worktrees: dual-checkout layouts (PL-009 D6)
// ---------------------------------------------------------------------------

/// The docs-deploy shape: primary = `first` at `<run>/docs`, extra =
/// `main` (head) at `<run>` — the inner cmd's `cd ..` must land in the
/// main checkout.
#[test]
fn extra_worktrees_dual_checkout_layout() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["dest"] = json!("run-x/docs");
    p["extra_worktrees"] = json!([{"ref": "refs/heads/main", "dir": "."}]);
    // pwd is the docs checkout; `..` is the main checkout at head.
    p["inner"] = shell_inner(&format!(
        "test \"$(git rev-parse HEAD)\" = \"{first}\" && \
         test \"$(cd .. && git rev-parse HEAD)\" = \"{head}\" && \
         test -f ../f && test -f ../sub/g"
    ));
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "n.done", "{}", out[0].payload);
    // Both worktrees destroyed — nothing tracked remains.
    let w = peregrin_git::Worktrees::new(&b);
    assert_eq!(w.list().unwrap().len(), 0);
}

#[test]
fn extra_worktrees_subdir_and_multiple_entries() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, head) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["dest"] = json!("run-y/docs");
    // Shallowest-first creation regardless of yaml order: the run
    // root (".") is listed AFTER the nested "vendor" entry on purpose.
    p["extra_worktrees"] = json!([
        {"ref": "main", "dir": "vendor"},
        {"ref": "refs/heads/main"},
    ]);
    p["inner"] = shell_inner(&format!(
        "test \"$(cd ../vendor && git rev-parse HEAD)\" = \"{head}\" && \
         test \"$(cd .. && git rev-parse HEAD)\" = \"{head}\""
    ));
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "n.done", "{}", out[0].payload);
    assert_eq!(peregrin_git::Worktrees::new(&b).list().unwrap().len(), 0);
}

#[test]
fn extra_worktrees_cleanup_on_inner_failure_and_keep() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let mut p = params(&repo, &b, &first, &shell_inner("exit 3"));
    p["dest"] = json!("run-z/docs");
    p["extra_worktrees"] = json!([{"ref": "main", "dir": "."}]);
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert_eq!(out[0].payload["inner_rc"], 3);
    assert_eq!(peregrin_git::Worktrees::new(&b).list().unwrap().len(), 0);

    // `keep` retains every checkout for debugging.
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["dest"] = json!("run-k/docs");
    p["keep"] = json!(true);
    p["extra_worktrees"] = json!([{"ref": "main", "dir": "."}]);
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "n.done");
    let w = peregrin_git::Worktrees::new(&b);
    let kept: Vec<_> = w
        .list()
        .unwrap()
        .into_iter()
        .filter_map(|l| match l {
            peregrin_git::Listing::Tracked(wt) if wt.kept => Some(wt.path),
            _ => None,
        })
        .collect();
    assert_eq!(kept.len(), 2, "kept: {kept:?}");
    for path in &kept {
        assert!(path.is_dir());
    }
}

#[test]
fn extra_worktrees_unresolvable_ref_is_node_failed() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let b = base(&tmp);
    let mut p = params(&repo, &b, &first, &shell_inner("true"));
    p["dest"] = json!("run-bad/docs");
    p["extra_worktrees"] = json!([{"ref": "refs/heads/nope", "dir": "."}]);
    let out = d.invoke(&node(p), &[], &ctx()).unwrap();
    assert_eq!(out[0].kind, "node.failed");
    assert!(
        out[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("refs/heads/nope")
    );
    // Nothing materialized — the primary was never created either.
    assert_eq!(peregrin_git::Worktrees::new(&b).list().unwrap().len(), 0);
}

#[test]
fn extra_worktrees_malformed_params_are_driver_errors() {
    let tmp = TempDir::new().unwrap();
    let (repo, first, _) = fixture_repo(tmp.path());
    let d = GitDriver::new(shell_lookup());
    let ctx = ctx();
    let b = base(&tmp);

    for bad in [
        json!("main"),                                  // not an array
        json!(["main"]),                                // not an object
        json!([{"dir": "."}]),                          // missing ref
        json!([{"ref": ""}]),                           // empty ref
        json!([{"ref": "--upload-pack=touch /tmp/x"}]), // flag injection
        json!([{"ref": "main@{u}"}]),                   // revision syntax
        json!([{"ref": "a..b"}]),                       // dotdot ref
        json!([{"ref": "main", "dir": "../escape"}]),   // dir escape
        json!([{"ref": "main", "dir": "/abs"}]),        // absolute dir
    ] {
        let mut p = params(&repo, &b, &first, &shell_inner("true"));
        p["extra_worktrees"] = bad;
        let err = d.invoke(&node(p), &[], &ctx).unwrap_err();
        assert!(
            err.reason.contains("extra_worktrees"),
            "reason: {}",
            err.reason
        );
    }
    assert!(!tmp.path().join("worktrees").exists());
}
