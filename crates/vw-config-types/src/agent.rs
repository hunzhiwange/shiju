use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

/// 委托代理访问默认能力及用户显式选择的方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub enum DelegateCapabilityPolicy {
    /// 仅允许配置的默认选择。
    Strict,
    /// 允许配置的默认选择和可信的用户显式选择。
    UserOverride,
    /// 允许全部可用能力。
    #[default]
    Unrestricted,
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
    /// 是否强制使用完整的 provider/model 绑定。
    #[serde(default)]
    pub force_model_selection: bool,
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
    #[serde(default = "default_delegate_agentic")]
    pub agentic: bool,
    /// 委托代理工具的访问策略。
    #[serde(default)]
    pub tool_policy: DelegateCapabilityPolicy,
    /// agentic 模式下注入的默认工具选择。
    #[serde(default, alias = "tools")]
    pub allowed_tools: Vec<String>,
    /// 委托代理技能的访问策略。
    #[serde(default)]
    pub skill_policy: DelegateCapabilityPolicy,
    /// 委托代理提示词中注入的默认技能选择。
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

fn default_delegate_agentic() -> bool {
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
            force_model_selection: false,
            system_prompt: None,
            api_key: None,
            temperature: None,
            top_p: None,
            identity_format: None,
            hidden: false,
            max_depth: default_max_depth(),
            agentic: default_delegate_agentic(),
            tool_policy: DelegateCapabilityPolicy::default(),
            allowed_tools: Vec::new(),
            skill_policy: DelegateCapabilityPolicy::default(),
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
            .field("force_model_selection", &self.force_model_selection)
            .field("system_prompt", &self.system_prompt)
            .field("api_key_configured", &self.api_key.is_some())
            .field("temperature", &self.temperature)
            .field("top_p", &self.top_p)
            .field("identity_format", &self.identity_format)
            .field("hidden", &self.hidden)
            .field("max_depth", &self.max_depth)
            .field("agentic", &self.agentic)
            .field("tool_policy", &self.tool_policy)
            .field("allowed_tools", &self.allowed_tools)
            .field("skill_policy", &self.skill_policy)
            .field("allowed_skills", &self.allowed_skills)
            .field("enabled_identity_files", &self.enabled_identity_files)
            .field("options", &self.options)
            .field("permission", &self.permission)
            .field("max_iterations", &self.max_iterations)
            .field("steps", &self.steps)
            .finish()
    }
}

impl AgentDefinitionConfig {
    /// 判断 provider 与 model 是否共同构成完整的模型绑定。
    pub fn has_complete_model_binding(&self) -> bool {
        !self.provider.trim().is_empty() && !self.model.trim().is_empty()
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
    pub mode: &'static str,
    pub hidden: bool,
    pub allow_as_subagent: bool,
    pub max_parallel_subagents: usize,
    pub max_depth: u32,
    pub allowed_tools: &'static [&'static str],
    pub permission: &'static str,
    pub default_temperature: Option<f64>,
    pub max_iterations: usize,
}

const READONLY_CODE_TOOLS: &[&str] = &["file_read", "ls", "glob", "grep", "lsp"];
const RESEARCHER_TOOLS: &[&str] = &[
    "file_read",
    "ls",
    "glob",
    "grep",
    "lsp",
    "web_fetch",
    "web_search_tool",
    "pdf_read",
    "image_info",
];
const CODER_TOOLS: &[&str] = &[
    "file_read",
    "file_write",
    "file_edit",
    "apply_patch",
    "bash",
    "ls",
    "glob",
    "grep",
    "lsp",
    "git_operations",
];
const MAIN_TOOLS: &[&str] = &[
    "AgentTool",
    "file_read",
    "file_write",
    "file_edit",
    "apply_patch",
    "bash",
    "ls",
    "glob",
    "grep",
    "lsp",
    "git_operations",
];

const READONLY_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow",
    "lsp": "allow"
}"#;
const RESEARCHER_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow",
    "lsp": "allow",
    "web_fetch": "allow",
    "web_search_tool": "allow",
    "pdf_read": "allow",
    "image_info": "allow"
}"#;
const CODER_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "file_write": "allow",
    "file_edit": "allow",
    "apply_patch": "allow",
    "bash": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow",
    "lsp": "allow",
    "git_operations": "allow"
}"#;
const MAIN_PERMISSION: &str = r#"{
    "*": "deny",
    "AgentTool": "allow",
    "file_read": "allow",
    "file_write": "allow",
    "file_edit": "allow",
    "apply_patch": "allow",
    "bash": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow",
    "lsp": "allow",
    "git_operations": "allow"
}"#;
pub const BUILTIN_AGENT_SPECS: [BuiltinAgentSpec; 5] = [
    BuiltinAgentSpec {
        key: "main",
        label: "主代理",
        description: "负责理解请求、直接执行，并在复杂任务中协调专业子代理。",
        kind: BuiltinAgentKind::Main,
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        max_depth: 0,
        allowed_tools: MAIN_TOOLS,
        permission: MAIN_PERMISSION,
        default_temperature: Some(0.7),
        max_iterations: 64,
    },
    BuiltinAgentSpec {
        key: "researcher",
        label: "资料研究",
        description: "查阅外部文档、开源实现与参考资料，输出带来源的研究结论。",
        kind: BuiltinAgentKind::Worker,
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: RESEARCHER_TOOLS,
        permission: RESEARCHER_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: 32,
    },
    BuiltinAgentSpec {
        key: "coder",
        label: "代码实现",
        description: "在明确范围内实现代码变更并完成必要验证。",
        kind: BuiltinAgentKind::Worker,
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: CODER_TOOLS,
        permission: CODER_PERMISSION,
        default_temperature: Some(0.2),
        max_iterations: 64,
    },
    BuiltinAgentSpec {
        key: "reviewer",
        label: "代码审查",
        description: "只读审查代码、测试与风险，给出有证据的缺陷报告。",
        kind: BuiltinAgentKind::Worker,
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: READONLY_CODE_TOOLS,
        permission: READONLY_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: 32,
    },
    BuiltinAgentSpec {
        key: "explore",
        label: "代码探索",
        description: "只读搜索本地代码、符号与调用关系，返回可操作的定位结果。",
        kind: BuiltinAgentKind::Worker,
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: READONLY_CODE_TOOLS,
        permission: READONLY_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: 32,
    },
];

pub fn is_retired_builtin_agent_key(key: &str) -> bool {
    matches!(key, "build" | "plan" | "compaction" | "title" | "summary")
}

pub fn builtin_agent_spec(key: &str) -> Option<&'static BuiltinAgentSpec> {
    BUILTIN_AGENT_SPECS.iter().find(|spec| spec.key == key)
}

pub fn builtin_agent_keys() -> Vec<&'static str> {
    BUILTIN_AGENT_SPECS.iter().map(|spec| spec.key).collect()
}

pub fn builtin_agent_config(key: &str) -> Option<AgentDefinitionConfig> {
    let spec = builtin_agent_spec(key)?;
    Some(AgentDefinitionConfig {
        label: Some(spec.label.to_string()),
        description: Some(spec.description.to_string()),
        builtin: true,
        mode: spec.mode.to_string(),
        enabled: true,
        allow_as_subagent: spec.allow_as_subagent,
        max_parallel_subagents: spec.max_parallel_subagents,
        temperature: spec.default_temperature,
        identity_format: Some("openclaw".to_string()),
        hidden: spec.hidden,
        max_depth: spec.max_depth,
        allowed_tools: spec.allowed_tools.iter().map(|tool| (*tool).to_string()).collect(),
        permission: serde_json::from_str(spec.permission).expect("builtin agent permission is valid JSON"),
        max_iterations: spec.max_iterations,
        ..AgentDefinitionConfig::default()
    })
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
        if key == "general" || is_retired_builtin_agent_key(key) {
            continue;
        }
        if let Some(spec) = builtin_agent_spec(key) {
            let mut builtin = builtin_agent_config(spec.key).expect("builtin spec has configuration");
            builtin.enabled &= value.enabled;
            if spec.allow_as_subagent && !value.allowed_tools.is_empty() {
                builtin.allowed_tools.retain(|tool| value.allowed_tools.contains(tool));
            }
            if key == "main" {
                builtin.max_parallel_subagents = value.max_parallel_subagents.max(1);
            }
            builtin.provider = value.provider.clone();
            builtin.model = value.model.clone();
            builtin.force_model_selection = value.force_model_selection;
            builtin.system_prompt = value.system_prompt.clone();
            builtin.api_key = value.api_key.clone();
            if value.temperature.is_some() {
                builtin.temperature = value.temperature;
            }
            builtin.top_p = value.top_p;
            builtin.tool_policy = value.tool_policy;
            builtin.allowed_skills = value.allowed_skills.clone();
            builtin.skill_policy = value.skill_policy;
            builtin.enabled_identity_files = value.enabled_identity_files.clone();
            builtin.options = value.options.clone();
            builtin.steps = value.steps;
            merged.insert(key.clone(), builtin);
        } else {
            merged.insert(key.clone(), value.clone());
        }
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
