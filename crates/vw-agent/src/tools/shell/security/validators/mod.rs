//! shell 安全 validator 注册与共享辅助逻辑。
//!
//! 每个 validator 聚焦一种危险模式，本模块负责按固定顺序构建流水线，并提供创建
//! finding、读取命令文本、检查 shell 边界等小工具。

mod backslash;
mod brace_expansion;
mod comment_quote_desync;
mod control_chars;
mod dangerous_vars;
mod empty_command;
mod git_commit;
mod hash_comment;
mod heredoc;
mod ifs_injection;
mod jq;
mod malformed;
mod metacharacters;
mod newline_injection;
mod obfuscated_flags;
mod proc_environ;
mod quoted_newline;
mod redirection;
mod substitution;
mod unicode_whitespace;
mod zsh_dangerous;

#[cfg(test)]
#[path = "metacharacters_tests.rs"]
mod metacharacters_tests;
#[cfg(test)]
#[path = "newline_injection_tests.rs"]
mod newline_injection_tests;
#[cfg(test)]
#[path = "obfuscated_flags_tests.rs"]
mod obfuscated_flags_tests;
#[cfg(test)]
#[path = "proc_environ_tests.rs"]
mod proc_environ_tests;
#[cfg(test)]
#[path = "quoted_newline_tests.rs"]
mod quoted_newline_tests;
#[cfg(test)]
#[path = "redirection_tests.rs"]
mod redirection_tests;
#[cfg(test)]
#[path = "substitution_tests.rs"]
mod substitution_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "unicode_whitespace_tests.rs"]
mod unicode_whitespace_tests;
#[cfg(test)]
#[path = "zsh_dangerous_tests.rs"]
mod zsh_dangerous_tests;

use super::{SecurityCategory, SecurityFinding, SecurityValidator, Severity};
use crate::tools::shell::ast::{CommandInfo, ParsedCommand, RedirectKind};
use regex::Regex;
use std::sync::LazyLock;
use vw_config_types::security::BashValidatorChecksConfig;

/// 构建默认 shell 安全校验器集合。
///
/// 参数：
/// - `strict`：是否把命令替换、进程替换等能力视为阻断级风险。
///
/// 返回值：按执行顺序排列的 validator 列表。
/// 错误处理：该函数不返回错误；正则初始化失败会在开发期通过 `expect` 暴露。
pub(super) fn build_validators(
    strict: bool,
    config: &BashValidatorChecksConfig,
) -> Vec<Box<dyn SecurityValidator>> {
    if !config.enabled {
        return Vec::new();
    }

    let mut validators: Vec<Box<dyn SecurityValidator>> = Vec::new();
    macro_rules! push_if {
        ($enabled:expr, $validator:expr) => {
            if $enabled {
                validators.push(Box::new($validator));
            }
        };
    }

    push_if!(config.empty_command, empty_command::EmptyCommandValidator);
    push_if!(config.heredoc, heredoc::HeredocValidator);
    push_if!(config.git_commit, git_commit::GitCommitValidator);
    push_if!(config.jq, jq::JqValidator);
    push_if!(config.metacharacters, metacharacters::MetacharactersValidator);
    push_if!(config.dangerous_vars, dangerous_vars::DangerousVarsValidator);
    push_if!(config.substitution, substitution::SubstitutionValidator::new(strict));
    push_if!(config.redirection, redirection::RedirectionValidator);
    push_if!(config.newline_injection, newline_injection::NewlineInjectionValidator);
    push_if!(config.ifs_injection, ifs_injection::IfsInjectionValidator);
    push_if!(config.proc_environ, proc_environ::ProcEnvironValidator);
    push_if!(config.malformed, malformed::MalformedValidator);
    push_if!(config.obfuscated_flags, obfuscated_flags::ObfuscatedFlagsValidator);
    push_if!(config.backslash, backslash::BackslashValidator);
    push_if!(config.brace_expansion, brace_expansion::BraceExpansionValidator);
    push_if!(config.unicode_whitespace, unicode_whitespace::UnicodeWhitespaceValidator);
    push_if!(config.hash_comment, hash_comment::HashCommentValidator);
    push_if!(config.comment_quote_desync, comment_quote_desync::CommentQuoteDesyncValidator);
    push_if!(config.quoted_newline, quoted_newline::QuotedNewlineValidator);
    push_if!(config.zsh_dangerous, zsh_dangerous::ZshDangerousValidator);
    push_if!(config.control_chars, control_chars::ControlCharsValidator);
    validators
}

pub(super) fn block(
    category: SecurityCategory,
    message: impl Into<String>,
    suggestion: Option<&str>,
) -> SecurityFinding {
    // 安全流水线通过 finding 传递阻断原因，避免在多个 validator 中混用异常控制流。
    SecurityFinding {
        severity: Severity::Block,
        category,
        message: message.into(),
        suggestion: suggestion.map(ToOwned::to_owned),
    }
}

pub(super) fn warn(
    category: SecurityCategory,
    message: impl Into<String>,
    suggestion: Option<&str>,
) -> SecurityFinding {
    SecurityFinding {
        severity: Severity::Warn,
        category,
        message: message.into(),
        suggestion: suggestion.map(ToOwned::to_owned),
    }
}

pub(super) fn raw(cmd: &ParsedCommand) -> &str {
    cmd.raw()
}

/// 取得结构化命令信息；fallback 解析时返回 `None`。
pub(super) fn info(cmd: &ParsedCommand) -> Option<&CommandInfo> {
    cmd.info()
}

/// 返回小写化后的命令名与参数 token。
pub(super) fn lower_tokens(cmd: &ParsedCommand) -> Vec<String> {
    match cmd {
        ParsedCommand::Ast(_, info) => std::iter::once(info.name.as_str())
            .chain(info.args.iter().map(String::as_str))
            .map(|token| token.to_ascii_lowercase())
            .collect(),
        ParsedCommand::Fallback { tokens, .. } => {
            tokens.iter().map(|token| token.to_ascii_lowercase()).collect()
        }
    }
}

/// 粗略判断 shell 引号、反引号或括号是否未闭合。
pub(super) fn looks_unbalanced_shell(command: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let mut backtick = false;
    let mut paren_depth = 0usize;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            chars.next();
            continue;
        }
        if single {
            if ch == '\'' {
                single = false;
            }
            continue;
        }
        if double {
            if ch == '"' {
                double = false;
            }
            continue;
        }
        if backtick {
            if ch == '`' {
                backtick = false;
            }
            continue;
        }

        match ch {
            '\'' => single = true,
            '"' => double = true,
            '`' => backtick = true,
            '(' => paren_depth += 1,
            ')' => {
                if paren_depth == 0 {
                    return true;
                }
                paren_depth -= 1;
            }
            _ => {}
        }
    }

    single || double || backtick || paren_depth > 0
}

pub(super) fn has_unquoted_hash(command: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;

    for (idx, ch) in command.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '#' if !single && !double && idx > 0 => {
                let prev = command[..idx].chars().last().unwrap_or(' ');
                // mid-word `#` 会让审阅者和 shell 对命令边界产生不同理解。
                if !prev.is_whitespace() {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

pub(super) fn has_quoted_newline(command: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '\n' | '\r' if single || double => return true,
            _ => {}
        }
    }

    false
}

pub(super) fn has_unquoted_newline(command: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;

    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if !single => escaped = true,
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '\n' | '\r' if !single && !double => return true,
            _ => {}
        }
    }

    false
}

pub(super) fn has_control_characters(command: &str) -> bool {
    command
        .chars()
        .any(|ch| (ch.is_control() && ch != '\n' && ch != '\t' && ch != '\r') || ch == '\u{7f}')
}

pub(super) fn contains_unicode_whitespace(command: &str) -> bool {
    command.chars().any(|ch| {
        matches!(
            ch,
            '\u{00A0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
        )
    })
}

pub(super) fn redirect_targets(cmd: &ParsedCommand) -> Vec<(RedirectKind, String, bool)> {
    info(cmd)
        .map(|info| {
            info.redirects
                .iter()
                .map(|redirect| (redirect.kind, redirect.target.clone(), redirect.is_fd_duplicate))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn contains_regex(regex: &LazyLock<Regex>, command: &str) -> bool {
    regex.is_match(command)
}
