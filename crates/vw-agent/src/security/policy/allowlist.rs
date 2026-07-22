//! Shell 命令 allowlist 辅助逻辑。
//!
//! 本模块只处理旧 allowlist 语义中的命令名/路径匹配和参数路径筛查。它不直接
//! 做最终授权决策，而是为 `SecurityPolicy` 和权限规则引擎提供局部判断能力。

use super::path_utils::{expand_user_path, looks_like_path};
use super::shell_lexer::{skip_env_assignments, split_unquoted_segments, strip_wrapping_quotes};
use super::shell_redirect::{attached_short_option_value, redirection_target};

enum RedirectionPathCheck {
    NotRedirection,
    Allowed,
    Blocked(String),
}

enum TokenPathCheck {
    Ignored,
    Blocked(String),
}

/// 判断 allowlist 条目是否匹配当前可执行文件。
///
/// `allowed` 是配置中的允许项，可以是 `*`、命令 basename 或路径；`executable`
/// 是命令行中解析出的可执行文件；`executable_base` 是其 basename。返回 `true`
/// 表示该条目允许执行该命令。空条目永远不匹配，路径条目会先展开 `~`，避免
/// 用户目录写法差异绕过路径比较。
pub fn is_allowlist_entry_match(allowed: &str, executable: &str, executable_base: &str) -> bool {
    let allowed = strip_wrapping_quotes(allowed.trim()).trim();
    if allowed.is_empty() {
        return false;
    }

    if allowed == "*" {
        return true;
    }

    if looks_like_path(allowed) {
        let allowed_path = expand_user_path(allowed);
        let executable_path = expand_user_path(executable);
        return executable_path == allowed_path;
    }

    allowed == executable_base
}

/// 从命令参数中找出第一个不允许访问的路径。
///
/// `command` 是待检查的 shell 命令，`is_path_allowed` 是调用方提供的路径授权
/// 谓词。返回第一个被拒绝的路径字符串；如果未发现路径参数或所有路径都允许，
/// 返回 `None`。
pub fn find_forbidden_path_argument<F>(command: &str, is_path_allowed: F) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let segments = split_unquoted_segments(command);
    for segment in segments {
        let cmd_part = skip_env_assignments(&segment);
        let mut words = cmd_part.split_whitespace();
        let executable_word = words.next();
        if executable_word.is_none() {
            continue;
        }
        let executable = executable_word.expect("executable word was checked");

        if let RedirectionPathCheck::Blocked(blocked) =
            check_redirection_path(executable, &is_path_allowed)
        {
            return Some(blocked);
        }

        for token in words {
            match check_argument_token(token, &is_path_allowed) {
                TokenPathCheck::Blocked(blocked) => return Some(blocked),
                TokenPathCheck::Ignored => {}
            }
        }
    }

    None
}

fn check_argument_token<F>(token: &str, is_path_allowed: &F) -> TokenPathCheck
where
    F: Fn(&str) -> bool,
{
    let candidate = strip_wrapping_quotes(token).trim();
    if candidate.is_empty() {
        return TokenPathCheck::Ignored;
    }
    if candidate.contains("://") {
        return TokenPathCheck::Ignored;
    }

    match check_redirection_path(candidate, is_path_allowed) {
        RedirectionPathCheck::Blocked(blocked) => return TokenPathCheck::Blocked(blocked),
        RedirectionPathCheck::Allowed => return TokenPathCheck::Ignored,
        RedirectionPathCheck::NotRedirection => {}
    }

    if candidate.starts_with('-') {
        return check_option_token(candidate, is_path_allowed);
    }

    check_plain_token(candidate, is_path_allowed)
}

fn check_option_token<F>(candidate: &str, is_path_allowed: &F) -> TokenPathCheck
where
    F: Fn(&str) -> bool,
{
    let blocked = forbidden_option_value(candidate, is_path_allowed);
    if blocked.is_none() {
        return TokenPathCheck::Ignored;
    }
    TokenPathCheck::Blocked(blocked.expect("blocked option path was checked"))
}

fn check_plain_token<F>(candidate: &str, is_path_allowed: &F) -> TokenPathCheck
where
    F: Fn(&str) -> bool,
{
    let blocked = forbidden_path_candidate(candidate, is_path_allowed);
    if blocked.is_none() {
        return TokenPathCheck::Ignored;
    }
    TokenPathCheck::Blocked(blocked.expect("blocked plain path was checked"))
}

fn check_redirection_path<F>(token: &str, is_path_allowed: &F) -> RedirectionPathCheck
where
    F: Fn(&str) -> bool,
{
    let target = redirection_target(strip_wrapping_quotes(token));
    if target.is_none() {
        return RedirectionPathCheck::NotRedirection;
    }
    let target = target.expect("redirection target was checked");
    match forbidden_path_candidate(target, is_path_allowed) {
        Some(blocked) => RedirectionPathCheck::Blocked(blocked),
        None => RedirectionPathCheck::Allowed,
    }
}

fn forbidden_option_value<F>(candidate: &str, is_path_allowed: &F) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let long_option_value = candidate.split_once('=');
    if long_option_value.is_some() {
        let (_, value) = long_option_value.expect("long option value was checked");
        return forbidden_path_candidate(value, is_path_allowed);
    }

    let short_option_value = attached_short_option_value(candidate);
    if short_option_value.is_none() {
        return None;
    }
    forbidden_path_candidate(
        short_option_value.expect("short option value was checked"),
        is_path_allowed,
    )
}

fn forbidden_path_candidate<F>(raw: &str, is_path_allowed: &F) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let candidate = strip_wrapping_quotes(raw.trim()).trim();
    if candidate.is_empty() {
        return None;
    }
    if candidate.contains("://") {
        return None;
    }
    // URL 由上面的协议检查排除；这里仅把看起来像文件系统路径的参数交给
    // 策略层，避免把普通标志值误判为路径。
    if !looks_like_path(candidate) {
        return None;
    }
    if is_path_allowed(candidate) {
        return None;
    }
    Some(candidate.to_string())
}

#[cfg(test)]
#[path = "allowlist_tests.rs"]
mod allowlist_tests;
