//! 维护系统设置状态及其按领域拆分的派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;
use vw_config_types::agent::{
    BuiltinAgentKind, DelegateAgentConfig, DelegateCapabilityPolicy, builtin_agent_config,
    builtin_agent_keys, builtin_agent_spec, is_retired_builtin_agent_key, merged_agent_configs,
};
use vw_config_types::tools::{is_builtin_required_tool_id, is_mcp_dynamic_tool_id};

#[derive(Debug, Clone)]
/// 表示 DelegateAgentSettingsEntry 相关的应用状态或派生数据。
pub(crate) struct DelegateAgentSettingsEntry {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) kind: AgentSettingsEntryKind,
    /// 内置角色的委托资格、递归深度和并发上限由角色目录固定。
    pub(crate) role_controls_locked: bool,
    /// 仅自定义角色和可委托的内置角色允许从设置页关闭。
    pub(crate) can_disable: bool,
    pub(crate) enabled: bool,
    pub(crate) allow_as_subagent: bool,
    pub(crate) max_parallel_subagents: u32,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) force_model_selection: bool,
    pub(crate) system_prompt_editor: text_editor::Content,
    pub(crate) api_key_input: String,
    pub(crate) temperature: f32,
    pub(crate) compact_context: bool,
    pub(crate) max_tool_iterations: u32,
    pub(crate) max_history_messages: u32,
    pub(crate) parallel_tools: bool,
    pub(crate) tool_dispatcher: String,
    pub(crate) max_depth: u32,
    pub(crate) agentic: bool,
    pub(crate) tool_policy: DelegateCapabilityPolicy,
    pub(crate) allowed_tools: Vec<String>,
    pub(crate) skill_policy: DelegateCapabilityPolicy,
    pub(crate) allowed_skills: Vec<String>,
    pub(crate) enabled_identity_files: Vec<String>,
    pub(crate) max_iterations: u32,
}

impl DelegateAgentSettingsEntry {
    /// 执行 from_config 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn from_config(key: &str, config: Option<DelegateAgentConfig>) -> Self {
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
            label: default_agent_label(key, &config),
            description: default_agent_description(key, &config),
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
            compact_context: false,
            max_tool_iterations: 50_000,
            max_history_messages: 100,
            parallel_tools: false,
            tool_dispatcher: "auto".to_string(),
            max_depth: config.max_depth.clamp(1, 32),
            agentic: config.agentic,
            tool_policy: config.tool_policy,
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

pub(crate) fn main_agent_model_fields(
    config: &DelegateAgentConfig,
    default_provider: Option<&str>,
    default_model: Option<&str>,
) -> (String, String) {
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
pub(crate) fn ordered_agent_keys(
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
pub(crate) fn filter_mcp_from_available_tools(tools: Vec<String>) -> Vec<String> {
    let mut tools =
        tools.into_iter().filter(|tool_id| !is_mcp_dynamic_tool_id(tool_id)).collect::<Vec<_>>();
    if !tools.iter().any(|tool_id| tool_id == "rustcodegraph") {
        tools.push("rustcodegraph".to_string());
    }
    tools.sort();
    tools.dedup();
    tools
}

/// 将用户可选选择与当前可见内置必选合并：`allowed ∪ (available ∩ 内置必选)`。
pub(crate) fn merge_builtin_required_tools(
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

pub(crate) fn apply_default_allowed_tools(
    entries: &mut [DelegateAgentSettingsEntry],
    available_tools: &[String],
) {
    if available_tools.is_empty() {
        return;
    }

    for entry in entries {
        if !entry.role_controls_locked && entry.allowed_tools.is_empty() {
            entry.allowed_tools = merge_builtin_required_tools(available_tools, available_tools);
        }
    }
}

pub(crate) fn agent_settings_entries_from_config(
    full_agent_cfg: &vw_config_types::config::Config,
) -> Vec<DelegateAgentSettingsEntry> {
    let delegate_agents_cfg = merged_agent_configs(&full_agent_cfg.agents);
    let agent_cfg = &full_agent_cfg.agent;
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
                tool_policy: config.tool_policy,
                allowed_tools: config.allowed_tools,
                skill_policy: config.skill_policy,
                allowed_skills: config.allowed_skills,
                enabled_identity_files: config.enabled_identity_files,
                options: config.options,
                permission: config.permission,
                max_iterations: config.max_iterations,
                steps: config.steps,
            };
            let mut ui_entry = DelegateAgentSettingsEntry::from_config(MAIN_AGENT_KEY, Some(entry));
            ui_entry.compact_context = agent_cfg.compact_context;
            ui_entry.max_tool_iterations =
                u32::try_from(agent_cfg.max_tool_iterations).unwrap_or(u32::MAX).max(1);
            ui_entry.max_history_messages = agent_cfg.max_history_messages.clamp(1, 1000) as u32;
            ui_entry.parallel_tools = agent_cfg.parallel_tools;
            ui_entry.tool_dispatcher = {
                let value = agent_cfg.tool_dispatcher.trim().to_string();
                if value.is_empty() { "auto".to_string() } else { value }
            };
            ui_entry
        })
        .collect()
}

#[derive(Debug, Clone)]
/// 表示 AgentsSettingsState 相关的应用状态或派生数据。
pub(crate) struct AgentsSettingsState {
    pub(crate) loading: bool,
    /// 只有从 Gateway 成功加载当前作用域后，界面状态才可作为保存来源。
    pub(crate) config_ready: bool,
    pub(crate) project_path: Option<String>,
    pub(crate) providers: Vec<ProviderSummary>,
    pub(crate) provider_models: Vec<ProviderModelsSummary>,
    pub(crate) entries: Vec<DelegateAgentSettingsEntry>,
    pub(crate) new_agent_key_input: String,
    pub(crate) selected_agent: String,
    pub(crate) active_detail_tab: String,
    pub(crate) active_tools_list_tab: String,
    pub(crate) active_prompt_tab: String,
    pub(crate) workspace_identity_files: Vec<WorkspaceIdentityFileState>,
    pub(crate) workspace_identity_root_path: Option<String>,
    pub(crate) available_tools: Vec<String>,
    pub(crate) save_error: Option<String>,
    pub(crate) save_in_flight: bool,
    pub(crate) save_pending: bool,
}

impl Default for AgentsSettingsState {
    fn default() -> Self {
        Self {
            loading: true,
            config_ready: false,
            project_path: None,
            providers: Vec::new(),
            provider_models: Vec::new(),
            entries: builtin_agent_keys()
                .iter()
                .map(|key| {
                    DelegateAgentSettingsEntry::from_config(
                        key,
                        Some(default_agent_config_for_key(key)),
                    )
                })
                .collect(),
            new_agent_key_input: String::new(),
            selected_agent: MAIN_AGENT_KEY.to_string(),
            active_detail_tab: AGENT_DETAIL_BASIC_TAB.to_string(),
            active_tools_list_tab: AGENT_TOOLS_OPTIONAL_TAB.to_string(),
            active_prompt_tab: AGENT_PROMPT_SYSTEM_TAB.to_string(),
            workspace_identity_files: WORKSPACE_IDENTITY_FILES
                .iter()
                .map(|(file_name, label)| WorkspaceIdentityFileState {
                    file_name: (*file_name).to_string(),
                    label: (*label).to_string(),
                    editor: text_editor::Content::with_text(""),
                    size_bytes: None,
                    modified_at_ms: None,
                })
                .collect(),
            workspace_identity_root_path: None,
            available_tools: Vec::new(),
            save_error: None,
            save_in_flight: false,
            save_pending: false,
        }
    }
}

#[cfg(test)]
#[path = "agents_tests.rs"]
mod agents_tests;
