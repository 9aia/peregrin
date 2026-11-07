//! The [`Supersede`] policy — the per-campaign `supersede:` config
//! shape — and the `{path}` key-template resolver.

use serde_json::Value;

use peregrin_core::{Event, js_string};

/// How pending jobs dedupe by supersede key (yaml `mode:`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// A new event replaces a pending job with the same key — only
    /// the newest trigger for a target runs. The replacement keeps
    /// the displaced job's queue position.
    #[default]
    LatestWins,
    /// Every event appends; same-key events each get a run.
    Fifo,
}

/// The `supersede:` campaign config — which runs happen when trigger
/// events outpace the executor. The schema mirror the campaign model
/// carries is [`peregrin_core::Supersede`] — `peregrin-core` can't
/// depend on this crate (the dep runs the other way), so the node
/// converts it via [`Supersede::from`] at run setup.
///
/// ```yaml
/// supersede:
///   key: "{repo}:{ref}"     # template over the trigger event payload
///   mode: latest-wins       # | fifo
///   cancel_running: true    # cancel the active run vs queue behind it
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supersede {
    /// Key template over the trigger event's `payload` —
    /// `{dotted.path}` placeholders resolve via dotted-path lookup
    /// and JS-style stringification; misses stay verbatim. An empty
    /// template (the default) resolves to a constant key, so every
    /// event targets the same logical run.
    #[serde(default)]
    pub key: String,
    /// Pending-job dedupe mode.
    #[serde(default)]
    pub mode: Mode,
    /// When `true`, a newly submitted event marks the active run
    /// superseded — [`crate::Queue::submit`] reports
    /// `cancel_active` and [`crate::Queue::take_cancel`] hands the
    /// superseded job to the caller. When `false`, new events always
    /// queue behind the active run.
    #[serde(default)]
    pub cancel_running: bool,
}

impl Supersede {
    /// Resolve this policy's key template against `event` — the
    /// supersede key `submit` would assign.
    #[must_use]
    pub fn key_of(&self, event: &Event) -> String {
        resolve_key(&self.key, event)
    }
}

impl From<peregrin_core::SupersedeMode> for Mode {
    fn from(mode: peregrin_core::SupersedeMode) -> Self {
        match mode {
            peregrin_core::SupersedeMode::LatestWins => Self::LatestWins,
            peregrin_core::SupersedeMode::Fifo => Self::Fifo,
        }
    }
}

/// The parsed campaign spec → the queue policy the scheduler runs.
impl From<peregrin_core::Supersede> for Supersede {
    fn from(spec: peregrin_core::Supersede) -> Self {
        Self {
            key: spec.key,
            mode: spec.mode.into(),
            cancel_running: spec.cancel_running,
        }
    }
}

/// Resolve a `{dotted.path}` key template against `event.payload`.
///
/// Each `{path}` placeholder walks the payload object one segment per
/// `.` and stringifies via [`js_string`] (`null` → `"null"`, objects
/// → `"[object Object]"`, arrays join with `,`). A lookup miss —
/// absent key, non-object step, empty or unclosed placeholder, an
/// interior containing `{` — leaves the placeholder text verbatim, so
/// the resolved key is always deterministic for a given event. A `{{`
/// pair emits a literal `{` and rescans the remainder, so `{{repo}}`
/// renders `{` + the resolved `repo` + `}`.
#[must_use]
pub fn resolve_key(template: &str, event: &Event) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            // Unclosed `{` — literal; rescan the text after it.
            out.push('{');
            rest = after_open;
            continue;
        };
        let path = &after_open[..close];
        if path.is_empty() || path.contains('{') {
            // `{}` or `{{...` — the `{` is literal; keep scanning
            // after it (`{{repo}}` → `{` + `{repo}}` → `{repo}`… the
            // inner placeholder still resolves on the next pass).
            out.push('{');
            rest = after_open;
            continue;
        }
        if let Some(value) = lookup(&event.payload, path) {
            out.push_str(&js_string(value));
        } else {
            out.push('{');
            out.push_str(path);
            out.push('}');
        }
        rest = &after_open[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Dotted-path walk over a payload object — `Some(&value)` on a full
/// hit (including explicit `null`), `None` on any miss.
fn lookup<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = payload;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}
