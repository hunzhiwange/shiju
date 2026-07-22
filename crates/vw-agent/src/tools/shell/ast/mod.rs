//! Shell 命令解析公共入口。
//!
//! 本模块组合 Bash AST 解析、命令信息提取和 shell words 回退逻辑，为安全策略和工具
//! 调度提供统一的命令视图。解析失败时不会伪造完整 AST，而是返回显式 fallback 结构。

pub mod command_info;
pub mod nodes;
pub mod parser;

pub use command_info::{WRAPPER_COMMANDS, strip_wrappers};
pub use nodes::{CommandInfo, CompoundOp, PipeSegment, Redirect, RedirectKind};
pub use parser::{BashAst, BashNode, ParseQuality};

#[derive(Debug, Clone, PartialEq, Eq)]
/// Shell 命令解析结果。
///
/// `Ast` 表示命令已成功解析并带有结构化命令信息；`Fallback` 表示仅能提供原始字符串
/// 与按 shell words 拆分的 token。
pub enum ParsedCommand {
    /// 基于 Bash AST 的结构化解析结果。
    Ast(BashAst, CommandInfo),
    /// 解析器无法提供高质量 AST 时的保守回退结果。
    Fallback { raw: String, tokens: Vec<String> },
}

/// 解析一条 shell 命令。
///
/// # 参数
///
/// - `command`: 待解析的 shell 命令文本。
///
/// # 返回值
///
/// 成功提取结构化信息时返回 `ParsedCommand::Ast`；解析质量不足或无法提取命令信息时
/// 返回 `ParsedCommand::Fallback`。
pub fn parse_command(command: &str) -> ParsedCommand {
    let (ast, quality) = BashAst::parse(command);
    if quality == ParseQuality::Fallback {
        return fallback_command(command);
    }

    match CommandInfo::from_ast(&ast) {
        Some(info) => ParsedCommand::Ast(ast, info),
        None => fallback_command(command),
    }
}

impl ParsedCommand {
    /// 返回原始命令文本。
    ///
    /// # 返回值
    ///
    /// AST 结果返回 AST 保存的 source；fallback 结果返回原始输入。
    pub fn raw(&self) -> &str {
        match self {
            Self::Ast(ast, _) => ast.source(),
            Self::Fallback { raw, .. } => raw,
        }
    }

    /// 返回结构化命令信息。
    ///
    /// # 返回值
    ///
    /// 仅 `Ast` 结果包含 `CommandInfo`；fallback 结果返回 `None`。
    pub fn info(&self) -> Option<&CommandInfo> {
        match self {
            Self::Ast(_, info) => Some(info),
            Self::Fallback { .. } => None,
        }
    }

    /// 返回命令名。
    ///
    /// # 返回值
    ///
    /// AST 结果使用结构化命令名；fallback 结果使用第一个 token。
    pub fn command_name(&self) -> Option<&str> {
        match self {
            Self::Ast(_, info) => Some(info.name.as_str()),
            Self::Fallback { tokens, .. } => tokens.first().map(String::as_str),
        }
    }

    /// 返回命令参数列表。
    ///
    /// # 返回值
    ///
    /// AST 结果返回结构化参数；fallback 结果返回第一个 token 之后的切片。
    pub fn args(&self) -> &[String] {
        match self {
            Self::Ast(_, info) => info.args.as_slice(),
            Self::Fallback { tokens, .. } => tokens.get(1..).unwrap_or(&[]),
        }
    }
}

fn fallback_command(command: &str) -> ParsedCommand {
    let tokens = shell_words::split(command).unwrap_or_default();
    // fallback 会降低结构化程度，记录 debug 信息便于排查解析器覆盖缺口。
    tracing::debug!(
        command = command,
        token_count = tokens.len(),
        "shell command parsing fell back"
    );
    ParsedCommand::Fallback { raw: command.to_string(), tokens }
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod parser_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "nodes_tests.rs"]
mod nodes_tests;

#[cfg(test)]
#[path = "command_info_tests.rs"]
mod command_info_tests;
