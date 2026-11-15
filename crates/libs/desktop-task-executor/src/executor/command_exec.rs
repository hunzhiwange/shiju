//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::backend_output::{
    emit_claude_line, emit_opencode_line, extract_claude_terminal_error,
    extract_opencode_terminal_error,
};
use super::process_utils::{
    build_command_failure_detail, exit_status_signal, tail_chars, to_shell_command,
    truncate_for_log_preview,
};
use super::programs::{
    ExecutorCommand, is_claude_program, is_opencode_program, spawn_executor_child,
};
use super::*;

/// 执行 execute_task_command 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn execute_task_command(cmd: &ExecutorCommand) -> Result<String, String> {
    let mut child = spawn_executor_child(cmd)?;

    let mut stdin_broken_pipe = false;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        if let Some(content) = &cmd.stdin_content
            && let Err(err) = stdin.write_all(content.as_bytes())
        {
            if err.kind() == std::io::ErrorKind::BrokenPipe {
                stdin_broken_pipe = true;
            } else {
                return Err(format!("Failed to write stdin: {}", err));
            }
        }
    }
    drop(child.stdin.take());

    let output =
        child.wait_with_output().map_err(|e| format!("Failed to wait for process: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if is_opencode_program(&cmd.program)
        && let Some(message) = extract_opencode_terminal_error(&stdout, &stderr)
    {
        return Err(format!("opencode 执行失败: {}", message));
    }
    if is_claude_program(&cmd.program)
        && let Some(message) = extract_claude_terminal_error(&stdout, &stderr)
    {
        return Err(format!("claude 执行失败: {}", message));
    }

    if output.status.success() {
        Ok(stdout)
    } else {
        let detail = build_command_failure_detail(
            output.status.code(),
            exit_status_signal(&output.status),
            &stdout,
            &stderr,
            stdin_broken_pipe,
        );
        Err(format!("Command failed: {}", detail))
    }
}

/// 执行 execute_task_command_with_streaming 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn execute_task_command_with_streaming(
    cmd: &ExecutorCommand,
    log_sender: Sender<TaskLogStream>,
) -> Result<String, String> {
    let shell_command = to_shell_command(&cmd.program, &cmd.args);
    let shell_command_preview = truncate_for_log_preview(&shell_command, 4000);
    let stdin_chars =
        cmd.stdin_content.as_ref().map(|content| content.chars().count()).unwrap_or(0);
    let prompt_preview = cmd
        .stdin_content
        .as_deref()
        .map(|content| truncate_for_log_preview(content, 1200))
        .unwrap_or_default();

    let _ = log_sender.send(TaskLogStream::Stdout(format!(
        "[EXEC] cwd={} program={} args={:?}",
        cmd.cwd, cmd.program, cmd.args
    )));
    let _ = log_sender.send(TaskLogStream::Stdout(format!("[EXEC_CMD] {}", shell_command_preview)));
    if stdin_chars > 0 {
        let _ = log_sender.send(TaskLogStream::Stdout(format!(
            "[EXEC_STDIN] chars={} preview={} ",
            stdin_chars, prompt_preview
        )));
    }

    execute_task_with_streaming(cmd, log_sender.clone()).inspect(|stdout| {
        let preview = truncate_for_log_preview(stdout, 1600);
        let _ = log_sender.send(TaskLogStream::Stdout(format!(
            "[EXEC_RESULT] stdout_chars={} preview={}",
            stdout.chars().count(),
            preview
        )));
    })
}

/// 执行 execute_task_with_streaming 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn execute_task_with_streaming(
    cmd: &ExecutorCommand,
    log_sender: Sender<TaskLogStream>,
) -> Result<String, String> {
    let is_opencode = is_opencode_program(&cmd.program);
    let is_claude = is_claude_program(&cmd.program);
    let mut child = spawn_executor_child(cmd)?;

    let mut stdin_broken_pipe = false;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        if let Some(content) = &cmd.stdin_content
            && let Err(err) = stdin.write_all(content.as_bytes())
        {
            if err.kind() == std::io::ErrorKind::BrokenPipe {
                stdin_broken_pipe = true;
            } else {
                return Err(format!("Failed to write stdin: {}", err));
            }
        }
    }
    drop(child.stdin.take());
    if stdin_broken_pipe {
        let _ = log_sender.send(TaskLogStream::Stderr(
            "[PROCESS] stdin 提前关闭(BrokenPipe)，继续收集进程输出".to_string(),
        ));
    }

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let stdout_sender = log_sender.clone();
    let stdout_capture = Arc::new(Mutex::new(String::new()));
    let stdout_capture_thread = Arc::clone(&stdout_capture);
    let stdout_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut buf = [0u8; 2048];
        let mut pending = String::new();
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
            if let Ok(mut out) = stdout_capture_thread.lock() {
                out.push_str(&chunk);
            }
            if is_opencode || is_claude {
                pending.push_str(&chunk);
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim_end_matches('\r').to_string();
                    pending = pending[pos + 1..].to_string();
                    if is_opencode {
                        emit_opencode_line(&stdout_sender, &line, false);
                    } else {
                        emit_claude_line(&stdout_sender, &line, false);
                    }
                }
            } else {
                pending.push_str(&chunk);
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim_end_matches('\r').to_string();
                    pending = pending[pos + 1..].to_string();
                    let _ = stdout_sender.send(TaskLogStream::Stdout(line));
                }
                if pending.chars().count() >= 2048 {
                    let partial = std::mem::take(&mut pending);
                    let _ = stdout_sender.send(TaskLogStream::Stdout(partial));
                }
            }
        }
        if is_opencode || is_claude {
            let line = pending.trim();
            if is_opencode {
                emit_opencode_line(&stdout_sender, line, false);
            } else {
                emit_claude_line(&stdout_sender, line, false);
            }
        } else if !pending.trim().is_empty() {
            let _ = stdout_sender.send(TaskLogStream::Stdout(pending));
        }
    });

    let stderr_sender = log_sender.clone();
    let stderr_capture = Arc::new(Mutex::new(String::new()));
    let stderr_capture_thread = Arc::clone(&stderr_capture);
    let stderr_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = [0u8; 2048];
        let mut pending = String::new();
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
            if let Ok(mut out) = stderr_capture_thread.lock() {
                out.push_str(&chunk);
            }
            if is_opencode || is_claude {
                pending.push_str(&chunk);
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim_end_matches('\r').to_string();
                    pending = pending[pos + 1..].to_string();
                    if is_opencode {
                        emit_opencode_line(&stderr_sender, &line, true);
                    } else {
                        emit_claude_line(&stderr_sender, &line, true);
                    }
                }
            } else {
                pending.push_str(&chunk);
                while let Some(pos) = pending.find('\n') {
                    let line = pending[..pos].trim_end_matches('\r').to_string();
                    pending = pending[pos + 1..].to_string();
                    let _ = stderr_sender.send(TaskLogStream::Stderr(line));
                }
                if pending.chars().count() >= 2048 {
                    let partial = std::mem::take(&mut pending);
                    let _ = stderr_sender.send(TaskLogStream::Stderr(partial));
                }
            }
        }
        if is_opencode || is_claude {
            let line = pending.trim();
            if is_opencode {
                emit_opencode_line(&stderr_sender, line, true);
            } else {
                emit_claude_line(&stderr_sender, line, true);
            }
        } else if !pending.trim().is_empty() {
            let _ = stderr_sender.send(TaskLogStream::Stderr(pending));
        }
    });

    let status = child.wait().map_err(|e| format!("Failed to wait for process: {}", e))?;
    let success = status.success();
    let code = status.code();
    let signal = exit_status_signal(&status);

    stdout_thread.join().map_err(|_| "stdout thread panicked".to_string())?;
    stderr_thread.join().map_err(|_| "stderr thread panicked".to_string())?;

    let stdout_full = stdout_capture.lock().map(|s| s.clone()).unwrap_or_else(|_| String::new());
    let stderr_full = stderr_capture.lock().map(|s| s.clone()).unwrap_or_else(|_| String::new());
    let stdout_tail = tail_chars(&stdout_full, 6000);
    let stderr_tail = tail_chars(&stderr_full, 6000);

    if is_opencode
        && let Some(message) = extract_opencode_terminal_error(&stdout_full, &stderr_full)
    {
        return Err(format!("opencode 执行失败: {}", message));
    }
    if is_claude && let Some(message) = extract_claude_terminal_error(&stdout_full, &stderr_full) {
        return Err(format!("claude 执行失败: {}", message));
    }

    if !success && !stdout_tail.trim().is_empty() {
        let _ = log_sender.send(TaskLogStream::Stdout(format!(
            "[FINAL_STDOUT_TAIL chars={}] {}",
            stdout_tail.chars().count(),
            stdout_tail
        )));
    }
    if !success && !stderr_tail.trim().is_empty() {
        let _ = log_sender.send(TaskLogStream::Stderr(format!(
            "[FINAL_STDERR_TAIL chars={}] {}",
            stderr_tail.chars().count(),
            stderr_tail
        )));
    }

    let _ = log_sender.send(TaskLogStream::ExitStatus { success, code, signal });

    if success {
        Ok(stdout_full)
    } else {
        let detail = build_command_failure_detail(
            code,
            signal,
            &stdout_full,
            &stderr_full,
            stdin_broken_pipe,
        );
        Err(format!("命令退出失败: {}", detail))
    }
}

#[cfg(test)]
#[path = "command_exec_tests.rs"]
mod command_exec_tests;
