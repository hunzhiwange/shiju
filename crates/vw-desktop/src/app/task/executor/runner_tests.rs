#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("runner_tests"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn retryable_gateway_error_accepts_git_index_lock_conflict() {
    let error = "gateway request failed: 400 Bad Request {\"error\":\"fatal: Unable to create '/repo/.git/index.lock': File exists.\\n\\nAnother git process seems to be running\"}";

    assert!(super::is_retryable_gateway_error(error));
    assert!(super::is_retryable_gateway_error(
        "gateway request failed: 400 Bad Request {\"error\":\"fatal: Unable to write index.\"}"
    ));
    assert!(!super::is_retryable_gateway_error(
        "gateway request failed: 400 Bad Request {\"error\":\"source branch is required\"}"
    ));
    assert!(!super::is_retryable_gateway_error(
        "gateway request failed: 400 Bad Request {\"error\":\"CONFLICT (content): Merge conflict in src/lib.rs\\nAutomatic merge failed; fix conflicts and then commit the result.\"}"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_git_merge_retry_uses_ten_attempts_and_capped_exponential_backoff() {
    assert_eq!(super::GATEWAY_GIT_MERGE_MAX_ATTEMPTS, 10);
    assert_eq!(super::gateway_git_merge_retry_delay(1).as_secs(), 1);
    assert_eq!(super::gateway_git_merge_retry_delay(2).as_secs(), 2);
    assert_eq!(super::gateway_git_merge_retry_delay(3).as_secs(), 4);
    assert_eq!(super::gateway_git_merge_retry_delay(6).as_secs(), 30);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_error_classifiers_distinguish_spawn_continue_and_runtime_failures() {
    for error in [
        "Failed to spawn ACP process",
        "ACP command is empty",
        "No such file or directory (os error 2)",
        "transport failed: OS ERROR 2",
    ] {
        assert!(super::is_gateway_spawn_retry_error(error), "spawn error: {error}");
    }
    assert!(!super::is_gateway_spawn_retry_error(
        "agent tool could not find project file Cargo.toml"
    ));

    for error in [
        "model response timeout",
        "LLM response timeout",
        "idle timeout",
        "completed without response",
        "模型无响应",
        "没有任何响应",
    ] {
        assert!(super::is_gateway_continue_retry_error(error), "continue error: {error}");
    }
    assert!(!super::is_gateway_continue_retry_error("permission denied"));
}

#[cfg(not(target_arch = "wasm32"))]
mod task_board_execution_worktree_gateway_runner {
    use serde_json::{Value, json};
    use std::ffi::{OsStr, OsString};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use vw_config_types::ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

    #[derive(Debug)]
    struct RecordedRequest {
        path: String,
        body: Value,
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    struct TestGateway {
        _home: tempfile::TempDir,
        _home_guard: EnvGuard,
        _config_guard: EnvGuard,
        rx: std::sync::mpsc::Receiver<RecordedRequest>,
        handle: std::thread::JoinHandle<()>,
    }

    impl TestGateway {
        fn start(project_path: String) -> Self {
            let home = tempfile::tempdir().expect("home");
            let home_guard = EnvGuard::set("HOME", home.path());
            let config_guard = EnvGuard::remove("VIBEWINDOW_CONFIG");
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().expect("addr").port();
            write_gateway_bootstrap_config(home.path(), port);
            let (tx, rx) = std::sync::mpsc::channel();
            let handle = std::thread::spawn(move || {
                let mut scenario_requests = 0usize;
                while scenario_requests < 3 {
                    let (mut stream, _) = listener.accept().expect("accept");
                    let request = read_request(&mut stream);
                    let response = match request.path.as_str() {
                        "/v1/projects/resolve" => json!({
                            "project": {
                                "id": "project-runner",
                                "name": "Runner Project",
                                "directory": project_path,
                                "display_path": project_path,
                                "status": "ready",
                                "created_at_ms": 1,
                                "updated_at_ms": 2,
                                "git": {
                                    "is_repo": true,
                                    "has_uncommitted_changes": false
                                }
                            }
                        }),
                        "/v1/task-pool/worktrees/merge-lock" => json!({"locked": true}),
                        "/v1/task-pool/worktrees/merge-unlock" => json!({"released": true}),
                        _ => json!({}),
                    };
                    if request.path == "/v1/projects/resolve"
                        || request.path.starts_with("/v1/task-pool/worktrees/")
                    {
                        scenario_requests = scenario_requests.saturating_add(1);
                    }
                    tx.send(request).expect("record");
                    write_response(&mut stream, &response.to_string());
                }
            });
            Self { _home: home, _home_guard: home_guard, _config_guard: config_guard, rx, handle }
        }

        fn take_requests(self) -> Vec<RecordedRequest> {
            self.handle.join().expect("server");
            self.rx.try_iter().collect()
        }
    }

    fn write_gateway_bootstrap_config(home: &std::path::Path, port: u16) {
        let mut config = GatewayClientSystemSettingsConfig::default();
        config.set_servers(
            vec![GatewayClientServerConfig {
                id: "runner".to_string(),
                name: "Runner".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                bearer_token: "test-token".to_string(),
                ..GatewayClientServerConfig::default()
            }],
            "runner".to_string(),
        );
        let path =
            vw_config_types::paths::home_config_dir(home).join("gateway-client-bootstrap.json");
        std::fs::create_dir_all(path.parent().expect("config parent")).expect("config dir");
        std::fs::write(path, serde_json::to_string(&config).expect("json")).expect("config");
    }

    fn read_request(stream: &mut TcpStream) -> RecordedRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        let mut header_end = None;
        while header_end.is_none() {
            let read = stream.read(&mut buffer).expect("read");
            assert!(read > 0);
            bytes.extend_from_slice(&buffer[..read]);
            header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
        }
        let header_end = header_end.expect("headers");
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
        let request_line = headers.lines().next().expect("request line");
        let path = request_line.split_whitespace().nth(1).expect("path").to_string();
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let mut body = bytes[(header_end + 4)..].to_vec();
        while body.len() < content_length {
            let read = stream.read(&mut buffer).expect("body");
            assert!(read > 0);
            body.extend_from_slice(&buffer[..read]);
        }
        body.truncate(content_length);
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).expect("json body")
        };
        RecordedRequest { path, body }
    }

    fn write_response(stream: &mut TcpStream, body: &str) {
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).expect("write");
    }

    #[test]
    fn task_board_execution_worktree_gateway_runner_merge_lock_guard_unlocks_gateway_lock() {
        let project_dir = tempfile::tempdir().expect("project");
        let project_path = project_dir.path().to_str().expect("utf8");
        let gateway = TestGateway::start(project_path.to_string());
        let mut task = crate::app::task::Task::new(1);
        task.id = "T-runner-lock".to_string();

        {
            let _guard =
                super::super::lock_merge_target_via_gateway(project_path, &task, "main", None)
                    .expect("lock");
        }

        let requests = gateway
            .take_requests()
            .into_iter()
            .filter(|request| {
                request.path == "/v1/projects/resolve"
                    || request.path.starts_with("/v1/task-pool/worktrees/")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            requests.iter().map(|request| request.path.as_str()).collect::<Vec<_>>(),
            vec![
                "/v1/projects/resolve",
                "/v1/task-pool/worktrees/merge-lock",
                "/v1/task-pool/worktrees/merge-unlock",
            ]
        );
        assert_eq!(requests[1].body["task_id"], "T-runner-lock");
        assert_eq!(requests[1].body["target_branch"], "main");
        assert_eq!(requests[2].body["task_id"], "T-runner-lock");
        assert_eq!(requests[2].body["target_branch"], "main");
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn task_plan_root_path(home: &std::path::Path) -> std::path::PathBuf {
    vw_config_types::paths::task_plan_storage_dir(home)
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_prompt_options_grants_full_access_for_internal_design_route() {
    let options = super::gateway_prompt_options_for_mode(
        "/tmp/project",
        false,
        super::GatewayPromptSessionMode::Fresh,
    );

    assert_eq!(options.get("cwd").and_then(serde_json::Value::as_str), Some("/tmp/project"));
    assert_eq!(options.get("full_access").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(options.get("acp_test").and_then(serde_json::Value::as_bool), Some(false));
    assert!(!options.contains_key("acp_permission_mode"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_prompt_options_uses_approve_all_for_acp_agents() {
    let options = super::gateway_prompt_options_for_mode(
        "/tmp/project",
        true,
        super::GatewayPromptSessionMode::Fresh,
    );

    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-all")
    );
    assert_eq!(
        options.get("acp_force_new_session").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("acp_close_client_after_request").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        options.get("acp_history_strategy").and_then(serde_json::Value::as_str),
        Some("discard")
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_prompt_options_continue_existing_reuses_acp_session() {
    let options = super::gateway_prompt_options_for_mode(
        "/tmp/project",
        true,
        super::GatewayPromptSessionMode::ContinueExisting,
    );

    assert_eq!(
        options.get("acp_force_new_session").and_then(serde_json::Value::as_bool),
        Some(false)
    );
    assert!(!options.contains_key("acp_history_strategy"));
    assert!(!options.contains_key("acp_history_recent_count"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_reasoning_effort_is_reused_for_plan_execution_review_and_continue() {
    use vw_gateway_client::vw_api_types::ReasoningEffortDto;

    for (stage, mode) in [
        ("plan", super::GatewayPromptSessionMode::Fresh),
        ("execution", super::GatewayPromptSessionMode::Fresh),
        ("review", super::GatewayPromptSessionMode::Fresh),
        ("continue", super::GatewayPromptSessionMode::ContinueExisting),
    ] {
        let request = super::gateway_task_stream_request(
            &format!("task-{stage}"),
            "/tmp/project",
            "gpt-5",
            stage,
            None,
            Some("codex".to_string()),
            Some(ReasoningEffortDto::Xhigh),
            mode,
        );
        assert_eq!(request.reasoning_effort, Some(ReasoningEffortDto::Xhigh), "{stage}");
    }

    let non_acp = super::gateway_task_stream_request(
        "task-default",
        "/tmp/project",
        "gpt-5",
        "execution",
        None,
        None,
        Some(ReasoningEffortDto::Xhigh),
        super::GatewayPromptSessionMode::Fresh,
    );
    assert_eq!(non_acp.reasoning_effort, None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_gateway_idle_timeout_allows_ten_minutes_without_stream_events() {
    assert_eq!(super::task_gateway_idle_timeout(), std::time::Duration::from_secs(10 * 60));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_timeout_retry_sends_completion_check_in_same_session() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        Some(&sender),
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            if calls.len() == 1 {
                Err("gateway 调度失败: gateway stream idle timeout after 600s without response"
                    .to_string())
            } else {
                Ok("done".to_string())
            }
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    assert_eq!(
        calls,
        vec![
            ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh),
            (
                super::TASK_GATEWAY_CONTINUE_PROMPT.to_string(),
                super::GatewayPromptSessionMode::ContinueExisting
            )
        ]
    );
    let logs = receiver.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(|entry| matches!(
        entry,
        super::TaskLogStream::Stderr(message)
            if message.contains("task-board-T1")
                && message.contains(super::TASK_GATEWAY_CONTINUE_PROMPT)
    )));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_timeout_without_provider_history_replays_original_prompt() {
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        false,
        None,
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            if calls.len() == 1 {
                Err("gateway stream idle timeout after 600s without response".to_string())
            } else {
                Ok("done".to_string())
            }
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0], ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh));
    assert_eq!(calls[1].1, super::GatewayPromptSessionMode::ContinueExisting);
    assert!(calls[1].0.contains(super::TASK_GATEWAY_CONTINUE_PROMPT));
    assert!(calls[1].0.contains("## 原始任务上下文\ninitial prompt"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_timeout_retry_continues_multiple_times_before_succeeding() {
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            if calls.len() < 3 {
                Err("gateway stream idle timeout after 900s without response".to_string())
            } else {
                Ok("done".to_string())
            }
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].1, super::GatewayPromptSessionMode::Fresh);
    assert!(calls[1..].iter().all(|(prompt, mode)| prompt == super::TASK_GATEWAY_CONTINUE_PROMPT
        && *mode == super::GatewayPromptSessionMode::ContinueExisting));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_timeout_retry_exhausts_attempts_and_reports_last_error() {
    let mut calls = 0usize;

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |_prompt, _mode| {
            calls = calls.saturating_add(1);
            Err("gateway stream idle timeout after 900s without response".to_string())
        },
    );

    // 初始 1 次 + 最大 continue 重试次数。
    assert_eq!(calls, 1 + super::TASK_GATEWAY_CONTINUE_RETRY_MAX_ATTEMPTS);
    let error = result.expect_err("exhausted retries should fail");
    assert!(error.contains("重试"));
    assert!(error.contains("idle timeout"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_timeout_retry_stops_on_non_retryable_error_during_continue() {
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            if calls.len() == 1 {
                Err("gateway stream idle timeout after 900s without response".to_string())
            } else {
                Err("权限被拒绝".to_string())
            }
        },
    );

    // 首次空窗触发一次 continue 重试，重试遇到不可重试错误后立即停止。
    assert_eq!(calls.len(), 2);
    assert_eq!(result.as_ref().map_err(String::as_str), Err("权限被拒绝"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn timeout_wrapper_respects_initial_continue_existing_mode() {
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout_in_mode(
        "task-board-T1",
        "subtask=SUB-2",
        "initial prompt",
        super::GatewayPromptSessionMode::ContinueExisting,
        true,
        None,
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            Ok("done".to_string())
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    assert_eq!(
        calls,
        vec![("initial prompt".to_string(), super::GatewayPromptSessionMode::ContinueExisting)]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_idle_timeout_error_is_retryable() {
    assert!(super::is_gateway_continue_retry_error(
        "gateway stream idle timeout after 600s without response"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn non_timeout_error_does_not_retry_continue() {
    let mut calls = 0usize;

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |_prompt, _mode| {
            calls = calls.saturating_add(1);
            Err("权限被拒绝".to_string())
        },
    );

    assert_eq!(result.as_ref().map(String::as_str).map_err(String::as_str), Err("权限被拒绝"));
    assert_eq!(calls, 1);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn acp_spawn_no_such_file_error_is_spawn_retryable() {
    assert!(super::is_gateway_spawn_retry_error("acp: No such file or directory (os error 2)"));
    assert!(super::is_gateway_spawn_retry_error(
        "failed to spawn acp agent process: No such file or directory (os error 2)"
    ));
    assert!(super::is_gateway_spawn_retry_error("acp command is empty"));
    // 运行期超时与权限错误不应被当作启动失败。
    assert!(!super::is_gateway_spawn_retry_error("gateway stream idle timeout"));
    assert!(!super::is_gateway_spawn_retry_error("权限被拒绝"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn spawn_failure_restarts_with_fresh_session_and_original_prompt() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        Some(&sender),
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            if calls.len() == 1 {
                Err("acp: No such file or directory (os error 2)".to_string())
            } else {
                Ok("done".to_string())
            }
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    // 重试必须沿用原始提示并以全新会话重新拉起进程，而不是 continue 死会话。
    assert_eq!(
        calls,
        vec![
            ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh),
            ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh),
        ]
    );
    let logs = receiver.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(|entry| matches!(
        entry,
        super::TaskLogStream::Stderr(message)
            if message.contains("ACP 进程启动失败") && message.contains("task-board-T1")
    )));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn spawn_failure_exhausts_attempts_and_reports_error() {
    let mut calls = 0usize;

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |_prompt, _mode| {
            calls = calls.saturating_add(1);
            Err("acp: No such file or directory (os error 2)".to_string())
        },
    );

    // 初始 1 次 + 最大 spawn 重试次数。
    assert_eq!(calls, 1 + super::TASK_GATEWAY_SPAWN_RETRY_MAX_ATTEMPTS);
    let error = result.expect_err("exhausted spawn retries should fail");
    assert!(error.contains("os error 2"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn spawn_failure_then_idle_timeout_falls_through_to_continue_retry() {
    let mut calls = Vec::new();

    let result = super::execute_prompt_with_continue_retry_on_timeout(
        "task-board-T1",
        "SUB-1",
        "initial prompt",
        true,
        None,
        |prompt, mode| {
            calls.push((prompt.to_string(), mode));
            match calls.len() {
                1 => Err("acp: No such file or directory (os error 2)".to_string()),
                2 => Err("gateway stream idle timeout after 600s without response".to_string()),
                _ => Ok("done".to_string()),
            }
        },
    );

    assert_eq!(result.as_deref(), Ok("done"));
    // 先因启动失败重新拉起一次（Fresh），随后空窗超时切换到 continue。
    assert_eq!(
        calls,
        vec![
            ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh),
            ("initial prompt".to_string(), super::GatewayPromptSessionMode::Fresh),
            (
                super::TASK_GATEWAY_CONTINUE_PROMPT.to_string(),
                super::GatewayPromptSessionMode::ContinueExisting
            ),
        ]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn write_task_plan_files_uses_task_folder_with_subtask_files() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T20260529.0001".to_string();
    task.prompt = "Implement task pool split".to_string();

    let mut first = crate::app::task::SubTask::new("Create plan structure".to_string());
    first.id = "SUB-one".to_string();
    first.boundary = "Only plan file layout".to_string();
    first.acceptance_criteria = vec!["plan.md exists".to_string()];
    first.target_files = vec!["crates/vw-desktop/src/app/task/executor/runner.rs".to_string()];
    let mut second = crate::app::task::SubTask::new("Execute subtask request".to_string());
    second.id = "SUB-two".to_string();
    task.subtasks = vec![first, second];

    super::write_task_plan_files_in_home(home.path(), &task).expect("plan files should write");

    let plan_root = task_plan_root_path(home.path());
    let task_dir = plan_root.join("task-T20260529-0001");
    assert!(task_dir.join("plan.md").exists());
    assert!(task_dir.join("001-SUB-one.md").exists());
    assert!(task_dir.join("002-SUB-two.md").exists());
    assert!(!plan_root.join("plan.md").exists());
    assert!(!plan_root.join("T20260529.0001.md").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_task_plan_started_at_uses_readable_seconds() {
    assert_eq!(super::format_task_plan_started_at(None), "-");
    assert_eq!(super::format_task_plan_started_at(Some(0)), "1970-01-01 00:00:00");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_task_plan_started_at_rejects_out_of_range_timestamp() {
    assert_eq!(super::format_task_plan_started_at(Some(u64::MAX)), "-");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn flush_gateway_output_lines_emits_complete_lines_and_forced_tail() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut pending = "first\r\nsecond".to_string();

    super::flush_gateway_output_lines(&mut pending, Some(&tx), false);
    assert_eq!(pending, "second");
    super::flush_gateway_output_lines(&mut pending, Some(&tx), true);
    assert!(pending.is_empty());

    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(value) if value == "first"));
    assert!(matches!(&logs[1], super::TaskLogStream::Stdout(value) if value == "second"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn flush_gateway_output_lines_skips_blank_lines() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut pending = "\n  \nvalue\n   ".to_string();

    super::flush_gateway_output_lines(&mut pending, Some(&tx), true);

    let logs = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(logs.len(), 1);
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(value) if value == "value"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_stream_requires_done_before_success() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut pending = "partial output".to_string();

    let result = super::ensure_gateway_stream_done(false, &mut pending, Some(&tx));

    assert_eq!(
        result.as_ref().map(|_| ()).map_err(String::as_str),
        Err("gateway 流式响应未完成：连接在 chat.done 前关闭")
    );
    assert!(pending.is_empty());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(value) if value == "partial output"));
    assert!(matches!(
        &logs[1],
        super::TaskLogStream::ExitStatus { success: false, code: None, signal: None }
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_plan_paths_use_user_storage_directory() {
    let home = std::path::Path::new("/tmp/vibewindow-home");
    let root = super::task_plan_root_dir(home);
    let dir = super::task_plan_dir(home, "T1.unsafe/中文");
    let file = super::task_plan_file_path(home, "T1.unsafe/中文");

    let plan_root = vw_config_types::paths::task_plan_storage_dir(home);
    let task_dir = plan_root.join("task-T1-unsafe");
    assert_eq!(root, plan_root);
    assert_eq!(dir, task_dir);
    assert_eq!(file, task_dir.join("plan.md"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn sanitize_task_plan_file_part_removes_unsafe_characters() {
    assert_eq!(super::sanitize_task_plan_file_part("SUB.1 abc/中文"), "SUB-1abc");
    assert_eq!(super::sanitize_task_plan_file_part("中文/ /"), "subtask");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn subtask_plan_file_name_uses_one_based_index_and_sanitized_id() {
    let mut subtask = crate::app::task::SubTask::new("Do work".to_string());
    subtask.id = "SUB.001/unsafe".to_string();

    assert_eq!(super::subtask_plan_file_name(0, &subtask), "001-SUB-001unsafe.md");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_task_plan_duration_handles_none_seconds_and_minutes() {
    assert_eq!(super::format_task_plan_duration(None), "-");
    assert_eq!(super::format_task_plan_duration(Some(999)), "0s");
    assert_eq!(super::format_task_plan_duration(Some(65_000)), "1m5s");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn subtask_status_label_covers_all_statuses() {
    use crate::app::task::SubTaskStatus;

    assert_eq!(super::subtask_status_label(SubTaskStatus::Pending), "pending");
    assert_eq!(super::subtask_status_label(SubTaskStatus::Running), "running");
    assert_eq!(super::subtask_status_label(SubTaskStatus::Completed), "completed");
    assert_eq!(super::subtask_status_label(SubTaskStatus::Failed), "failed");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_markdown_list_uses_empty_text_or_trimmed_items() {
    assert_eq!(super::format_markdown_list(&[], "none"), "- none\n");
    assert_eq!(
        super::format_markdown_list(&[" one ".to_string(), "two".to_string()], "none"),
        "- one\n- two\n"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_task_plan_markdown_lists_subtasks_and_metadata() {
    let mut task = crate::app::task::Task::new(1);
    task.id = "T1".to_string();
    task.description = "Test plan".to_string();
    task.prompt = "Original prompt".to_string();
    task.status = crate::app::task::TaskStatus::Planning;
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();
    subtask.boundary = "Only first".to_string();
    subtask.target_files = vec!["src/lib.rs".to_string()];
    task.subtasks = vec![subtask];

    let markdown = super::build_task_plan_markdown(&task);

    assert!(markdown.contains("# T1"));
    assert!(markdown.contains("- 状态: 任务拆分"));
    assert!(markdown.contains("Original prompt"));
    assert!(markdown.contains("[First](001-SUB-one.md)"));
    assert!(markdown.contains("目标文件: src/lib.rs"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_plan_markdown_includes_subtask_batches() {
    let mut task = crate::app::task::Task::new(1);
    task.id = "T1".to_string();
    task.prompt = "Prompt".to_string();

    let mut first = crate::app::task::SubTask::new("First".to_string());
    first.id = "SUB-one".to_string();
    first.batch = 1;

    let mut second = crate::app::task::SubTask::new("Second".to_string());
    second.id = "SUB-two".to_string();
    second.batch = 2;
    second.depends_on_batches = vec![1];
    task.subtasks = vec![first, second];

    let overview = super::build_task_plan_markdown(&task);
    assert!(overview.contains("批次: 1"));
    assert!(overview.contains("依赖批次: 无"));
    assert!(overview.contains("批次: 2"));
    assert!(overview.contains("依赖批次: 1"));

    let second_md = super::build_subtask_plan_markdown(&task, &task.subtasks[1], 1, 2);
    assert!(second_md.contains("- 批次: 2"));
    assert!(second_md.contains("- 依赖批次: 1"));

    let first_md = super::build_subtask_plan_markdown(&task, &task.subtasks[0], 0, 2);
    assert!(first_md.contains("- 批次: 1"));
    assert!(first_md.contains("- 依赖批次: 无"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_subtask_plan_markdown_uses_defaults_for_empty_fields() {
    let mut task = crate::app::task::Task::new(1);
    task.id = "T1".to_string();
    task.prompt = "Prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();

    let markdown = super::build_subtask_plan_markdown(&task, &subtask, 0, 1);

    assert!(markdown.contains("# 子任务 001: First"));
    assert!(markdown.contains("- 任务ID: T1"));
    assert!(markdown.contains("完成本子任务标题所描述的范围"));
    assert!(markdown.contains("- 未指定，由执行者按边界判断"));
    assert!(markdown.contains("- 完成后能用本子任务边界中的行为或检查方式验证"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn write_task_plan_files_removes_stale_markdown_files() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-stale".to_string();
    task.prompt = "Prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();
    task.subtasks = vec![subtask];
    let task_dir = task_plan_root_path(home.path()).join("task-T-stale");
    std::fs::create_dir_all(&task_dir).expect("task dir should exist");
    std::fs::write(task_dir.join("999-stale.md"), "old").expect("stale md should be written");
    std::fs::write(task_dir.join("notes.txt"), "keep").expect("txt should be written");

    super::write_task_plan_files_in_home(home.path(), &task).expect("plan files should write");

    assert!(!task_dir.join("999-stale.md").exists());
    assert!(task_dir.join("notes.txt").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_task_split_prompt_uses_create_task_plans_template_batches_and_fixed_output_dir() {
    let mut task = crate::app::task::Task::new(1);
    task.id = "T1".to_string();
    task.description = "Build tests".to_string();
    task.prompt = "Need coverage".to_string();

    let home = std::path::Path::new("/tmp/vibewindow-home");
    let plan_dir = super::task_split_plan_abs_dir(home, &task);
    let prompt = super::build_task_split_prompt(&task, &plan_dir);

    // 内置 runtime create-task-plans 语义，不再依赖根目录 skills。
    assert!(prompt.contains("create-task-plans-runtime"));
    assert!(prompt.contains("do-task-plans-runtime"));
    assert!(prompt.contains("crates/vw-agent/assets/skills/create-task-plans-runtime/SKILL.md"));
    assert!(prompt.contains("crates/vw-agent/assets/skills/do-task-plans-runtime/SKILL.md"));
    assert!(!prompt.contains("../../../../../../skills/"));
    assert!(!prompt.contains("只输出 JSON"));
    assert!(!prompt.contains("不要把一个任务机械拆成"));

    // 确定性计划目录与 create-task-plans 产物。
    assert!(prompt.contains(plan_dir.to_string_lossy().as_ref()));
    assert!(!prompt.contains(".plans/"));
    assert!(prompt.contains(&super::task_split_plan_dir_name(&task)));
    assert!(prompt.contains("plan.md"));
    assert!(prompt.contains("prompt.md"));
    assert!(prompt.contains("tasks.json"));
    assert!(prompt.contains("Markdown 文件一一对应"));

    // 批次与依赖批次规则。
    assert!(prompt.contains("当前批次"));
    assert!(prompt.contains("依赖批次"));
    assert!(prompt.contains("同一批次"));

    // 自动化排障：不询问用户，先用仓库上下文自行收敛。
    assert!(prompt.contains("不要询问用户"));

    // 工具护栏与任务身份。
    assert!(prompt.contains("node_repl"));
    assert!(prompt.contains("任务ID: T1"));
    assert!(prompt.contains("标题: Build tests"));
    assert!(prompt.contains("Need coverage"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_split_plan_dir_name_is_deterministic_from_task_id() {
    let mut task = crate::app::task::Task::new(1);
    task.id = "T20260628.0001".to_string();

    let name = super::task_split_plan_dir_name(&task);

    assert!(name.contains("T20260628-0001"));
    assert_eq!(name, super::task_split_plan_dir_name(&task));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn format_git_summary_and_normalize_gateway_path_are_stable() {
    assert_eq!(
        super::format_git_summary("ok", "skip", "done"),
        "Git动作摘要: add=ok; commit=skip; merge=done"
    );
    assert_eq!(super::normalize_gateway_path("C:\\repo\\worktree\\"), "C:/repo/worktree");
    assert_eq!(super::normalize_gateway_path("/repo/worktree///"), "/repo/worktree");
    assert_eq!(super::normalize_gateway_path(""), "");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_merge_conflict_classifier_blocks_conflicts_from_retry_policy() {
    for error in [
        "automatic merge failed; fix conflicts and then commit the result.",
        "merge conflict in src/lib.rs",
        "CONFLICT (content): Merge conflict in README.md",
        "存在冲突，请手动解决",
    ] {
        let normalized = error.to_ascii_lowercase();
        assert!(super::is_gateway_merge_conflict_error(&normalized), "{error}");
        assert!(!super::is_retryable_gateway_error(error), "{error}");
    }

    assert!(!super::is_gateway_merge_conflict_error("connection reset by peer"));
    assert!(super::is_retryable_gateway_error("connection reset by peer"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn subtask_dependency_batch_labels_preserve_order_and_empty_state() {
    assert_eq!(super::format_subtask_depends_on_batches(&[]), "无");
    assert_eq!(super::format_subtask_depends_on_batches(&[3, 1, 2]), "3, 1, 2");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn read_plan_context_falls_back_to_generated_markdown() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-fallback".to_string();
    task.prompt = "Prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();
    task.subtasks = vec![subtask.clone()];

    let (plan, subtask_plan) = super::read_plan_context(home.path(), &task, &subtask, 0, 1);

    assert!(plan.contains("# T-fallback"));
    assert!(subtask_plan.contains("# 子任务 001: First"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_plan_reads_ignore_legacy_project_cache() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let project = tempfile::TempDir::new().expect("temp project should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T.legacy/中文".to_string();
    task.prompt = "Current prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("Current subtask".to_string());
    subtask.id = "SUB-one".to_string();
    task.subtasks = vec![subtask.clone()];

    let legacy_dir = project.path().join(".plans").join("task-T-legacy");
    std::fs::create_dir_all(&legacy_dir).expect("legacy plan directory should be created");
    std::fs::write(legacy_dir.join("prompt.md"), "legacy prompt")
        .expect("legacy prompt should be written");
    std::fs::write(legacy_dir.join("plan.md"), "legacy plan")
        .expect("legacy plan should be written");
    std::fs::write(legacy_dir.join("001-SUB-one.md"), "legacy subtask")
        .expect("legacy subtask should be written");
    std::fs::write(legacy_dir.join("tasks.json"), "legacy tasks")
        .expect("legacy tasks should be written");

    let current_dir = super::task_split_plan_abs_dir(home.path(), &task);
    std::fs::create_dir_all(&current_dir).expect("current plan directory should be created");
    std::fs::write(current_dir.join("prompt.md"), "current prompt")
        .expect("current prompt should be written");

    let prompt = super::read_task_plan_prompt_md(home.path(), &task)
        .expect("current prompt should be read");
    let (plan, subtask_plan) =
        super::read_plan_context(home.path(), &task, &subtask, 0, task.subtasks.len());

    assert_eq!(prompt, "current prompt");
    assert!(plan.contains("Current prompt"));
    assert!(subtask_plan.contains("Current subtask"));
    assert!(!plan.contains("legacy plan"));
    assert!(!subtask_plan.contains("legacy subtask"));
    assert!(!current_dir.join("tasks.json").exists());
    assert_eq!(
        std::fs::read_to_string(legacy_dir.join("tasks.json"))
            .expect("legacy cache should remain untouched"),
        "legacy tasks"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn persist_subtask_status_updates_matching_subtask_and_writes_plan() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-status".to_string();
    task.prompt = "Prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();
    task.subtasks = vec![subtask];

    super::persist_subtask_status(
        home.path(),
        &mut task,
        "SUB-one",
        |subtask| {
            subtask.mark_completed();
        },
        None,
    );

    assert_eq!(task.subtasks[0].status, crate::app::task::SubTaskStatus::Completed);
    assert!(task_plan_root_path(home.path()).join("task-T-status").join("plan.md").exists());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn persist_subtask_status_ignores_missing_subtask_without_writing_plan() {
    let home = tempfile::TempDir::new().expect("temp home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-missing-status".to_string();
    task.prompt = "Prompt".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.id = "SUB-one".to_string();
    task.subtasks = vec![subtask];

    super::persist_subtask_status(
        home.path(),
        &mut task,
        "SUB-missing",
        |subtask| {
            subtask.mark_completed();
        },
        None,
    );

    assert_eq!(task.subtasks[0].status, crate::app::task::SubTaskStatus::Pending);
    assert!(
        !task_plan_root_path(home.path()).join("task-T-missing-status").join("plan.md").exists()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_subtask_execution_prompt_combines_contexts() {
    let mut task = crate::app::task::Task::new(1);
    task.prompt = "Original".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.batch = 1;

    let prompt =
        super::build_subtask_execution_prompt(&task, &subtask, " plan body ", " subtask body ");

    assert!(prompt.contains("# plan.md\nplan body"));
    assert!(prompt.contains("# 子任务.md\nsubtask body"));
    assert!(prompt.contains("# 原始需求\nOriginal"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_subtask_execution_prompt_embeds_do_task_plans_batches_without_install_requirement() {
    let mut task = crate::app::task::Task::new(1);
    task.prompt = "Original".to_string();
    let mut subtask = crate::app::task::SubTask::new("Second".to_string());
    subtask.batch = 2;
    subtask.depends_on_batches = vec![1];

    let prompt =
        super::build_subtask_execution_prompt(&task, &subtask, "plan body", "subtask body");

    // 内置 do-task-plans 流程，不要求用户项目安装技能。
    assert!(prompt.contains("do-task-plans"));
    assert!(prompt.contains("无需安装") || prompt.contains("不依赖") && prompt.contains("安装"));
    assert!(prompt.contains("内置 do-task-plans"));

    // 批次上下文：当前批次与依赖批次。
    assert!(prompt.contains("当前批次: 2"));
    assert!(prompt.contains("依赖批次: 1"));

    // 只读 plan.md 约束。
    assert!(prompt.contains("plan.md") && prompt.contains("只读"));

    // 自动化排障优先：能自行排障就继续，不要立即询问用户。
    assert!(prompt.contains("不要询问用户"));
    assert!(prompt.contains("自行"));

    // 阻塞记录写入对应编号任务文件，完成证据写入最终回复。
    assert!(prompt.contains("已阻塞"));
    assert!(prompt.contains("任务文件"));
    assert!(prompt.contains("完成证据") || prompt.contains("完成") && prompt.contains("证据"));
    assert!(prompt.contains("最终回复"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn build_subtask_execution_prompt_shows_empty_depends_on_batches_as_none() {
    let mut task = crate::app::task::Task::new(1);
    task.prompt = "Original".to_string();
    let mut subtask = crate::app::task::SubTask::new("First".to_string());
    subtask.batch = 1;

    let prompt = super::build_subtask_execution_prompt(&task, &subtask, "plan", "subtask");

    assert!(prompt.contains("当前批次: 1"));
    assert!(prompt.contains("依赖批次: 无"));
}

#[cfg(not(target_arch = "wasm32"))]
fn batch_subtask(
    id: &str,
    batch: u32,
    depends_on_batches: Vec<u32>,
    target_files: Vec<&str>,
) -> crate::app::task::SubTask {
    let mut subtask = crate::app::task::SubTask::new(id.to_string());
    subtask.id = id.to_string();
    subtask.batch = batch;
    subtask.depends_on_batches = depends_on_batches;
    subtask.target_files = target_files.into_iter().map(ToString::to_string).collect();
    subtask
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn batch_scheduler_groups_ready_subtasks_by_dependency_order() {
    let subtasks = vec![
        batch_subtask("a", 1, vec![], vec!["a.rs"]),
        batch_subtask("b", 1, vec![], vec!["b.rs"]),
        batch_subtask("c", 2, vec![1], vec!["c.rs"]),
        batch_subtask("d", 3, vec![1, 2], vec!["d.rs"]),
    ];

    let stages = super::build_batch_execution_stages(&subtasks).expect("topology should be valid");

    // 第一阶段是无依赖的同批两个子任务，可并发；后续阶段按批次号升序推进。
    assert_eq!(stages, vec![vec![0, 1], vec![2], vec![3]]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn batch_scheduler_rejects_cycles_or_forward_dependencies() {
    // 前向依赖：批次 1 依赖更大的批次 2。
    let forward = vec![
        batch_subtask("a", 1, vec![2], vec!["a.rs"]),
        batch_subtask("b", 2, vec![], vec!["b.rs"]),
    ];
    assert!(super::build_batch_execution_stages(&forward).is_err());

    // 自依赖：批次 1 依赖批次 1。
    let self_dep = vec![batch_subtask("a", 1, vec![1], vec!["a.rs"])];
    assert!(super::build_batch_execution_stages(&self_dep).is_err());

    // 依赖不存在的批次。
    let missing = vec![batch_subtask("a", 2, vec![1], vec!["a.rs"])];
    assert!(super::build_batch_execution_stages(&missing).is_err());

    // 空子任务集合。
    assert!(super::build_batch_execution_stages(&[]).is_err());

    // 同批显式目标文件冲突。
    let conflict = vec![
        batch_subtask("a", 1, vec![], vec!["shared.rs"]),
        batch_subtask("b", 1, vec![], vec!["shared.rs"]),
    ];
    assert!(super::build_batch_execution_stages(&conflict).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn batch_execution_uses_distinct_sessions_for_parallel_subtasks() {
    let first = super::subtask_session_id("T1", "SUB-a");
    let second = super::subtask_session_id("T1", "SUB-b");

    // 同任务不同子任务必须使用不同会话，避免并发复用同一个会话。
    assert_ne!(first, second);
    assert!(first.contains("task-board-T1"));
    assert!(first.contains("SUB-a"));
    assert!(second.contains("SUB-b"));
    // 同一子任务的会话 ID 是确定的。
    assert_eq!(first, super::subtask_session_id("T1", "SUB-a"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_create_task_plans_prompt_extracts_copyable_task_prompts_and_batches() {
    let prompt_md = "# 任务执行提示\n\n说明文字，不属于任何任务。\n\n\
## 任务 1: 状态机基础类型 【批次 1】 依赖批次 无\n\n\
```text\n\
请使用技能 ./skills/do-task-plans/SKILL.md 执行 `.plans/demo/1-状态机基础类型.md`。\n\n\
开始前先阅读 plan.md。\n\
```\n\n\
## 任务 2: 存储状态更新接入状态机 【批次 2】 依赖批次 1\n\n\
```text\n\
请执行 `.plans/demo/2-存储状态更新接入状态机.md`。\n\
```\n\n\
## 任务 3: Matrix 校验 【批次 3】 依赖批次 1,2\n\n\
```text\n\
请执行 `.plans/demo/3-matrix校验.md`。\n\
```\n";

    let parsed =
        super::parse_create_task_plans_prompt(prompt_md).expect("应能解析合法的 prompt.md");

    // 顺序与每个 `## 任务 N` 一一对应。
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].title, "状态机基础类型");
    assert_eq!(parsed[1].title, "存储状态更新接入状态机");
    assert_eq!(parsed[2].title, "Matrix 校验");

    // 批次解析。
    assert_eq!(parsed[0].batch, 1);
    assert_eq!(parsed[1].batch, 2);
    assert_eq!(parsed[2].batch, 3);

    // 依赖批次：无依赖、单依赖、多依赖。
    assert!(parsed[0].depends_on_batches.is_empty());
    assert_eq!(parsed[1].depends_on_batches, vec![1]);
    assert_eq!(parsed[2].depends_on_batches, vec![1, 2]);

    // 完整可复制执行提示来自紧随标题的 ```text 代码块。
    assert!(parsed[0].execution_prompt.contains("请使用技能 ./skills/do-task-plans/SKILL.md 执行"));
    assert!(parsed[0].execution_prompt.contains("1-状态机基础类型.md"));
    assert!(parsed[0].execution_prompt.contains("开始前先阅读 plan.md"));
    assert!(parsed[2].execution_prompt.contains("3-matrix校验.md"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_create_task_plans_prompt_dedupes_sorts_and_rejects_self_dependency() {
    let prompt_md = "## 任务 1: A 【批次 2】 依赖批次 2,1,1\n\n```text\n执行 A\n```\n";

    // 依赖批次包含当前批次（自依赖）必须明确失败。
    assert!(super::parse_create_task_plans_prompt(prompt_md).is_err());

    let ok = "## 任务 1: A 【批次 3】 依赖批次 2,1,1\n\n```text\n执行 A\n```\n";
    let parsed = super::parse_create_task_plans_prompt(ok).expect("应能解析");
    // 去重并升序排序。
    assert_eq!(parsed[0].depends_on_batches, vec![1, 2]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_create_task_plans_prompt_rejects_missing_code_block() {
    let prompt_md = "## 任务 1: A 【批次 1】 依赖批次 无\n\n这里缺少 text 代码块。\n";

    // 缺失代码块的任务必须返回明确错误，不伪造成功子任务。
    assert!(super::parse_create_task_plans_prompt(prompt_md).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_create_task_plans_prompt_rejects_empty_prompt() {
    // prompt.md 没有任何 `## 任务 N` 时明确失败。
    assert!(super::parse_create_task_plans_prompt("# 标题\n\n没有任务。\n").is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn execute_task_plan_reports_missing_prompt_md_explicitly() {
    let home = tempfile::tempdir().expect("临时 home");
    let mut task = crate::app::task::Task::new(1);
    task.prompt = "Original".to_string();

    // 计划目录存在但缺少 prompt.md：必须返回包含 prompt.md 路径的明确错误，
    // 不静默回退旧 JSON 解析。
    let plan_dir = super::task_split_plan_abs_dir(home.path(), &task);
    std::fs::create_dir_all(&plan_dir).expect("创建计划目录");

    let err = super::read_task_plan_prompt_md(home.path(), &task)
        .expect_err("缺失 prompt.md 必须报错");
    assert!(err.contains("prompt.md"), "错误信息应包含 prompt.md: {err}");
    assert!(err.contains(plan_dir.to_string_lossy().as_ref()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_task_stream_request_maps_delegate_route_and_auto_model() {
    let request = super::gateway_task_stream_request(
        "task-session",
        "/tmp/project",
        "auto",
        "run task",
        Some("delegate-agent".to_string()),
        None,
        Some(vw_gateway_client::vw_api_types::ReasoningEffortDto::Xhigh),
        super::GatewayPromptSessionMode::Fresh,
    );
    let value = serde_json::to_value(request).expect("request should serialize");

    assert_eq!(value["session_id"], "task-session");
    assert_eq!(
        value["messages"],
        serde_json::json!([{
            "role": "user",
            "content": "run task"
        }])
    );
    assert!(value.get("model").is_none());
    assert!(value.get("reasoning_effort").is_none());
    assert_eq!(value["agent"], "delegate-agent");
    assert!(value.get("acp_agent").is_none());
    assert_eq!(value["options"]["agent"], "delegate-agent");
    assert_eq!(value["options"]["cwd"], "/tmp/project");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_stream_delta_and_done_update_state_and_emit_lines() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut visible_filter = super::GatewayVisibleOutputFilter::default();
    let mut output = String::new();
    let mut pending = String::new();
    let mut stream_error = None;
    let mut done_seen = false;

    let keep_streaming = super::handle_gateway_stream_event(
        vw_gateway_client::GatewayChatStreamEvent::Delta("first\npartial".to_string()),
        &mut visible_filter,
        &mut output,
        &mut pending,
        &mut stream_error,
        &mut done_seen,
        Some(&tx),
    );
    assert!(keep_streaming);
    assert_eq!(output, "first\npartial");
    assert_eq!(pending, "partial");
    assert!(stream_error.is_none());
    assert!(!done_seen);

    let keep_streaming = super::handle_gateway_stream_event(
        vw_gateway_client::GatewayChatStreamEvent::Done {
            finish_reason: None,
            usage: None,
            message_id: None,
            parent_message_id: None,
        },
        &mut visible_filter,
        &mut output,
        &mut pending,
        &mut stream_error,
        &mut done_seen,
        Some(&tx),
    );
    assert!(keep_streaming);
    assert!(done_seen);
    assert!(pending.is_empty());

    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(line) if line == "first"));
    assert!(matches!(&logs[1], super::TaskLogStream::Stdout(line) if line == "partial"));
    assert!(matches!(
        &logs[2],
        super::TaskLogStream::Stdout(line) if line == "[GATEWAY] done finish_reason=unknown"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_stream_hides_reasoning_chunks_split_across_deltas() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut visible_filter = super::GatewayVisibleOutputFilter::default();
    let mut output = String::new();
    let mut pending = String::new();
    let mut stream_error = None;
    let mut done_seen = false;

    for delta in ["<thi", "nk>reasoning effort 用户消息", "</thi", "nk>\nanswer"] {
        assert!(super::handle_gateway_stream_event(
            vw_gateway_client::GatewayChatStreamEvent::Delta(delta.to_string()),
            &mut visible_filter,
            &mut output,
            &mut pending,
            &mut stream_error,
            &mut done_seen,
            Some(&tx),
        ));
    }
    assert!(super::handle_gateway_stream_event(
        vw_gateway_client::GatewayChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        },
        &mut visible_filter,
        &mut output,
        &mut pending,
        &mut stream_error,
        &mut done_seen,
        Some(&tx),
    ));

    assert_eq!(output.trim(), "answer");
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(!logs.iter().any(|entry| matches!(
        entry,
        super::TaskLogStream::Stdout(line) if line.contains("reasoning effort")
    )));
    assert!(
        logs.iter()
            .any(|entry| matches!(entry, super::TaskLogStream::Stdout(line) if line == "answer"))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_stream_error_flushes_tail_and_stops_streaming() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut visible_filter = super::GatewayVisibleOutputFilter::default();
    let mut output = "completed".to_string();
    let mut pending = "tail".to_string();
    let mut stream_error = None;
    let mut done_seen = false;

    let keep_streaming = super::handle_gateway_stream_event(
        vw_gateway_client::GatewayChatStreamEvent::Error("backend failed".to_string()),
        &mut visible_filter,
        &mut output,
        &mut pending,
        &mut stream_error,
        &mut done_seen,
        Some(&tx),
    );

    assert!(!keep_streaming);
    assert_eq!(output, "completed");
    assert!(pending.is_empty());
    assert_eq!(stream_error.as_deref(), Some("backend failed"));
    assert!(!done_seen);
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(line) if line == "tail"));
    assert!(matches!(
        &logs[1],
        super::TaskLogStream::Stderr(line) if line == "[GATEWAY] error backend failed"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_stream_step_events_emit_indexes_and_ignore_unknown_payloads() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut visible_filter = super::GatewayVisibleOutputFilter::default();
    let mut output = String::new();
    let mut pending = String::new();
    let mut stream_error = None;
    let mut done_seen = false;

    for payload in [
        serde_json::json!({"type": "chat.step_start", "step_index": 4}),
        serde_json::json!({"type": "chat.step_finish"}),
        serde_json::json!({"type": "chat.unknown", "step_index": 9}),
    ] {
        assert!(super::handle_gateway_stream_event(
            vw_gateway_client::GatewayChatStreamEvent::Other(payload),
            &mut visible_filter,
            &mut output,
            &mut pending,
            &mut stream_error,
            &mut done_seen,
            Some(&tx),
        ));
    }

    let logs = rx.try_iter().collect::<Vec<_>>();
    assert_eq!(logs.len(), 2);
    assert!(matches!(
        &logs[0],
        super::TaskLogStream::Stdout(line) if line == "[GATEWAY] step_start 4"
    ));
    assert!(matches!(
        &logs[1],
        super::TaskLogStream::Stdout(line) if line == "[GATEWAY] step_finish 0"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn fail_gateway_stream_flushes_pending_output_and_emits_failed_exit() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut pending = "last output".to_string();

    let result = super::fail_gateway_stream(&mut pending, Some(&tx), "dispatch failed".to_string());

    assert_eq!(result.as_ref().map(String::as_str).map_err(String::as_str), Err("dispatch failed"));
    assert!(pending.is_empty());
    let logs = rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(&logs[0], super::TaskLogStream::Stdout(line) if line == "last output"));
    assert!(matches!(
        &logs[1],
        super::TaskLogStream::ExitStatus { success: false, code: None, signal: None }
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_task_heading_supports_chinese_punctuation_and_sorted_dependencies() {
    let parsed = super::parse_task_heading("## 任务 7： 状态机收口 【批次 4】 依赖批次 3、1，2 1")
        .expect("heading should parse");

    assert_eq!(parsed.0, "状态机收口");
    assert_eq!(parsed.1, 4);
    assert_eq!(parsed.2, vec![1, 2, 3]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_task_heading_rejects_malformed_batch_and_dependencies() {
    for (heading, expected_error) in [
        ("## 工作 1: A 【批次 1】 依赖批次 无", "缺少“任务”前缀"),
        ("## 任务 1: A", "缺少“【批次 N】”"),
        ("## 任务 1: 【批次 1】 依赖批次 无", "任务标题为空"),
        ("## 任务 1: A 【批次 x】 依赖批次 无", "无法解析批次编号"),
        ("## 任务 1: A 【批次 2】 依赖批次 3", "不能大于当前批次"),
        ("## 任务 1: A 【批次 2】 依赖批次 x", "无法解析依赖批次"),
        ("## 任务 1: A 【批次 2", "格式不正确"),
    ] {
        let error = super::parse_task_heading(heading).expect_err("heading should be rejected");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?} in error for {heading:?}, got {error:?}"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_heading_detection_requires_hash_prefix_task_word_and_number() {
    assert!(super::is_task_heading("  ## 任务 12: Valid"));
    assert!(!super::is_task_heading("任务 12: Missing hashes"));
    assert!(!super::is_task_heading("## 任务说明"));
    assert!(!super::is_task_heading("## 工作 12: Wrong marker"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn parse_create_task_plans_prompt_rejects_unclosed_or_empty_code_blocks() {
    let unclosed = "## 任务 1: A 【批次 1】 依赖批次 无\n\n```text\n执行 A\n";
    let empty = "## 任务 1: A 【批次 1】 依赖批次 无\n\n```text\n  \n```\n";

    let unclosed_error =
        super::parse_create_task_plans_prompt(unclosed).expect_err("unclosed block should fail");
    assert!(unclosed_error.contains("代码块未闭合"));
    let empty_error =
        super::parse_create_task_plans_prompt(empty).expect_err("empty block should fail");
    assert!(empty_error.contains("执行提示代码块为空"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn read_task_plan_prompt_md_returns_exact_generated_content() {
    let home = tempfile::tempdir().expect("temporary home should be created");
    let mut task = crate::app::task::Task::new(1);
    task.id = "T-read-plan".to_string();
    let plan_dir = super::task_split_plan_abs_dir(home.path(), &task);
    std::fs::create_dir_all(&plan_dir).expect("plan directory should be created");
    std::fs::write(plan_dir.join("prompt.md"), "# generated\n\ncontent\n")
        .expect("prompt should be written");

    let content =
        super::read_task_plan_prompt_md(home.path(), &task).expect("prompt should be read");

    assert_eq!(content, "# generated\n\ncontent\n");
}
