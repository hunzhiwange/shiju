//! shell 命令沙箱决策入口。
//!
//! 该模块判断某条命令是否应该进入平台沙箱，并导出沙箱执行器与策略类型。决策层只考虑
//! 配置、后端可用性和命令排除规则；具体平台包装在 `executor` 中完成。

pub mod executor;
pub mod policy;

pub use executor::SandboxExecutor;
pub use policy::{FilesystemPolicy, NetworkPolicy, SandboxConfig};

use crate::tools::shell::ast::{ParsedCommand, WRAPPER_COMMANDS, strip_wrappers};

/// shell 沙箱启用决策。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDecision {
    /// 是否使用沙箱执行。
    pub use_sandbox: bool,
    /// 做出该决策的原因。
    pub reason: SandboxReason,
}

/// 沙箱启用或禁用原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxReason {
    /// 配置和后端均允许，默认启用沙箱。
    EnabledByDefault,
    /// 当前上下文允许覆盖，并显式关闭了沙箱。
    DisabledByOverride,
    /// 当前命令命中了配置中的排除列表。
    DisabledForCommand,
    /// 沙箱配置全局关闭。
    DisabledGlobally,
    /// 当前平台后端不可用。
    BackendUnavailable,
}

/// 判断命令是否应该使用 shell 沙箱。
///
/// 参数：
/// - `cmd`：解析后的 shell 命令。
/// - `config`：沙箱配置。
///
/// 返回值：包含布尔决策和原因的 [`SandboxDecision`]。
/// 错误处理：该函数不返回错误；不可用后端、禁用配置和排除命令都通过原因枚举表达。
pub fn should_use_sandbox(cmd: &ParsedCommand, config: &SandboxConfig) -> SandboxDecision {
    if !config.enabled {
        return SandboxDecision { use_sandbox: false, reason: SandboxReason::DisabledGlobally };
    }

    if config.allow_override && config.override_enabled {
        return SandboxDecision { use_sandbox: false, reason: SandboxReason::DisabledByOverride };
    }

    if !executor::SandboxExecutor::backend_available() {
        return SandboxDecision { use_sandbox: false, reason: SandboxReason::BackendUnavailable };
    }

    if stripped_command_name(cmd).is_some_and(|name| {
        config.excluded_commands.iter().any(|excluded| excluded.eq_ignore_ascii_case(&name))
    }) {
        return SandboxDecision { use_sandbox: false, reason: SandboxReason::DisabledForCommand };
    }

    SandboxDecision { use_sandbox: true, reason: SandboxReason::EnabledByDefault }
}

fn stripped_command_name(cmd: &ParsedCommand) -> Option<String> {
    match cmd {
        ParsedCommand::Ast(_, info) => Some(strip_wrappers(info).name),
        ParsedCommand::Fallback { tokens, .. } => strip_wrapper_tokens(tokens.to_vec()).pop_first(),
    }
}

trait PopFirst {
    fn pop_first(self) -> Option<String>;
}

impl PopFirst for Vec<String> {
    fn pop_first(self) -> Option<String> {
        self.into_iter().next()
    }
}

fn strip_wrapper_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut current = tokens;

    loop {
        let Some(name) = current.first().map(|value| value.as_str()) else {
            return current;
        };
        if !WRAPPER_COMMANDS.contains(&name) {
            return current;
        }

        let next = match name {
            // 排除命令要针对真实业务命令生效，因此需要剥离 env/timeout/nice/shell 等包装器。
            "env" => strip_env_tokens(&current),
            "timeout" => strip_timeout_tokens(&current),
            "nice" => strip_nice_tokens(&current),
            "bash" | "sh" | "zsh" | "dash" => strip_shell_tokens(&current),
            _ => strip_generic_tokens(&current),
        };

        if next.is_empty() {
            return current;
        }
        current = next;
    }
}

fn strip_env_tokens(tokens: &[String]) -> Vec<String> {
    let mut index = 1;
    while index < tokens.len() {
        let arg = tokens[index].as_str();
        if arg == "-u" {
            index += 2;
            continue;
        }
        if arg.starts_with("-u") && arg.len() > 2 {
            index += 1;
            continue;
        }
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') || is_env_assignment(arg) {
            index += 1;
            continue;
        }
        break;
    }
    tokens.get(index..).unwrap_or(&[]).to_vec()
}

fn strip_timeout_tokens(tokens: &[String]) -> Vec<String> {
    let mut index = 1;
    while index < tokens.len() {
        let arg = tokens[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-s" || arg == "-k" || arg == "--signal" || arg == "--kill-after" {
            index += 2;
            continue;
        }
        if arg.starts_with("--signal=")
            || arg.starts_with("--kill-after=")
            || arg == "--foreground"
            || arg == "--preserve-status"
            || arg.starts_with('-')
        {
            index += 1;
            continue;
        }
        index += 1;
        break;
    }
    tokens.get(index..).unwrap_or(&[]).to_vec()
}

fn strip_nice_tokens(tokens: &[String]) -> Vec<String> {
    let mut index = 1;
    while index < tokens.len() {
        let arg = tokens[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-n" || arg == "--adjustment" {
            index += 2;
            continue;
        }
        if arg.starts_with("--adjustment=") || is_nice_priority(arg) || arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    tokens.get(index..).unwrap_or(&[]).to_vec()
}

fn strip_shell_tokens(tokens: &[String]) -> Vec<String> {
    let mut index = 1;
    while index < tokens.len() {
        let arg = tokens[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-c" || arg == "-lc" {
            let Some(command) = tokens.get(index + 1) else {
                return Vec::new();
            };
            let Ok(nested) = shell_words::split(command) else {
                return Vec::new();
            };
            return strip_wrapper_tokens(nested);
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    tokens.get(index..).unwrap_or(&[]).to_vec()
}

fn strip_generic_tokens(tokens: &[String]) -> Vec<String> {
    let mut index = 1;
    while index < tokens.len() {
        let arg = tokens[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    tokens.get(index..).unwrap_or(&[]).to_vec()
}

fn is_env_assignment(value: &str) -> bool {
    let Some((name, _)) = value.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().next().is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && name.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_nice_priority(value: &str) -> bool {
    value.len() > 1 && value.starts_with('-') && value[1..].chars().all(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod sandbox_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
