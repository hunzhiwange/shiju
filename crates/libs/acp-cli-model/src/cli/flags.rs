//! CLI 标志位的解析、校验与归一化。

use std::path::{Component, PathBuf};

use acp_agent_registry::agent_registry::{
    DEFAULT_AGENT_NAME, resolve_agent_command, resolve_agent_spec_with_overrides,
};
use acp_config::config::ResolvedAcpxConfig;
use acp_types::types::{
    AcpAgentConfig, AuthPolicy, NonInteractivePermissionPolicy, OutputFormat, OutputPolicy,
    PermissionMode,
};

const DEFAULT_QUEUE_OWNER_TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionFlags {
    pub approve_all: bool,
    pub approve_reads: bool,
    pub deny_all: bool,
}

pub fn has_explicit_permission_mode_flag(flags: &PermissionFlags) -> bool {
    flags.approve_all || flags.approve_reads || flags.deny_all
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalFlags {
    pub agent: Option<String>,
    pub cwd: String,
    pub auth_policy: Option<AuthPolicy>,
    pub non_interactive_permissions: NonInteractivePermissionPolicy,
    pub json_strict: bool,
    pub suppress_reads: bool,
    pub timeout: Option<u64>,
    pub ttl: u64,
    pub verbose: bool,
    pub format: OutputFormat,
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<i64>,
    pub prompt_retries: Option<u64>,
    pub approve_all: bool,
    pub approve_reads: bool,
    pub deny_all: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptFlags {
    pub session: Option<String>,
    pub wait: Option<bool>,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExecFlags {
    pub file: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionsNewFlags {
    pub name: Option<String>,
    pub resume_session: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionsHistoryFlags {
    pub limit: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusFlags {
    pub session: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GlobalFlagOptions {
    pub agent: Option<String>,
    pub cwd: Option<String>,
    pub auth_policy: Option<AuthPolicy>,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub json_strict: bool,
    pub suppress_reads: bool,
    pub timeout: Option<u64>,
    pub ttl: Option<u64>,
    pub verbose: bool,
    pub format: Option<OutputFormat>,
    pub model: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_turns: Option<i64>,
    pub prompt_retries: Option<u64>,
    pub approve_all: bool,
    pub approve_reads: bool,
    pub deny_all: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct FlagsError {
    message: String,
}

impl FlagsError {
    fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

fn parse_number(value: &str) -> Result<f64, FlagsError> {
    value.trim().parse::<f64>().map_err(|_| FlagsError::new(format!("Invalid number \"{value}\"")))
}

fn parse_integer(value: &str, label: &str, minimum: i64) -> Result<i64, FlagsError> {
    let parsed = parse_number(value)?;
    if !parsed.is_finite() || parsed.fract() != 0.0 || parsed < minimum as f64 {
        return Err(FlagsError::new(label.to_string()));
    }
    Ok(parsed as i64)
}

pub fn parse_output_format(value: &str) -> Result<OutputFormat, FlagsError> {
    match value.trim() {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "quiet" => Ok(OutputFormat::Quiet),
        other => Err(FlagsError::new(format!(
            "Invalid format \"{other}\". Expected one of: text, json, quiet"
        ))),
    }
}

pub fn parse_auth_policy(value: &str) -> Result<AuthPolicy, FlagsError> {
    match value.trim() {
        "skip" => Ok(AuthPolicy::Skip),
        "fail" => Ok(AuthPolicy::Fail),
        other => Err(FlagsError::new(format!(
            "Invalid auth policy \"{other}\". Expected one of: skip, fail"
        ))),
    }
}

pub fn parse_non_interactive_permission_policy(
    value: &str,
) -> Result<NonInteractivePermissionPolicy, FlagsError> {
    match value.trim() {
        "deny" => Ok(NonInteractivePermissionPolicy::Deny),
        "fail" => Ok(NonInteractivePermissionPolicy::Fail),
        other => Err(FlagsError::new(format!(
            "Invalid non-interactive permission policy \"{other}\". Expected one of: deny, fail"
        ))),
    }
}

pub fn parse_timeout_seconds(value: &str) -> Result<u64, FlagsError> {
    let parsed = parse_number(value)?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(FlagsError::new("Timeout must be a positive number of seconds"));
    }
    Ok((parsed * 1000.0).round() as u64)
}

pub fn parse_ttl_seconds(value: &str) -> Result<u64, FlagsError> {
    let parsed = parse_number(value)?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(FlagsError::new("TTL must be a non-negative number of seconds"));
    }
    Ok((parsed * 1000.0).round() as u64)
}

pub fn parse_session_name(value: &str) -> Result<String, FlagsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FlagsError::new("Session name must not be empty"));
    }
    Ok(trimmed.to_string())
}

pub fn parse_non_empty_value(label: &str, value: &str) -> Result<String, FlagsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FlagsError::new(format!("{label} must not be empty")));
    }
    Ok(trimmed.to_string())
}

pub fn parse_history_limit(value: &str) -> Result<usize, FlagsError> {
    let parsed = parse_integer(value, "Limit must be a positive integer", 1)?;
    usize::try_from(parsed).map_err(|_| FlagsError::new("Limit must be a positive integer"))
}

pub fn parse_allowed_tools(value: &str) -> Result<Vec<String>, FlagsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let items = trimmed.split(',').map(str::trim).map(ToString::to_string).collect::<Vec<_>>();
    if items.iter().any(String::is_empty) {
        return Err(FlagsError::new(
            "Allowed tools must be a comma-separated list without empty entries",
        ));
    }
    Ok(items)
}

pub fn parse_max_turns(value: &str) -> Result<i64, FlagsError> {
    parse_integer(value, "Max turns must be a positive integer", 1)
}

pub fn parse_prompt_retries(value: &str) -> Result<u64, FlagsError> {
    let parsed = parse_integer(value, "Prompt retries must be a non-negative integer", 0)?;
    u64::try_from(parsed)
        .map_err(|_| FlagsError::new("Prompt retries must be a non-negative integer"))
}

pub fn resolve_permission_mode(
    flags: &PermissionFlags,
    default_mode: PermissionMode,
) -> Result<PermissionMode, FlagsError> {
    let selected = [flags.approve_all, flags.approve_reads, flags.deny_all]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if selected > 1 {
        return Err(FlagsError::new(
            "Use only one permission mode: --approve-all, --approve-reads, or --deny-all",
        ));
    }
    if flags.approve_all {
        return Ok(PermissionMode::ApproveAll);
    }
    if flags.approve_reads {
        return Ok(PermissionMode::ApproveReads);
    }
    if flags.deny_all {
        return Ok(PermissionMode::DenyAll);
    }
    Ok(default_mode)
}

#[cfg(test)]
#[path = "flags_tests.rs"]
mod flags_tests;

fn default_cwd_string() -> String {
    std::env::current_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| ".".to_string())
}

pub fn resolve_global_flags(
    options: &GlobalFlagOptions,
    config: &ResolvedAcpxConfig,
) -> Result<GlobalFlags, FlagsError> {
    let format = options.format.unwrap_or(config.format);
    let json_strict = options.json_strict;
    let verbose = options.verbose;

    if json_strict && !matches!(format, OutputFormat::Json) {
        return Err(FlagsError::new("--json-strict requires --format json"));
    }
    if json_strict && verbose {
        return Err(FlagsError::new("--json-strict cannot be combined with --verbose"));
    }

    let model =
        options.model.as_deref().map(|value| parse_non_empty_value("Model", value)).transpose()?;

    Ok(GlobalFlags {
        agent: options.agent.clone(),
        cwd: options.cwd.clone().unwrap_or_else(default_cwd_string),
        auth_policy: options.auth_policy.or(Some(config.auth_policy)),
        non_interactive_permissions: options
            .non_interactive_permissions
            .unwrap_or(config.non_interactive_permissions),
        json_strict,
        suppress_reads: options.suppress_reads,
        timeout: options.timeout.or(config.timeout_ms),
        ttl: options.ttl.unwrap_or(if config.ttl_ms == 0 {
            DEFAULT_QUEUE_OWNER_TTL_MS
        } else {
            config.ttl_ms
        }),
        verbose,
        format,
        model,
        allowed_tools: options.allowed_tools.clone(),
        max_turns: options.max_turns,
        prompt_retries: options.prompt_retries,
        approve_all: options.approve_all,
        approve_reads: options.approve_reads,
        deny_all: options.deny_all,
    })
}

pub fn resolve_output_policy(format: OutputFormat, json_strict: bool) -> OutputPolicy {
    OutputPolicy {
        format,
        json_strict,
        suppress_reads: false,
        suppress_non_json_stderr: json_strict,
        queue_error_already_emitted: !matches!(format, OutputFormat::Quiet),
        suppress_sdk_console_errors: json_strict,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAgentInvocation {
    pub agent_name: String,
    pub agent_command: String,
    pub agent_config: Option<AcpAgentConfig>,
    pub cwd: String,
}

fn resolve_absolute_path(path: &str) -> String {
    let cwd = strip_current_dir_components(PathBuf::from(path));
    let absolute = if cwd.is_absolute() {
        cwd
    } else {
        std::env::current_dir().map_or_else(|_| cwd.clone(), |base| base.join(&cwd))
    };

    normalize_absolute_components(absolute).to_string_lossy().into_owned()
}

fn strip_current_dir_components(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        push_path_component(&mut normalized, component, false);
    }
    normalized
}

fn normalize_absolute_components(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        push_path_component(&mut normalized, component, true);
    }
    normalized
}

fn push_path_component(normalized: &mut PathBuf, component: Component<'_>, collapse_parent: bool) {
    match component {
        Component::CurDir => {}
        Component::ParentDir if collapse_parent => {
            normalized.pop();
        }
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            normalized.push(component.as_os_str());
        }
        Component::Normal(part) => normalized.push(part),
    }
}

pub fn resolve_agent_invocation(
    explicit_agent_name: Option<&str>,
    global_flags: &GlobalFlags,
    config: &ResolvedAcpxConfig,
) -> Result<ResolvedAgentInvocation, FlagsError> {
    let override_command = global_flags
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if override_command.is_some() && explicit_agent_name.is_some() {
        return Err(FlagsError::new("Do not combine positional agent with --agent override"));
    }

    let agent_name = explicit_agent_name
        .map(ToString::to_string)
        .or_else(|| {
            let trimmed = config.default_agent.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .unwrap_or_else(|| DEFAULT_AGENT_NAME.to_string());

    let agent_command = override_command
        .unwrap_or_else(|| resolve_agent_command(&agent_name, Some(&config.agents)));
    let agent_config = if global_flags.agent.is_some() {
        None
    } else {
        resolve_agent_spec_with_overrides(&agent_name, Some(&config.agents))
            .map(|spec| AcpAgentConfig::from(&spec))
    };

    Ok(ResolvedAgentInvocation {
        agent_name,
        agent_command,
        agent_config,
        cwd: resolve_absolute_path(&global_flags.cwd),
    })
}

pub fn resolve_session_name_from_flags(
    flags: &StatusFlags,
    global_session: Option<&str>,
) -> Result<Option<String>, FlagsError> {
    if let Some(session) = &flags.session {
        return Ok(Some(parse_session_name(session)?));
    }
    global_session.map(parse_session_name).transpose()
}
