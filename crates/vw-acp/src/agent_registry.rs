//! ACP 内置代理注册表。
//!
//! 该模块集中维护 VibeWindow 可直接识别的 ACP 代理名称、启动命令、
//! 兼容别名以及用户配置覆盖的合并逻辑。调用方通过这里把用户输入的
//! 代理名称解析成可执行命令，或获取结构化的命令规格用于进程启动。

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
use vw_shared::shell::{resolve_executable, shell_profile_env_var};

use crate::types::AcpAgentConfig;

/// 默认代理名称。
///
/// 当调用方没有显式选择 ACP 代理时，使用该名称在注册表中解析命令。
pub const DEFAULT_AGENT_NAME: &str = "codex";

const PI_ADAPTER_PACKAGE_RANGE: &str = "^0.0.22";
const CLAUDE_ADAPTER_PACKAGE_RANGE: &str = "^0.26.0";
const CODEX_ADAPTER_PACKAGE: &str = "@agentclientprotocol/codex-acp@latest";
const CODEX_ADAPTER_ARGS: &[&str] = &["-y", CODEX_ADAPTER_PACKAGE];
const LEGACY_CODEX_ADAPTER_PACKAGES: &[&str] = &["@zed-industries/codex-acp", "@openai/codex-acp"];
const OBSOLETE_CODEX_NODE_REPL_CONFIG: &str = "mcp_servers.node_repl.enabled=false";
const GEMINI_ACP_ARGS: &[&str] =
    &["@google/gemini-cli@latest", "--experimental-acp", "--approval-mode", "yolo", "--skip-trust"];
const GEMINI_LOCAL_ARGS: &[&str] =
    &["--experimental-acp", "--approval-mode", "yolo", "--skip-trust"];
const QODER_ACP_ARGS: &[&str] =
    &["@qoder-ai/qodercli@latest", "--acp", "--permission-mode", "bypass_permissions"];
const QODER_LOCAL_ARGS: &[&str] = &["--acp", "--permission-mode", "bypass_permissions"];
const QWEN_ACP_ARGS: &[&str] =
    &["@qwen-code/qwen-code@latest", "--acp", "--experimental-skills", "--approval-mode", "yolo"];
const QWEN_LOCAL_ARGS: &[&str] = &["--acp", "--experimental-skills", "--approval-mode", "yolo"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
/// 一个 ACP 代理的结构化启动规格。
///
/// 该结构保留命令、参数和环境变量的边界，避免调用方只能处理拼接后的
/// shell 字符串。序列化时使用 camelCase，便于和配置文件或前端模型对接。
pub struct AgentCommandSpec {
    /// 面向用户展示的代理名称。
    pub display_name: String,
    /// 代理可执行文件或启动器命令。
    pub command: String,
    /// 传递给命令的参数列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// 启动代理时额外注入的环境变量。
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

impl AgentCommandSpec {
    /// 返回兼容旧注册表格式的命令行文本。
    ///
    /// 返回值仅用于展示或旧接口兼容；实际执行时应优先使用结构化的
    /// `command` 和 `args` 字段，避免重新解析 shell 文本带来的歧义。
    pub fn command_line(&self) -> String {
        std::iter::once(self.command.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<&AgentCommandSpec> for AcpAgentConfig {
    fn from(value: &AgentCommandSpec) -> Self {
        Self { command: value.command.clone(), args: value.args.clone(), env: value.env.clone() }
    }
}

struct BuiltInAgentDefinition {
    name: &'static str,
    display_name: &'static str,
    command: &'static str,
    args: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
    local_command_candidates: &'static [&'static str],
    local_args: &'static [&'static str],
}

fn opencode_binary_name() -> &'static str {
    if cfg!(windows) { "opencode.exe" } else { "opencode" }
}

fn resolve_opencode_command() -> Option<String> {
    if let Some(path) = shell_profile_env_var("OPENCODE_BIN") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
    {
        let candidate = home.join(".opencode").join("bin").join(opencode_binary_name());
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    resolve_executable(opencode_binary_name()).map(|path| path.to_string_lossy().to_string())
}

fn resolve_local_agent_command(candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|candidate| {
        resolve_executable(candidate).map(|path| path.to_string_lossy().to_string())
    })
}

fn built_in_agent_command(definition: &BuiltInAgentDefinition) -> (String, Vec<String>) {
    if definition.name == "opencode" {
        if let Some(command) = resolve_opencode_command() {
            return (command, vec!["acp".to_string()]);
        }
    } else if let Some(command) = resolve_local_agent_command(definition.local_command_candidates) {
        return (command, definition.local_args.iter().map(|arg| (*arg).to_string()).collect());
    }

    (definition.command.to_string(), definition.args.iter().map(|arg| (*arg).to_string()).collect())
}

fn is_codex_adapter_package_arg(arg: &str, package: &str) -> bool {
    arg == package || arg.strip_prefix(package).is_some_and(|suffix| suffix.starts_with('@'))
}

fn normalize_codex_adapter_package_arg(arg: String) -> String {
    if LEGACY_CODEX_ADAPTER_PACKAGES
        .iter()
        .any(|package| is_codex_adapter_package_arg(&arg, package))
    {
        return CODEX_ADAPTER_PACKAGE.to_string();
    }
    arg
}

fn is_obsolete_codex_node_repl_config(arg: &str) -> bool {
    arg == OBSOLETE_CODEX_NODE_REPL_CONFIG
        || arg
            .strip_prefix("--config=")
            .is_some_and(|value| value == OBSOLETE_CODEX_NODE_REPL_CONFIG)
        || arg.strip_prefix("-c=").is_some_and(|value| value == OBSOLETE_CODEX_NODE_REPL_CONFIG)
}

fn normalize_codex_args(args: &[String]) -> Vec<String> {
    let mut normalized = Vec::with_capacity(args.len());
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if matches!(arg.as_str(), "-c" | "--config")
            && args.get(index + 1).is_some_and(|next| next == OBSOLETE_CODEX_NODE_REPL_CONFIG)
        {
            index += 2;
            continue;
        }
        if is_obsolete_codex_node_repl_config(arg) {
            index += 1;
            continue;
        }
        normalized.push(normalize_codex_adapter_package_arg(arg.clone()));
        index += 1;
    }
    normalized
}

fn normalize_agent_spec_for_name(name: &str, spec: &AgentCommandSpec) -> AgentCommandSpec {
    let normalized_name = normalize_agent_name(name);
    AgentCommandSpec {
        display_name: spec.display_name.trim().to_string(),
        command: spec.command.trim().to_string(),
        args: if normalized_name == DEFAULT_AGENT_NAME {
            normalize_codex_args(&spec.args)
        } else {
            spec.args.clone()
        },
        env: spec.env.clone(),
    }
}

const BUILT_IN_AGENT_DEFINITIONS: &[BuiltInAgentDefinition] = &[
    BuiltInAgentDefinition {
        name: "auggie",
        display_name: "Auggie CLI",
        command: "npx",
        args: &["@augmentcode/auggie@latest", "--acp"],
        env: &[("AUGMENT_DISABLE_AUTO_UPDATE", "1")],
        local_command_candidates: &["auggie"],
        local_args: &["--acp"],
    },
    BuiltInAgentDefinition {
        name: "claude",
        display_name: "Claude Code",
        command: "npx",
        args: &["-y", "@agentclientprotocol/claude-agent-acp@^0.26.0"],
        env: &[],
        local_command_candidates: &["claude-agent-acp"],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "codex",
        display_name: "Codex CLI",
        command: "npx",
        args: CODEX_ADAPTER_ARGS,
        env: &[],
        local_command_candidates: &["codex-acp"],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "copilot",
        display_name: "GitHub Copilot",
        command: "copilot",
        args: &["--acp", "--stdio"],
        env: &[],
        local_command_candidates: &["copilot"],
        local_args: &["--acp", "--stdio"],
    },
    BuiltInAgentDefinition {
        name: "cursor",
        display_name: "Cursor",
        command: "cursor-agent",
        args: &["acp"],
        env: &[],
        local_command_candidates: &[],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "droid",
        display_name: "Factory droid",
        command: "droid",
        args: &["exec", "--output-format", "acp"],
        env: &[],
        local_command_candidates: &[],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "gemini",
        display_name: "Gemini CLI",
        command: "npx",
        args: GEMINI_ACP_ARGS,
        env: &[],
        local_command_candidates: &["gemini", "gemini-cli"],
        local_args: GEMINI_LOCAL_ARGS,
    },
    BuiltInAgentDefinition {
        name: "iflow",
        display_name: "iFlow",
        command: "iflow",
        args: &["--experimental-acp"],
        env: &[],
        local_command_candidates: &[],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "kilocode",
        display_name: "Kilocode",
        command: "npx",
        args: &["-y", "@kilocode/cli", "acp"],
        env: &[],
        local_command_candidates: &["kilocode"],
        local_args: &["acp"],
    },
    BuiltInAgentDefinition {
        name: "kimi",
        display_name: "Kimi Code CLI",
        command: "kimi",
        args: &["acp"],
        env: &[],
        local_command_candidates: &[],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "kiro",
        display_name: "Kiro",
        command: "npx",
        args: &["@kiro/kiro-agent@latest", "--acp"],
        env: &[],
        local_command_candidates: &["kiro-agent"],
        local_args: &["--acp"],
    },
    BuiltInAgentDefinition {
        name: "opencode",
        display_name: "OpenCode",
        command: "npx",
        args: &["opencode-ai@latest", "acp"],
        env: &[],
        local_command_candidates: &["opencode"],
        local_args: &["acp"],
    },
    BuiltInAgentDefinition {
        name: "openclaw",
        display_name: "OpenClaw",
        command: "npx",
        args: &["openclaw", "acp"],
        env: &[],
        local_command_candidates: &["openclaw"],
        local_args: &["acp"],
    },
    BuiltInAgentDefinition {
        name: "pi",
        display_name: "PI",
        command: "npx",
        args: &["-y", "pi-acp@^0.0.22"],
        env: &[],
        local_command_candidates: &["pi-acp"],
        local_args: &[],
    },
    BuiltInAgentDefinition {
        name: "qoder",
        display_name: "Qoder CLI",
        command: "npx",
        args: QODER_ACP_ARGS,
        env: &[],
        local_command_candidates: &["qodercli"],
        local_args: QODER_LOCAL_ARGS,
    },
    BuiltInAgentDefinition {
        name: "qwen",
        display_name: "Qwen Code",
        command: "npx",
        args: QWEN_ACP_ARGS,
        env: &[],
        local_command_candidates: &["qwen", "qwen-code"],
        local_args: QWEN_LOCAL_ARGS,
    },
    BuiltInAgentDefinition {
        name: "trae",
        display_name: "TRAE CLI",
        command: "traecli",
        args: &["acp", "serve"],
        env: &[],
        local_command_candidates: &[],
        local_args: &[],
    },
];

const AGENT_ALIASES: &[(&str, &str)] = &[
    ("agentclientprotocol-claude", "claude"),
    ("auggie-cli", "auggie"),
    ("auggie cli", "auggie"),
    ("claude code", "claude"),
    ("claudecode", "claude"),
    ("codex-cli", "codex"),
    ("codex cli", "codex"),
    ("copilot-cli", "copilot"),
    ("factory-droid", "droid"),
    ("factory droid", "droid"),
    ("factorydroid", "droid"),
    ("gemini-cli", "gemini"),
    ("gemini cli", "gemini"),
    ("github-copilot", "copilot"),
    ("github copilot", "copilot"),
    ("githubcopilot", "copilot"),
    ("kiro agent", "kiro"),
    ("kiro-agent", "kiro"),
    ("kiro-cli-chat", "kiro"),
    ("kimi-code", "kimi"),
    ("kimi code", "kimi"),
    ("kimi code cli", "kimi"),
    ("kimi-code-cli", "kimi"),
    ("open-code", "opencode"),
    ("qoder cli", "qoder"),
    ("pi-acp", "pi"),
    ("qoder-cli", "qoder"),
    ("qwen code", "qwen"),
    ("qwen-code", "qwen"),
    ("trae cli", "trae"),
    ("trae-cli", "trae"),
];

/// 规范化用户输入的代理名称。
///
/// 参数会被去除首尾空白并转换为小写，返回值用于注册表查找和别名匹配。
pub fn normalize_agent_name(value: &str) -> String {
    value.trim().to_lowercase()
}

fn built_in_agent_definitions() -> &'static [BuiltInAgentDefinition] {
    BUILT_IN_AGENT_DEFINITIONS
}

fn alias_target(name: &str) -> Option<&'static str> {
    AGENT_ALIASES.iter().find_map(|(alias, target)| (*alias == name).then_some(*target))
}

/// 返回所有内置 ACP 代理的结构化启动规格。
///
/// 返回值按代理 key 排序，且会尽量优先解析本地已安装的命令；如果找不到
/// 本地命令，则回退到内置的 `npx` 或默认命令。
pub fn built_in_agent_specs() -> BTreeMap<String, AgentCommandSpec> {
    built_in_agent_definitions()
        .iter()
        .map(|definition| {
            let (command, args) = built_in_agent_command(definition);
            let env = definition
                .env
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect::<HashMap<_, _>>();
            (
                definition.name.to_string(),
                AgentCommandSpec {
                    display_name: definition.display_name.to_string(),
                    command,
                    args,
                    env,
                },
            )
        })
        .collect()
}

fn legacy_agent_registry() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("pi".to_string(), format!("npx pi-acp@{PI_ADAPTER_PACKAGE_RANGE}")),
        ("openclaw".to_string(), "openclaw acp".to_string()),
        ("codex".to_string(), format!("npx {CODEX_ADAPTER_PACKAGE}")),
        (
            "claude-legacy".to_string(),
            format!("npx -y @agentclientprotocol/claude-agent-acp@{CLAUDE_ADAPTER_PACKAGE_RANGE}"),
        ),
    ])
}

/// 返回兼容旧调用方的内置代理命令注册表。
///
/// 返回值将结构化规格拼接为命令行文本，并包含仍需保留的旧 key。
pub fn built_in_agent_registry() -> BTreeMap<String, String> {
    let mut registry = legacy_agent_registry();
    for (name, spec) in built_in_agent_specs() {
        registry.insert(name, spec.command_line());
    }
    registry
}

/// 合并内置代理规格和用户覆盖项。
///
/// `overrides` 中的空名称或空命令会被忽略。返回值以规范化名称作为 key，
/// 用户覆盖项会替换同名内置规格；该函数不执行命令，也不校验命令是否存在。
pub fn merge_agent_specs(
    overrides: Option<&HashMap<String, AgentCommandSpec>>,
) -> BTreeMap<String, AgentCommandSpec> {
    let mut merged = built_in_agent_specs();
    if let Some(overrides) = overrides {
        for (name, spec) in overrides {
            let normalized = normalize_agent_name(name);
            if normalized.is_empty() || spec.command.trim().is_empty() {
                continue;
            }
            merged.insert(normalized.clone(), normalize_agent_spec_for_name(&normalized, spec));
        }
    }
    merged
}

/// 合并内置代理注册表和用户覆盖项，并返回命令行文本视图。
///
/// 参数和错误处理规则与 [`merge_agent_specs`] 一致；该函数主要服务仍使用
/// 字符串命令的旧接口。
pub fn merge_agent_registry(
    overrides: Option<&HashMap<String, AgentCommandSpec>>,
) -> BTreeMap<String, String> {
    let mut merged = built_in_agent_registry();
    if let Some(overrides) = overrides {
        for (name, spec) in overrides {
            let normalized = normalize_agent_name(name);
            if normalized.is_empty() || spec.command.trim().is_empty() {
                continue;
            }
            let spec = normalize_agent_spec_for_name(&normalized, spec);
            merged.insert(normalized, spec.command_line());
        }
    }
    merged
}

/// 将代理名称解析为可执行命令行。
///
/// `agent_name` 会先按规范化名称查找，再按兼容别名查找。找不到匹配项时，
/// 返回原始输入，允许调用方显式传入自定义命令。该函数不会启动进程，也不会
/// 对命令进行权限或存在性检查。
pub fn resolve_agent_command(
    agent_name: &str,
    overrides: Option<&HashMap<String, AgentCommandSpec>>,
) -> String {
    let normalized = normalize_agent_name(agent_name);
    let registry = merge_agent_registry(overrides);
    registry
        .get(&normalized)
        .cloned()
        .or_else(|| alias_target(&normalized).and_then(|target| registry.get(target).cloned()))
        .unwrap_or_else(|| agent_name.to_string())
}

/// 从内置注册表解析代理的结构化规格。
///
/// 返回 `None` 表示名称和别名均未命中内置代理；调用方可据此决定是否使用
/// 自定义命令或提示用户。
pub fn resolve_agent_spec(agent_name: &str) -> Option<AgentCommandSpec> {
    let normalized = normalize_agent_name(agent_name);
    let registry = built_in_agent_specs();
    registry
        .get(&normalized)
        .cloned()
        .or_else(|| alias_target(&normalized).and_then(|target| registry.get(target).cloned()))
}

/// 从内置注册表和用户覆盖项中解析代理规格。
///
/// 用户覆盖项优先级高于内置规格。返回 `None` 表示没有任何可识别的规格，
/// 不代表命令不可执行。
pub fn resolve_agent_spec_with_overrides(
    agent_name: &str,
    overrides: Option<&HashMap<String, AgentCommandSpec>>,
) -> Option<AgentCommandSpec> {
    let normalized = normalize_agent_name(agent_name);
    let registry = merge_agent_specs(overrides);
    registry
        .get(&normalized)
        .cloned()
        .or_else(|| alias_target(&normalized).and_then(|target| registry.get(target).cloned()))
}

/// 列出当前可展示的代理名称。
///
/// 当传入 `overrides` 时，返回值包含用户覆盖或新增的代理 key；列表已按
/// `BTreeMap` 的顺序稳定排序。
pub fn list_built_in_agents(overrides: Option<&HashMap<String, AgentCommandSpec>>) -> Vec<String> {
    merge_agent_registry(overrides).into_keys().collect()
}

#[cfg(test)]
#[path = "agent_registry_tests.rs"]
mod agent_registry_tests;
