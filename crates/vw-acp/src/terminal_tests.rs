use super::*;
use agent_client_protocol as acp;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

struct FailingReader;

impl tokio::io::AsyncRead for FailingReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Err(std::io::Error::other("read failed")))
    }
}

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("serialize failed"))
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vw-acp-terminal-tests-{label}-{}", std::process::id()))
}

fn approve_all_manager(cwd: PathBuf) -> TerminalManager {
    TerminalManager::new(TerminalManagerOptions {
        cwd,
        permission_mode: PermissionMode::ApproveAll,
        ..TerminalManagerOptions::default()
    })
}

#[test]
fn trim_to_utf8_boundary_keeps_valid_suffix() {
    let text = "aé日";
    let trimmed = trim_to_utf8_boundary(text.as_bytes(), 4);

    assert_eq!(std::str::from_utf8(&trimmed).unwrap(), "日");
}

#[test]
fn trim_to_utf8_boundary_falls_back_when_suffix_is_only_continuation_bytes() {
    assert_eq!(trim_to_utf8_boundary(&[0x61, 0x80, 0x80], 2), vec![0x80, 0x80]);
}

#[test]
fn to_command_line_renders_quoted_arguments() {
    let args = vec!["hello world".to_string(), "plain".to_string()];

    assert_eq!(to_command_line("echo", &args), r#"echo "hello world" "plain""#);
}

#[tokio::test]
async fn approve_reads_auto_approves_default_execute_inside_cwd() {
    let root = unique_temp_dir("auto");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");

    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveReads,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        ..TerminalManagerOptions::default()
    });

    let approved = manager
        .is_execute_approved(&root, "echo ok")
        .await
        .expect("workspace command should not prompt");

    assert!(approved);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn approve_reads_default_execute_fails_without_prompt_outside_cwd() {
    if can_prompt_for_permission() {
        return;
    }

    let root = unique_temp_dir("prompt-root");
    let outside = unique_temp_dir("prompt-outside");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&root).expect("create temp root");
    std::fs::create_dir_all(&outside).expect("create outside root");
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveReads,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        ..TerminalManagerOptions::default()
    });

    let err = manager
        .is_execute_approved(&outside, "echo needs-prompt")
        .await
        .expect_err("unpromptable command outside cwd should fail");

    assert!(err.to_string().contains("Permission prompt unavailable"));
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[tokio::test]
async fn update_permission_policy_replaces_mode_and_defaults_non_interactive_policy() {
    let root = unique_temp_dir("policy");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let mut manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::DenyAll,
        non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
        ..TerminalManagerOptions::default()
    });

    assert!(!manager.is_execute_approved(&root, "echo before").await.unwrap());
    manager.update_permission_policy(PermissionMode::ApproveAll, None);

    assert_eq!(manager.non_interactive_permissions, NonInteractivePermissionPolicy::Deny);
    assert!(manager.is_execute_approved(&root, "echo after").await.unwrap());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn trim_to_utf8_boundary_handles_zero_limit() {
    assert!(trim_to_utf8_boundary("hello".as_bytes(), 0).is_empty());
}

#[test]
fn trim_to_utf8_boundary_keeps_short_buffer_unchanged() {
    assert_eq!(trim_to_utf8_boundary(b"ok", 8), b"ok");
}

#[test]
fn to_command_line_omits_empty_arguments() {
    assert_eq!(to_command_line("pwd", &[]), "pwd");
}

#[test]
fn terminal_cwd_uses_default_when_request_has_no_cwd() {
    let default_cwd = PathBuf::from("/tmp/vw-acp-terminal-default");
    let record = Map::new();

    let cwd = terminal_cwd(&record, &default_cwd).expect("default cwd should be accepted");

    assert_eq!(cwd, default_cwd);
}

#[test]
fn terminal_cwd_rejects_relative_request_cwd() {
    let mut record = Map::new();
    record.insert("cwd".to_string(), json!("relative"));

    let err = terminal_cwd(&record, Path::new("/tmp")).expect_err("relative cwd should fail");

    assert!(err.to_string().contains("cwd must be absolute"));
}

#[test]
fn normalize_absolute_path_removes_dot_and_parent_components() {
    assert_eq!(
        normalize_absolute_path(Path::new("/tmp/vw-acp/./child/../root")),
        PathBuf::from("/tmp/vw-acp/root")
    );
    assert_eq!(normalize_absolute_path(Path::new("./child")), PathBuf::from("child"));
}

#[test]
fn is_within_root_accepts_root_and_descendant_only() {
    let root = Path::new("/tmp/vw-acp-root");

    assert!(is_within_root(root, Path::new("/tmp/vw-acp-root")));
    assert!(is_within_root(root, Path::new("/tmp/vw-acp-root/child")));
    assert!(!is_within_root(root, Path::new("/tmp/vw-acp-root-sibling")));
}

#[test]
fn env_overrides_ignores_malformed_entries() {
    let mut record = Map::new();
    record.insert(
        "env".to_string(),
        json!([
            { "name": "GOOD", "value": "one" },
            { "name": "MISSING_VALUE" },
            { "value": "missing-name" },
            { "name": 7, "value": "bad-name" },
            { "name": "BAD_VALUE", "value": 7 },
            "plain"
        ]),
    );

    let env = env_overrides(&record);

    assert_eq!(env.len(), 1);
    assert_eq!(env.get("GOOD").map(String::as_str), Some("one"));
}

#[test]
fn output_byte_limit_defaults_and_accepts_custom_values() {
    let empty = Map::new();
    let mut custom = Map::new();
    custom.insert("outputByteLimit".to_string(), json!(7));

    assert_eq!(output_byte_limit(&empty), DEFAULT_TERMINAL_OUTPUT_LIMIT_BYTES);
    assert_eq!(output_byte_limit(&custom), 7);
}

#[test]
fn request_helpers_extract_required_fields() {
    let record = request_record(&json!({ "terminalId": "term-test", "args": ["a", 9, "b"] }))
        .expect("json object should serialize as record");

    assert_eq!(required_string(&record, "terminalId").unwrap(), "term-test");
    assert_eq!(string_array(&record, "args"), vec!["a", "b"]);
    assert!(required_string(&record, "missing").is_err());
}

#[test]
fn request_record_rejects_serialized_non_object() {
    let err = request_record(&json!(["not", "object"])).expect_err("array should fail");

    assert!(err.to_string().contains("request parameters must serialize to an object"));
}

#[test]
fn request_record_propagates_serialization_errors() {
    let err = request_record(&FailingSerialize).expect_err("serialization error should fail");

    assert!(err.to_string().contains("serialize failed"));
}

#[test]
fn terminal_id_from_request_trims_and_rejects_blank_values() {
    let trimmed = terminal_id_from_request(&acp::TerminalOutputRequest::new(
        "session-testcov-0072",
        "  term-trim  ",
    ))
    .expect("trimmed terminal id should be accepted");
    let err =
        terminal_id_from_request(&acp::TerminalOutputRequest::new("session-testcov-0072", "   "))
            .expect_err("blank terminal id should fail");

    assert_eq!(trimmed, "term-trim");
    assert!(err.to_string().contains("Missing required field: terminalId"));
}

#[test]
fn terminal_id_from_request_reports_serialization_and_missing_field_errors() {
    let serialize_err =
        terminal_id_from_request(&FailingSerialize).expect_err("serialization error should fail");
    let missing_err = terminal_id_from_request(&json!({ "sessionId": "session-testcov-0072" }))
        .expect_err("missing terminal id should fail");

    assert!(serialize_err.to_string().contains("serialize failed"));
    assert!(missing_err.to_string().contains("Missing required field: terminalId"));
}

#[test]
fn terminal_response_helpers_round_trip_protocol_shapes() {
    let create = create_terminal_response("term-test".to_string());
    let output = terminal_output_response(
        "done".to_string(),
        true,
        Some(TerminalExitState { exit_code: Some(3), signal: Some("SIGTERM".to_string()) }),
    );
    let wait =
        wait_for_terminal_exit_response(TerminalExitState { exit_code: Some(4), signal: None });

    assert_eq!(terminal_id_from_response(&create).unwrap(), "term-test");
    assert_eq!(output.output, "done");
    assert!(output.truncated);
    assert_eq!(output.exit_status.as_ref().and_then(|status| status.exit_code), Some(3));
    assert_eq!(wait.exit_status.exit_code, Some(4));
}

#[test]
fn terminal_output_response_omits_exit_status_while_process_is_running() {
    let output = terminal_output_response("pending".to_string(), false, None);

    assert_eq!(output.output, "pending");
    assert!(!output.truncated);
    assert!(output.exit_status.is_none());
}

#[test]
fn response_record_helpers_render_invalid_or_missing_fields_as_null() {
    let missing = json!({});
    let malformed = json!({ "exitCode": "zero", "signal": 9 });

    assert_eq!(response_record_number_or_null(&missing, "exitCode"), "null");
    assert_eq!(response_record_string_or_null(&missing, "signal"), "null");
    assert_eq!(response_record_number_or_null(&malformed, "exitCode"), "null");
    assert_eq!(response_record_string_or_null(&malformed, "signal"), "null");
    assert_eq!(response_record_number_or_null(&FailingSerialize, "exitCode"), "null");
    assert_eq!(response_record_string_or_null(&FailingSerialize, "signal"), "null");
}

#[test]
fn empty_release_request_uses_shutdown_session_and_terminal_id() {
    let request = empty_release_request("term-shutdown");
    let record = request_record(&request).expect("request should serialize as record");

    assert_eq!(record.get("sessionId").and_then(Value::as_str), Some("shutdown"));
    assert_eq!(terminal_id_from_request(&request).unwrap(), "term-shutdown");
}

#[test]
fn signal_name_maps_known_and_unknown_unix_signals() {
    #[cfg(unix)]
    {
        assert_eq!(signal_name(libc::SIGABRT).as_deref(), Some("SIGABRT"));
        assert_eq!(signal_name(libc::SIGALRM).as_deref(), Some("SIGALRM"));
        assert_eq!(signal_name(libc::SIGHUP).as_deref(), Some("SIGHUP"));
        assert_eq!(signal_name(libc::SIGINT).as_deref(), Some("SIGINT"));
        assert_eq!(signal_name(libc::SIGKILL).as_deref(), Some("SIGKILL"));
        assert_eq!(signal_name(libc::SIGPIPE).as_deref(), Some("SIGPIPE"));
        assert_eq!(signal_name(libc::SIGQUIT).as_deref(), Some("SIGQUIT"));
        assert_eq!(signal_name(libc::SIGSEGV).as_deref(), Some("SIGSEGV"));
        assert_eq!(signal_name(libc::SIGTERM).as_deref(), Some("SIGTERM"));
        assert_eq!(signal_name(libc::SIGUSR1).as_deref(), Some("SIGUSR1"));
        assert_eq!(signal_name(libc::SIGUSR2).as_deref(), Some("SIGUSR2"));
        assert_eq!(signal_name(9_999).as_deref(), Some("SIG9999"));
    }
}

#[tokio::test]
async fn managed_terminal_tracks_output_truncation_and_exit_once() {
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let terminal = ManagedTerminal::new(5, command_tx);

    terminal.append_output(&[]);
    terminal.append_output("aé日".as_bytes());
    terminal.record_exit(exit_status_from_failed_wait());
    terminal.record_exit(exit_status_from_failed_wait());
    terminal.clear_output();
    let snapshot = terminal.snapshot();

    assert!(snapshot.truncated);
    assert!(snapshot.output.is_empty());
    assert_eq!(snapshot.exit_status.and_then(|status| status.exit_code), Some(1));
}

#[tokio::test]
async fn spawn_output_task_stops_on_reader_error() {
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let terminal = Arc::new(ManagedTerminal::new(8, command_tx));

    spawn_output_task(FailingReader, terminal.clone());
    tokio::task::yield_now().await;

    assert!(terminal.snapshot().output.is_empty());
}

#[tokio::test]
async fn default_confirm_execute_denies_when_prompt_unavailable() {
    if can_prompt_for_permission() {
        return;
    }

    let approved = default_confirm_execute("echo blocked".to_string())
        .await
        .expect("prompt helper should return denial without a tty");

    assert!(!approved);
}

#[tokio::test]
async fn kill_process_errors_when_controller_channel_is_closed() {
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    drop(command_rx);
    let terminal = Arc::new(ManagedTerminal::new(8, command_tx));
    let manager = approve_all_manager(PathBuf::from("/tmp"));

    let err = manager
        .kill_process(&terminal)
        .await
        .expect_err("closed controller should fail before exit is recorded");

    assert!(err.to_string().contains("terminal process controller unavailable"));
}

#[tokio::test]
async fn run_terminal_process_stops_when_command_channel_closes() {
    let root = unique_temp_dir("closed-command");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let mut child = build_spawn_command("sh", &HashMap::new());
    child
        .arg("-c")
        .arg("sleep 30")
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let child = child.spawn().expect("spawn child");
    let (terminal_tx, _terminal_rx) = mpsc::unbounded_channel();
    let terminal = Arc::new(ManagedTerminal::new(8, terminal_tx));
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    drop(command_tx);

    run_terminal_process(child, terminal.clone(), command_rx, 1).await;

    assert!(!terminal.has_exited());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn kill_child_process_returns_when_signal_cannot_be_sent() {
    let root = unique_temp_dir("signal-missing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let mut child = build_spawn_command("sh", &HashMap::new());
    child
        .arg("-c")
        .arg("true")
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = child.spawn().expect("spawn child");
    let _ = child.wait().await.expect("child should exit");
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let terminal = Arc::new(ManagedTerminal::new(8, command_tx));

    kill_child_process(&mut child, &terminal, 1).await;

    assert!(!terminal.has_exited());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn kill_child_process_returns_when_terminal_already_exited() {
    let root = unique_temp_dir("already-exited");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let mut child = build_spawn_command("sh", &HashMap::new());
    child
        .arg("-c")
        .arg("sleep 30")
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = child.spawn().expect("spawn child");
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let terminal = Arc::new(ManagedTerminal::new(8, command_tx));
    terminal.record_exit(exit_status_from_failed_wait());

    kill_child_process(&mut child, &terminal, 1).await;
    let _ = child.start_kill();
    let _ = child.wait().await;

    assert!(terminal.has_exited());
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[tokio::test]
async fn kill_child_process_escalates_to_kill_when_term_is_ignored() {
    let root = unique_temp_dir("signal-kill");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let mut child = build_spawn_command("perl", &HashMap::new());
    child
        .arg("-e")
        .arg("$|=1; $SIG{TERM}='IGNORE'; print \"ready\\n\"; while (1) { sleep 1 }")
        .current_dir(&root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = child.spawn().expect("spawn child");
    let mut stdout = child.stdout.take().expect("stdout should be piped");
    let mut ready = [0_u8; 6];
    stdout.read_exact(&mut ready).await.expect("child should become ready");
    let (command_tx, _command_rx) = mpsc::unbounded_channel();
    let terminal = Arc::new(ManagedTerminal::new(8, command_tx));

    kill_child_process(&mut child, &terminal, 100).await;

    assert!(terminal.has_exited());
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn create_wait_output_and_release_terminal_successfully() {
    let root = unique_temp_dir("lifecycle");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = approve_all_manager(root.clone());

    let create_response = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "printf \"$TESTCOV_VALUE\"".to_string()])
                .env(vec![acp::EnvVariable::new("TESTCOV_VALUE", "terminal-ok")])
                .cwd(root.clone()),
        )
        .await
        .expect("terminal should start");
    let terminal_id = create_response.terminal_id.clone();

    let wait_response = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal should exit");
    let output_response = manager
        .terminal_output(&acp::TerminalOutputRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal output should be available");
    manager
        .release_terminal(&acp::ReleaseTerminalRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("release should succeed");
    let err = manager
        .terminal_output(&acp::TerminalOutputRequest::new("session-testcov-0072", terminal_id))
        .await
        .expect_err("released terminal should be unknown");

    assert_eq!(wait_response.exit_status.exit_code, Some(0));
    assert_eq!(output_response.output, "terminal-ok");
    assert!(!output_response.truncated);
    assert_eq!(output_response.exit_status.as_ref().and_then(|status| status.exit_code), Some(0));
    assert!(err.to_string().contains("Unknown terminal"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn create_terminal_reports_cwd_permission_and_spawn_errors() {
    let root = unique_temp_dir("create-errors");
    let outside = unique_temp_dir("create-outside");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&root).expect("create temp root");
    std::fs::create_dir_all(&outside).expect("create outside root");
    let manager = approve_all_manager(root.clone());

    let cwd_err = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .cwd(PathBuf::from("relative")),
        )
        .await
        .expect_err("relative cwd should fail");
    let spawn_err = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new(
                "session-testcov-0072",
                "definitely-missing-vw-acp-terminal-command",
            )
            .cwd(root.clone()),
        )
        .await
        .expect_err("missing command should fail");

    assert!(cwd_err.to_string().contains("cwd must be absolute"));
    assert!(!spawn_err.to_string().is_empty());

    // 默认 confirm + Fail 策略仅在无法弹出交互提示时直接报错；存在 TTY 时
    // create_terminal 会阻塞在 stdin 读取上，因此与同类测试一致地跳过该分支。
    if !can_prompt_for_permission() {
        let prompt_manager = TerminalManager::new(TerminalManagerOptions {
            cwd: root.clone(),
            permission_mode: PermissionMode::ApproveReads,
            non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
            ..TerminalManagerOptions::default()
        });
        let prompt_err = prompt_manager
            .create_terminal(
                &acp::CreateTerminalRequest::new("session-testcov-0072", "sh").cwd(outside.clone()),
            )
            .await
            .expect_err("unpromptable outside cwd should fail");
        assert!(prompt_err.to_string().contains("Permission prompt unavailable"));
    }

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

#[tokio::test]
async fn terminal_methods_reject_blank_and_unknown_terminal_ids() {
    let manager = approve_all_manager(PathBuf::from("/tmp"));

    let output_blank = manager
        .terminal_output(&acp::TerminalOutputRequest::new("session-testcov-0072", "   "))
        .await
        .expect_err("blank output id should fail");
    let wait_blank = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            "   ",
        ))
        .await
        .expect_err("blank wait id should fail");
    let kill_blank = manager
        .kill_terminal(&acp::KillTerminalRequest::new("session-testcov-0072", "   "))
        .await
        .expect_err("blank kill id should fail");
    let release_blank = manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", "   "))
        .await
        .expect_err("blank release id should fail");
    let wait_unknown = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            "missing",
        ))
        .await
        .expect_err("unknown wait id should fail");
    let kill_unknown = manager
        .kill_terminal(&acp::KillTerminalRequest::new("session-testcov-0072", "missing"))
        .await
        .expect_err("unknown kill id should fail");

    assert!(output_blank.to_string().contains("Missing required field: terminalId"));
    assert!(wait_blank.to_string().contains("Missing required field: terminalId"));
    assert!(kill_blank.to_string().contains("Missing required field: terminalId"));
    assert!(release_blank.to_string().contains("Missing required field: terminalId"));
    assert!(wait_unknown.to_string().contains("Unknown terminal"));
    assert!(kill_unknown.to_string().contains("Unknown terminal"));
}

#[tokio::test]
async fn release_terminal_reports_controller_errors() {
    let root = unique_temp_dir("release-controller");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = approve_all_manager(root.clone());
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    drop(command_rx);
    manager
        .terminals
        .lock()
        .insert("broken".to_string(), Arc::new(ManagedTerminal::new(8, command_tx)));

    let err = manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", "broken"))
        .await
        .expect_err("closed controller should fail release");

    assert!(err.to_string().contains("terminal process controller unavailable"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn release_terminal_is_idempotent_for_unknown_terminal() {
    let root = unique_temp_dir("release-missing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let operations = Arc::new(Mutex::new(Vec::new()));
    let captured = operations.clone();
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some(Arc::new(move |operation| {
            captured.lock().push(operation);
        })),
        ..TerminalManagerOptions::default()
    });

    manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", "missing"))
        .await
        .expect("missing terminal release should be idempotent");

    assert!(operations.lock().iter().any(|operation| {
        operation.method == ClientOperationMethod::TerminalRelease
            && operation.status == ClientOperationStatus::Completed
            && operation.details.as_deref() == Some("already released")
    }));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn shutdown_releases_tracked_terminals() {
    let root = unique_temp_dir("shutdown");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        kill_grace_ms: Some(10),
        ..TerminalManagerOptions::default()
    });
    let create_response = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "sleep 30".to_string()])
                .cwd(root.clone()),
        )
        .await
        .expect("terminal should start");
    let terminal_id = create_response.terminal_id.clone();

    manager.shutdown().await;
    let err = manager
        .terminal_output(&acp::TerminalOutputRequest::new("session-testcov-0072", terminal_id))
        .await
        .expect_err("shutdown should release tracked terminal");

    assert!(err.to_string().contains("Unknown terminal"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn terminal_output_respects_output_byte_limit() {
    let root = unique_temp_dir("limit");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = approve_all_manager(root.clone());
    let create_response = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "printf abcdef".to_string()])
                .output_byte_limit(3)
                .cwd(root.clone()),
        )
        .await
        .expect("terminal should start");
    let terminal_id = create_response.terminal_id.clone();

    let _ = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal should exit");
    let output_response = manager
        .terminal_output(&acp::TerminalOutputRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal output should be available");
    manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", terminal_id))
        .await
        .expect("release should succeed");

    assert_eq!(output_response.output, "def");
    assert!(output_response.truncated);
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn kill_terminal_stops_long_running_process() {
    let root = unique_temp_dir("kill");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        kill_grace_ms: Some(10),
        ..TerminalManagerOptions::default()
    });
    let create_response = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "sleep 30".to_string()])
                .cwd(root.clone()),
        )
        .await
        .expect("terminal should start");
    let terminal_id = create_response.terminal_id.clone();

    manager
        .kill_terminal(&acp::KillTerminalRequest::new("session-testcov-0072", terminal_id.clone()))
        .await
        .expect("kill should succeed");
    let exit_response = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal should exit after kill");
    manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", terminal_id))
        .await
        .expect("release should succeed");

    assert!(
        exit_response.exit_status.exit_code.is_some() || exit_response.exit_status.signal.is_some()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn deny_all_rejects_terminal_create() {
    let root = unique_temp_dir("deny");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::DenyAll,
        ..TerminalManagerOptions::default()
    });

    let err = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "printf denied".to_string()])
                .cwd(root.clone()),
        )
        .await
        .expect_err("deny all should block create");

    assert!(err.to_string().contains("Permission denied"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn custom_confirm_execute_controls_approve_reads() {
    let root = unique_temp_dir("confirm");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveReads,
        confirm_execute: Some(Arc::new(|command_line| {
            Box::pin(async move { Ok(command_line.contains("allowed")) })
        })),
        ..TerminalManagerOptions::default()
    });

    assert!(manager.is_execute_approved(&root, "echo allowed").await.expect("confirm should run"));
    assert!(!manager.is_execute_approved(&root, "echo denied").await.expect("confirm should run"));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn terminal_manager_emits_running_and_completed_operations() {
    let root = unique_temp_dir("ops");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create temp root");
    let operations = Arc::new(Mutex::new(Vec::new()));
    let captured = operations.clone();
    let manager = TerminalManager::new(TerminalManagerOptions {
        cwd: root.clone(),
        permission_mode: PermissionMode::ApproveAll,
        on_operation: Some(Arc::new(move |operation| {
            captured.lock().push(operation);
        })),
        ..TerminalManagerOptions::default()
    });

    let create_response = manager
        .create_terminal(
            &acp::CreateTerminalRequest::new("session-testcov-0072", "sh")
                .args(vec!["-c".to_string(), "printf ops".to_string()])
                .cwd(root.clone()),
        )
        .await
        .expect("terminal should start");
    let terminal_id = create_response.terminal_id.clone();
    let _ = manager
        .wait_for_terminal_exit(&acp::WaitForTerminalExitRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal should exit");
    let _ = manager
        .terminal_output(&acp::TerminalOutputRequest::new(
            "session-testcov-0072",
            terminal_id.clone(),
        ))
        .await
        .expect("terminal output should be available");
    manager
        .release_terminal(&acp::ReleaseTerminalRequest::new("session-testcov-0072", terminal_id))
        .await
        .expect("release should succeed");

    let operations = operations.lock();
    assert!(operations.iter().any(|operation| {
        operation.method == ClientOperationMethod::TerminalCreate
            && operation.status == ClientOperationStatus::Running
    }));
    assert!(operations.iter().any(|operation| {
        operation.method == ClientOperationMethod::TerminalCreate
            && operation.status == ClientOperationStatus::Completed
            && operation
                .details
                .as_deref()
                .is_some_and(|details| details.contains("terminalId=term_"))
    }));
    assert!(operations.iter().any(|operation| {
        operation.method == ClientOperationMethod::TerminalOutput
            && operation.status == ClientOperationStatus::Completed
    }));
    let _ = std::fs::remove_dir_all(root);
}
