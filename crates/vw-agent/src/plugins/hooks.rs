//! 插件钩子清单的静态解析与路径校验。

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use super::audit::canonical_plugin_root;

/// Supported plugin hook lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginHookEvent {
    BeforeToolCall,
    AfterToolCall,
    BeforeLlmCall,
    UserPromptSubmit,
    InstructionsLoaded,
}

impl PluginHookEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BeforeToolCall => "before_tool_call",
            Self::AfterToolCall => "after_tool_call",
            Self::BeforeLlmCall => "before_llm_call",
            Self::UserPromptSubmit => "user_prompt_submit",
            Self::InstructionsLoaded => "instructions_loaded",
        }
    }
}

/// Execution mode for a plugin hook declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginHookMode {
    Blocking,
}

impl PluginHookMode {
    fn parse(raw: Option<String>) -> Result<Self> {
        let Some(raw) = raw else {
            return Ok(Self::Blocking);
        };
        match raw.trim() {
            "" | "blocking" => Ok(Self::Blocking),
            other => bail!("unsupported plugin hook mode: {other}"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
        }
    }
}

/// A hook declaration after schema parsing and path audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHook {
    pub event: PluginHookEvent,
    pub command: String,
    pub args: Vec<String>,
    pub timeout_secs: Option<u64>,
    pub working_dir: PathBuf,
    pub mode: PluginHookMode,
    pub manifest_path: PathBuf,
}

/// Read a plugin `hooks/hooks.json` file without executing any hook command.
pub fn load_plugin_hooks_manifest(
    hooks_path: &Path,
    plugin_root: &Path,
    workspace_dir: &Path,
) -> Result<Vec<PluginHook>> {
    let canonical_root = canonical_plugin_root(plugin_root)?;
    let canonical_workspace = workspace_dir.canonicalize().with_context(|| {
        format!("failed to canonicalize workspace: {}", workspace_dir.display())
    })?;
    let hooks_path = canonical_hooks_path(hooks_path, &canonical_root)?;
    let bytes = fs::read(&hooks_path).with_context(|| {
        format!("failed to read plugin hooks manifest: {}", hooks_path.display())
    })?;
    let raw: RawHookManifest = serde_json::from_slice(&bytes).with_context(|| {
        format!("failed to parse plugin hooks manifest: {}", hooks_path.display())
    })?;

    raw.into_hooks()
        .into_iter()
        .map(|hook| normalize_hook(hook, &hooks_path, &canonical_root, &canonical_workspace))
        .collect()
}

fn canonical_hooks_path(hooks_path: &Path, canonical_root: &Path) -> Result<PathBuf> {
    let path = hooks_path
        .canonicalize()
        .with_context(|| format!("plugin hooks manifest not found: {}", hooks_path.display()))?;
    if !path.starts_with(canonical_root) {
        bail!("Plugin hooks manifest path escapes plugin root: {}", hooks_path.display());
    }
    if !path.is_file() {
        bail!("Plugin hooks manifest must be a file: {}", hooks_path.display());
    }
    Ok(path)
}

fn normalize_hook(
    raw: RawPluginHook,
    manifest_path: &Path,
    canonical_root: &Path,
    canonical_workspace: &Path,
) -> Result<PluginHook> {
    let event = parse_event(&raw.event)?;
    let command = raw.command.trim().to_string();
    if command.is_empty() {
        bail!("plugin hook command must not be empty");
    }

    Ok(PluginHook {
        event,
        command,
        args: raw.args,
        timeout_secs: raw.timeout_secs,
        working_dir: audited_working_dir(
            raw.working_dir.as_deref(),
            canonical_root,
            canonical_workspace,
        )?,
        mode: PluginHookMode::parse(raw.mode)?,
        manifest_path: manifest_path.to_path_buf(),
    })
}

fn parse_event(raw: &str) -> Result<PluginHookEvent> {
    match raw.trim() {
        "before_tool_call" => Ok(PluginHookEvent::BeforeToolCall),
        "after_tool_call" => Ok(PluginHookEvent::AfterToolCall),
        "before_llm_call" => Ok(PluginHookEvent::BeforeLlmCall),
        "user_prompt_submit" => Ok(PluginHookEvent::UserPromptSubmit),
        "instructions_loaded" => Ok(PluginHookEvent::InstructionsLoaded),
        other => bail!("unsupported plugin hook event: {other}"),
    }
}

fn audited_working_dir(
    raw: Option<&str>,
    canonical_root: &Path,
    canonical_workspace: &Path,
) -> Result<PathBuf> {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return Ok(canonical_root.to_path_buf());
    };

    let raw_path = Path::new(raw);
    let candidate =
        if raw_path.is_absolute() { raw_path.to_path_buf() } else { canonical_root.join(raw_path) };
    let working_dir = candidate
        .canonicalize()
        .with_context(|| format!("plugin hook working_dir does not exist: {raw}"))?;
    if !working_dir.is_dir() {
        bail!("plugin hook working_dir must be a directory: {raw}");
    }
    if !working_dir.starts_with(canonical_root) && !working_dir.starts_with(canonical_workspace) {
        bail!("plugin hook working_dir escapes plugin root and workspace: {raw}");
    }
    Ok(working_dir)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawHookManifest {
    List(Vec<RawPluginHook>),
    Object {
        #[serde(default)]
        hooks: Vec<RawPluginHook>,
    },
}

impl RawHookManifest {
    fn into_hooks(self) -> Vec<RawPluginHook> {
        match self {
            Self::List(hooks) => hooks,
            Self::Object { hooks } => hooks,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawPluginHook {
    event: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    mode: Option<String>,
}
