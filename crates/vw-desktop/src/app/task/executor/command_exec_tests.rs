use super::*;
use std::sync::mpsc;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("command_exec_tests"));
}

fn shell_cmd(script: &str) -> ExecutorCommand {
    ExecutorCommand {
        program: "sh".to_string(),
        args: vec!["-c".to_string(), script.to_string()],
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    }
}

#[cfg(unix)]
fn temp_executable(name: &str, body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(body.as_bytes()).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    (dir, path)
}

fn collect_logs(rx: &mpsc::Receiver<TaskLogStream>) -> Vec<TaskLogStream> {
    rx.try_iter().collect()
}

#[test]
fn execute_task_command_returns_stdout_for_success() {
    let cmd = shell_cmd("printf 'hello'");

    let output = execute_task_command(&cmd).unwrap();

    assert_eq!(output, "hello");
}

#[test]
fn execute_task_command_writes_stdin_and_reports_failure_detail() {
    let mut cmd = shell_cmd("cat");
    cmd.stdin_content = Some("from stdin".to_string());
    assert_eq!(execute_task_command(&cmd).unwrap(), "from stdin");

    let cmd = shell_cmd("printf 'out'; printf 'err' >&2; exit 7");
    let error = execute_task_command(&cmd).unwrap_err();
    assert!(error.starts_with("Command failed: code=7 stderr=err"));
}

#[test]
fn execute_task_command_reports_spawn_error() {
    let cmd = ExecutorCommand {
        program: "/definitely/missing/vw-command".to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };

    let error = execute_task_command(&cmd).unwrap_err();

    assert!(error.contains("Failed to spawn"));
}

#[cfg(unix)]
#[test]
fn execute_task_command_detects_opencode_and_claude_terminal_errors() {
    let (_opencode_dir, opencode) = temp_executable(
        "opencode",
        "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"error\",\"message\":\"bad open\"}'\nexit 0\n",
    );
    let cmd = ExecutorCommand {
        program: opencode.to_string_lossy().to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };
    let error = execute_task_command(&cmd).unwrap_err();
    assert_eq!(error, "opencode 执行失败: bad open");

    let (_claude_dir, claude) = temp_executable(
        "claude",
        "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"error\",\"message\":\"bad claude\"}' >&2\nexit 0\n",
    );
    let cmd = ExecutorCommand {
        program: claude.to_string_lossy().to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };
    let error = execute_task_command(&cmd).unwrap_err();
    assert_eq!(error, "claude 执行失败: bad claude");
}

#[test]
fn execute_task_command_with_streaming_emits_exec_metadata_and_result() {
    let (tx, rx) = mpsc::channel();
    let mut cmd = shell_cmd("printf 'hello\\n'; printf 'warn\\n' >&2");
    cmd.stdin_content = Some("prompt text".to_string());

    let output = execute_task_command_with_streaming(&cmd, tx).unwrap();
    let logs = collect_logs(&rx);

    assert_eq!(output, "hello\n");
    assert!(logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC] cwd="))
    ));
    assert!(logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC_CMD] sh -c"))
    ));
    assert!(logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.contains("[EXEC_STDIN] chars=11"))
    ));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stdout(value) if value == "hello")));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stderr(value) if value == "warn")));
    assert!(logs.iter().any(|log| matches!(
        log,
        TaskLogStream::ExitStatus { success: true, code: Some(0), signal: None }
    )));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC_RESULT] stdout_chars=6"))));
}

#[test]
fn execute_task_with_streaming_reports_failure_tails_and_exit_status() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("printf 'out\\n'; printf 'err\\n' >&2; exit 9");

    let error = execute_task_with_streaming(&cmd, tx).unwrap_err();
    let logs = collect_logs(&rx);

    assert!(error.starts_with("命令退出失败: code=9 stderr=err"));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stdout(value) if value == "out")));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stderr(value) if value == "err")));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stdout(value) if value.contains("[FINAL_STDOUT_TAIL chars=4] out"))));
    assert!(logs.iter().any(|log| matches!(log, TaskLogStream::Stderr(value) if value.contains("[FINAL_STDERR_TAIL chars=4] err"))));
    assert!(logs.iter().any(|log| matches!(
        log,
        TaskLogStream::ExitStatus { success: false, code: Some(9), signal: None }
    )));
}

#[test]
fn execute_task_with_streaming_flushes_partial_lines() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("printf 'partial-out'; printf 'partial-err' >&2");

    let output = execute_task_with_streaming(&cmd, tx).unwrap();
    let logs = collect_logs(&rx);

    assert_eq!(output, "partial-out");
    assert!(
        logs.iter()
            .any(|log| matches!(log, TaskLogStream::Stdout(value) if value == "partial-out"))
    );
    assert!(
        logs.iter()
            .any(|log| matches!(log, TaskLogStream::Stderr(value) if value == "partial-err"))
    );
}

#[cfg(unix)]
#[test]
fn execute_task_with_streaming_formats_backend_json_lines() {
    let (_opencode_dir, opencode) = temp_executable(
        "opencode",
        "#!/bin/sh\nprintf '%s\\n' '{\"text\":\"hello\"}'\nprintf 'raw\\n' >&2\n",
    );
    let (tx, rx) = mpsc::channel();
    let cmd = ExecutorCommand {
        program: opencode.to_string_lossy().to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };

    let output = execute_task_with_streaming(&cmd, tx).unwrap();
    let logs = collect_logs(&rx);

    assert!(output.contains("\"hello\""));
    assert!(
        logs.iter()
            .any(|log| matches!(log, TaskLogStream::Stdout(value) if value == "[OPENCODE] hello"))
    );
    assert!(
        logs.iter().any(
            |log| matches!(log, TaskLogStream::Stderr(value) if value == "[OPENCODE_RAW] raw")
        )
    );
}

#[cfg(unix)]
#[test]
fn execute_task_with_streaming_detects_backend_errors_after_threads_finish() {
    let (_claude_dir, claude) = temp_executable(
        "claude",
        "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"error\",\"message\":\"stream bad\"}'\nexit 0\n",
    );
    let (tx, rx) = mpsc::channel();
    let cmd = ExecutorCommand {
        program: claude.to_string_lossy().to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };

    let error = execute_task_with_streaming(&cmd, tx).unwrap_err();
    let logs = collect_logs(&rx);

    assert_eq!(error, "claude 执行失败: stream bad");
    assert!(
        logs.iter().any(
            |log| matches!(log, TaskLogStream::Stdout(value) if value == "[CLAUDE] stream bad")
        )
    );
}

#[test]
fn execute_task_command_with_streaming_omits_empty_stdin_metadata() {
    let (tx, rx) = mpsc::channel();
    let mut cmd = shell_cmd("printf 'done'");
    cmd.stdin_content = Some(String::new());

    let output = execute_task_command_with_streaming(&cmd, tx).unwrap();
    let logs = collect_logs(&rx);

    assert_eq!(output, "done");
    assert!(!logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC_STDIN]"))
    ));
}

#[test]
fn execute_task_command_with_streaming_omits_result_metadata_on_failure() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("printf 'failure' >&2; exit 4");

    let error = execute_task_command_with_streaming(&cmd, tx).unwrap_err();
    let logs = collect_logs(&rx);

    assert!(error.starts_with("命令退出失败: code=4 stderr=failure"));
    assert!(logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC_CMD] sh -c"))
    ));
    assert!(!logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value.starts_with("[EXEC_RESULT]"))
    ));
}

#[test]
fn execute_task_with_streaming_flushes_large_unterminated_chunks() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("printf '%02048dX' 0; printf '%02048dY' 1 >&2");

    let output = execute_task_with_streaming(&cmd, tx).unwrap();
    let logs = collect_logs(&rx);
    let stdout_logs = logs
        .iter()
        .filter_map(|log| match log {
            TaskLogStream::Stdout(value) => Some(value.as_str()),
            _ => None,
        })
        .collect::<String>();
    let stderr_logs = logs
        .iter()
        .filter_map(|log| match log {
            TaskLogStream::Stderr(value) => Some(value.as_str()),
            _ => None,
        })
        .collect::<String>();

    assert_eq!(output, format!("{}X", "0".repeat(2048)));
    assert_eq!(stdout_logs, output);
    assert_eq!(stderr_logs, format!("{}1Y", "0".repeat(2047)));
}

#[test]
fn execute_task_with_streaming_ignores_disconnected_log_receiver() {
    let (tx, rx) = mpsc::channel();
    drop(rx);
    let cmd = shell_cmd("printf 'out\\n'; printf 'err\\n' >&2");

    let output = execute_task_with_streaming(&cmd, tx).unwrap();

    assert_eq!(output, "out\n");
}

#[cfg(unix)]
#[test]
fn execute_task_with_streaming_detects_opencode_terminal_error() {
    let (_opencode_dir, opencode) = temp_executable(
        "opencode",
        "#!/bin/sh\nprintf '%s\\n' '{\"type\":\"error\",\"message\":\"open stream bad\"}' >&2\nexit 0\n",
    );
    let (tx, rx) = mpsc::channel();
    let cmd = ExecutorCommand {
        program: opencode.to_string_lossy().to_string(),
        args: Vec::new(),
        cwd: std::env::current_dir().unwrap().to_string_lossy().to_string(),
        stdin_content: None,
    };

    let error = execute_task_with_streaming(&cmd, tx).unwrap_err();
    let logs = collect_logs(&rx);

    assert_eq!(error, "opencode 执行失败: open stream bad");
    assert!(logs.iter().any(
        |log| matches!(log, TaskLogStream::Stdout(value) if value == "[OPENCODE] open stream bad")
    ));
}

#[cfg(unix)]
#[test]
fn execute_task_with_streaming_reports_signal_exit() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("kill -TERM $$");

    let error = execute_task_with_streaming(&cmd, tx).unwrap_err();
    let logs = collect_logs(&rx);

    assert_eq!(error, "命令退出失败: signal=15(SIGTERM)");
    assert!(logs.iter().any(|log| matches!(
        log,
        TaskLogStream::ExitStatus { success: false, code: None, signal: Some(15) }
    )));
}

#[test]
fn execute_task_with_streaming_flushes_large_unterminated_stderr_before_exit() {
    let (tx, rx) = mpsc::channel();
    let cmd = shell_cmd("printf '%02049d' 7 >&2");

    let output = execute_task_with_streaming(&cmd, tx).expect("command succeeds");
    let stderr = collect_logs(&rx)
        .into_iter()
        .filter_map(|log| match log {
            TaskLogStream::Stderr(value) => Some(value),
            _ => None,
        })
        .collect::<String>();

    assert!(output.is_empty());
    assert_eq!(stderr, format!("{}7", "0".repeat(2048)));
}
