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
        missing_secrets: Vec::new(),
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
fn strict_allows_deferred_scope_expressions() {
    let fx = StubEffects::new();
    let ctx = ctx_with(&[], &fx, true);
    // `config.*`/`event.*` are jinja-scope placeholders — not "unresolved".
    assert_eq!(
        resolve_template("${config.x} ${event.type}", &ctx).unwrap(),
        "${config.x} ${event.type}"
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
    let mut ctx = ctx_with(&[], &fx, true);
    ctx.missing_secrets = vec!["token".to_string(), "key".to_string()];
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
    };
    assert_eq!(
        resolve_template_opts("${echo(42)} ${echo(4.5)} ${echo(true)}", &ctx, &opts).unwrap(),
        "42 4.5 true"
    );
}
