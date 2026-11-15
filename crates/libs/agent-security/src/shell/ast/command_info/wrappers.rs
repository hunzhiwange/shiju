//! Shell 包装命令剥离逻辑。
//!
//! 本模块识别 `timeout`、`env`、`nice`、shell `-c` 等常见 wrapper，并返回真正要执行的
//! 内层命令信息。安全策略通常关心最终执行对象，因此 wrapper 剥离必须显式、可预测。

use super::super::nodes::CommandInfo;

/// 可被剥离的常见 wrapper 命令列表。
///
/// 该列表只包含当前解析逻辑明确支持的 wrapper，避免把未知命令误当作透明包装层。
pub const WRAPPER_COMMANDS: &[&str] = &[
    "timeout", "nice", "stdbuf", "env", "time", "nohup", "ionice", "taskset", "chrt", "cpulimit",
    "prlimit", "unshare", "setsid", "bash", "sh", "zsh", "dash",
];

/// 剥离命令前缀中的常见 wrapper。
///
/// # 参数
///
/// - `info`: 已提取的命令信息。
///
/// # 返回值
///
/// 如果能识别 wrapper，则返回内层命令；无法安全剥离时返回原始命令信息。
pub fn strip_wrappers(info: &CommandInfo) -> CommandInfo {
    let mut current = info.clone();

    loop {
        if !WRAPPER_COMMANDS.contains(&current.name.as_str()) {
            return current;
        }

        let next = match current.name.as_str() {
            "env" => strip_env_wrapper(&current),
            "timeout" => strip_timeout_wrapper(&current),
            "nice" => strip_nice_wrapper(&current),
            "bash" | "sh" | "zsh" | "dash" => strip_shell_wrapper(&current),
            _ => strip_generic_wrapper(&current),
        };

        match next {
            Some(info) => current = info,
            // 参数形态不完整时保留当前命令，避免把 wrapper 自身误删后丢失风险信息。
            None => return current,
        }
    }
}

fn strip_env_wrapper(info: &CommandInfo) -> Option<CommandInfo> {
    let mut index = 0;
    while index < info.args.len() {
        let arg = info.args[index].as_str();
        if arg == "-u" {
            index += 2;
            continue;
        }
        if arg.starts_with("-u") && arg.len() > 2 {
            index += 1;
            continue;
        }
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        if is_env_assignment(arg) {
            index += 1;
            continue;
        }
        break;
    }
    rebuild_wrapped_command(info, &info.args[index..])
}

fn strip_timeout_wrapper(info: &CommandInfo) -> Option<CommandInfo> {
    let mut index = 0;
    while index < info.args.len() {
        let arg = info.args[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-s" || arg == "-k" || arg == "--signal" || arg == "--kill-after" {
            index += 2;
            continue;
        }
        if arg.starts_with("--signal=")
            || arg.starts_with("--kill-after=")
            || arg == "--foreground"
            || arg == "--preserve-status"
        {
            index += 1;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        index += 1;
        break;
    }
    rebuild_wrapped_command(info, &info.args[index..])
}

fn strip_nice_wrapper(info: &CommandInfo) -> Option<CommandInfo> {
    let mut index = 0;
    while index < info.args.len() {
        let arg = info.args[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg == "-n" || arg == "--adjustment" {
            index += 2;
            continue;
        }
        if arg.starts_with("--adjustment=") || is_nice_priority(arg) {
            index += 1;
            continue;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    rebuild_wrapped_command(info, &info.args[index..])
}

fn strip_shell_wrapper(info: &CommandInfo) -> Option<CommandInfo> {
    let mut index = 0;
    while index < info.args.len() {
        let arg = info.args[index].as_str();
        if arg == "-c" || arg == "-lc" {
            let command = info.args.get(index + 1)?;
            // shell -c 的真实执行面在字符串参数里，需要重新解析后继续剥离嵌套 wrapper。
            return CommandInfo::from_command(command).map(|parsed| strip_wrappers(&parsed));
        }
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    rebuild_wrapped_command(info, &info.args[index..])
}

fn strip_generic_wrapper(info: &CommandInfo) -> Option<CommandInfo> {
    let mut index = 0;
    while index < info.args.len() {
        let arg = info.args[index].as_str();
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }
    rebuild_wrapped_command(info, &info.args[index..])
}

fn rebuild_wrapped_command(original: &CommandInfo, remaining: &[String]) -> Option<CommandInfo> {
    let name = remaining.first()?.clone();
    Some(CommandInfo {
        name,
        args: remaining[1..].to_vec(),
        redirects: original.redirects.clone(),
        pipes: original.pipes.clone(),
        subcommands: original.subcommands.clone(),
        has_command_substitution: original.has_command_substitution,
        has_process_substitution: original.has_process_substitution,
        has_glob: original.has_glob,
        has_variable_expansion: original.has_variable_expansion,
        compound_operator: original.compound_operator,
    })
}

fn is_env_assignment(value: &str) -> bool {
    let Some((name, _)) = value.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().next().is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && name.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_nice_priority(value: &str) -> bool {
    value.len() > 1 && value.starts_with('-') && value[1..].chars().all(|ch| ch.is_ascii_digit())
}
#[cfg(test)]
#[path = "wrappers_tests.rs"]
mod wrappers_tests;
