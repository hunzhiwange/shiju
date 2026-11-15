use iced::widget::text_editor;
use config_agent::{BuiltinAgentKind, DelegateAgentConfig, DelegateCapabilityPolicy, builtin_agent_config, builtin_agent_spec, builtin_agent_keys, is_retired_builtin_agent_key, merged_agent_configs};
pub const MAIN_AGENT_KEY: &str = "main";
pub const AGENT_PROMPT_SYSTEM_TAB: &str = "system_prompt";
pub const WORKSPACE_IDENTITY_FILES: [(&str, &str); 7] = [
    ("AGENTS.md", "智能体 AGENTS"),
    ("SOUL.md", "核心逻辑 SOUL"),
    ("TOOLS.md", "工具集 TOOLS"),
    ("IDENTITY.md", "身份机制 IDENTITY"),
    ("USER.md", "用户系统 USER"),
    ("HEARTBEAT.md", "心跳机制 HEARTBEAT"),
    ("MEMORY.md", "记忆系统 MEMORY"),
];

#[derive(Debug, Clone)]
pub struct WorkspaceIdentityFileState {
    pub file_name: String,
    pub label: String,
    pub editor: text_editor::Content,
    pub size_bytes: Option<u64>,
    pub modified_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSettingsEntryKind {
    Main,
    BuiltinWorker,
    Custom,
}

use config_tools::{is_builtin_required_tool_id, is_mcp_dynamic_tool_id};
#[derive(Debug, Clone)]
/// 表示 DelegateAgentSettingsEntry 相关的应用状态或派生数据。
pub struct DelegateAgentSettingsEntry {
    pub key: String,
    pub builtin: bool,
    pub label: String,
    pub runtime: config_agent_profile::AgentRuntime,
    pub defaults: config_agent_profile::AgentDefaults,
    pub recommended_prompts: Vec<String>,
    pub description: String,
    pub description_editor: text_editor::Content,
    pub kind: AgentSettingsEntryKind,
    /// 内置角色的委托资格、递归深度和并发上限由角色目录固定。
    pub role_controls_locked: bool,
    /// 仅自定义角色和可委托的内置角色允许从设置页关闭。
    pub can_disable: bool,
    pub enabled: bool,
    pub allow_as_subagent: bool,
    pub max_parallel_subagents: u32,
    pub provider: String,
    pub model: String,
    pub force_model_selection: bool,
    pub system_prompt_editor: text_editor::Content,
    pub api_key_input: String,
    pub temperature: f32,
    pub compact_context: bool,
    pub max_history_messages: u32,
    pub parallel_tools: bool,
    pub max_depth: u32,
    pub agentic: bool,
    pub allowed_tools: Vec<String>,
    pub skill_policy: DelegateCapabilityPolicy,
    pub allowed_skills: Vec<String>,
    pub enabled_identity_files: Vec<String>,
    pub max_iterations: u32,
}

impl DelegateAgentSettingsEntry {
    pub fn can_start_session(&self) -> bool {
        self.enabled && !matches!(self.kind, AgentSettingsEntryKind::BuiltinWorker)
    }

    /// 执行 from_config 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn from_config(key: &str, config: Option<DelegateAgentConfig>) -> Self {
        let kind = if key == MAIN_AGENT_KEY {
            AgentSettingsEntryKind::Main
        } else if matches!(
            builtin_agent_spec(key).map(|spec| spec.kind),
            Some(BuiltinAgentKind::Worker)
        ) {
            AgentSettingsEntryKind::BuiltinWorker
        } else {
            AgentSettingsEntryKind::Custom
        };
        let config = config.unwrap_or_else(|| default_agent_config_for_key(key));

        Self {
            key: key.to_string(),
            builtin: config.builtin || builtin_agent_spec(key).is_some(),
            label: default_agent_label(key, &config),
            runtime: config.runtime.clone(),
            defaults: config.defaults.clone(),
            recommended_prompts: config.recommended_prompts.clone(),
            description: default_agent_description(key, &config),
            description_editor: text_editor::Content::with_text(&default_agent_description(
                key, &config,
            )),
            kind,
            role_controls_locked: builtin_agent_spec(key).is_some(),
            can_disable: builtin_agent_spec(key).map(|spec| spec.allow_as_subagent).unwrap_or(true),
            enabled: if key == MAIN_AGENT_KEY { true } else { config.enabled },
            allow_as_subagent: config.allow_as_subagent,
            max_parallel_subagents: config.max_parallel_subagents.clamp(1, 32) as u32,
            provider: config.provider,
            model: config.model,
            force_model_selection: config.force_model_selection,
            system_prompt_editor: text_editor::Content::with_text(
                config
                    .system_prompt
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_default(),
            ),
            api_key_input: config.api_key.unwrap_or_default(),
            temperature: config
                .temperature
                .or_else(|| builtin_agent_spec(key).and_then(|spec| spec.default_temperature))
                .unwrap_or(0.7)
                .clamp(0.0, 2.0) as f32,
            compact_context: config.compact_context,
            max_history_messages: config.max_history_messages.clamp(1, 1000) as u32,
            parallel_tools: config.parallel_tools,
            max_depth: config.max_depth.clamp(1, 32),
            agentic: config.agentic,
            allowed_tools: config.allowed_tools,
            skill_policy: config.skill_policy,
            allowed_skills: config.allowed_skills,
            enabled_identity_files: config.enabled_identity_files,
            max_iterations: u32::try_from(config.max_iterations).unwrap_or(u32::MAX).max(1),
        }
    }
}

fn default_agent_config_for_key(key: &str) -> DelegateAgentConfig {
    builtin_agent_config(key).unwrap_or_default()
}

pub fn main_agent_model_fields(
    config: &DelegateAgentConfig,
    default_provider: Option<&str>,
    default_model: Option<&str>,
) -> (String, String) {
    if !config.provider.trim().is_empty() && !config.model.trim().is_empty() {
        return (config.provider.trim().to_string(), config.model.trim().to_string());
    }

    if let Some((provider, model)) = default_model
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.split_once('/'))
        .filter(|(provider, model)| !provider.trim().is_empty() && !model.trim().is_empty())
    {
        return (provider.trim().to_string(), model.trim().to_string());
    }

    let provider = if config.provider.trim().is_empty() {
        default_provider.map(str::trim).unwrap_or_default().to_string()
    } else {
        config.provider.trim().to_string()
    };
    let model = if config.model.trim().is_empty() {
        default_model.map(str::trim).unwrap_or_default().to_string()
    } else {
        config.model.trim().to_string()
    };
    (provider, model)
}

fn default_agent_label(key: &str, config: &DelegateAgentConfig) -> String {
    config
        .label
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| builtin_agent_spec(key).map(|spec| spec.label.to_string()))
        .unwrap_or_else(|| key.to_string())
}

fn default_agent_description(key: &str, config: &DelegateAgentConfig) -> String {
    config
        .description
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| builtin_agent_spec(key).map(|spec| spec.description.to_string()))
        .unwrap_or_default()
}

/// 执行 ordered_agent_keys 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn ordered_agent_keys(
    configured_agents: &std::collections::HashMap<String, DelegateAgentConfig>,
) -> Vec<String> {
    let mut custom_agent_keys = configured_agents
        .keys()
        .filter(|key| key.as_str() != "general" && !is_retired_builtin_agent_key(key))
        .filter(|key| builtin_agent_spec(key.as_str()).is_none())
        .cloned()
        .collect::<Vec<_>>();
    custom_agent_keys.sort();
    builtin_agent_keys().into_iter().map(str::to_string).chain(custom_agent_keys).collect()
}

/// 规范化委托代理设置页工具清单，并补入按项目动态注册的内置工具。
///
/// gateway 的 `/v1/tools` 清单绑定其启动工作区，而 RustCodeGraph 按当前会话项目
/// 动态注册，因此不能仅依赖 gateway 清单决定它是否在设置页可见。
pub fn filter_mcp_from_available_tools(tools: Vec<String>) -> Vec<String> {
    normalize_available_tool_catalog(tools)
        .into_iter()
        .filter(|tool_id| !is_mcp_dynamic_tool_id(tool_id))
        .collect()
}

/// 规范化网关实际注册的完整工具目录，保留仍有效的动态 MCP 工具。
pub fn normalize_available_tool_catalog(tools: Vec<String>) -> Vec<String> {
    let mut tools =
        tools.into_iter().filter(|tool_id| !tool_id.trim().is_empty()).collect::<Vec<_>>();
    tools.sort();
    tools.dedup();
    tools
}

/// 将用户可选选择与当前可见内置必选合并：`allowed ∪ (available ∩ 内置必选)`。
pub fn merge_builtin_required_tools(
    allowed: &[String],
    available: &[String],
) -> Vec<String> {
    let mut merged = allowed.to_vec();
    for tool in available.iter().filter(|tool_id| is_builtin_required_tool_id(tool_id)) {
        if !merged.iter().any(|existing| existing == tool) {
            merged.push(tool.clone());
        }
    }
    merged.sort();
    merged.dedup();
    merged
}

pub fn apply_default_allowed_tools(
    entries: &mut [DelegateAgentSettingsEntry],
    available_tools: &[String],
) {
    if available_tools.is_empty() {
        return;
    }

    for entry in entries {
        if !entry.role_controls_locked && entry.allowed_tools.is_empty() {
            let defaults = available_tools
                .iter()
                .filter(|tool| {
                    config_tools::DEFAULT_ENABLED_TOOL_IDS.contains(&tool.as_str())
                })
                .cloned()
                .collect::<Vec<_>>();
            entry.allowed_tools = merge_builtin_required_tools(&defaults, available_tools);
        }
    }
}

/// 删除当前运行时工具目录中已不存在的历史选择。
pub fn retain_available_allowed_tools(
    entries: &mut [DelegateAgentSettingsEntry],
    available_tools: &[String],
) -> bool {
    let available =
        available_tools.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    let mut changed = false;

    for entry in entries {
        let original = entry.allowed_tools.clone();
        entry.allowed_tools.retain(|tool_id| available.contains(tool_id.as_str()));
        entry.allowed_tools.sort();
        entry.allowed_tools.dedup();
        changed |= entry.allowed_tools != original;
    }

    changed
}

pub fn agent_settings_entries_from_config(
    full_agent_cfg: &config_aggregate::Config,
) -> Vec<DelegateAgentSettingsEntry> {
    let delegate_agents_cfg = merged_agent_configs(&full_agent_cfg.agents);
    let default_provider_cfg = full_agent_cfg.default_provider.clone();
    let default_model_cfg = full_agent_cfg.default_model.clone();
    let default_temperature_cfg = full_agent_cfg.default_temperature;

    ordered_agent_keys(&delegate_agents_cfg)
        .into_iter()
        .map(|key| {
            let config = delegate_agents_cfg.get(&key).cloned();
            if key != MAIN_AGENT_KEY {
                return DelegateAgentSettingsEntry::from_config(&key, config);
            }

            let config =
                config.unwrap_or_else(|| builtin_agent_config(MAIN_AGENT_KEY).unwrap_or_default());
            let (provider, model) = main_agent_model_fields(
                &config,
                default_provider_cfg.as_deref(),
                default_model_cfg.as_deref(),
            );
            let entry = DelegateAgentConfig {
                runtime: config.runtime,
                recommended_prompts: config.recommended_prompts,
                defaults: config.defaults,
                label: config.label,
                description: config.description,
                builtin: config.builtin,
                mode: "primary".to_string(),
                enabled: true,
                allow_as_subagent: config.allow_as_subagent,
                max_parallel_subagents: config.max_parallel_subagents,
                provider,
                model,
                force_model_selection: config.force_model_selection,
                system_prompt: config.system_prompt,
                api_key: config.api_key,
                temperature: Some(config.temperature.unwrap_or(default_temperature_cfg)),
                top_p: config.top_p,
                identity_format: Some("openclaw".to_string()),
                hidden: config.hidden,
                max_depth: config.max_depth,
                agentic: config.agentic,
                allowed_tools: config.allowed_tools,
                skill_policy: config.skill_policy,
                allowed_skills: config.allowed_skills,
                enabled_identity_files: config.enabled_identity_files,
                options: config.options,
                permission: config.permission,
                max_iterations: config.max_iterations,
                compact_context: config.compact_context,
                max_history_messages: config.max_history_messages,
                parallel_tools: config.parallel_tools,
                steps: config.steps,
            };
            DelegateAgentSettingsEntry::from_config(MAIN_AGENT_KEY, Some(entry))
        })
        .collect()
}

#[cfg(test)] mod tests;
