//! Shell 命令信息提取入口。
//!
//! 本模块把解析后的 Bash AST 或原始命令字符串转换为 `CommandInfo`。启用
//! `shell-ast` 特性时优先走 tree-sitter AST；未启用或 AST 无法提取时使用保守的
//! 字符串扫描器，保证调用方仍能获得基础命令名、参数和重定向信息。

#[cfg(feature = "shell-ast")]
mod ast_walk;
mod scan;
mod wrappers;

use super::nodes::CommandInfo;
use super::parser::BashAst;

const MAX_SUBCOMMAND_DEPTH: usize = 8;

pub use wrappers::{WRAPPER_COMMANDS, strip_wrappers};

impl CommandInfo {
    #[cfg(feature = "shell-ast")]
    /// 从 Bash AST 中提取命令信息。
    ///
    /// # 参数
    ///
    /// - `ast`: 已解析的 Bash AST。
    ///
    /// # 返回值
    ///
    /// 成功时返回命令结构；当 AST 缺少可识别命令节点时回退解析原始命令文本。
    pub fn from_ast(ast: &BashAst) -> Option<Self> {
        ast_walk::from_ast(ast).or_else(|| Self::from_command(ast.source()))
    }

    #[cfg(not(feature = "shell-ast"))]
    /// 从 Bash AST 的原始文本中提取命令信息。
    ///
    /// # 参数
    ///
    /// - `ast`: 持有原始命令文本的 Bash AST 包装。
    ///
    /// # 返回值
    ///
    /// 成功时返回命令结构；当前构建未启用 AST 遍历，因此直接使用字符串扫描器。
    pub fn from_ast(ast: &BashAst) -> Option<Self> {
        Self::from_command(ast.source())
    }

    /// 从原始 shell 命令字符串中提取命令信息。
    ///
    /// # 参数
    ///
    /// - `command`: 待分析的 shell 命令。
    ///
    /// # 返回值
    ///
    /// 成功时返回命令名、参数、管道、重定向和扩展特征；空命令或无法解析时返回 `None`。
    pub fn from_command(command: &str) -> Option<Self> {
        scan::parse_command_info(command.trim(), 0)
    }
}
#[cfg(test)]
mod tests;
