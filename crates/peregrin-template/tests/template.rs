//! Port of `ac-template-engine/tests/index.test.ts` plus strict-mode and
//! capability edge cases.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use peregrin_template::{
    Context, Effects, Options, SystemEffects, TemplateFunction, builtins, resolve_template,
    resolve_template_opts, resolve_value, resolve_value_for_node,
};
use serde_json::{Map, Value, json};

/// Scripted effects — deterministic env/fs/shell/clock for tests.
struct StubEffects {
    env: HashMap<String, String>,
    files: HashMap<PathBuf, Vec<u8>>,
    shell_out: Result<String, String>,
    random: f64,
}

impl StubEffects {
    fn new() -> Self {
        Self {
            env: HashMap::new(),
            files: HashMap::new(),
            shell_out: Ok(String::new()),
            random: 0.5,
        }
    }
}

impl Effects for StubEffects {
    fn env(&self, name: &str) -> Option<String> {
        self.env.get(name).cloned()
    }
    fn cwd(&self) -> PathBuf {
        PathBuf::from("/work")
    }
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| format!("ENOENT: {}", path.display()))
    }
    fn shell(&self, _command: &str) -> Result<String, String> {
        self.shell_out.clone()
    }
    fn now_unix(&self) -> i64 {
        1_700_000_000
    }
    fn now_iso(&self) -> String {
        "2023-11-14T22:13:20.000Z".to_string()
    }
    fn random(&self) -> f64 {
        self.random
    }
    fn uuid(&self) -> String {
        "00000000-0000-4000-8000-000000000000".to_string()
    }
}

fn caps(entries: &[(&str, Value)]) -> Map<String, Value> {
    entries
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn ctx_with<'a>(
    caps_entries: &[(&str, Value)],
    effects: &'a dyn Effects,
    strict: bool,
) -> Context<'a> {
    Context {
        strict,
        missing_secrets: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        capabilities: caps(caps_entries),
        effects,
    }
}

fn node() -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n1".into(),
        driver: "shell".into(),
        params: json!({}),
        on: vec![],
        ..Default::default()
    }
}

// --- TS suite: permissive pass-through -----------------------------------

#[test]
fn passes_through_text_without_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("hello world", &ctx).unwrap(),
        "hello world"
    );
}

#[test]
fn preserves_jinja_style_known_prefix_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${config.region}-${secret.token}", &ctx).unwrap(),
        "${config.region}-${secret.token}"
    );
}

#[test]
fn preserves_event_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template(
            "${event.type} from ${event.source}: ${event.payload.ref}",
            &ctx
        )
        .unwrap(),
        "${event.type} from ${event.source}: ${event.payload.ref}"
    );
}

#[test]
fn preserves_node_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let node = node();
    let out = resolve_template_opts(
        "${node.id} fired ${node.fires}",
        &ctx,
        &Options {
            node: Some(&node),
            functions: &[],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "${node.id} fired ${node.fires}");
}

#[test]
fn preserves_wildcard_shard_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let out = resolve_value_for_node(&json!("${event.shard}"), &ctx, &node()).unwrap();
    assert_eq!(out, json!("${event.shard}"));
}

#[test]
fn preserves_bash_variables_and_default_syntax() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("echo ${HOME}", &ctx).unwrap(),
        "echo ${HOME}"
    );
    assert_eq!(
        resolve_template("echo ${WORKTREE:-/tmp}", &ctx).unwrap(),
        "echo ${WORKTREE:-/tmp}"
    );
    assert_eq!(resolve_template("echo $PATH", &ctx).unwrap(), "echo $PATH");
}

#[test]
fn preserves_a_mix_of_bash_and_known_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template(
            "echo '${ATTEMPT}/${config.attempts}' in ${WORKTREE:-.}",
            &ctx
        )
        .unwrap(),
        "echo '${ATTEMPT}/${config.attempts}' in ${WORKTREE:-.}"
    );
}

#[test]
fn preserves_unknown_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${notknown}", &ctx).unwrap(),
        "${notknown}"
    );
}

#[test]
fn preserves_pipe_filter_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${config.name | trim | upper}", &ctx).unwrap(),
        "${config.name | trim | upper}"
    );
}

#[test]
fn preserves_file_and_shell_filter_placeholders() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${ \"/tmp/data.txt\" | file }", &ctx).unwrap(),
        "${ \"/tmp/data.txt\" | file }"
    );
    assert_eq!(
        resolve_template("${ \"echo hi\" | shell }", &ctx).unwrap(),
        "${ \"echo hi\" | shell }"
    );
}

#[test]
fn recurses_into_dicts_and_lists_via_resolve_value() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let value = resolve_value(
        &json!({
            "a": "${config.x}",
            "b": ["${secret.y}", 123],
            "c": {"d": "literal"},
        }),
        &ctx,
    )
    .unwrap();
    assert_eq!(
        value,
        json!({
            "a": "${config.x}",
            "b": ["${secret.y}", 123],
            "c": {"d": "literal"},
        })
    );
}

// --- TS suite: whitelisted function resolution ---------------------------

#[test]
fn resolves_env_with_a_default() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("env", json!(true))], &fx, false);
    assert_eq!(
        resolve_template("${env(\"MISSING\", \"fallback\")}", &ctx).unwrap(),
        "fallback"
    );
}

#[test]
fn resolves_env_present() {
    let mut fx = StubEffects::new();
    fx.env.insert("HOME".to_string(), "/root".to_string());
    let ctx = ctx_with(&[("env", json!(true))], &fx, false);
    assert_eq!(resolve_template("${env(\"HOME\")}", &ctx).unwrap(), "/root");
}

#[test]
fn resolves_random() {
    let mut fx = StubEffects::new();
    fx.random = 0.99;
    let ctx = ctx_with(&[("random", json!(true))], &fx, false);
    let text = resolve_template("${random(1, 10)}", &ctx).unwrap();
    let n: i64 = text.parse().unwrap();
    assert!((1..=10).contains(&n));
    // Integer bounds land on `floor(min + r * (max - min + 1))`.
    assert_eq!(n, 10);
}

#[test]
fn resolves_random_float_range() {
    let mut fx = StubEffects::new();
    fx.random = 0.5;
    let ctx = ctx_with(&[("random", json!(true))], &fx, false);
    // `${random()}` defaults to 0..=1 — both integral → integer path.
    assert_eq!(resolve_template("${random()}", &ctx).unwrap(), "1");
    // Non-integral bounds take the float path: `min + r * (max - min)`.
    let text = resolve_template("${random(0.5, 2.5)}", &ctx).unwrap();
    assert_eq!(text, "1.5");
}

#[test]
fn resolves_uuid() {
    let ctx = ctx_with(&[("uuid", json!(true))], &SystemEffects, false);
    let text = resolve_template("${uuid()}", &ctx).unwrap();
    let parts: Vec<&str> = text.split('-').collect();
    assert_eq!(parts.len(), 5);
    let lengths = [8, 4, 4, 4, 12];
    for (part, len) in parts.iter().zip(lengths) {
        assert_eq!(part.len(), len);
        assert!(part.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

#[test]
fn resolves_now() {
    let ctx = ctx_with(&[("now", json!(true))], &SystemEffects, false);
    let text = resolve_template("${now()}", &ctx).unwrap();
    let b = text.as_bytes();
    assert!(
        b.len() > 10 && b[4] == b'-' && b[7] == b'-' && b[10] == b'T',
        "{text}"
    );
}

#[test]
fn resolves_now_variants() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("now", json!(true))], &fx, false);
    assert_eq!(
        resolve_template("${now.iso()}", &ctx).unwrap(),
        "2023-11-14T22:13:20.000Z"
    );
    assert_eq!(
        resolve_template("${now.unix()}", &ctx).unwrap(),
        "1700000000"
    );
}

#[test]
fn resolves_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "hello").unwrap();
    let path = tmp.path().to_string_lossy().to_string();
    let ctx = ctx_with(&[("file", json!(true))], &SystemEffects, false);
    let template = format!("${{file(\"{path}\")}}");
    assert_eq!(resolve_template(&template, &ctx).unwrap(), "hello");
}

#[test]
fn resolves_shell() {
    let ctx = ctx_with(
        &[("shell", json!({"allow": ["echo *"]}))],
        &SystemEffects,
        false,
    );
    assert_eq!(
        resolve_template("${shell(\"echo hello\")}", &ctx).unwrap(),
        "hello"
    );
}

#[test]
fn preserves_unwhitelisted_functions() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${random(1, 10)}", &ctx).unwrap(),
        "${random(1, 10)}"
    );
}

// --- Rust extras: strict mode, capabilities, errors, custom fns -----------

#[test]
fn strict_fails_on_unresolved_placeholders() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[], &fx, true);
    let err = resolve_template("a ${notknown} b", &ctx).unwrap_err();
    assert_eq!(err.reason, "unresolved placeholders: ${notknown}");
}

#[test]
fn strict_flags_leftover_scope_placeholders() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[], &fx, true);
    // The scope pass ran — leftover `config.*`/`event.*` are unresolved.
    let err = resolve_template("${config.x} ${event.type}", &ctx).unwrap_err();
    assert_eq!(
        err.reason,
        "unresolved placeholders: ${config.x}, ${event.type}"
    );
    // Deferred I/O-builtin dotted forms still pass the audit.
    assert_eq!(
        resolve_template("${env.HOME} ${file.x}", &ctx).unwrap(),
        "${env.HOME} ${file.x}"
    );
}

#[test]
fn strict_flags_pipe_filters_as_unresolved() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[], &fx, true);
    let err = resolve_template("${config.name | upper}", &ctx).unwrap_err();
    assert!(err.reason.starts_with("unresolved placeholders:"));
}

#[test]
fn strict_fails_on_missing_secrets() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[], &fx, true);
    *ctx.missing_secrets.borrow_mut() = vec!["token".to_string(), "key".to_string()];
    let err = resolve_template("nothing to resolve", &ctx).unwrap_err();
    assert_eq!(err.reason, "missing secrets: token, key");
}

#[test]
fn strict_capability_check_denies_unmatched_args() {
    let mut fx = StubEffects::new();
    fx.shell_out = Ok("oops".into());
    let ctx = ctx_with(&[("shell", json!({"allow": ["echo *"]}))], &fx, true);
    let err = resolve_template("${shell(\"rm -rf /\")}", &ctx).unwrap_err();
    assert_eq!(err.reason, "capability 'shell' not granted for 'rm -rf /'");
}

#[test]
fn strict_capability_check_allows_matching_globs() {
    let mut fx = StubEffects::new();
    fx.shell_out = Ok("hi".into());
    let ctx = ctx_with(&[("shell", json!({"allow": ["echo *"]}))], &fx, true);
    assert_eq!(
        resolve_template("${shell(\"echo hi\")}", &ctx).unwrap(),
        "hi"
    );
}

#[test]
fn strict_env_cap_array() {
    let mut fx = StubEffects::new();
    fx.env.insert("HOME_DIR".into(), "/root".into());
    let ctx = ctx_with(&[("env", json!(["HOME*"]))], &fx, true);
    assert_eq!(
        resolve_template("${env(\"HOME_DIR\")}", &ctx).unwrap(),
        "/root"
    );
    let err = resolve_template("${env(\"PATH\")}", &ctx).unwrap_err();
    assert_eq!(err.reason, "capability 'env' not granted for 'PATH'");
}

#[test]
fn strict_env_cap_deny_beats_allow() {
    let mut fx = StubEffects::new();
    fx.env.insert("SECRET_X".into(), "s3".into());
    let ctx = ctx_with(
        &[("env", json!({"allow": ["*"], "deny": ["SECRET*"]}))],
        &fx,
        true,
    );
    let err = resolve_template("${env(\"SECRET_X\")}", &ctx).unwrap_err();
    assert_eq!(err.reason, "capability 'env' not granted for 'SECRET_X'");
}

#[test]
fn cap_false_preserves_placeholder() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("env", json!(false))], &fx, false);
    assert_eq!(
        resolve_template("${env(\"X\")}", &ctx).unwrap(),
        "${env(\"X\")}"
    );
}

#[test]
fn function_errors_propagate() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("env", json!(true))], &fx, false);
    let err = resolve_template("${env()}", &ctx).unwrap_err();
    assert_eq!(err.reason, "env() requires a string name");
    let ctx = ctx_with(&[("now", json!(true))], &fx, false);
    let err = resolve_template("${now.foo()}", &ctx).unwrap_err();
    assert_eq!(err.reason, "now.foo() is not supported");
    let ctx = ctx_with(&[("file", json!(true))], &fx, false);
    let err = resolve_template("${file(123)}", &ctx).unwrap_err();
    assert_eq!(err.reason, "file() requires a string path");
    let ctx = ctx_with(&[("shell", json!(true))], &fx, false);
    let err = resolve_template("${shell()}", &ctx).unwrap_err();
    assert_eq!(err.reason, "shell() requires a string command");
}

#[test]
fn file_failure_is_a_template_error() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("file", json!(true))], &fx, false);
    let err = resolve_template("${file(\"missing.txt\")}", &ctx).unwrap_err();
    assert!(err.reason.starts_with("file() failed: "), "{}", err.reason);
}

#[test]
fn relative_file_paths_join_cwd() {
    let mut fx = StubEffects::new();
    fx.files
        .insert(PathBuf::from("/work/data/x.txt"), b"hit".to_vec());
    let ctx = ctx_with(&[("file", json!(true))], &fx, false);
    assert_eq!(
        resolve_template("${file(\"data/./x.txt\")}", &ctx).unwrap(),
        "hit"
    );
}

#[test]
fn custom_functions_resolve_and_override() {
    let fx = StubEffects::new();
    let greet = TemplateFunction::new("greet", true, false, |args, _ctx, _node, _method| {
        Ok(json!(format!("hi {}", args[0].as_str().unwrap_or("?"))))
    });
    let env_override = TemplateFunction::new("env", true, true, |_args, _ctx, _node, _method| {
        Ok(json!("custom"))
    });
    let ctx = ctx_with(&[("greet", json!(true)), ("env", json!(true))], &fx, false);
    let opts = Options {
        node: None,
        functions: &[greet, env_override],
        ..Default::default()
    };
    assert_eq!(
        resolve_template_opts("${greet(\"ada\")} ${env(\"X\")}", &ctx, &opts).unwrap(),
        "hi ada custom"
    );
}

#[test]
fn custom_function_receives_node_and_method() {
    let fx = StubEffects::new();
    let probe = TemplateFunction::new("probe", true, false, |_args, _ctx, node, method| {
        Ok(json!(format!(
            "{}:{}",
            node.map_or("-".into(), |n| n.id.clone()),
            method.unwrap_or("-")
        )))
    });
    let ctx = ctx_with(&[("probe", json!(true))], &fx, false);
    let node = node();
    let opts = Options {
        node: Some(&node),
        functions: &[probe],
        ..Default::default()
    };
    assert_eq!(
        resolve_template_opts("${probe.m()}", &ctx, &opts).unwrap(),
        "n1:m"
    );
}

#[test]
fn builtin_metadata_matches_ts() {
    let meta: HashMap<String, (bool, bool)> = builtins()
        .iter()
        .map(|f| (f.name.clone(), (f.deterministic, f.requires_capability)))
        .collect();
    assert_eq!(meta["env"], (true, true));
    assert_eq!(meta["file"], (true, true));
    assert_eq!(meta["shell"], (true, true));
    assert_eq!(meta["now"], (false, false));
    assert_eq!(meta["random"], (false, false));
    assert_eq!(meta["uuid"], (false, false));
}

#[test]
fn empty_and_malformed_placeholders_pass_through() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    for text in [
        "${}",
        "${unclosed",
        "a } b",
        "${f(}",
        "${1x()}",
        "${a.b.c()}",
    ] {
        assert_eq!(resolve_template(text, &ctx).unwrap(), text, "{text}");
    }
}

#[test]
fn defaults_normalize_var_whitespace() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    assert_eq!(
        resolve_template("${  V  :-  d }", &ctx).unwrap(),
        "${V:-  d }"
    );
    // Nested defaults rewrite recursively; whitespace inside the
    // default part is preserved verbatim (TS only trims the var side).
    assert_eq!(
        resolve_template("${ A :- ${ B :- x } }", &ctx).unwrap(),
        "${A:- ${B:- x } }"
    );
}

#[test]
fn number_args_stay_numbers() {
    let fx = StubEffects::new();
    let echo = TemplateFunction::new("echo", true, false, |args, _ctx, _node, _method| {
        Ok(args[0].clone())
    });
    let ctx = ctx_with(&[("echo", json!(true))], &fx, false);
    let opts = Options {
        node: None,
        functions: &[echo],
        ..Default::default()
    };
    assert_eq!(
        resolve_template_opts("${echo(42)} ${echo(4.5)} ${echo(true)}", &ctx, &opts).unwrap(),
        "42 4.5 true"
    );
}

// ---------------------------------------------------------------------
// Scope resolution (`docs/reference/scope-resolution.md`)
// ---------------------------------------------------------------------

use peregrin_template::{EventScope, NodeScope, ParamsScope, Scope, resolve_params};
use std::collections::BTreeMap;

fn scope_event() -> peregrin_core::Event {
    peregrin_core::Event {
        kind: "ci.passed".to_string(),
        payload: json!({"ref": {"sha": "abc123"}, "tags": ["v1", "v2"], "num": 7}),
        source: "upstream".to_string(),
        id: "evt-1".to_string(),
        ts: 42,
    }
}

fn scope_config() -> Map<String, Value> {
    json!({
        "region": "us-east",
        "nested": {"deep": {"x": "y"}},
        "scalar": "leaf",
        "arr": [1, null, "x"],
        "obj": {"k": 1},
        "nil": null,
        "flag": true,
        "pi": 2.5
    })
    .as_object()
    .unwrap()
    .clone()
}

fn scope_secrets() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("TOKEN".to_string(), "s3cr3t".to_string()),
        ("API.KEY".to_string(), "k".to_string()),
    ])
}

fn on_node() -> peregrin_core::Node {
    peregrin_core::Node {
        id: "n1".to_string(),
        on: vec!["ci.*".to_string(), "ci.**".to_string()],
        ..Default::default()
    }
}

fn opts_with_scope(scope: Scope<'_>) -> Options<'_> {
    Options {
        node: None,
        functions: &[],
        scope,
    }
}

// --- event ------------------------------------------------------------

#[test]
fn scope_event_envelope_fields() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let event = scope_event();
    let node = on_node();
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        ..Default::default()
    };
    let out = resolve_template_opts(
        "${event.type}|${event.source}|${event.id}|${event.ts}|${ event.type }",
        &ctx,
        &opts_with_scope(scope),
    )
    .unwrap();
    assert_eq!(out, "ci.passed|upstream|evt-1|42|ci.passed");
}

#[test]
fn scope_event_payload_walks_nested_paths() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let event = scope_event();
    let node = on_node();
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${event.payload.ref.sha}", &ctx, &opts).unwrap(),
        "abc123"
    );
    assert_eq!(
        resolve_template_opts("${event.payload.tags}", &ctx, &opts).unwrap(),
        "v1,v2"
    );
    assert_eq!(
        resolve_template_opts("${event.payload.num}", &ctx, &opts).unwrap(),
        "7"
    );
    // The payload object itself stringifies like a JS object.
    assert_eq!(
        resolve_template_opts("${event.payload}", &ctx, &opts).unwrap(),
        "[object Object]"
    );
}

#[test]
fn scope_event_match_and_shard_captures() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let event = scope_event();
    let node = on_node();
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        ..Default::default()
    };
    let out = resolve_template_opts(
        "${event.match.1}/${event.shard}",
        &ctx,
        &opts_with_scope(scope),
    )
    .unwrap();
    // `ci.*` matches first — `shard` aliases `match.1`.
    assert_eq!(out, "passed/passed");
}

#[test]
fn scope_event_match_multi_segment_capture() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let mut event = scope_event();
    event.kind = "ci.a.b".to_string();
    let node = peregrin_core::Node {
        on: vec!["other".to_string(), "ci.**".to_string()],
        ..on_node()
    };
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    // First *matching* pattern wins — `ci.**` captures "a.b".
    assert_eq!(
        resolve_template_opts("${event.match.1}", &ctx, &opts).unwrap(),
        "a.b"
    );
    // Higher-index captures and trailing paths miss.
    assert_eq!(
        resolve_template_opts("${event.match.2}", &ctx, &opts).unwrap(),
        "${event.match.2}"
    );
    assert_eq!(
        resolve_template_opts("${event.shard.x}", &ctx, &opts).unwrap(),
        "${event.shard.x}"
    );
}

#[test]
fn scope_event_misses_and_unbound() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let event = scope_event();
    let node = on_node();
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    for text in [
        "${event.bogus}",          // unknown field
        "${event.type.x}",         // scalar mid-path
        "${event.payload.no}",     // missing key
        "${event.payload.tags.x}", // non-object step
        "${event.match}",          // bare match
        "${event.match.x.y}",      // trailing path on a capture
        "${event}",                // bare namespace
    ] {
        assert_eq!(
            resolve_template_opts(text, &ctx, &opts).unwrap(),
            text,
            "{text}"
        );
    }
    // No bound trigger — every event.* lookup misses.
    let unbound = Options {
        node: None,
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("${event.type}", &ctx, &unbound).unwrap(),
        "${event.type}"
    );
}

// --- config / state / node --------------------------------------------

#[test]
fn scope_config_paths_misses_and_stringify() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let config = scope_config();
    let scope = Scope {
        config: Some(&config),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${config.region}", &ctx, &opts).unwrap(),
        "us-east"
    );
    assert_eq!(
        resolve_template_opts("${config.nested.deep.x}", &ctx, &opts).unwrap(),
        "y"
    );
    assert_eq!(
        resolve_template_opts(
            "${config.arr}|${config.obj}|${config.nil}|${config.flag}|${config.pi}",
            &ctx,
            &opts
        )
        .unwrap(),
        "1,,x|[object Object]||true|2.5"
    );
    for text in [
        "${config.missing}",
        "${config.scalar.x}", // scalar mid-path
        "${config}",          // bare namespace
        "${config..x}",       // empty segment
        "${config.}",         // trailing dot
    ] {
        assert_eq!(
            resolve_template_opts(text, &ctx, &opts).unwrap(),
            text,
            "{text}"
        );
    }
    // Unbound config misses too.
    let unbound = Options {
        node: None,
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("${config.region}", &ctx, &unbound).unwrap(),
        "${config.region}"
    );
}

#[test]
fn scope_state_lookup_and_walk() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let record = Map::from_iter([
        ("greeting".to_string(), json!("hi")),
        ("deep".to_string(), json!({"a": {"b": "z"}})),
    ]);
    let state_view = |key: &str| record.get(key).cloned();
    let scope = Scope {
        state: Some(&state_view),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${state.greeting}", &ctx, &opts).unwrap(),
        "hi"
    );
    assert_eq!(
        resolve_template_opts("${state.deep.a.b}", &ctx, &opts).unwrap(),
        "z"
    );
    for text in ["${state.absent}", "${state.greeting.x}", "${state}"] {
        assert_eq!(
            resolve_template_opts(text, &ctx, &opts).unwrap(),
            text,
            "{text}"
        );
    }
    // Unbound state misses.
    let unbound = Options {
        node: None,
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("${state.greeting}", &ctx, &unbound).unwrap(),
        "${state.greeting}"
    );
}

#[test]
fn scope_node_fields() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let scope = Scope {
        node: Some(NodeScope { id: "n1", fires: 3 }),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${node.id} fired ${node.fires}", &ctx, &opts).unwrap(),
        "n1 fired 3"
    );
    assert_eq!(
        resolve_template_opts("${node.bogus}", &ctx, &opts).unwrap(),
        "${node.bogus}"
    );
    // Unbound node misses even when options.node is set.
    let node = node();
    let unbound = Options {
        node: Some(&node),
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("${node.id}", &ctx, &unbound).unwrap(),
        "${node.id}"
    );
}

// --- params -------------------------------------------------------------

#[test]
fn scope_params_resolved_binding() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let resolved = Map::from_iter([
        ("a".to_string(), json!("va")),
        ("deep".to_string(), json!({"x": 1})),
    ]);
    let scope = Scope {
        params: ParamsScope::Resolved(&resolved),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${params.a}/${params.deep.x}", &ctx, &opts).unwrap(),
        "va/1"
    );
    assert_eq!(
        resolve_template_opts("${params.missing}", &ctx, &opts).unwrap(),
        "${params.missing}"
    );
    assert_eq!(
        resolve_template_opts("${params.a.y}", &ctx, &opts).unwrap(),
        "${params.a.y}"
    );
    // Unbound params misses.
    let unbound = Options {
        node: None,
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("${params.a}", &ctx, &unbound).unwrap(),
        "${params.a}"
    );
}

#[test]
fn resolve_params_declared_resolves_cross_references() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let node = peregrin_core::Node {
        params: json!({
            "a": "1",
            "b": "${params.a}-x",
            "deep": {"x": "v"},
            "walked": "${params.deep.x}",
            "miss": "${params.absent}",
            "via_default": "${params.absent:-fb}",
            "chain": "${params.absent:-${params.a}}",
        }),
        ..node()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[],
        scope: Scope {
            params: ParamsScope::declared(),
            ..Default::default()
        },
    };
    let out = resolve_params(&node.params, &ctx, &opts).unwrap();
    assert_eq!(out["a"], json!("1"));
    assert_eq!(out["b"], json!("1-x"));
    assert_eq!(out["walked"], json!("v"));
    // Lookup misses stay verbatim (lenient pass-through).
    assert_eq!(out["miss"], json!("${params.absent}"));
    assert_eq!(out["via_default"], json!("fb"));
    assert_eq!(out["chain"], json!("1"));
}

#[test]
fn resolve_params_declared_cycle_is_a_miss() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let node = peregrin_core::Node {
        params: json!({
            "self": "${params.self}",
            "a": "${params.b}",
            "b": "${params.a}",
        }),
        ..Default::default()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[],
        scope: Scope {
            params: ParamsScope::declared(),
            ..Default::default()
        },
    };
    let out = resolve_params(&node.params, &ctx, &opts).unwrap();
    // Direct and transitive cycles are in-progress keys — misses.
    assert_eq!(out["self"], json!("${params.self}"));
    assert!(matches!(out["a"].as_str(), Some(s) if s.starts_with("${params.")));
    assert!(matches!(out["b"].as_str(), Some(s) if s.starts_with("${params.")));
}

#[test]
fn resolve_params_declared_memoizes() {
    let fx = StubEffects::new();
    let calls = std::rc::Rc::new(std::cell::RefCell::new(0_u32));
    let calls2 = calls.clone();
    let tick = TemplateFunction::new("tick", false, false, move |_args, _ctx, _node, _m| {
        *calls2.borrow_mut() += 1;
        Ok(json!("t"))
    });
    let ctx = ctx_with(&[("tick", json!(true))], &fx, false);
    let node = peregrin_core::Node {
        params: json!({
            "a": "${tick()}",
            "b": "${params.a}",
            "c": "${params.a}",
        }),
        ..Default::default()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[tick],
        scope: Scope {
            params: ParamsScope::declared(),
            ..Default::default()
        },
    };
    let out = resolve_params(&node.params, &ctx, &opts).unwrap();
    assert_eq!(out["b"], json!("t"));
    assert_eq!(out["c"], json!("t"));
    // The referenced key resolved once, for the whole pass.
    assert_eq!(*calls.borrow(), 1);
}

#[test]
fn resolve_params_fallthrough_and_errors() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let config = scope_config();
    // Non-object value falls through to plain resolution.
    let node = peregrin_core::Node {
        params: json!("${config.region}"),
        ..Default::default()
    };
    let scope = Scope {
        config: Some(&config),
        params: ParamsScope::declared(),
        ..Default::default()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[],
        scope,
    };
    assert_eq!(
        resolve_params(&node.params, &ctx, &opts).unwrap(),
        json!("us-east")
    );
    // A non-Declared binding resolves each value plainly.
    let resolved_map = Map::from_iter([("a".to_string(), json!("va"))]);
    let node = peregrin_core::Node {
        params: json!({"x": "${params.a}"}),
        ..Default::default()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[],
        scope: Scope {
            params: ParamsScope::Resolved(&resolved_map),
            ..Default::default()
        },
    };
    assert_eq!(
        resolve_params(&node.params, &ctx, &opts).unwrap(),
        json!({"x": "va"})
    );
    // Declared scope without a bound node — lookups miss.
    let opts = Options {
        node: None,
        functions: &[],
        scope: Scope {
            params: ParamsScope::declared(),
            ..Default::default()
        },
    };
    assert_eq!(
        resolve_template_opts("${params.a}", &ctx, &opts).unwrap(),
        "${params.a}"
    );
    // Nested resolution errors propagate (strict unresolved audit).
    let strict = ctx_with(&[], &fx, true);
    let node = peregrin_core::Node {
        params: json!({"a": "${params.b}", "b": "${config.nope}"}),
        ..Default::default()
    };
    let opts = Options {
        node: Some(&node),
        functions: &[],
        scope: Scope {
            params: ParamsScope::declared(),
            ..Default::default()
        },
    };
    let err = resolve_params(&node.params, &strict, &opts).unwrap_err();
    assert!(
        err.reason.starts_with("unresolved placeholders:"),
        "{}",
        err.reason
    );
}

// --- secret -------------------------------------------------------------

#[test]
fn scope_secret_gate_lookup_and_misses() {
    let fx = StubEffects::new();
    let secrets = scope_secrets();
    // Ungated — the placeholder is unresolved.
    let ctx = ctx_with(&[], &fx, false);
    let scope = Scope {
        secrets: Some(&secrets),
        ..Default::default()
    };
    let opts = opts_with_scope(scope);
    assert_eq!(
        resolve_template_opts("${secret.TOKEN}", &ctx, &opts).unwrap(),
        "${secret.TOKEN}"
    );
    // `secrets: false` is ungated too.
    let ctx = ctx_with(&[("secrets", json!(false))], &fx, false);
    assert_eq!(
        resolve_template_opts("${secret.TOKEN}", &ctx, &opts).unwrap(),
        "${secret.TOKEN}"
    );
    // Gated — hit substitutes, dotted names work, miss substitutes "".
    let ctx = ctx_with(&[("secrets", json!(true))], &fx, false);
    assert_eq!(
        resolve_template_opts("${secret.TOKEN}", &ctx, &opts).unwrap(),
        "s3cr3t"
    );
    assert_eq!(
        resolve_template_opts("${secret.API.KEY}", &ctx, &opts).unwrap(),
        "k"
    );
    assert_eq!(
        resolve_template_opts("x${secret.MISSING}y", &ctx, &opts).unwrap(),
        "xy"
    );
    // Miss with a default substitutes the default — no ledger entry.
    assert_eq!(
        resolve_template_opts("${secret.MISSING:-fb}", &ctx, &opts).unwrap(),
        "fb"
    );
    // Ungated + default — the default still applies (miss semantics).
    let ungated = ctx_with(&[], &fx, false);
    assert_eq!(
        resolve_template_opts("${secret.MISSING:-fb}", &ungated, &opts).unwrap(),
        "fb"
    );
    // Unbound secrets record + gate — every name misses.
    let unbound = Options {
        node: None,
        functions: &[],
        scope: Scope::default(),
    };
    assert_eq!(
        resolve_template_opts("x${secret.MISSING}y", &ctx, &unbound).unwrap(),
        "xy"
    );
    assert_eq!(
        resolve_template_opts("${secret}", &ctx, &opts).unwrap(),
        "${secret}"
    );
    assert_eq!(
        resolve_template_opts("${secret.}", &ctx, &opts).unwrap(),
        "${secret.}"
    );
}

#[test]
fn scope_secret_strict_ledger_and_grants() {
    let fx = StubEffects::new();
    let secrets = scope_secrets();
    let scope = Scope {
        secrets: Some(&secrets),
        ..Default::default()
    };
    // Strict miss substitutes "" and lands in the ledger → audit fails.
    let strict = ctx_with(&[("secrets", json!(true))], &fx, true);
    let err =
        resolve_template_opts("x${secret.MISSING}y", &strict, &opts_with_scope(scope)).unwrap_err();
    assert_eq!(err.reason, "missing secrets: MISSING");
    let secrets2 = scope_secrets();
    let opts = opts_with_scope(Scope {
        secrets: Some(&secrets2),
        ..Default::default()
    });
    // A non-empty default suppresses the miss entirely.
    let strict2 = ctx_with(&[("secrets", json!(true))], &fx, true);
    assert_eq!(
        resolve_template_opts("${secret.MISSING:-fb}", &strict2, &opts).unwrap(),
        "fb"
    );
    // An empty default counts as no default — "" + ledger.
    let strict3 = ctx_with(&[("secrets", json!(true))], &fx, true);
    let err = resolve_template_opts("${secret.MISSING:-}", &strict3, &opts).unwrap_err();
    assert_eq!(err.reason, "missing secrets: MISSING");
    // Strict grant matching: list globs must cover the name.
    let granted = ctx_with(&[("secrets", json!(["TOK*"]))], &fx, true);
    assert_eq!(
        resolve_template_opts("${secret.TOKEN}", &granted, &opts).unwrap(),
        "s3cr3t"
    );
    let err = resolve_template_opts("${secret.API.KEY}", &granted, &opts).unwrap_err();
    assert_eq!(err.reason, "capability 'secrets' not granted for 'API.KEY'");
    // Ungated under strict — the audit flags the placeholder itself.
    let ungated = ctx_with(&[], &fx, true);
    let err = resolve_template_opts("${secret.TOKEN}", &ungated, &opts).unwrap_err();
    assert_eq!(err.reason, "unresolved placeholders: ${secret.TOKEN}");
    // Ungated under strict + default — default applies, no flag.
    assert_eq!(
        resolve_template_opts("${secret.TOKEN:-fb}", &ungated, &opts).unwrap(),
        "fb"
    );
}

// --- misses, defaults, depth -------------------------------------------

#[test]
fn scope_defaults_consume_only_on_miss() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[("env", json!(true))], &fx, false);
    let config = scope_config();
    let opts = opts_with_scope(Scope {
        config: Some(&config),
        ..Default::default()
    });
    assert_eq!(
        resolve_template_opts("${config.miss:-fb}", &ctx, &opts).unwrap(),
        "fb"
    );
    // A hit ignores the default — even when the value stringifies to "".
    assert_eq!(
        resolve_template_opts("${config.nil:-fb}", &ctx, &opts).unwrap(),
        ""
    );
    // Defaults chain through the full pipeline.
    assert_eq!(
        resolve_template_opts("${config.a:-${config.b:-${config.region}}}", &ctx, &opts).unwrap(),
        "us-east"
    );
    let mut fx2 = StubEffects::new();
    fx2.env.insert("FALLBACK".to_string(), "envd".to_string());
    let ctx2 = ctx_with(&[("env", json!(true))], &fx2, false);
    assert_eq!(
        resolve_template_opts("${config.a:-${env(\"FALLBACK\")}}", &ctx2, &opts).unwrap(),
        "envd"
    );
}

#[test]
fn scope_default_chain_depth_cap() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let config = scope_config();
    let opts = opts_with_scope(Scope {
        config: Some(&config),
        ..Default::default()
    });
    // Eight chained defaults resolve.
    let eight = "${config.z0:-${config.z1:-${config.z2:-${config.z3:-${config.z4:-${config.z5:-${config.z6:-${config.z7:-TAIL}}}}}}}}";
    assert_eq!(resolve_template_opts(eight, &ctx, &opts).unwrap(), "TAIL");
    // Nine — the innermost expression is past the cap and stays verbatim.
    let nine = "${config.z0:-${config.z1:-${config.z2:-${config.z3:-${config.z4:-${config.z5:-${config.z6:-${config.z7:-${config.z8:-TAIL}}}}}}}}}";
    assert_eq!(
        resolve_template_opts(nine, &ctx, &opts).unwrap(),
        "${config.z8:-TAIL}"
    );
    // Secret miss at the cap substitutes "" like a plain miss.
    let secrets = scope_secrets();
    let deep_secret = "${config.a:-${config.b:-${config.c:-${config.d:-${config.e:-${config.f:-${config.g:-${config.h:-${secret.MISS}}}}}}}}}";
    let ctx = ctx_with(&[("secrets", json!(true))], &fx, false);
    let opts = opts_with_scope(Scope {
        config: Some(&config),
        secrets: Some(&secrets),
        ..Default::default()
    });
    assert_eq!(resolve_template_opts(deep_secret, &ctx, &opts).unwrap(), "");
}

#[test]
fn scope_expr_grammar_rejections_stay_verbatim() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let config = scope_config();
    let opts = opts_with_scope(Scope {
        config: Some(&config),
        ..Default::default()
    });
    for text in [
        "${config.get()}",    // parses as a call — left for stage 2
        "${config.a()}",      // call-shaped
        "${config.x | trim}", // filters aren't scope exprs
        "${bogus.x}",         // foreign namespace
        "${env.PATH}",        // deferred I/O-builtin dotted form
        "${HOME}",            // bare foreign name
        "${1abc.x}",          // invalid ns
        "${config..x}",       // empty segment
        "${unclosed",
        "${}",
    ] {
        assert_eq!(
            resolve_template_opts(text, &ctx, &opts).unwrap(),
            text,
            "{text}"
        );
    }
    // Foreign namespaces don't consume `:-` defaults.
    assert_eq!(
        resolve_template_opts("${bogus.x:-d}", &ctx, &opts).unwrap(),
        "${bogus.x:-d}"
    );
    // Text around placeholders is preserved.
    assert_eq!(
        resolve_template_opts("a ${config.region} b", &ctx, &opts).unwrap(),
        "a us-east b"
    );
}

#[test]
fn strict_audit_catches_leftover_scope_placeholders() {
    let fx = StubEffects::new();
    let config = scope_config();
    let strict = ctx_with(&[], &fx, true);
    let opts = opts_with_scope(Scope {
        config: Some(&config),
        ..Default::default()
    });
    // Bound-but-missing keys fail the audit; hits resolve cleanly.
    let err = resolve_template_opts("${config.miss}", &strict, &opts).unwrap_err();
    assert_eq!(err.reason, "unresolved placeholders: ${config.miss}");
    assert_eq!(
        resolve_template_opts("${config.region} ok", &strict, &opts).unwrap(),
        "us-east ok"
    );
    // A bound miss with a default resolves and audits clean.
    assert_eq!(
        resolve_template_opts("${config.miss:-d}", &strict, &opts).unwrap(),
        "d"
    );
}

#[test]
fn resolve_value_recurses_scope_resolution() {
    let fx = StubEffects::new();
    let ctx = Context::new(&fx);
    let event = scope_event();
    let config = scope_config();
    let node = on_node();
    let scope = Scope {
        event: Some(EventScope {
            event: &event,
            patterns: &node.on,
        }),
        config: Some(&config),
        ..Default::default()
    };
    let out = peregrin_template::resolve_value_opts(
        &json!({
            "a": "${event.type}",
            "b": ["${config.region}", 1],
            "c": {"d": "${event.shard}"},
        }),
        &ctx,
        &opts_with_scope(scope),
    )
    .unwrap();
    assert_eq!(
        out,
        json!({"a": "ci.passed", "b": ["us-east", 1], "c": {"d": "passed"}})
    );
}
