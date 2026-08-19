//! Config 工具实现。
//!
//! 本模块提供统一的配置读取和受控写入入口。简单配置通过 `setting/value`
//! 直接访问，复杂历史分区仍桥接到专用工具。写操作会经过自治策略和配置校验，
//! 避免工具在只读或无效配置状态下静默修改本地设置。

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::config::{self, Config};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::sync::Arc;
use vw_api_types::tools::ToolResultContentDto;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    #[serde(default)]
    setting: Option<String>,
    #[serde(default)]
    value: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
enum SettingKind {
    Boolean,
    Integer,
    Number,
    String,
}

#[derive(Debug, Clone, Copy)]
struct SettingSpec {
    key: &'static str,
    aliases: &'static [&'static str],
    path: &'static [&'static str],
    kind: SettingKind,
    description: &'static str,
    options: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    setting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ConfigResponse {
    fn to_value(&self) -> anyhow::Result<Value> {
        serde_json::to_value(self).map_err(Into::into)
    }

    fn to_legacy_result(&self) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: self.success,
            output: serde_json::to_string_pretty(self)?,
            error: self.error.clone(),
        })
    }

    fn summary(&self) -> String {
        if !self.success {
            return self.error.clone().unwrap_or_else(|| "配置操作失败".to_string());
        }

        match self.operation {
            Some("get") => match (&self.setting, &self.value) {
                (Some(setting), Some(value)) => {
                    format!("{} = {}", setting, value_label(value))
                }
                _ => "当前配置概览".to_string(),
            },
            Some("set") => match (&self.setting, &self.new_value) {
                (Some(setting), Some(value)) => {
                    format!("{} -> {}", setting, value_label(value))
                }
                _ => "配置已更新".to_string(),
            },
            _ => "配置".to_string(),
        }
    }

    fn model_text(&self) -> String {
        if !self.success {
            return self.error.clone().unwrap_or_else(|| "Config failed".to_string());
        }

        match self.operation {
            Some("get") => match (&self.setting, &self.value) {
                (Some(setting), Some(value)) => {
                    format!("{} = {}", setting, value_label(value))
                }
                _ => "Retrieved current configuration overview".to_string(),
            },
            Some("set") => match (&self.setting, &self.new_value) {
                (Some(setting), Some(value)) => {
                    format!("Set {} to {}", setting, value_label(value))
                }
                _ => "Configuration updated".to_string(),
            },
            _ => "Configuration".to_string(),
        }
    }
}

const AUTONOMY_LEVEL_OPTIONS: &[&str] = &["read_only", "supervised", "full"];
const BROWSER_BACKEND_OPTIONS: &[&str] = &["agent_browser", "rust_native", "computer_use", "auto"];
const BROWSER_OPEN_OPTIONS: &[&str] =
    &["default", "new_window", "new_tab", "disable", "brave", "chrome", "firefox"];
const WEB_FETCH_PROVIDER_OPTIONS: &[&str] =
    &["fast_html2md", "nanohtml2text", "firecrawl", "tavily"];
const WEB_SEARCH_PROVIDER_OPTIONS: &[&str] =
    &["duckduckgo", "brave", "serper", "google", "bing", "firecrawl", "tavily"];
const TERMINAL_SHELL_OPTIONS: &[&str] = &["bash", "zsh"];
const TERMINAL_THEME_OPTIONS: &[&str] = &["system", "ui", "solarized_dark", "monokai"];

const SUPPORTED_SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        key: "defaultProvider",
        aliases: &["default_provider", "model_provider"],
        path: &["default_provider"],
        kind: SettingKind::String,
        description: "Default provider name. Pass \"default\" to clear an explicit override.",
        options: &[],
    },
    SettingSpec {
        key: "defaultModel",
        aliases: &["default_model", "model"],
        path: &["default_model"],
        kind: SettingKind::String,
        description: "Default model name. Pass \"default\" to clear an explicit override.",
        options: &[],
    },
    SettingSpec {
        key: "defaultTemperature",
        aliases: &["default_temperature"],
        path: &["default_temperature"],
        kind: SettingKind::Number,
        description: "Default temperature. Pass \"default\" to restore the default value.",
        options: &[],
    },
    SettingSpec {
        key: "autonomy.level",
        aliases: &["autonomy.level"],
        path: &["autonomy", "level"],
        kind: SettingKind::String,
        description: "Autonomy level.",
        options: AUTONOMY_LEVEL_OPTIONS,
    },
    SettingSpec {
        key: "autonomy.workspaceOnly",
        aliases: &["autonomy.workspace_only"],
        path: &["autonomy", "workspace_only"],
        kind: SettingKind::Boolean,
        description: "Whether operations are restricted to the workspace.",
        options: &[],
    },
    SettingSpec {
        key: "autonomy.requireApprovalForMediumRisk",
        aliases: &["autonomy.require_approval_for_medium_risk"],
        path: &["autonomy", "require_approval_for_medium_risk"],
        kind: SettingKind::Boolean,
        description: "Whether medium-risk operations require approval.",
        options: &[],
    },
    SettingSpec {
        key: "autonomy.blockHighRiskCommands",
        aliases: &["autonomy.block_high_risk_commands"],
        path: &["autonomy", "block_high_risk_commands"],
        kind: SettingKind::Boolean,
        description: "Whether high-risk commands are blocked.",
        options: &[],
    },
    SettingSpec {
        key: "agents.main.compactContext",
        aliases: &["agents.main.compact_context"],
        path: &["agents", "main", "compact_context"],
        kind: SettingKind::Boolean,
        description: "Whether compact context is enabled.",
        options: &[],
    },
    SettingSpec {
        key: "agents.main.parallelTools",
        aliases: &["agents.main.parallel_tools"],
        path: &["agents", "main", "parallel_tools"],
        kind: SettingKind::Boolean,
        description: "Whether parallel tool execution is allowed.",
        options: &[],
    },
    SettingSpec {
        key: "agents.main.maxIterations",
        aliases: &["agents.main.max_iterations"],
        path: &["agents", "main", "max_iterations"],
        kind: SettingKind::Integer,
        description: "Maximum number of tool iterations allowed per message.",
        options: &[],
    },
    SettingSpec {
        key: "browser.enabled",
        aliases: &["browser.enabled"],
        path: &["browser", "enabled"],
        kind: SettingKind::Boolean,
        description: "Whether browser_open is enabled.",
        options: &[],
    },
    SettingSpec {
        key: "browser.backend",
        aliases: &["browser.backend"],
        path: &["browser", "backend"],
        kind: SettingKind::String,
        description: "Browser automation backend.",
        options: BROWSER_BACKEND_OPTIONS,
    },
    SettingSpec {
        key: "browser.browserOpen",
        aliases: &["browser.browser_open"],
        path: &["browser", "browser_open"],
        kind: SettingKind::String,
        description: "Opening mode for browser_open.",
        options: BROWSER_OPEN_OPTIONS,
    },
    SettingSpec {
        key: "httpRequest.enabled",
        aliases: &["http_request.enabled"],
        path: &["http_request", "enabled"],
        kind: SettingKind::Boolean,
        description: "Whether the http_request tool is enabled.",
        options: &[],
    },
    SettingSpec {
        key: "httpRequest.timeoutSecs",
        aliases: &["http_request.timeout_secs"],
        path: &["http_request", "timeout_secs"],
        kind: SettingKind::Integer,
        description: "Timeout for http_request in seconds.",
        options: &[],
    },
    SettingSpec {
        key: "webFetch.enabled",
        aliases: &["web_fetch.enabled"],
        path: &["web_fetch", "enabled"],
        kind: SettingKind::Boolean,
        description: "Whether the web_fetch tool is enabled.",
        options: &[],
    },
    SettingSpec {
        key: "webFetch.provider",
        aliases: &["web_fetch.provider"],
        path: &["web_fetch", "provider"],
        kind: SettingKind::String,
        description: "Provider for web_fetch.",
        options: WEB_FETCH_PROVIDER_OPTIONS,
    },
    SettingSpec {
        key: "webSearch.enabled",
        aliases: &["web_search.enabled"],
        path: &["web_search", "enabled"],
        kind: SettingKind::Boolean,
        description: "Whether web_search is enabled.",
        options: &[],
    },
    SettingSpec {
        key: "webSearch.provider",
        aliases: &["web_search.provider"],
        path: &["web_search", "provider"],
        kind: SettingKind::String,
        description: "Provider for web_search.",
        options: WEB_SEARCH_PROVIDER_OPTIONS,
    },
    SettingSpec {
        key: "webSearch.maxResults",
        aliases: &["web_search.max_results"],
        path: &["web_search", "max_results"],
        kind: SettingKind::Integer,
        description: "Maximum number of results returned by web_search.",
        options: &[],
    },
    SettingSpec {
        key: "appUi.theme",
        aliases: &["app_ui.theme", "app_ui.system_settings.app_theme"],
        path: &["app_ui", "system_settings", "app_theme"],
        kind: SettingKind::String,
        description: "Desktop application theme.",
        options: &[],
    },
    SettingSpec {
        key: "appUi.terminalTheme",
        aliases: &["app_ui.terminal_theme", "app_ui.system_settings.terminal_theme"],
        path: &["app_ui", "system_settings", "terminal_theme"],
        kind: SettingKind::String,
        description: "Terminal theme.",
        options: TERMINAL_THEME_OPTIONS,
    },
    SettingSpec {
        key: "appUi.terminalShell",
        aliases: &["app_ui.terminal_shell", "app_ui.system_settings.terminal_shell"],
        path: &["app_ui", "system_settings", "terminal_shell"],
        kind: SettingKind::String,
        description: "Terminal shell.",
        options: TERMINAL_SHELL_OPTIONS,
    },
    SettingSpec {
        key: "appUi.editorFollowSystemTheme",
        aliases: &[
            "app_ui.editor_follow_system_theme",
            "app_ui.system_settings.editor_follow_system_theme",
        ],
        path: &["app_ui", "system_settings", "editor_follow_system_theme"],
        kind: SettingKind::Boolean,
        description: "Whether the editor theme follows the system theme.",
        options: &[],
    },
];

fn supported_settings_description() -> String {
    SUPPORTED_SETTINGS
        .iter()
        .map(|spec| {
            let value_hint = if spec.options.is_empty() {
                match spec.kind {
                    SettingKind::Boolean => "true/false".to_string(),
                    SettingKind::Integer => "integer".to_string(),
                    SettingKind::Number => "number".to_string(),
                    SettingKind::String => "string".to_string(),
                }
            } else {
                spec.options
                    .iter()
                    .map(|option| format!("\"{}\"", option))
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            format!("- {}: {} - {}", spec.key, value_hint, spec.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn value_label(value: &Value) -> String {
    match value {
        Value::Null => "default".to_string(),
        Value::String(text) => text.clone(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn setting_spec(raw: &str) -> Option<&'static SettingSpec> {
    let trimmed = raw.trim();
    SUPPORTED_SETTINGS.iter().find(|spec| {
        spec.key.eq_ignore_ascii_case(trimmed)
            || spec.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(trimmed))
    })
}

fn read_nested_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn write_nested_value(root: &mut Value, path: &[&str], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    if !root.is_object() {
        *root = Value::Object(Map::new());
    }

    let Some(object) = root.as_object_mut() else {
        return;
    };

    if path.len() == 1 {
        if value.is_null() {
            object.remove(path[0]);
        } else {
            object.insert(path[0].to_string(), value);
        }
        return;
    }

    let child = object.entry(path[0].to_string()).or_insert_with(|| Value::Object(Map::new()));
    write_nested_value(child, &path[1..], value);
}

fn read_setting_value(config: &Config, spec: &SettingSpec) -> anyhow::Result<Value> {
    let value = serde_json::to_value(config)?;
    let raw = read_nested_value(&value, spec.path).cloned().unwrap_or(Value::Null);
    Ok(format_setting_value(spec, raw))
}

fn materialize_setting_defaults(config: &mut Config, spec: &SettingSpec) {
    if spec.path.starts_with(&["agents", "main"]) {
        vw_config_types::agent::ensure_main_agent_config(&mut config.agents);
    }
}

fn format_setting_value(spec: &SettingSpec, value: Value) -> Value {
    match spec.key {
        "defaultProvider" | "defaultModel" => {
            if value.is_null() {
                Value::String("default".to_string())
            } else {
                value
            }
        }
        _ => value,
    }
}

fn coerce_bool(value: Value, setting: &str) -> anyhow::Result<Value> {
    match value {
        Value::Bool(boolean) => Ok(Value::Bool(boolean)),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => anyhow::bail!("{} requires true or false", setting),
        },
        _ => anyhow::bail!("{} requires true or false", setting),
    }
}

fn coerce_integer(value: Value, setting: &str) -> anyhow::Result<Value> {
    match value {
        Value::Number(number) => {
            if let Some(parsed) = number.as_u64() {
                Ok(json!(parsed))
            } else {
                anyhow::bail!("{} requires an integer", setting)
            }
        }
        Value::String(text) => {
            let parsed = text
                .trim()
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("{} requires an integer", setting))?;
            Ok(json!(parsed))
        }
        _ => anyhow::bail!("{} requires an integer", setting),
    }
}

fn coerce_number(value: Value, setting: &str) -> anyhow::Result<Value> {
    match value {
        Value::Number(number) => {
            let parsed =
                number.as_f64().ok_or_else(|| anyhow::anyhow!("{} requires a number", setting))?;
            Ok(json!(parsed))
        }
        Value::String(text) => {
            let parsed = text
                .trim()
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("{} requires a number", setting))?;
            Ok(json!(parsed))
        }
        _ => anyhow::bail!("{} requires a number", setting),
    }
}

fn coerce_string(value: Value, setting: &str) -> anyhow::Result<Value> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                anyhow::bail!("{} requires a non-empty string", setting);
            }
            Ok(Value::String(trimmed.to_string()))
        }
        _ => anyhow::bail!("{} requires a string", setting),
    }
}

fn normalize_setting_value(spec: &SettingSpec, value: Value) -> anyhow::Result<Value> {
    let normalized = match spec.key {
        "defaultProvider" | "defaultModel" => match value {
            Value::String(text) if text.trim().eq_ignore_ascii_case("default") => Value::Null,
            other => coerce_string(other, spec.key)?,
        },
        "defaultTemperature" => match value {
            Value::String(text) if text.trim().eq_ignore_ascii_case("default") => {
                json!(Config::default().default_temperature)
            }
            other => coerce_number(other, spec.key)?,
        },
        _ => match spec.kind {
            SettingKind::Boolean => coerce_bool(value, spec.key)?,
            SettingKind::Integer => coerce_integer(value, spec.key)?,
            SettingKind::Number => coerce_number(value, spec.key)?,
            SettingKind::String => coerce_string(value, spec.key)?,
        },
    };

    if !spec.options.is_empty() {
        let Some(candidate) = normalized.as_str() else {
            anyhow::bail!("{} requires one of: {}", spec.key, spec.options.join(", "));
        };
        if !spec.options.iter().any(|option| option.eq_ignore_ascii_case(candidate)) {
            anyhow::bail!(
                "Invalid value \"{}\" for {}. Options: {}",
                candidate,
                spec.key,
                spec.options.join(", ")
            );
        }
    }

    Ok(normalized)
}

fn snapshot_value(config: &Config) -> Value {
    json!({
        "workspace_dir": config.workspace_dir,
        "config_path": config.config_path,
        "default_provider": config.default_provider,
        "default_model": config.default_model,
        "default_temperature": config.default_temperature,
        "browser": config.browser,
        "http_request": config.http_request,
        "web_fetch": config.web_fetch,
        "web_search": config.web_search,
        "autonomy": config.autonomy,
        "app_ui": config.app_ui,
    })
}

#[derive(Clone)]
/// VibeWindow 配置读写工具。
pub struct ConfigTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl ConfigTool {
    /// 创建新的配置工具。
    ///
    /// # 参数
    ///
    /// - `config`: 当前配置快照，提供配置文件路径和工作区路径。
    /// - `security`: 安全策略，用于限制写操作。
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    fn blocked_write_response(&self, setting: &str, message: &str) -> ConfigResponse {
        ConfigResponse {
            success: false,
            operation: Some("set"),
            setting: Some(setting.to_string()),
            value: None,
            previous_value: None,
            new_value: None,
            error: Some(message.to_string()),
        }
    }

    async fn load_current_config(&self) -> anyhow::Result<Config> {
        config::get_project_without_env(&self.config.workspace_dir)
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    async fn handle_setting_request(
        &self,
        raw_setting: &str,
        value: Option<Value>,
    ) -> anyhow::Result<ConfigResponse> {
        let operation = if value.is_some() { "set" } else { "get" };
        let Some(spec) = setting_spec(raw_setting) else {
            return Ok(ConfigResponse {
                success: false,
                operation: Some(operation),
                setting: Some(raw_setting.trim().to_string()),
                value: None,
                previous_value: None,
                new_value: None,
                error: Some(format!("Unknown setting: \"{}\"", raw_setting.trim())),
            });
        };

        let canonical_setting = spec.key.to_string();
        let mut current = if value.is_some() {
            config::get_project(&self.config.workspace_dir)
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
        } else {
            self.load_current_config().await?
        };
        materialize_setting_defaults(&mut current, spec);

        if let Some(value) = value {
            if !self.security.can_act() {
                return Ok(self.blocked_write_response(
                    canonical_setting.as_str(),
                    "Action blocked: autonomy is read-only",
                ));
            }
            if !self.security.record_action() {
                return Ok(self.blocked_write_response(
                    canonical_setting.as_str(),
                    "Action blocked: rate limit exceeded",
                ));
            }

            let previous_value = read_setting_value(&current, spec)?;
            let normalized_value = match normalize_setting_value(spec, value) {
                Ok(value) => value,
                Err(error) => {
                    return Ok(ConfigResponse {
                        success: false,
                        operation: Some("set"),
                        setting: Some(canonical_setting),
                        value: None,
                        previous_value: Some(previous_value),
                        new_value: None,
                        error: Some(error.to_string()),
                    });
                }
            };

            let mut updated_value = serde_json::to_value(&current)?;
            // 先在 JSON 层写入嵌套字段，再反序列化回强类型 Config，让 serde 和
            // validate_config 共同守住结构边界。
            write_nested_value(&mut updated_value, spec.path, normalized_value);

            let mut next = match serde_json::from_value::<Config>(updated_value) {
                Ok(config) => config,
                Err(error) => {
                    return Ok(ConfigResponse {
                        success: false,
                        operation: Some("set"),
                        setting: Some(canonical_setting),
                        value: None,
                        previous_value: Some(previous_value),
                        new_value: None,
                        error: Some(error.to_string()),
                    });
                }
            };
            next.workspace_dir = current.workspace_dir.clone();
            next.config_path = current.config_path.clone();

            if let Err(error) = config::validate_config(&next) {
                return Ok(ConfigResponse {
                    success: false,
                    operation: Some("set"),
                    setting: Some(canonical_setting),
                    value: None,
                    previous_value: Some(previous_value),
                    new_value: None,
                    error: Some(error.to_string()),
                });
            }

            let updated = match config::update_project_from_configs(
                &self.config.workspace_dir,
                &current,
                &next,
            )
            .await
            {
                Ok(updated) => updated,
                Err(error) => {
                    return Ok(ConfigResponse {
                        success: false,
                        operation: Some("set"),
                        setting: Some(canonical_setting),
                        value: None,
                        previous_value: Some(previous_value),
                        new_value: None,
                        error: Some(error.to_string()),
                    });
                }
            };
            let new_value = read_setting_value(&updated, spec)?;
            return Ok(ConfigResponse {
                success: true,
                operation: Some("set"),
                setting: Some(canonical_setting),
                value: None,
                previous_value: Some(previous_value),
                new_value: Some(new_value),
                error: None,
            });
        }

        Ok(ConfigResponse {
            success: true,
            operation: Some("get"),
            setting: Some(canonical_setting),
            value: Some(read_setting_value(&current, spec)?),
            previous_value: None,
            new_value: None,
            error: None,
        })
    }

    async fn execute_request(&self, args: Args) -> anyhow::Result<ConfigResponse> {
        if let Some(setting) =
            args.setting.as_deref().map(str::trim).filter(|value| !value.is_empty())
        {
            return self.handle_setting_request(setting, args.value).await;
        }

        let current = self.load_current_config().await?;
        Ok(ConfigResponse {
            success: true,
            operation: Some("get"),
            setting: None,
            value: Some(snapshot_value(&current)),
            previous_value: None,
            new_value: None,
            error: None,
        })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "Config"
    }

    fn description(&self) -> &str {
        "Get or set VibeWindow configuration. Use setting/value to read or update a supported setting."
    }

    fn parameters_schema(&self) -> Value {
        let setting_enum: Vec<String> =
            SUPPORTED_SETTINGS.iter().map(|spec| spec.key.to_string()).collect();
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "setting": {
                    "type": "string",
                    "enum": setting_enum,
                    "description": format!(
                        "Configuration key. Omit value to read it. Supported settings:\n{}",
                        supported_settings_description()
                    )
                },
                "value": {
                    "type": ["string", "boolean", "number"],
                    "description": "New configuration value. Omit it to read the setting; pass the string \"default\" to reset supported optional settings."
                },
            }
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("Config")
            .with_aliases(vec!["config".to_string()])
            .with_read_only(false)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn call(&self, args: Value) -> anyhow::Result<ToolCallResult> {
        let args: Args = serde_json::from_value(args)?;
        let response = self.execute_request(args).await?;
        let data = response.to_value()?;
        let success = response.success;
        let summary = response.summary();
        let metadata = json!({
            "operation": response.operation,
            "setting": response.setting,
            "success": success,
        });

        Ok(ToolCallResult {
            data: data.clone(),
            model_result: Value::String(response.model_text()),
            content_blocks: vec![ToolResultContentDto::Json { value: data.clone() }],
            render_hint: Some(ToolRenderHint {
                title: Some("Config".to_string()),
                kind: Some("config".to_string()),
                summary: (!summary.trim().is_empty()).then_some(summary),
                metadata,
            }),
            telemetry: Some(ToolCallTelemetry { success, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        self.execute_request(args).await?.to_legacy_result()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
