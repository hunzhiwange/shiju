//! 复合命令退出语义模型，负责区分 grep、find、diff 等命令的非零退出码含义。

use crate::shell::ast::ParsedCommand;

/// ExitSemantics 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitSemantics {
    Default,
    Grep,
    Find,
    Diff,
    Test,
}

impl ExitSemantics {
    /// 执行 for_command 操作，并返回调用方需要的结果。
    pub fn for_command(name: &str) -> Self {
        match name {
            "grep" | "rg" | "ag" | "ack" => Self::Grep,
            "find" => Self::Find,
            "diff" => Self::Diff,
            "test" | "[" | "[[" => Self::Test,
            _ => Self::Default,
        }
    }

    /// 执行 for_parsed_command 操作，并返回调用方需要的结果。
    pub fn for_parsed_command(cmd: &ParsedCommand) -> Self {
        Self::for_command(cmd.command_name().unwrap_or_default())
    }

    /// 执行 interpret 操作，并返回调用方需要的结果。
    pub fn interpret(&self, exit_code: Option<i32>) -> ExitInterpretation {
        let Some(exit_code) = exit_code else {
            return ExitInterpretation::Error {
                message: "Command terminated without an exit code".into(),
            };
        };

        match self {
            Self::Default => match exit_code {
                0 => ExitInterpretation::Success,
                _ => ExitInterpretation::Error {
                    message: format!("Command exited with code {exit_code}"),
                },
            },
            Self::Grep => match exit_code {
                0 => ExitInterpretation::Success,
                1 => ExitInterpretation::NoMatches,
                2 => ExitInterpretation::Error { message: "grep encountered an error".into() },
                _ => ExitInterpretation::Error {
                    message: format!("Unexpected grep exit code {exit_code}"),
                },
            },
            Self::Find => match exit_code {
                0 => ExitInterpretation::Success,
                1 => ExitInterpretation::PartialSuccess,
                _ => ExitInterpretation::Error {
                    message: format!("find exited with code {exit_code}"),
                },
            },
            Self::Diff => match exit_code {
                0 => ExitInterpretation::Success,
                1 => ExitInterpretation::DifferencesFound,
                _ => ExitInterpretation::Error {
                    message: format!("diff exited with code {exit_code}"),
                },
            },
            Self::Test => match exit_code {
                0 => ExitInterpretation::ConditionTrue,
                1 => ExitInterpretation::ConditionFalse,
                _ => ExitInterpretation::Error {
                    message: format!("test exited with code {exit_code}"),
                },
            },
        }
    }
}

/// ExitInterpretation 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitInterpretation {
    Success,
    NoMatches,
    PartialSuccess,
    DifferencesFound,
    ConditionTrue,
    ConditionFalse,
    Error { message: String },
}

impl ExitInterpretation {
    /// 执行 is_error_for_llm 操作，并返回调用方需要的结果。
    pub fn is_error_for_llm(&self) -> bool {
        matches!(self, Self::Error { .. })
    }
}

#[cfg(test)]
#[path = "semantics_tests.rs"]
mod semantics_tests;
