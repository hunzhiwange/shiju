use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

/// 代理自治等级。
///
/// 用于决定代理在没有人工参与的情况下可以执行到什么程度。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AutonomyLevel {
    /// 仅允许只读操作。
    ReadOnly,
    /// 需要监督的模式。
    Supervised,
    /// 允许完全自主执行的默认模式。
    #[default]
    Full,
}

impl std::str::FromStr for AutonomyLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "read_only" | "readonly" => Ok(Self::ReadOnly),
            "supervised" => Ok(Self::Supervised),
            "full" => Ok(Self::Full),
            _ => Err(format!(
                "invalid autonomy level '{s}': expected read_only, supervised, or full"
            )),
        }
    }
}

/// Shell 重定向处理策略。
///
/// 用于控制 shell 命令中出现的 `>`、`>>` 等重定向语法如何被处理。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShellRedirectPolicy {
    /// 直接阻止包含重定向的命令。
    Block,
    /// 剥离重定向片段后再执行。
    Strip,
    /// 保留重定向并直接执行。
    #[default]
    Allow,
}

/// 密钥与敏感信息存储配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecretsConfig {
    /// 是否对持久化的敏感信息进行加密。
    #[serde(default = "default_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

/// 代理身份文档配置。
///
/// 用于指定身份文档格式，以及文档来自文件还是内联内容。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IdentityConfig {
    /// 身份文档格式，例如 `openclaw`。
    #[serde(default = "default_identity_format")]
    pub format: String,
    /// 外部身份文档文件路径。
    #[serde(default)]
    pub aieos_path: Option<String>,
    /// 内联身份文档内容。
    #[serde(default)]
    pub aieos_inline: Option<String>,
}

fn default_identity_format() -> String {
    "openclaw".into()
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self { format: default_identity_format(), aieos_path: None, aieos_inline: None }
    }
}

/// 非 CLI 通道的自然语言审批模式。
///
/// 用于控制如 IM、协作平台等非命令行入口中，代理如何处理自然语言批准请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonCliNaturalLanguageApprovalMode {
    /// 禁用自然语言审批。
    Disabled,
    /// 先请求确认，再继续执行。
    RequestConfirm,
    /// 直接把自然语言批准当作有效批准。
    #[default]
    Direct,
}

/// 代理自治配置。
///
/// 该结构集中定义代理在执行命令与工具调用时的权限边界、审批阈值以及资源预算。
/// 它是代理风险控制的第一层配置。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AutonomyConfig {
    /// 代理自治等级。
    pub level: AutonomyLevel,
    /// 是否仅允许在工作区范围内操作。
    pub workspace_only: bool,
    /// 允许执行的命令白名单。
    pub allowed_commands: Vec<String>,
    /// 禁止访问的路径列表。
    pub forbidden_paths: Vec<String>,
    /// 每小时允许执行的最大动作数。
    pub max_actions_per_hour: u32,
    /// 每日允许消耗的最大成本，单位为美分。
    pub max_cost_per_day_cents: u32,

    /// 中风险操作是否需要审批。
    pub require_approval_for_medium_risk: bool,

    /// 是否阻止高风险命令。
    pub block_high_risk_commands: bool,

    /// Shell 重定向处理策略。
    #[serde(default)]
    pub shell_redirect_policy: ShellRedirectPolicy,

    /// 允许透传到 shell 环境中的环境变量名列表。
    #[serde(default)]
    pub shell_env_passthrough: Vec<String>,

    /// 默认可自动批准的工具名列表。
    #[serde(default = "default_auto_approve")]
    pub auto_approve: Vec<String>,

    /// 无论如何都要询问用户的工具名列表。
    #[serde(default = "default_always_ask")]
    pub always_ask: Vec<String>,

    /// 额外允许访问的根目录列表。
    #[serde(default)]
    pub allowed_roots: Vec<String>,

    /// 非 CLI 场景中禁用的工具列表。
    #[serde(default = "default_non_cli_excluded_tools")]
    pub non_cli_excluded_tools: Vec<String>,

    /// 非 CLI 场景中可充当审批人的主体列表。
    #[serde(default)]
    pub non_cli_approval_approvers: Vec<String>,

    /// 非 CLI 场景的默认自然语言审批模式。
    #[serde(default)]
    pub non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode,

    /// 是否允许潜在危险的 shell 模式。
    #[serde(default)]
    pub allow_unsafe_shell_patterns: bool,

    /// 按通道覆盖的自然语言审批模式映射。
    #[serde(default)]
    pub non_cli_natural_language_approval_mode_by_channel:
        HashMap<String, NonCliNaturalLanguageApprovalMode>,
}

fn default_auto_approve() -> Vec<String> {
    vec!["file_read".into(), "memory_recall".into()]
}

fn default_always_ask() -> Vec<String> {
    vec![]
}

fn default_non_cli_excluded_tools() -> Vec<String> {
    [
        "bash",
        "file_write",
        "git_operations",
        "browser",
        "browser_open",
        "http_request",
        "schedule",
        "cron_add",
        "cron_remove",
        "cron_update",
        "cron_run",
        "memory_store",
        "memory_forget",
        "delegate",
        "screenshot",
    ]
    .into_iter()
    .map(std::string::ToString::to_string)
    .collect()
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: AutonomyLevel::Full,
            workspace_only: false,
            allowed_commands: vec![
                "git".into(),
                "npm".into(),
                "cargo".into(),
                "ls".into(),
                "cat".into(),
                "grep".into(),
                "find".into(),
                "echo".into(),
                "pwd".into(),
                "wc".into(),
                "head".into(),
                "tail".into(),
                "date".into(),
            ],
            forbidden_paths: vec![
                "/etc".into(),
                "/root".into(),
                "/home".into(),
                "/usr".into(),
                "/bin".into(),
                "/sbin".into(),
                "/lib".into(),
                "/opt".into(),
                "/boot".into(),
                "/dev".into(),
                "/proc".into(),
                "/sys".into(),
                "/var".into(),
                "/tmp".into(),
                "~/.ssh".into(),
                "~/.gnupg".into(),
                "~/.aws".into(),
                "~/.config".into(),
            ],
            max_actions_per_hour: 2_000,
            max_cost_per_day_cents: 50_000,
            require_approval_for_medium_risk: false,
            block_high_risk_commands: false,
            shell_redirect_policy: ShellRedirectPolicy::Allow,
            shell_env_passthrough: vec![],
            auto_approve: default_auto_approve(),
            always_ask: default_always_ask(),
            allowed_roots: Vec::new(),
            non_cli_excluded_tools: default_non_cli_excluded_tools(),
            non_cli_approval_approvers: Vec::new(),
            non_cli_natural_language_approval_mode: NonCliNaturalLanguageApprovalMode::default(),
            non_cli_natural_language_approval_mode_by_channel: HashMap::new(),
            allow_unsafe_shell_patterns: false,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
#[path = "autonomy_tests.rs"]
mod tests;
