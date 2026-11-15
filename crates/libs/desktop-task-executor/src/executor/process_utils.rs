//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::*;

/// 执行 shell_escape_arg 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn shell_escape_arg(arg: &str) -> String {
    let is_plain = arg
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '='));
    if is_plain { arg.to_string() } else { format!("'{}'", arg.replace('\'', "'\\''")) }
}

/// 执行 to_shell_command 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn to_shell_command(program: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_escape_arg(program));
    for arg in args {
        parts.push(shell_escape_arg(arg));
    }
    parts.join(" ")
}

/// 执行 tail_chars 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn tail_chars(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        s.to_string()
    } else {
        s.chars().skip(total - max_chars).collect::<String>()
    }
}

/// 执行 exit_status_signal 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn exit_status_signal(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }

    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

fn signal_name(signal: i32) -> Option<&'static str> {
    match signal {
        1 => Some("SIGHUP"),
        2 => Some("SIGINT"),
        3 => Some("SIGQUIT"),
        6 => Some("SIGABRT"),
        9 => Some("SIGKILL"),
        11 => Some("SIGSEGV"),
        13 => Some("SIGPIPE"),
        15 => Some("SIGTERM"),
        _ => None,
    }
}

fn format_process_exit_detail(code: Option<i32>, signal: Option<i32>) -> String {
    if let Some(signal) = signal {
        if let Some(name) = signal_name(signal) {
            return format!("signal={}({})", signal, name);
        }
        return format!("signal={}", signal);
    }

    format!("code={}", code.map(|v| v.to_string()).unwrap_or_else(|| "None".to_string()))
}

/// 执行 build_command_failure_detail 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn build_command_failure_detail(
    code: Option<i32>,
    signal: Option<i32>,
    stdout: &str,
    stderr: &str,
    stdin_broken_pipe: bool,
) -> String {
    let exit_detail = format_process_exit_detail(code, signal);
    let stderr_tail = tail_chars(stderr, 4000);
    if !stderr_tail.trim().is_empty() {
        return format!("{} stderr={}", exit_detail, stderr_tail);
    }

    let stdout_tail = tail_chars(stdout, 4000);
    if !stdout_tail.trim().is_empty() {
        return format!("{} stdout={}", exit_detail, stdout_tail);
    }

    if stdin_broken_pipe {
        return format!("{} stdin=BrokenPipe(对端提前关闭输入)", exit_detail);
    }

    exit_detail
}

/// 执行 normalize_path 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn normalize_path(path: &str) -> String {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// 执行 emit_stdout_log 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn emit_stdout_log(sender: Option<&Sender<TaskLogStream>>, message: impl Into<String>) {
    if let Some(sender) = sender {
        let _ = sender.send(TaskLogStream::Stdout(message.into()));
    }
}

/// 执行 emit_stderr_log 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn emit_stderr_log(sender: Option<&Sender<TaskLogStream>>, message: impl Into<String>) {
    if let Some(sender) = sender {
        let _ = sender.send(TaskLogStream::Stderr(message.into()));
    }
}

/// 执行 truncate_for_log_preview 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn truncate_for_log_preview(value: &str, max_chars: usize) -> String {
    let _ = max_chars;
    value.replace('\n', "\\n")
}

#[cfg(test)]
#[path = "process_utils_tests.rs"]
mod process_utils_tests;
