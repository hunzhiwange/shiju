//! ACP 配置文件的加载、解析与归一化。
//!
//! 本模块负责把 ACP 全局配置、项目配置、项目 `.vibewindow.json` 覆盖以及命令行传入的配置标识整合成
//! 统一的运行时配置结构，供 CLI 和会话运行时共享使用。
//!
//! # 主要职责
//!
//! - 解析磁盘上的配置文件
//! - 合并 ACP 全局配置与项目级配置
//! - 生成面向运行时的已解析配置视图
//! - 提供初始化默认配置文件的辅助逻辑

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

use agent_client_protocol::McpServer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::fs;

use crate::ParseMcpServersError;
use crate::agent_registry::{AgentCommandSpec, DEFAULT_AGENT_NAME, normalize_agent_name};
use crate::mcp_servers::{McpServerConfig, McpServerManagement, parse_mcp_server_configs};
use crate::types::{AuthPolicy, NonInteractivePermissionPolicy, OutputFormat, PermissionMode};

const DEFAULT_TIMEOUT_MS: Option<u64> = None;
const DEFAULT_TTL_MS: u64 = 300_000;
const DEFAULT_PERMISSION_MODE: PermissionMode = PermissionMode::ApproveAll;
const DEFAULT_NON_INTERACTIVE_PERMISSION_POLICY: NonInteractivePermissionPolicy =
    NonInteractivePermissionPolicy::Deny;
const DEFAULT_AUTH_POLICY: AuthPolicy = AuthPolicy::Skip;
const DEFAULT_OUTPUT_FORMAT: OutputFormat = OutputFormat::Text;
const DEFAULT_QUEUE_MAX_DEPTH: usize = 16;
const DEFAULT_DISABLE_EXEC: bool = false;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("HOME directory is unavailable")]
    HomeDirUnavailable,
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write config file {path}: {source}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create config directory {path}: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid JSON in {path}: {reason}")]
    InvalidJson { path: String, reason: String },
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    McpServers(#[from] ParseMcpServersError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigAgentEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAcpxConfig {
    pub default_agent: String,
    pub default_permissions: PermissionMode,
    pub non_interactive_permissions: NonInteractivePermissionPolicy,
    pub auth_policy: AuthPolicy,
    pub ttl_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    pub queue_max_depth: usize,
    pub format: OutputFormat,
    pub agents: HashMap<String, AgentCommandSpec>,
    pub auth: HashMap<String, String>,
    pub disable_exec: bool,
    pub mcp_servers: Vec<McpServer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_server_management: Vec<McpServerManagement>,
    pub global_path: String,
    pub project_path: String,
    pub has_global_config: bool,
    pub has_project_config: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDisplay {
    pub default_agent: String,
    pub default_permissions: PermissionMode,
    pub non_interactive_permissions: NonInteractivePermissionPolicy,
    pub auth_policy: AuthPolicy,
    pub ttl: u64,
    pub timeout: Option<u64>,
    pub queue_max_depth: usize,
    pub format: OutputFormat,
    pub agents: HashMap<String, ConfigAgentEntry>,
    pub auth_methods: Vec<String>,
    pub disable_exec: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitGlobalConfigFileResult {
    pub path: String,
    pub created: bool,
}

#[derive(Debug)]
struct ConfigFileLoadResult {
    config: Option<Map<String, Value>>,
    exists: bool,
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn field_exists(config: Option<&Map<String, Value>>, field: &str) -> bool {
    config.is_some_and(|config| config.contains_key(field))
}

fn config_value<'a>(config: Option<&'a Map<String, Value>>, field: &str) -> Option<&'a Value> {
    config.and_then(|config| config.get(field))
}

fn invalid(field: &str, source_path: &str, expected: &str) -> ConfigError {
    ConfigError::Invalid(format!("Invalid config {field} in {source_path}: expected {expected}"))
}

fn parse_ttl_ms(value: Option<&Value>, source_path: &str) -> Result<Option<u64>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(seconds) = value.as_f64() else {
        return Err(invalid("ttl", source_path, "non-negative seconds"));
    };
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(invalid("ttl", source_path, "non-negative seconds"));
    }
    Ok(Some((seconds * 1_000.0).round() as u64))
}

fn parse_timeout_ms(value: Option<&Value>, source_path: &str) -> Result<Option<u64>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(seconds) = value.as_f64() else {
        return Err(invalid("timeout", source_path, "positive seconds or null"));
    };
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(invalid("timeout", source_path, "positive seconds or null"));
    }
    Ok(Some((seconds * 1_000.0).round() as u64))
}

fn parse_queue_max_depth(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<usize>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(depth) = value.as_u64() else {
        return Err(invalid("queueMaxDepth", source_path, "positive integer"));
    };
    if depth == 0 {
        return Err(invalid("queueMaxDepth", source_path, "positive integer"));
    }
    Ok(Some(depth as usize))
}

fn parse_permission_mode(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<PermissionMode>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(invalid(
            "defaultPermissions",
            source_path,
            "approve-all, approve-reads, or deny-all",
        ));
    };
    let parsed = match value {
        "approve-all" => PermissionMode::ApproveAll,
        "approve-reads" => PermissionMode::ApproveReads,
        "deny-all" => PermissionMode::DenyAll,
        _ => {
            return Err(invalid(
                "defaultPermissions",
                source_path,
                "approve-all, approve-reads, or deny-all",
            ));
        }
    };
    Ok(Some(parsed))
}

fn parse_non_interactive_permission_policy(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<NonInteractivePermissionPolicy>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(invalid("nonInteractivePermissions", source_path, "deny or fail"));
    };
    let parsed = match value {
        "deny" => NonInteractivePermissionPolicy::Deny,
        "fail" => NonInteractivePermissionPolicy::Fail,
        _ => {
            return Err(invalid("nonInteractivePermissions", source_path, "deny or fail"));
        }
    };
    Ok(Some(parsed))
}

fn parse_auth_policy(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<AuthPolicy>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(invalid("authPolicy", source_path, "skip or fail"));
    };
    let parsed = match value {
        "skip" => AuthPolicy::Skip,
        "fail" => AuthPolicy::Fail,
        _ => return Err(invalid("authPolicy", source_path, "skip or fail")),
    };
    Ok(Some(parsed))
}

fn parse_output_format(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<OutputFormat>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(invalid("format", source_path, "text, json, or quiet"));
    };
    let parsed = match value {
        "text" => OutputFormat::Text,
        "json" => OutputFormat::Json,
        "quiet" => OutputFormat::Quiet,
        _ => return Err(invalid("format", source_path, "text, json, or quiet")),
    };
    Ok(Some(parsed))
}

fn parse_default_agent(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<String>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(invalid("defaultAgent", source_path, "non-empty string"));
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid("defaultAgent", source_path, "non-empty string"));
    }
    Ok(Some(normalize_agent_name(trimmed)))
}

fn parse_agents(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<HashMap<String, AgentCommandSpec>>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(agents) = value.as_object() else {
        return Err(invalid("agents", source_path, "object"));
    };

    let mut parsed = HashMap::new();
    for (name, raw) in agents {
        let Some(raw) = raw.as_object() else {
            return Err(ConfigError::Invalid(format!(
                "Invalid config agents.{name} in {source_path}: expected object with command"
            )));
        };
        let display_name = raw
            .get("name")
            .map(|value| {
                value.as_str().ok_or_else(|| {
                    ConfigError::Invalid(format!(
                        "Invalid config agents.{name}.name in {source_path}: expected string"
                    ))
                })
            })
            .transpose()?
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| name.trim().to_string());
        let Some(command) = raw.get("command").and_then(Value::as_str) else {
            return Err(ConfigError::Invalid(format!(
                "Invalid config agents.{name}.command in {source_path}: expected non-empty string"
            )));
        };
        let command = command.trim();
        if command.is_empty() {
            return Err(ConfigError::Invalid(format!(
                "Invalid config agents.{name}.command in {source_path}: expected non-empty string"
            )));
        }
        let args = match raw.get("args") {
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| {
                    value.as_str().map(str::to_string).ok_or_else(|| {
                        ConfigError::Invalid(format!(
                            "Invalid config agents.{name}.args in {source_path}: expected string array"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(ConfigError::Invalid(format!(
                    "Invalid config agents.{name}.args in {source_path}: expected string array"
                )));
            }
            None => Vec::new(),
        };
        let env = match raw.get("env") {
            Some(Value::Object(entries)) => entries
                .iter()
                .map(|(key, value)| {
                    value
                        .as_str()
                        .map(|value| (key.clone(), value.to_string()))
                        .ok_or_else(|| {
                            ConfigError::Invalid(format!(
                                "Invalid config agents.{name}.env.{key} in {source_path}: expected string"
                            ))
                        })
                })
                .collect::<Result<HashMap<_, _>, _>>()?,
            Some(_) => {
                return Err(ConfigError::Invalid(format!(
                    "Invalid config agents.{name}.env in {source_path}: expected object"
                )));
            }
            None => HashMap::new(),
        };
        parsed.insert(
            normalize_agent_name(name),
            AgentCommandSpec { display_name, command: command.to_string(), args, env },
        );
    }

    Ok(Some(parsed))
}

fn merge_agent_maps(
    base: Option<HashMap<String, AgentCommandSpec>>,
    overlay: Option<HashMap<String, AgentCommandSpec>>,
) -> HashMap<String, AgentCommandSpec> {
    let mut merged = base.unwrap_or_default();
    if let Some(overlay) = overlay {
        merged.extend(overlay);
    }
    merged
}

#[cfg(windows)]
fn default_vibewindow_config_dir() -> Result<PathBuf, ConfigError> {
    if let Some(home) = env::var_os("USERPROFILE") {
        return Ok(vw_config_types::paths::home_config_dir(PathBuf::from(home)));
    }
    let Some(home) = env::var_os("HOME") else {
        return Err(ConfigError::HomeDirUnavailable);
    };
    Ok(vw_config_types::paths::home_config_dir(PathBuf::from(home)))
}

#[cfg(not(windows))]
fn default_vibewindow_config_dir() -> Result<PathBuf, ConfigError> {
    let Some(home) = env::var_os("HOME") else {
        return Err(ConfigError::HomeDirUnavailable);
    };
    Ok(vw_config_types::paths::home_config_dir(PathBuf::from(home)))
}

fn project_config_override_path(project_path: &Path) -> PathBuf {
    let project_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
    project_dir.join(".vibewindow.json")
}

fn parse_auth(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<HashMap<String, String>>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(auth) = value.as_object() else {
        return Err(invalid("auth", source_path, "object"));
    };

    let mut parsed = HashMap::new();
    for (method_id, credential) in auth {
        let Some(credential) = credential.as_str() else {
            return Err(ConfigError::Invalid(format!(
                "Invalid config auth.{method_id} in {source_path}: expected non-empty string"
            )));
        };
        let credential = credential.trim();
        if credential.is_empty() {
            return Err(ConfigError::Invalid(format!(
                "Invalid config auth.{method_id} in {source_path}: expected non-empty string"
            )));
        }
        parsed.insert(method_id.clone(), credential.to_string());
    }
    Ok(Some(parsed))
}

fn parse_disable_exec(
    value: Option<&Value>,
    source_path: &str,
) -> Result<Option<bool>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    value.as_bool().map(Some).ok_or_else(|| invalid("disableExec", source_path, "boolean"))
}

async fn read_config_file(file_path: &Path) -> Result<ConfigFileLoadResult, ConfigError> {
    match fs::read_to_string(file_path).await {
        Ok(payload) => {
            let parsed: Value =
                serde_json::from_str(&payload).map_err(|error| ConfigError::InvalidJson {
                    path: path_to_string(file_path),
                    reason: error.to_string(),
                })?;
            let Some(config) = parsed.as_object() else {
                return Err(ConfigError::Invalid(format!(
                    "Invalid config in {}: expected top-level JSON object",
                    path_to_string(file_path)
                )));
            };
            Ok(ConfigFileLoadResult { config: Some(config.clone()), exists: true })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ConfigFileLoadResult { config: None, exists: false })
        }
        Err(error) => Err(ConfigError::Read { path: path_to_string(file_path), source: error }),
    }
}

fn merge_maps(
    global: Option<HashMap<String, String>>,
    project: Option<HashMap<String, String>>,
) -> HashMap<String, String> {
    let mut merged = global.unwrap_or_default();
    if let Some(project) = project {
        merged.extend(project);
    }
    merged
}

fn parse_mcp_server_config_pair(
    value: &Value,
    source_path: &str,
) -> Result<(Vec<McpServer>, Vec<McpServerManagement>), ConfigError> {
    let configs = parse_mcp_server_configs(value, source_path)?;
    let (servers, management): (Vec<_>, Vec<_>) = configs
        .into_iter()
        .map(|McpServerConfig { server, management }| (server, management))
        .unzip();
    Ok((servers, management))
}

fn resolve_path(path: impl AsRef<Path>) -> Result<PathBuf, ConfigError> {
    let path = path.as_ref();
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(env::current_dir()
        .map_err(|error| ConfigError::Read { path: ".".to_string(), source: error })?
        .join(path))
}

pub fn default_global_config_path() -> Result<PathBuf, ConfigError> {
    Ok(default_vibewindow_config_dir()?.join("acp").join("config.json"))
}

pub fn project_config_path(cwd: impl AsRef<Path>) -> Result<PathBuf, ConfigError> {
    Ok(resolve_path(cwd)?.join(".vwacprc.json"))
}

pub async fn load_resolved_config(
    cwd: impl AsRef<Path>,
) -> Result<ResolvedAcpxConfig, ConfigError> {
    let global_path = default_global_config_path()?;
    let project_path = project_config_path(cwd)?;
    load_resolved_config_from_paths(global_path, project_path).await
}

pub async fn load_resolved_config_from_paths(
    global_path: impl AsRef<Path>,
    project_path: impl AsRef<Path>,
) -> Result<ResolvedAcpxConfig, ConfigError> {
    let global_path = global_path.as_ref().to_path_buf();
    let project_path = project_path.as_ref().to_path_buf();
    let vibewindow_path = project_config_override_path(&project_path);
    let global_path_string = path_to_string(&global_path);
    let project_path_string = path_to_string(&project_path);
    let vibewindow_path_string = path_to_string(&vibewindow_path);

    let (global_result, project_result, vibewindow_result) = tokio::join!(
        read_config_file(&global_path),
        read_config_file(&project_path),
        read_config_file(&vibewindow_path)
    );
    let global_result = global_result?;
    let project_result = project_result?;
    let vibewindow_result = vibewindow_result?;

    let global_config = global_result.config.as_ref();
    let project_config = project_result.config.as_ref();
    let vibewindow_config = vibewindow_result.config.as_ref();

    let default_agent =
        parse_default_agent(config_value(project_config, "defaultAgent"), &project_path_string)?
            .or(parse_default_agent(
                config_value(global_config, "defaultAgent"),
                &global_path_string,
            )?)
            .unwrap_or_else(|| DEFAULT_AGENT_NAME.to_string());

    let default_permissions = parse_permission_mode(
        config_value(project_config, "defaultPermissions"),
        &project_path_string,
    )?
    .or(parse_permission_mode(
        config_value(global_config, "defaultPermissions"),
        &global_path_string,
    )?)
    .unwrap_or(DEFAULT_PERMISSION_MODE);

    let non_interactive_permissions = parse_non_interactive_permission_policy(
        config_value(project_config, "nonInteractivePermissions"),
        &project_path_string,
    )?
    .or(parse_non_interactive_permission_policy(
        config_value(global_config, "nonInteractivePermissions"),
        &global_path_string,
    )?)
    .unwrap_or(DEFAULT_NON_INTERACTIVE_PERMISSION_POLICY);

    let auth_policy =
        parse_auth_policy(config_value(project_config, "authPolicy"), &project_path_string)?
            .or(parse_auth_policy(config_value(global_config, "authPolicy"), &global_path_string)?)
            .unwrap_or(DEFAULT_AUTH_POLICY);

    let ttl_ms = parse_ttl_ms(config_value(project_config, "ttl"), &project_path_string)?
        .or(parse_ttl_ms(config_value(global_config, "ttl"), &global_path_string)?)
        .unwrap_or(DEFAULT_TTL_MS);

    let timeout_ms = if field_exists(project_config, "timeout") {
        parse_timeout_ms(config_value(project_config, "timeout"), &project_path_string)?
    } else if field_exists(global_config, "timeout") {
        parse_timeout_ms(config_value(global_config, "timeout"), &global_path_string)?
    } else {
        DEFAULT_TIMEOUT_MS
    };

    let format = parse_output_format(config_value(project_config, "format"), &project_path_string)?
        .or(parse_output_format(config_value(global_config, "format"), &global_path_string)?)
        .unwrap_or(DEFAULT_OUTPUT_FORMAT);

    let queue_max_depth =
        parse_queue_max_depth(config_value(project_config, "queueMaxDepth"), &project_path_string)?
            .or(parse_queue_max_depth(
                config_value(global_config, "queueMaxDepth"),
                &global_path_string,
            )?)
            .unwrap_or(DEFAULT_QUEUE_MAX_DEPTH);

    let vibewindow_agents =
        parse_agents(config_value(vibewindow_config, "acp"), &vibewindow_path_string)?;
    let global_agents = merge_agent_maps(
        parse_agents(config_value(global_config, "acp"), &global_path_string)?,
        parse_agents(config_value(global_config, "agents"), &global_path_string)?,
    );
    let project_agents = merge_agent_maps(
        parse_agents(config_value(project_config, "acp"), &project_path_string)?,
        parse_agents(config_value(project_config, "agents"), &project_path_string)?,
    );
    let agents = merge_agent_maps(
        Some(merge_agent_maps(vibewindow_agents, Some(global_agents))),
        Some(project_agents),
    );
    let auth = merge_maps(
        parse_auth(config_value(global_config, "auth"), &global_path_string)?,
        parse_auth(config_value(project_config, "auth"), &project_path_string)?,
    );

    let (mcp_servers, mcp_server_management) = if field_exists(project_config, "mcpServers") {
        parse_mcp_server_config_pair(
            config_value(project_config, "mcpServers").unwrap_or(&Value::Null),
            &project_path_string,
        )?
    } else if field_exists(global_config, "mcpServers") {
        parse_mcp_server_config_pair(
            config_value(global_config, "mcpServers").unwrap_or(&Value::Null),
            &global_path_string,
        )?
    } else {
        (Vec::new(), Vec::new())
    };

    let disable_exec =
        parse_disable_exec(config_value(project_config, "disableExec"), &project_path_string)?
            .or(parse_disable_exec(
                config_value(global_config, "disableExec"),
                &global_path_string,
            )?)
            .unwrap_or(DEFAULT_DISABLE_EXEC);

    Ok(ResolvedAcpxConfig {
        default_agent,
        default_permissions,
        non_interactive_permissions,
        auth_policy,
        ttl_ms,
        timeout_ms,
        queue_max_depth,
        format,
        agents,
        auth,
        disable_exec,
        mcp_servers,
        mcp_server_management,
        global_path: global_path_string,
        project_path: project_path_string,
        has_global_config: global_result.exists,
        has_project_config: project_result.exists,
    })
}

pub fn to_config_display(config: &ResolvedAcpxConfig) -> ConfigDisplay {
    let agents = config
        .agents
        .iter()
        .map(|(name, command)| {
            (
                name.clone(),
                ConfigAgentEntry {
                    name: Some(command.display_name.clone()),
                    command: command.command.clone(),
                    args: command.args.clone(),
                    env: command.env.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut auth_methods = config.auth.keys().cloned().collect::<Vec<_>>();
    auth_methods.sort_unstable();

    ConfigDisplay {
        default_agent: config.default_agent.clone(),
        default_permissions: config.default_permissions,
        non_interactive_permissions: config.non_interactive_permissions,
        auth_policy: config.auth_policy,
        ttl: config.ttl_ms / 1_000,
        timeout: config.timeout_ms.map(|timeout_ms| timeout_ms / 1_000),
        queue_max_depth: config.queue_max_depth,
        format: config.format,
        agents,
        auth_methods,
        disable_exec: config.disable_exec,
    }
}

pub async fn init_global_config_file() -> Result<InitGlobalConfigFileResult, ConfigError> {
    init_global_config_file_at(default_global_config_path()?).await
}

pub async fn init_global_config_file_at(
    config_path: impl AsRef<Path>,
) -> Result<InitGlobalConfigFileResult, ConfigError> {
    let config_path = config_path.as_ref().to_path_buf();
    let config_path_string = path_to_string(&config_path);

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).await.map_err(|error| ConfigError::CreateDir {
            path: path_to_string(parent),
            source: error,
        })?;
    }

    if fs::try_exists(&config_path)
        .await
        .map_err(|error| ConfigError::Read { path: config_path_string.clone(), source: error })?
    {
        return Ok(InitGlobalConfigFileResult { path: config_path_string, created: false });
    }

    let payload = json!({
        "defaultAgent": DEFAULT_AGENT_NAME,
        "defaultPermissions": "approve-all",
        "nonInteractivePermissions": "deny",
        "authPolicy": "skip",
        "ttl": 300,
        "timeout": Value::Null,
        "queueMaxDepth": DEFAULT_QUEUE_MAX_DEPTH,
        "format": "text",
        "agents": {},
        "auth": {}
    });
    let content = format!(
        "{}\n",
        serde_json::to_string_pretty(&payload).expect("default config payload should serialize")
    );
    fs::write(&config_path, content)
        .await
        .map_err(|error| ConfigError::Write { path: config_path_string.clone(), source: error })?;

    Ok(InitGlobalConfigFileResult { path: config_path_string, created: true })
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
