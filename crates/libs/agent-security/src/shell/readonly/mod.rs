//! shell 命令 readonly 约束检查。
//!
//! 该模块把解析后的命令映射到 readonly 白名单，检查命令名、子命令和参数标志是否仍在
//! 只读能力边界内。安全策略采用默认拒绝：不在白名单、未知子命令、未知标志、glob 或
//! 变量展开都会被视为非 readonly。

use crate::shell::ast::{ParsedCommand, strip_wrappers};
use crate::shell::sed::validate_sed_command;

mod allowlist;
mod flag_validator;

pub use allowlist::{COMMAND_ALLOWLIST, CommandAllowlistEntry};

/// readonly 检查的结构化结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadonlyCheckResult {
    /// 命令及参数全部落在 readonly 允许范围内。
    Readonly,
    /// 命令明确不是 readonly，并携带可展示或记录的原因。
    NotReadonly { reason: String },
    /// 命令存在未被白名单确认安全的标志。
    UnknownFlag { flag: String },
}

impl ReadonlyCheckResult {
    /// 判断检查结果是否为明确 readonly。
    ///
    /// 返回值：仅 `Readonly` 返回 `true`。
    /// 错误处理：该函数不返回错误。
    pub fn is_readonly(&self) -> bool {
        matches!(self, Self::Readonly)
    }
}

/// 检查解析后的 shell 命令是否满足 readonly 约束。
///
/// 参数：
/// - `cmd`：结构化 AST 或 fallback token 表示的 shell 命令。
///
/// 返回值：返回 [`ReadonlyCheckResult`]，包含只读判定或阻断原因。
/// 错误处理：该函数不返回 `Result`；不支持、不确定或危险路径会映射为
/// `NotReadonly` 或 `UnknownFlag`。
pub fn check_readonly_constraints(cmd: &ParsedCommand) -> ReadonlyCheckResult {
    let (name, args, has_glob, has_variable_expansion) = match cmd {
        ParsedCommand::Ast(_, info) => {
            let info = strip_wrappers(info);
            (
                base_command_name(&info.name).to_string(),
                info.args,
                info.has_glob,
                info.has_variable_expansion,
            )
        }
        ParsedCommand::Fallback { tokens, .. } => {
            let Some((name, args)) = tokens.split_first() else {
                return ReadonlyCheckResult::NotReadonly { reason: "empty command".into() };
            };
            (base_command_name(name).to_string(), args.to_vec(), false, false)
        }
    };

    if name == "sed" {
        // sed 既可读取也可原地改写文件，必须复用专用校验识别 `-i` 和危险脚本标志。
        return match validate_sed_command(cmd) {
            crate::shell::sed::SedValidationResult::Allowed { in_place, .. }
                if !in_place =>
            {
                ReadonlyCheckResult::Readonly
            }
            crate::shell::sed::SedValidationResult::Allowed { .. } => {
                ReadonlyCheckResult::NotReadonly {
                    reason: "sed in-place edit is not read-only".into(),
                }
            }
            crate::shell::sed::SedValidationResult::Blocked { reason } => {
                ReadonlyCheckResult::NotReadonly { reason }
            }
        };
    }

    let Some(entry) = allowlist::find_allowlist_entry(&name) else {
        return ReadonlyCheckResult::NotReadonly {
            reason: format!("command `{name}` is not in readonly allowlist"),
        };
    };

    let consumed = match allowlist::matched_subcommand_len(entry, &args) {
        Some(consumed) => consumed,
        None if entry.subcommands.is_some() => {
            return ReadonlyCheckResult::NotReadonly {
                reason: "subcommand is not marked read-only".into(),
            };
        }
        None => 0,
    };

    let mut after_double_dash = false;
    for arg in &args[consumed..] {
        if !after_double_dash && arg == "--" {
            after_double_dash = true;
            continue;
        }
        if after_double_dash || arg == "-" {
            continue;
        }
        if !arg.starts_with('-') && !arg.starts_with("+%") {
            continue;
        }

        // 不安全标志优先于未知标志处理，保证已知危险能力给出确定阻断原因。
        if flag_validator::is_unsafe_flag(arg, entry) {
            return ReadonlyCheckResult::NotReadonly { reason: format!("unsafe flag: {arg}") };
        }

        if !entry.allow_any_flag && !flag_validator::is_safe_flag(arg, entry) {
            return ReadonlyCheckResult::UnknownFlag { flag: arg.clone() };
        }
    }

    if name == "find" && has_glob && !has_variable_expansion {
        return ReadonlyCheckResult::Readonly;
    }

    if has_glob || has_variable_expansion {
        // 未引用的 glob/变量展开会把访问路径推迟到 shell 执行时决定，无法静态证明只读。
        return ReadonlyCheckResult::NotReadonly {
            reason: "unquoted glob or variable expansion".into(),
        };
    }

    ReadonlyCheckResult::Readonly
}

fn base_command_name(command: &str) -> &str {
    command.rsplit('/').next().unwrap_or(command)
}

#[cfg(test)]
#[path = "allowlist_tests.rs"]
mod allowlist_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
