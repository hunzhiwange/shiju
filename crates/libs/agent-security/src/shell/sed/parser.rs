//! sed 原地替换编辑解析器。
//!
//! 该模块把已通过安全校验的 `sed -i 's/.../.../flags' file` 命令转换为结构化编辑，
//! 供文件编辑路径复用。解析器不放宽 validation 的限制，只在允许的 substitute
//! 命令上提取目标文件、模式、替换文本和标志。

use std::path::PathBuf;

use regex::Regex;
use thiserror::Error;

use crate::shell::ast::ParsedCommand;

use super::validation::{
    SedCommandKind, SedInvocation, SedValidationResult, parse_substitute_parts,
    validate_sed_command,
};

/// 结构化 sed 替换编辑。
///
/// 该类型只表示单文件原地 substitute 操作，避免把任意 sed 脚本带入文件修改路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SedEdit {
    /// 被修改的文件路径。
    pub file: PathBuf,
    /// substitute 的正则模式。
    pub pattern: String,
    /// substitute 的替换文本。
    pub replacement: String,
    /// substitute 的标志字符串，例如 `g`。
    pub flags: String,
    /// 是否启用扩展正则。
    pub extended_regex: bool,
}

/// sed 编辑解析错误。
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SedParseError {
    /// 命令不是原地 substitute 编辑。
    #[error("sed command is not an in-place substitute edit")]
    NotInPlaceSubstitute,
    /// 安全校验阶段阻断了该 sed 命令。
    #[error("sed validation failed: {0}")]
    Validation(String),
    /// 原地编辑必须只有一个目标文件。
    #[error("sed edit must target exactly one file")]
    InvalidTargetCount,
    /// substitute 表达式无法拆分为 pattern/replacement/flags。
    #[error("sed substitute expression is malformed")]
    MalformedSubstitute,
    /// substitute pattern 无法编译为 Rust regex。
    #[error("failed to compile regex: {0}")]
    InvalidRegex(String),
}

impl SedEdit {
    /// 从解析后的 shell 命令中提取 sed 原地替换编辑。
    ///
    /// 参数：
    /// - `cmd`：解析后的 shell 命令。
    ///
    /// 返回值：成功时返回单文件 substitute 编辑。
    /// 错误处理：非 sed、非原地替换、目标数量错误、脚本不安全或表达式 malformed
    /// 都会返回 [`SedParseError`]。
    pub fn parse(cmd: &ParsedCommand) -> Result<Self, SedParseError> {
        let validation = validate_sed_command(cmd);
        let SedValidationResult::Allowed { kind, in_place, files, extended_regex } = validation
        else {
            let SedValidationResult::Blocked { reason } = validation else { unreachable!() };
            if parse_invocation(cmd)
                .is_some_and(|invocation| invocation.in_place && invocation.script.starts_with('s'))
            {
                return Err(SedParseError::MalformedSubstitute);
            }
            return Err(SedParseError::Validation(reason));
        };

        if !in_place {
            return Err(SedParseError::NotInPlaceSubstitute);
        }
        let SedCommandKind::Substitute { script } = kind else {
            return Err(SedParseError::NotInPlaceSubstitute);
        };
        if files.len() != 1 {
            return Err(SedParseError::InvalidTargetCount);
        }

        let (pattern, replacement, flags) =
            parse_substitute_parts(&script).ok_or(SedParseError::MalformedSubstitute)?;

        Ok(Self { file: PathBuf::from(&files[0]), pattern, replacement, flags, extended_regex })
    }

    /// 将 sed 编辑应用到给定文本内容。
    ///
    /// 参数：
    /// - `content`：待替换的原始文本。
    ///
    /// 返回值：替换后的文本。
    /// 错误处理：正则编译失败会返回 [`SedParseError::InvalidRegex`]。
    pub fn apply_to_content(&self, content: &str) -> Result<String, SedParseError> {
        let regex = Regex::new(&self.pattern)
            .map_err(|error| SedParseError::InvalidRegex(error.to_string()))?;
        let replace_all = self.flags.contains('g');

        let mut output = String::new();
        for chunk in content.split_inclusive('\n') {
            let replaced = if replace_all {
                regex.replace_all(chunk, self.replacement.as_str()).to_string()
            } else {
                regex.replace(chunk, self.replacement.as_str()).to_string()
            };
            output.push_str(&replaced);
        }

        if !content.ends_with('\n') && output.ends_with('\n') && !replace_all {
            output.pop();
        }

        Ok(output)
    }
}

/// 暴露给测试和相邻模块的 sed invocation 解析入口。
pub(crate) fn parse_invocation(cmd: &ParsedCommand) -> Option<SedInvocation> {
    SedInvocation::from_command(cmd)
}
