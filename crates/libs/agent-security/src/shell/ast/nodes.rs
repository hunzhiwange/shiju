//! Shell AST 解析产物的数据结构。
//!
//! 本模块只定义命令信息、重定向、管道和复合操作符等纯数据类型。它们被序列化后可供
//! 安全策略、工具输出和测试断言共享，不包含解析或执行逻辑。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// 一条 shell 命令的结构化信息。
///
/// 该结构同时描述主命令、参数、重定向、管道段和嵌套子命令，并记录 glob、变量展开
/// 等会影响实际执行范围的语法特征。
pub struct CommandInfo {
    /// 命令名或解析到的可执行入口。
    pub name: String,
    /// 命令参数，保持解析后的顺序。
    pub args: Vec<String>,
    /// 当前命令及可汇总子结构中的重定向信息。
    pub redirects: Vec<Redirect>,
    /// 管道中的各段命令信息。
    pub pipes: Vec<PipeSegment>,
    /// 命令替换或进程替换中发现的嵌套命令。
    pub subcommands: Vec<CommandInfo>,
    /// 是否包含 `$()` 或反引号命令替换。
    pub has_command_substitution: bool,
    /// 是否包含 `<(...)` 或 `>(...)` 进程替换。
    pub has_process_substitution: bool,
    /// 是否包含可能扩展路径集合的 glob 模式。
    pub has_glob: bool,
    /// 是否包含 shell 变量展开。
    pub has_variable_expansion: bool,
    /// 顶层复合操作符；普通简单命令为 `None`。
    pub compound_operator: Option<CompoundOp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Shell 重定向信息。
///
/// 记录重定向类型、目标以及是否为文件描述符复制。
pub struct Redirect {
    /// 重定向类型。
    pub kind: RedirectKind,
    /// 重定向目标，可能是路径、描述符或 heredoc 标记。
    pub target: String,
    /// 是否为 `2>&1`、`<&0` 等文件描述符复制形式。
    pub is_fd_duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// 管道中的单个命令段。
pub struct PipeSegment {
    /// 该管道段的命令信息。
    pub info: CommandInfo,
    /// 该段在管道中的 0 基位置。
    pub position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// 顶层复合 shell 操作符。
pub enum CompoundOp {
    /// `&&` 条件执行。
    And,
    /// `||` 条件执行。
    Or,
    /// `;` 顺序执行。
    Sequence,
    /// `|` 管道执行。
    Pipe,
    /// `( ... )` 子 shell。
    Subshell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Shell 重定向类型。
pub enum RedirectKind {
    /// 标准输入重定向。
    Stdin,
    /// 标准输出重定向。
    Stdout,
    /// 标准错误重定向。
    Stderr,
    /// 标准输出追加。
    Append,
    /// 标准输出与标准错误合并重定向。
    StdoutAndStderr,
    /// 标准错误追加。
    StderrAppend,
    /// heredoc 或 here-string。
    Heredoc,
}
