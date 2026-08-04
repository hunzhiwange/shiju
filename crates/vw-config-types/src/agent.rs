use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 多工作区注册表配置（`[workspaces]`）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[derive(Default)]
pub struct WorkspacesConfig {
    /// 是否启用进程内工作区注册表行为。
    #[serde(default)]
    pub enabled: bool,
    /// 可选的工作区注册表根目录覆盖值。
    /// 未设置时默认使用 `<config_dir>/workspaces`。
    #[serde(default)]
    pub root: Option<String>,
}

impl WorkspacesConfig {
    /// 根据配置与运行时上下文解析工作区注册表根目录。
    pub fn resolve_root(&self, config_dir: &Path) -> PathBuf {
        match self.root.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => {
                #[cfg(not(target_arch = "wasm32"))]
                let expanded = shellexpand::tilde(value).into_owned();
                #[cfg(target_arch = "wasm32")]
                let expanded = value.to_string();
                let path = PathBuf::from(expanded);
                if path.is_absolute() { path } else { config_dir.join(path) }
            }
            None => config_dir.join("workspaces"),
        }
    }
}

/// 统一的 agent 定义配置。
///
/// 该结构同时服务于主 Agent、内建 agent 预设与自定义 agent。
/// `mode` 仍保留用于兼容历史配置，但新流程不再依赖这个字段。
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentDefinitionConfig {
    /// 可选的人类可读标签。
    #[serde(default)]
    pub label: Option<String>,
    /// 可选的用途说明。
    #[serde(default)]
    pub description: Option<String>,
    /// 是否属于内建预设项。
    #[serde(default)]
    pub builtin: bool,
    /// 兼容字段：历史注册模式，`main` 之外的新流程不再依赖它。
    #[serde(default = "default_delegate_agent_mode")]
    pub mode: String,
    /// 该代理是否启用注册与使用。
    #[serde(default = "default_delegate_agent_enabled")]
    pub enabled: bool,
    /// 是否允许该代理被其他 Agent 作为子智能体启动。
    #[serde(default = "default_allow_as_subagent")]
    pub allow_as_subagent: bool,
    /// 该代理每轮最多同时启动的子智能体数量。
    #[serde(default = "default_max_parallel_subagents")]
    pub max_parallel_subagents: usize,
    /// Provider 名称，例如 `ollama`、`openrouter`、`anthropic`。
    pub provider: String,
    /// 模型名称。
    pub model: String,
    /// 可选的 system prompt。
    #[serde(default, alias = "prompt")]
    pub system_prompt: Option<String>,
    /// 可选的 API Key 覆盖值。
    #[serde(default)]
    pub api_key: Option<String>,
    /// 温度参数覆盖值。
    #[serde(default)]
    pub temperature: Option<f64>,
    /// top_p 参数覆盖值。
    #[serde(default)]
    pub top_p: Option<f64>,
    /// 兼容字段：身份文档格式。
    #[serde(default)]
    pub identity_format: Option<String>,
    /// 是否在 agent 列表中隐藏。
    #[serde(default)]
    pub hidden: bool,
    /// 嵌套委派的最大递归深度。
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// 是否启用 agentic 子代理模式（多轮工具调用循环）。
    #[serde(default)]
    pub agentic: bool,
    /// agentic 模式下可用工具白名单。
    #[serde(default, alias = "tools")]
    pub allowed_tools: Vec<String>,
    /// 委托代理可见的技能白名单。
    #[serde(default, alias = "skills")]
    pub allowed_skills: Vec<String>,
    /// 允许加载到提示词中的工作区身份文件白名单。
    #[serde(default)]
    pub enabled_identity_files: Vec<String>,
    /// 额外模型选项。
    #[serde(default)]
    pub options: HashMap<String, Value>,
    /// 权限配置。
    #[serde(default)]
    pub permission: Value,
    /// agentic 模式下最大工具调用迭代次数。
    #[serde(default = "default_max_tool_iterations", alias = "max_turns")]
    pub max_iterations: usize,
    /// 最大执行步数限制。
    #[serde(default)]
    pub steps: Option<i64>,
}

/// 历史类型名别名。
pub type DelegateAgentConfig = AgentDefinitionConfig;

fn default_max_depth() -> u32 {
    3
}

fn default_delegate_agent_mode() -> String {
    "all".to_string()
}

fn default_delegate_agent_enabled() -> bool {
    true
}

fn default_allow_as_subagent() -> bool {
    true
}

fn default_max_parallel_subagents() -> usize {
    4
}

fn default_max_tool_iterations() -> usize {
    50_000
}

impl Default for AgentDefinitionConfig {
    fn default() -> Self {
        Self {
            label: None,
            description: None,
            builtin: false,
            mode: default_delegate_agent_mode(),
            enabled: default_delegate_agent_enabled(),
            allow_as_subagent: default_allow_as_subagent(),
            max_parallel_subagents: default_max_parallel_subagents(),
            provider: String::new(),
            model: String::new(),
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: default_max_depth(),
            agentic: false,
            allowed_tools: Vec::new(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: default_max_tool_iterations(),
            steps: None,
        }
    }
}

impl std::fmt::Debug for AgentDefinitionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentDefinitionConfig")
            .field("label", &self.label)
            .field("description", &self.description)
            .field("builtin", &self.builtin)
            .field("mode", &self.mode)
            .field("enabled", &self.enabled)
            .field("allow_as_subagent", &self.allow_as_subagent)
            .field("max_parallel_subagents", &self.max_parallel_subagents)
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("system_prompt", &self.system_prompt)
            .field("api_key_configured", &self.api_key.is_some())
            .field("temperature", &self.temperature)
            .field("top_p", &self.top_p)
            .field("identity_format", &self.identity_format)
            .field("hidden", &self.hidden)
            .field("max_depth", &self.max_depth)
            .field("agentic", &self.agentic)
            .field("allowed_tools", &self.allowed_tools)
            .field("allowed_skills", &self.allowed_skills)
            .field("enabled_identity_files", &self.enabled_identity_files)
            .field("options", &self.options)
            .field("permission", &self.permission)
            .field("max_iterations", &self.max_iterations)
            .field("steps", &self.steps)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentKind {
    Main,
    Worker,
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinAgentSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: BuiltinAgentKind,
    pub default_temperature: Option<f64>,
}

pub const BUILTIN_AGENT_SPECS: [BuiltinAgentSpec; 11] = [
    BuiltinAgentSpec {
        key: "main",
        label: "Main",
        description: "默认主 Agent，会继承主会话运行时参数。",
        kind: BuiltinAgentKind::Main,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "researcher",
        label: "Researcher",
        description: "偏研究与信息收集。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "coder",
        label: "Coder",
        description: "偏实现与代码修改。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "reviewer",
        label: "Reviewer",
        description: "偏审查与风险识别。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "build",
        label: "Build",
        description: "偏构建与验证。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "plan",
        label: "Plan",
        description: "偏规划与拆解。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "general",
        label: "General",
        description: "通用型子 Agent。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "explore",
        label: "Explore",
        description: "偏只读探索与代码搜索。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "compaction",
        label: "Compaction",
        description: "偏摘要压缩与上下文整理。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
    BuiltinAgentSpec {
        key: "title",
        label: "Title",
        description: "偏标题生成。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.5),
    },
    BuiltinAgentSpec {
        key: "summary",
        label: "Summary",
        description: "偏总结生成。",
        kind: BuiltinAgentKind::Worker,
        default_temperature: Some(0.7),
    },
];

pub fn builtin_agent_spec(key: &str) -> Option<&'static BuiltinAgentSpec> {
    BUILTIN_AGENT_SPECS.iter().find(|spec| spec.key == key)
}

pub fn builtin_agent_keys() -> Vec<&'static str> {
    BUILTIN_AGENT_SPECS.iter().map(|spec| spec.key).collect()
}

pub fn builtin_agent_config(key: &str) -> Option<AgentDefinitionConfig> {
    let spec = builtin_agent_spec(key)?;
    let mut config = AgentDefinitionConfig {
        label: Some(spec.label.to_string()),
        description: Some(spec.description.to_string()),
        builtin: true,
        mode: match spec.kind {
            BuiltinAgentKind::Main => "primary".to_string(),
            BuiltinAgentKind::Worker => "all".to_string(),
        },
        enabled: true,
        temperature: spec.default_temperature,
        identity_format: Some("openclaw".to_string()),
        ..AgentDefinitionConfig::default()
    };

    match key {
        "main" => {
            config.mode = "primary".to_string();
        }
        "build" => {
            config.mode = "primary".to_string();
            config.permission = json!({
                "question": "allow",
                "AskUserQuestion": "allow",
                "plan_enter": "allow"
            });
        }
        "plan" => {
            config.mode = "primary".to_string();
            config.permission = json!({
                "question": "allow",
                "AskUserQuestion": "allow",
                "plan_exit": "allow",
                "todoread": "deny",
                "todowrite": "deny",
                "TodoRead": "deny",
                "TodoWrite": "deny",
                "edit": {
                    "*": "deny",
                    ".vibewindow/plans/*.md": "allow"
                }
            });
        }
        "general" => {
            config.mode = "subagent".to_string();
            config.permission = json!({
                "todoread": "deny",
                "todowrite": "deny",
                "TodoRead": "deny",
                "TodoWrite": "deny"
            });
        }
        "explore" => {
            config.mode = "subagent".to_string();
            config.permission = json!({
                "*": "deny",
                "grep": "allow",
                "glob": "allow",
                "ls": "allow",
                "bash": "allow",
                "lsp": "allow",
                "webfetch": "allow",
                "WebFetch": "allow",
                "websearch": "allow",
                "WebSearch": "allow",
                "read": "allow"
            });
        }
        "compaction" | "title" | "summary" => {
            config.mode = "primary".to_string();
            config.hidden = true;
            config.permission = json!({ "*": "deny" });
        }
        _ => {}
    }

    Some(config)
}

pub fn merged_agent_configs(
    configured: &HashMap<String, AgentDefinitionConfig>,
) -> HashMap<String, AgentDefinitionConfig> {
    let mut merged = BUILTIN_AGENT_SPECS
        .iter()
        .filter_map(|spec| {
            builtin_agent_config(spec.key).map(|config| (spec.key.to_string(), config))
        })
        .collect::<HashMap<_, _>>();
    for (key, value) in configured {
        let mut merged_value = value.clone();
        if let Some(spec) = builtin_agent_spec(key) {
            if merged_value.label.is_none() {
                merged_value.label = Some(spec.label.to_string());
            }
            if merged_value.description.is_none() {
                merged_value.description = Some(spec.description.to_string());
            }
            merged_value.builtin = true;
        }
        merged.insert(key.clone(), merged_value);
    }
    merged
}

fn default_agents_ipc_db_path() -> String {
    crate::paths::agents_ipc_db_path()
}

fn default_agents_ipc_staleness_secs() -> u64 {
    300
}

/// 进程间代理通信配置（`[agents_ipc]` 配置段）。
///
/// 启用后会注册 IPC 工具，使同一主机上的独立 VibeWindow 进程可通过共享
/// SQLite 数据库相互发现并交换消息。默认关闭，关闭时无额外开销。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentsIpcConfig {
    /// 是否启用进程间代理通信工具。
    #[serde(default)]
    pub enabled: bool,
    /// 共享 SQLite 数据库路径（同一主机上的所有代理共用一个文件）。
    #[serde(default = "default_agents_ipc_db_path")]
    pub db_path: String,
    /// 在该时间窗口内未出现的代理会被视为离线，单位为秒。
    #[serde(default = "default_agents_ipc_staleness_secs")]
    pub staleness_secs: u64,
}

impl Default for AgentsIpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            db_path: default_agents_ipc_db_path(),
            staleness_secs: default_agents_ipc_staleness_secs(),
        }
    }
}

fn default_coordination_enabled() -> bool {
    true
}

fn default_coordination_lead_agent() -> String {
    "delegate-lead".into()
}

fn default_coordination_max_inbox_messages_per_agent() -> usize {
    256
}

fn default_coordination_max_dead_letters() -> usize {
    256
}

fn default_coordination_max_context_entries() -> usize {
    512
}

fn default_coordination_max_seen_message_ids() -> usize {
    4096
}

/// 委派协同运行时配置（`[coordination]` 配置段）。
///
/// 控制 `delegate` 与 `delegate_coordination_status` 工具使用的强类型
/// 委派消息总线集成行为。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CoordinationConfig {
    /// 是否启用委派协同追踪与运行时总线集成。
    #[serde(default = "default_coordination_enabled")]
    pub enabled: bool,
    /// 作为协调者收发件人的逻辑主代理身份标识。
    #[serde(default = "default_coordination_lead_agent")]
    pub lead_agent: String,
    /// 每个已注册代理保留的收件箱消息上限。
    #[serde(default = "default_coordination_max_inbox_messages_per_agent")]
    pub max_inbox_messages_per_agent: usize,
    /// 保留的死信条目上限。
    #[serde(default = "default_coordination_max_dead_letters")]
    pub max_dead_letters: usize,
    /// 保留的共享上下文条目上限（`ContextPatch` 状态键）。
    #[serde(default = "default_coordination_max_context_entries")]
    pub max_context_entries: usize,
    /// 已处理消息 ID 的去重窗口最大保留数量。
    #[serde(default = "default_coordination_max_seen_message_ids")]
    pub max_seen_message_ids: usize,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            enabled: default_coordination_enabled(),
            lead_agent: default_coordination_lead_agent(),
            max_inbox_messages_per_agent: default_coordination_max_inbox_messages_per_agent(),
            max_dead_letters: default_coordination_max_dead_letters(),
            max_context_entries: default_coordination_max_context_entries(),
            max_seen_message_ids: default_coordination_max_seen_message_ids(),
        }
    }
}

/// 代理编排配置（`[agent]` 配置段）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentConfig {
    /// 为 `true` 时使用紧凑上下文：`bootstrap_max_chars=6000`、`rag_chunk_limit=2`。
    /// 适合 13B 或更小规模的模型。
    #[serde(default)]
    pub compact_context: bool,
    /// 每条用户消息允许的最大工具调用循环轮数。默认值为 `50000`。
    /// 设置为 `0` 时会回退到默认值 `50000`。
    #[serde(default = "default_agent_max_tool_iterations")]
    pub max_tool_iterations: usize,
    /// 每个会话保留的历史消息上限。默认值为 `100`。
    #[serde(default = "default_agent_max_history_messages")]
    pub max_history_messages: usize,
    /// 是否在单次迭代内启用并行工具执行。默认值为 `false`。
    #[serde(default)]
    pub parallel_tools: bool,
    /// 工具分发策略，例如 `"auto"`。默认值为 `"auto"`。
    #[serde(default = "default_agent_tool_dispatcher")]
    pub tool_dispatcher: String,
}

fn default_agent_max_tool_iterations() -> usize {
    50_000
}

fn default_agent_max_history_messages() -> usize {
    100
}

fn default_agent_tool_dispatcher() -> String {
    "auto".into()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            compact_context: false,
            max_tool_iterations: default_agent_max_tool_iterations(),
            max_history_messages: default_agent_max_history_messages(),
            parallel_tools: false,
            tool_dispatcher: default_agent_tool_dispatcher(),
        }
    }
}
#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
