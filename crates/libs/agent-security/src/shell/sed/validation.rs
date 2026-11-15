//! sed 命令安全校验。
//!
//! 该模块只允许可审阅的 print 与 substitute 命令，并显式阻断 sed 的写文件、执行命令、
//! 外部脚本文件和复杂控制语法。sed 功能强大但语义紧凑，因此这里采用小白名单而不是
//! 尝试支持完整 sed 语言。

use crate::shell::ast::{ParsedCommand, strip_wrappers};

/// 允许的 sed 命令类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SedCommandKind {
    /// 只打印匹配范围的脚本。
    Print { script: String },
    /// substitute 脚本。
    Substitute { script: String },
}

/// sed 安全校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SedValidationResult {
    /// 命令落在允许范围内。
    Allowed { kind: SedCommandKind, in_place: bool, files: Vec<String>, extended_regex: bool },
    /// 命令被阻断，并携带原因。
    Blocked { reason: String },
}

/// 校验 sed 命令是否处于允许的安全子集。
///
/// 参数：
/// - `cmd`：解析后的 shell 命令。
///
/// 返回值：允许时返回命令类型、是否原地编辑、文件列表和正则模式；阻断时返回原因。
/// 错误处理：该函数不返回 `Result`；所有不支持和危险路径都映射为 `Blocked`。
pub fn validate_sed_command(cmd: &ParsedCommand) -> SedValidationResult {
    let Some(invocation) = SedInvocation::from_command(cmd) else {
        return SedValidationResult::Blocked { reason: "not a sed command".into() };
    };

    if !invocation.script.is_ascii() {
        return SedValidationResult::Blocked { reason: "sed script must be ASCII".into() };
    }
    if invocation.script.contains('{')
        || invocation.script.contains('#')
        || invocation.script.contains('!')
        || invocation.script.contains('~')
        || invocation.script.contains("\\\n")
    {
        // 这些语法能引入分支、否定、注释或续行，静态审阅成本高，因此不进入安全子集。
        return SedValidationResult::Blocked { reason: "sed script uses blocked syntax".into() };
    }

    if looks_like_print(&invocation.script) {
        return SedValidationResult::Allowed {
            kind: SedCommandKind::Print { script: invocation.script },
            in_place: invocation.in_place,
            files: invocation.files,
            extended_regex: invocation.extended_regex,
        };
    }

    if let Some((_, _, flags)) = parse_substitute_parts(&invocation.script) {
        if flags.contains('w') || flags.contains('W') || flags.contains('e') || flags.contains('E')
        {
            return SedValidationResult::Blocked {
                reason: "sed substitute uses blocked write/exec flags".into(),
            };
        }
        return SedValidationResult::Allowed {
            kind: SedCommandKind::Substitute { script: invocation.script },
            in_place: invocation.in_place,
            files: invocation.files,
            extended_regex: invocation.extended_regex,
        };
    }

    SedValidationResult::Blocked {
        reason: "only sed print and substitute commands are allowed".into(),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SedInvocation {
    pub script: String,
    pub files: Vec<String>,
    pub in_place: bool,
    pub extended_regex: bool,
}

impl SedInvocation {
    /// 从 shell 命令中解析 sed 调用的核心字段。
    ///
    /// 返回值：非 sed 命令、外部脚本文件或缺失脚本时返回 `None`。
    pub(crate) fn from_command(cmd: &ParsedCommand) -> Option<Self> {
        let (name, args) = match cmd {
            ParsedCommand::Ast(_, info) => {
                let info = strip_wrappers(info);
                (info.name, info.args)
            }
            ParsedCommand::Fallback { tokens, .. } => {
                let (name, args) = tokens.split_first()?;
                (name.clone(), args.to_vec())
            }
        };

        if name.rsplit('/').next()? != "sed" {
            return None;
        }

        let mut script = None;
        let mut files = Vec::new();
        let mut in_place = false;
        let mut extended_regex = false;
        let mut index = 0usize;

        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "-n" => index += 1,
                "-E" | "-r" => {
                    extended_regex = true;
                    index += 1;
                }
                "-i" => {
                    in_place = true;
                    if args.get(index + 1).is_some_and(|value| value.is_empty()) {
                        index += 2;
                    } else {
                        index += 1;
                    }
                }
                "-e" => {
                    script = args.get(index + 1).cloned();
                    index += 2;
                }
                "-f" => return None,
                _ if arg.starts_with("-i") => {
                    in_place = true;
                    index += 1;
                }
                _ if arg.starts_with('-') && script.is_none() => index += 1,
                _ => {
                    if script.is_none() {
                        script = Some(arg.to_string());
                    } else {
                        files.push(arg.to_string());
                    }
                    index += 1;
                }
            }
        }

        Some(Self { script: script?, files, in_place, extended_regex })
    }
}

fn looks_like_print(script: &str) -> bool {
    let trimmed = script.trim();
    trimmed.ends_with('p')
        && trimmed
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, ',' | ';' | '$' | 'p' | ' ' | '\t'))
}

pub(crate) fn parse_substitute_parts(script: &str) -> Option<(String, String, String)> {
    let mut chars = script.chars();
    if chars.next()? != 's' {
        return None;
    }
    let delimiter = chars.next()?;
    let rest: String = chars.collect();
    let (pattern, rest) = take_until_delimiter(&rest, delimiter)?;
    let (replacement, flags) = take_until_delimiter(rest, delimiter)?;
    Some((pattern, replacement, flags.to_string()))
}

fn take_until_delimiter(input: &str, delimiter: char) -> Option<(String, &str)> {
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == delimiter {
            return Some((input[..index].to_string(), &input[index + ch.len_utf8()..]));
        }
    }
    None
}
