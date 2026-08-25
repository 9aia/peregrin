use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Command {
    pub name: String,
    pub target: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct CommandOutcome {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandOutcome {
    pub fn new(exit_code: i32) -> Self {
        Self {
            exit_code,
            ..Default::default()
        }
    }

    pub fn with_stdout(mut self, output: impl AsRef<[u8]>) -> Self {
        self.stdout = output.as_ref().to_vec();
        self
    }

    pub fn with_stderr(mut self, output: impl AsRef<[u8]>) -> Self {
        self.stderr = output.as_ref().to_vec();
        self
    }
}

#[derive(Debug)]
pub enum CliPluginError {
    Io(std::io::Error),
    UnknownCommand(String),
    UnknownCapability(String),
    Plugin(String),
}

impl fmt::Display for CliPluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliPluginError::Io(e) => write!(f, "io error: {e}"),
            CliPluginError::UnknownCommand(name) => write!(f, "unknown command: {name}"),
            CliPluginError::UnknownCapability(name) => write!(f, "unknown capability: {name}"),
            CliPluginError::Plugin(msg) => write!(f, "plugin error: {msg}"),
        }
    }
}

impl std::error::Error for CliPluginError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliPluginError::Io(e) => Some(e),
            _ => None,
        }
    }
}

pub trait CliPlugin {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &[&str];
    fn run(
        &self,
        command: &Command,
        ctx: &CliContext<'_>,
    ) -> Result<CommandOutcome, CliPluginError>;
}

pub struct CliContext<'a> {
    pub registry: &'a PluginRegistry,
    pub manifest: Option<ProjectManifest>,
}

impl<'a> CliContext<'a> {
    pub fn new(registry: &'a PluginRegistry, manifest: Option<ProjectManifest>) -> Self {
        Self { registry, manifest }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ProjectTools {
    pub formatter: Option<String>,
    pub typechecker: Option<String>,
    pub compiler: Option<String>,
    pub linter: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ProjectManifest {
    pub tools: ProjectTools,
}

impl ProjectManifest {
    pub fn load() -> Result<Self, CliPluginError> {
        let start = std::env::current_dir().map_err(CliPluginError::Io)?;
        let candidates = ["Peregrin.toml", "peregrin.toml", "project.toml"];

        let mut found = None;
        let mut dir: Option<&Path> = Some(&start);
        while let Some(d) = dir {
            for name in &candidates {
                let p = d.join(name);
                if p.exists() {
                    found = Some(std::fs::read_to_string(&p).map_err(CliPluginError::Io)?);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
            dir = d.parent();
        }

        let content = match found {
            Some(c) => c,
            None => return Ok(Self::default()),
        };

        let value: toml::Value =
            toml::from_str(&content).map_err(|e| CliPluginError::Plugin(format!("{e}")))?;

        let mut tools = ProjectTools::default();
        if let Some(table) = value.get("tools").and_then(|v| v.as_table()) {
            if let Some(v) = table.get("formatter").and_then(|v| v.as_str()) {
                tools.formatter = Some(v.to_string());
            }
            if let Some(v) = table.get("typechecker").and_then(|v| v.as_str()) {
                tools.typechecker = Some(v.to_string());
            }
            if let Some(v) = table.get("compiler").and_then(|v| v.as_str()) {
                tools.compiler = Some(v.to_string());
            }
            if let Some(v) = table.get("linter").and_then(|v| v.as_str()) {
                tools.linter = Some(v.to_string());
            }
        }

        Ok(Self { tools })
    }
}

pub struct PluginRegistry {
    plugins: Vec<Box<dyn CliPlugin + 'static>>,
    by_capability: HashMap<String, Vec<usize>>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugins", &self.plugins.len())
            .field("by_capability", &self.by_capability)
            .finish()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            by_capability: HashMap::new(),
        }
    }

    pub fn register<P: CliPlugin + 'static>(&mut self, plugin: P) {
        let idx = self.plugins.len();
        for &cap in plugin.capabilities() {
            self.by_capability
                .entry(cap.to_string())
                .or_default()
                .push(idx);
        }
        self.plugins.push(Box::new(plugin));
    }

    pub fn resolve(&self, capability: &str, preferred: Option<&str>) -> Option<&dyn CliPlugin> {
        let indices = self.by_capability.get(capability)?;
        if let Some(name) = preferred {
            for &idx in indices {
                let p = &*self.plugins[idx];
                if p.name() == name {
                    return Some(p);
                }
            }
        }
        let p = &*self.plugins[*indices.first()?];
        Some(p)
    }

    pub fn run(
        &self,
        command: &Command,
        ctx: &CliContext<'_>,
    ) -> Result<CommandOutcome, CliPluginError> {
        let plugin = self
            .resolve(&command.name, command.target.as_deref())
            .ok_or_else(|| CliPluginError::UnknownCommand(command.name.clone()))?;
        plugin.run(command, ctx)
    }
}

pub struct InProcessPlugin<F>
where
    F: for<'a, 'b> Fn(&'a Command, &'b CliContext<'b>) -> Result<CommandOutcome, CliPluginError>
        + 'static,
{
    name: String,
    capabilities: Vec<&'static str>,
    run_fn: F,
}

impl<F> InProcessPlugin<F>
where
    F: for<'a, 'b> Fn(&'a Command, &'b CliContext<'b>) -> Result<CommandOutcome, CliPluginError>
        + 'static,
{
    pub fn new(name: impl Into<String>, capabilities: &[&'static str], run_fn: F) -> Self {
        Self {
            name: name.into(),
            capabilities: capabilities.to_vec(),
            run_fn,
        }
    }
}

impl<F> CliPlugin for InProcessPlugin<F>
where
    F: for<'a, 'b> Fn(&'a Command, &'b CliContext<'b>) -> Result<CommandOutcome, CliPluginError>
        + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &[&str] {
        &self.capabilities
    }

    fn run(
        &self,
        command: &Command,
        ctx: &CliContext<'_>,
    ) -> Result<CommandOutcome, CliPluginError> {
        (self.run_fn)(command, ctx)
    }
}

#[derive(Debug)]
pub enum ProcessPluginError {
    NotImplemented,
}

impl fmt::Display for ProcessPluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessPluginError::NotImplemented => write!(f, "process plugin not implemented"),
        }
    }
}

impl std::error::Error for ProcessPluginError {}

pub struct ProcessPlugin {
    name: String,
    capabilities: Vec<&'static str>,
    path: PathBuf,
}

impl ProcessPlugin {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        capabilities: &[&'static str],
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            capabilities: capabilities.to_vec(),
        }
    }

    pub fn spawn(&self) -> Result<std::process::Child, std::io::Error> {
        std::process::Command::new(&self.path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
    }

    #[allow(dead_code)]
    pub fn request(&self, _method: &str, _params: String) -> Result<String, ProcessPluginError> {
        Err(ProcessPluginError::NotImplemented)
    }
}

impl CliPlugin for ProcessPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &[&str] {
        &self.capabilities
    }

    fn run(
        &self,
        _command: &Command,
        _ctx: &CliContext<'_>,
    ) -> Result<CommandOutcome, CliPluginError> {
        Err(CliPluginError::Plugin(format!(
            "{}",
            ProcessPluginError::NotImplemented
        )))
    }
}
