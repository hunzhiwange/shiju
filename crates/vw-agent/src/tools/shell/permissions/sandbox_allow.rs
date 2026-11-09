//! 沙箱自动放行策略，负责在受限沙箱中识别安全的只读命令。

use crate::tools::shell::ast::ParsedCommand;
use crate::tools::shell::readonly::check_readonly_constraints;

/// SandboxAutoAllow 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxAutoAllow {
    auto_allow_in_sandbox: Vec<&'static str>,
    excluded_commands: Vec<String>,
}

impl Default for SandboxAutoAllow {
    fn default() -> Self {
        Self {
            auto_allow_in_sandbox: vec![
                "ls", "find", "cat", "head", "tail", "wc", "sort", "uniq", "grep", "rg", "ag",
                "ack", "echo", "printf", "date", "whoami", "pwd", "test", "[", "[[",
            ],
            excluded_commands: Vec::new(),
        }
    }
}

impl SandboxAutoAllow {
    /// 执行 with_excluded_commands 操作，并返回调用方需要的结果。
    pub fn with_excluded_commands(excluded_commands: Vec<String>) -> Self {
        Self { excluded_commands, ..Self::default() }
    }

    /// 执行 should_auto_allow 操作，并返回调用方需要的结果。
    pub fn should_auto_allow(&self, cmd: &ParsedCommand, in_sandbox: bool) -> bool {
        if !in_sandbox {
            return false;
        }

        let Some(name) = command_name(cmd) else {
            return false;
        };

        if self.excluded_commands.iter().any(|excluded| excluded == name) {
            return false;
        }

        if self.auto_allow_in_sandbox.iter().any(|allowed| allowed == &name) {
            return true;
        }

        check_readonly_constraints(cmd).is_readonly() || matches_interpreter_expression(cmd)
    }
}

fn command_name(cmd: &ParsedCommand) -> Option<&str> {
    match cmd {
        ParsedCommand::Ast(_, info) => Some(info.name.as_str()),
        ParsedCommand::Fallback { tokens, .. } => tokens.first().map(String::as_str),
    }
}

fn command_args(cmd: &ParsedCommand) -> Vec<&str> {
    match cmd {
        ParsedCommand::Ast(_, info) => info.args.iter().map(String::as_str).collect(),
        ParsedCommand::Fallback { tokens, .. } => {
            tokens.iter().skip(1).map(String::as_str).collect()
        }
    }
}

fn matches_interpreter_expression(cmd: &ParsedCommand) -> bool {
    match command_name(cmd) {
        Some("python3" | "python") => {
            matches!(command_args(cmd).as_slice(), ["-c", ..] | ["-I", "-c", ..])
        }
        Some("node" | "ruby") => matches!(command_args(cmd).as_slice(), ["-e", ..]),
        _ => false,
    }
}
