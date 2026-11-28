use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use config_agent_profile::{AgentDefault, AgentDefaults, AgentModelReference, AgentRuntime};

pub const DEFAULT_MAX_ITERATIONS: usize = 50_000;

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

/// 委托代理技能的可见范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub enum DelegateCapabilityPolicy {
    /// 仅允许配置的默认选择。
    Strict,
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
    /// 新会话使用的运行引擎。
    #[serde(default)]
    pub runtime: AgentRuntime,
    /// 供用户填入编辑框的推荐提示词。
    #[serde(default)]
    pub recommended_prompts: Vec<String>,
    /// 新会话的默认选择。
    #[serde(default)]
    pub defaults: AgentDefaults,
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
    /// agentic 模式下注册给模型的工具选择。
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
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    /// 是否为该智能体启用紧凑上下文。
    #[serde(default)]
    pub compact_context: bool,
    /// 该智能体每个会话保留的历史消息上限。
    #[serde(default = "default_agent_max_history_messages")]
    pub max_history_messages: usize,
    /// 是否允许该智能体在同一轮并行执行工具。
    #[serde(default)]
    pub parallel_tools: bool,
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

fn default_max_iterations() -> usize {
    DEFAULT_MAX_ITERATIONS
}

impl Default for AgentDefinitionConfig {
    fn default() -> Self {
        Self {
            runtime: AgentRuntime::default(),
            recommended_prompts: Vec::new(),
            defaults: AgentDefaults::default(),
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
            allowed_tools: Vec::new(),
            skill_policy: DelegateCapabilityPolicy::default(),
            allowed_skills: Vec::new(),
            enabled_identity_files: Vec::new(),
            options: HashMap::new(),
            permission: Value::Null,
            max_iterations: default_max_iterations(),
            compact_context: false,
            max_history_messages: default_agent_max_history_messages(),
            parallel_tools: false,
            steps: None,
        }
    }
}

impl std::fmt::Debug for AgentDefinitionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentDefinitionConfig")
            .field("runtime", &self.runtime)
            .field("recommended_prompt_count", &self.recommended_prompts.len())
            .field("defaults", &self.defaults)
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
            .field("allowed_tools", &self.allowed_tools)
            .field("skill_policy", &self.skill_policy)
            .field("allowed_skills", &self.allowed_skills)
            .field("enabled_identity_files", &self.enabled_identity_files)
            .field("options", &self.options)
            .field("permission", &self.permission)
            .field("max_iterations", &self.max_iterations)
            .field("compact_context", &self.compact_context)
            .field("max_history_messages", &self.max_history_messages)
            .field("parallel_tools", &self.parallel_tools)
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
    Butler,
    Worker,
    Specialist,
}

pub const SHIJU_BUTLER_AGENT_KEY: &str = "shiju-butler";

pub fn can_delegate_from_agent(key: &str) -> bool {
    matches!(key, "main" | SHIJU_BUTLER_AGENT_KEY)
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinAgentSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: BuiltinAgentKind,
    pub system_prompt: Option<&'static str>,
    pub default_skills: &'static [&'static str],
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

const READONLY_CODE_TOOLS: &[&str] = &["file_read", "ls", "glob", "grep"];
const RESEARCHER_TOOLS: &[&str] =
    &["file_read", "ls", "glob", "grep", "web_fetch", "web_search_tool", "pdf_read"];
const CODER_TOOLS: &[&str] =
    &["file_read", "file_write", "file_edit", "apply_patch", "bash", "ls", "glob", "grep"];
const MAIN_TOOLS: &[&str] = config_tools::DEFAULT_ENABLED_TOOL_IDS;
const DOCUMENT_TOOLS: &[&str] = &["file_read", "ls", "glob", "skill", "bash"];
const PPT_SKILLS: &[&str] = &["officecli", "officecli-pptx"];
const EXCEL_SKILLS: &[&str] = &["officecli", "officecli-xlsx"];

const READONLY_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow"
}"#;
const RESEARCHER_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "ls": "allow",
    "glob": "allow",
    "grep": "allow",
    "web_fetch": "allow",
    "web_search_tool": "allow",
    "pdf_read": "allow"
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
    "grep": "allow"
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
    "grep": "allow"
}"#;
const DOCUMENT_PERMISSION: &str = r#"{
    "*": "deny",
    "file_read": "allow",
    "ls": "allow",
    "glob": "allow",
    "skill": "allow",
    "bash": "ask"
}"#;

pub const BUILTIN_AGENT_SPECS: [BuiltinAgentSpec; 10] = [
    BuiltinAgentSpec {
        key: "main",
        label: "主代理",
        description: "负责理解请求、直接执行，并在复杂任务中协调专业子代理。",
        kind: BuiltinAgentKind::Main,
        system_prompt: None,
        default_skills: &[],
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        max_depth: 0,
        allowed_tools: MAIN_TOOLS,
        permission: MAIN_PERMISSION,
        default_temperature: Some(0.7),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: SHIJU_BUTLER_AGENT_KEY,
        label: "识矩管家",
        description: "通过对话管理识矩的系统设置和定时任务。",
        kind: BuiltinAgentKind::Butler,
        system_prompt: Some(include_str!("shiju_butler.txt")),
        default_skills: &["skill-creator"],
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 4,
        max_depth: 0,
        // An empty selection uses the live catalog until the user saves a tool range.
        allowed_tools: &[],
        permission: r#"{"*":"allow"}"#,
        default_temperature: Some(0.3),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "researcher",
        label: "资料研究",
        description: "查阅外部文档、开源实现与参考资料，输出带来源的研究结论。",
        kind: BuiltinAgentKind::Worker,
        system_prompt: None,
        default_skills: &[],
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: RESEARCHER_TOOLS,
        permission: RESEARCHER_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "coder",
        label: "代码实现",
        description: "在明确范围内实现代码变更并完成必要验证。",
        kind: BuiltinAgentKind::Worker,
        system_prompt: None,
        default_skills: &[],
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: CODER_TOOLS,
        permission: CODER_PERMISSION,
        default_temperature: Some(0.2),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "reviewer",
        label: "代码审查",
        description: "只读审查代码、测试与风险，给出有证据的缺陷报告。",
        kind: BuiltinAgentKind::Worker,
        system_prompt: None,
        default_skills: &[],
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: READONLY_CODE_TOOLS,
        permission: READONLY_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "explore",
        label: "代码探索",
        description: "只读搜索本地代码、符号与调用关系，返回可操作的定位结果。",
        kind: BuiltinAgentKind::Worker,
        system_prompt: None,
        default_skills: &[],
        mode: "subagent",
        hidden: false,
        allow_as_subagent: true,
        max_parallel_subagents: 1,
        max_depth: 1,
        allowed_tools: READONLY_CODE_TOOLS,
        permission: READONLY_PERMISSION,
        default_temperature: Some(0.1),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "office-ppt",
        label: "PPT 制作",
        description: "制作和编辑 PowerPoint 演示文稿，检查版式、图表和文字。",
        kind: BuiltinAgentKind::Specialist,
        system_prompt: Some(include_str!("office_ppt.txt")),
        default_skills: PPT_SKILLS,
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 1,
        max_depth: 0,
        allowed_tools: DOCUMENT_TOOLS,
        permission: DOCUMENT_PERMISSION,
        default_temperature: Some(0.3),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "office-excel",
        label: "Excel 工作簿",
        description: "创建、编辑和分析 Excel 工作簿，处理公式、数据验证和图表。",
        kind: BuiltinAgentKind::Specialist,
        system_prompt: Some(include_str!("office_excel.txt")),
        default_skills: EXCEL_SKILLS,
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 1,
        max_depth: 0,
        allowed_tools: DOCUMENT_TOOLS,
        permission: DOCUMENT_PERMISSION,
        default_temperature: Some(0.3),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "office-pdf",
        label: "PDF 处理",
        description: "提取、生成、合并或拆分 PDF，并处理可填写表单。",
        kind: BuiltinAgentKind::Specialist,
        system_prompt: Some(include_str!("office_pdf.txt")),
        default_skills: &["pdf"],
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 1,
        max_depth: 0,
        allowed_tools: DOCUMENT_TOOLS,
        permission: DOCUMENT_PERMISSION,
        default_temperature: Some(0.3),
        max_iterations: DEFAULT_MAX_ITERATIONS,
    },
    BuiltinAgentSpec {
        key: "mermaid-diagrams",
        label: "Mermaid 制图",
        description: "绘制并渲染流程图、时序图、状态图和实体关系图。",
        kind: BuiltinAgentKind::Specialist,
        system_prompt: Some(include_str!("mermaid_diagrams.txt")),
        default_skills: &["mermaid"],
        mode: "primary",
        hidden: false,
        allow_as_subagent: false,
        max_parallel_subagents: 1,
        max_depth: 0,
        allowed_tools: DOCUMENT_TOOLS,
        permission: DOCUMENT_PERMISSION,
        default_temperature: Some(0.3),
        max_iterations: DEFAULT_MAX_ITERATIONS,
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
    let allowed_skills: Vec<String> =
        spec.default_skills.iter().map(|skill| (*skill).to_string()).collect();
    let skills = if allowed_skills.is_empty() {
        AgentDefault::Inherit
    } else {
        AgentDefault::Fixed(allowed_skills.clone())
    };
    Some(AgentDefinitionConfig {
        defaults: AgentDefaults {
            full_access: matches!(spec.kind, BuiltinAgentKind::Main),
            skills,
            ..AgentDefaults::default()
        },
        label: Some(spec.label.to_string()),
        description: Some(spec.description.to_string()),
        system_prompt: spec.system_prompt.map(str::to_string),
        builtin: true,
        mode: spec.mode.to_string(),
        enabled: spec.kind != BuiltinAgentKind::Specialist,
        allow_as_subagent: spec.allow_as_subagent,
        max_parallel_subagents: spec.max_parallel_subagents,
        temperature: spec.default_temperature,
        identity_format: Some("openclaw".to_string()),
        hidden: spec.hidden,
        max_depth: spec.max_depth,
        allowed_tools: spec.allowed_tools.iter().map(|tool| (*tool).to_string()).collect(),
        allowed_skills,
        permission: serde_json::from_str(spec.permission)
            .expect("builtin agent permission is valid JSON"),
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
            let mut builtin =
                builtin_agent_config(spec.key).expect("builtin spec has configuration");
            builtin.label = value.label.clone().or(builtin.label);
            builtin.description = value.description.clone().or(builtin.description);
            builtin.recommended_prompts = value.recommended_prompts.clone();
            builtin.defaults = value.defaults.clone();
            builtin.enabled = value.enabled;
            builtin.allow_as_subagent &= value.allow_as_subagent;
            if spec.allow_as_subagent && !value.allowed_tools.is_empty() {
                builtin.allowed_tools.retain(|tool| value.allowed_tools.contains(tool));
            }
            if spec.kind == BuiltinAgentKind::Butler {
                builtin.allowed_tools.clone_from(&value.allowed_tools);
            }
            if key == "main" {
                if !value.allowed_tools.is_empty() {
                    builtin.allowed_tools = value.allowed_tools.clone();
                }
                builtin.max_parallel_subagents = value.max_parallel_subagents.max(1);
            }
            builtin.provider = value.provider.clone();
            builtin.model = value.model.clone();
            builtin.force_model_selection = value.force_model_selection;
            builtin.system_prompt = value.system_prompt.clone().or(builtin.system_prompt);
            builtin.api_key = value.api_key.clone();
            if value.temperature.is_some() {
                builtin.temperature = value.temperature;
            }
            builtin.top_p = value.top_p;
            // An older Butler profile can inherit skills without a saved selection.
            if spec.kind != BuiltinAgentKind::Butler
                || !value.allowed_skills.is_empty()
                || !matches!(value.defaults.skills, AgentDefault::Inherit)
            {
                builtin.allowed_skills.clone_from(&value.allowed_skills);
            }
            builtin.skill_policy = value.skill_policy;
            builtin.enabled_identity_files = value.enabled_identity_files.clone();
            builtin.options = value.options.clone();
            builtin.compact_context = value.compact_context;
            builtin.max_history_messages = value.max_history_messages;
            builtin.parallel_tools = value.parallel_tools;
            builtin.max_iterations = value.max_iterations;
            builtin.steps = value.steps;
            merged.insert(key.clone(), builtin);
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    merged
}

/// 返回合并内建边界和用户配置后的主代理定义。
pub fn effective_main_agent_config(
    configured: &HashMap<String, AgentDefinitionConfig>,
) -> AgentDefinitionConfig {
    merged_agent_configs(configured)
        .remove("main")
        .expect("builtin main agent configuration exists")
}

/// 确保可变配置中存在主代理定义，用于显式运行时覆盖。
pub fn ensure_main_agent_config(
    configured: &mut HashMap<String, AgentDefinitionConfig>,
) -> &mut AgentDefinitionConfig {
    configured.entry("main".to_string()).or_insert_with(|| {
        builtin_agent_config("main").expect("builtin main agent configuration exists")
    })
}

fn default_agent_max_history_messages() -> usize {
    100
}
#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;

#[cfg(test)]
mod agent_profile_tests;
