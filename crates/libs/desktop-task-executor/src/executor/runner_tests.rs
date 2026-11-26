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
    use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};

    static ENDPOINT: std::sync::Mutex<Option<vw_gateway_client::GatewayEndpoint>> = std::sync::Mutex::new(None);
    fn endpoint() -> vw_gateway_client::GatewayEndpoint { ENDPOINT.lock().unwrap().clone().expect("endpoint") }

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
        *ENDPOINT.lock().unwrap() = Some(desktop_config::config_gateway::gateway_client_endpoint_from_config(&config));
        let path =
            config_runtime_paths::home_config_dir(home).join("gateway-client-bootstrap.json");
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
        let mut task = desktop_task_model::Task::new(1);
        task.id = "T-runner-lock".to_string();

        {
            let _guard =
                super::super::lock_merge_target_via_gateway(endpoint, project_path, &task, "main", None)
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
#[test]
fn task_pool_desktop_background_execution_inherits_agent_permissions() {
    let options = super::gateway_prompt_options_for_mode(
        "/tmp/project",
        false,
        super::GatewayPromptSessionMode::Fresh,
    );

    assert_eq!(options.get("cwd").and_then(serde_json::Value::as_str), Some("/tmp/project"));
    assert!(!options.contains_key("full_access"));
    assert!(!options.contains_key("acp_test"));
    assert!(!options.contains_key("acp_permission_mode"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn gateway_prompt_options_uses_approval_for_background_acp_agents() {
    let options = super::gateway_prompt_options_for_mode(
        "/tmp/project",
        true,
        super::GatewayPromptSessionMode::Fresh,
    );

    assert!(!options.contains_key("full_access"));
    assert_eq!(options["acp_background"], true);
    assert_eq!(
        options.get("acp_permission_mode").and_then(serde_json::Value::as_str),
        Some("approve-reads")
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
    use vw_api_chat_identity::ReasoningEffortDto;

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
        assert_eq!(request.agent.as_deref(), Some("acp-codex"), "{stage}");
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
    assert_eq!(non_acp.reasoning_effort, Some(ReasoningEffortDto::Xhigh));
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
fn normalize_gateway_path_is_stable() {
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
fn gateway_task_stream_request_uses_selected_acp_backend_native_model() {
    use vw_api_acp_model::acp_model_source::AcpModelSource;

    let request = super::gateway_task_stream_request(
        "task-plan-session",
        "/tmp/project",
        desktop_task_model::TASK_MODEL_ACP_NATIVE,
        "plan task",
        Some("reviewer".to_owned()),
        Some("codex".to_owned()),
        None,
        super::GatewayPromptSessionMode::Fresh,
    );
    assert_eq!(request.agent.as_deref(), Some("acp-codex"));
    assert_eq!(request.acp_model_source, AcpModelSource::CliLocal);
    assert_eq!(request.model, None);
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
        Some(vw_api_chat_identity::ReasoningEffortDto::Xhigh),
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
    assert_eq!(value["reasoning_effort"], "xhigh");
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
        gateway_chat_stream::GatewayChatStreamEvent::Delta("first\npartial".to_string()),
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
        gateway_chat_stream::GatewayChatStreamEvent::Done {
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
            gateway_chat_stream::GatewayChatStreamEvent::Delta(delta.to_string()),
            &mut visible_filter,
            &mut output,
            &mut pending,
            &mut stream_error,
            &mut done_seen,
            Some(&tx),
        ));
    }
    assert!(super::handle_gateway_stream_event(
        gateway_chat_stream::GatewayChatStreamEvent::Done {
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
        gateway_chat_stream::GatewayChatStreamEvent::Error("backend failed".to_string()),
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
            gateway_chat_stream::GatewayChatStreamEvent::Other(payload),
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
mod agent_routing {
use super::super::{resolve_task_execution_acp_agent, resolve_task_execution_delegate_agent};
use desktop_task_model::Task;
#[test]
fn resolve_task_execution_acp_agent_prefers_selected_agent() {
    let task = Task {
        acp_agent: Some("claude".to_string()),
        ..Default::default()
    };

    let acp_agent = resolve_task_execution_acp_agent(&task);

    assert_eq!(acp_agent.as_deref(), Some("claude"));
}

#[test]
fn resolve_task_execution_acp_agent_keeps_unknown_agent() {
    let task = Task {
        acp_agent: Some("custom-acp".to_string()),
        ..Default::default()
    };

    let acp_agent = resolve_task_execution_acp_agent(&task);

    assert_eq!(acp_agent.as_deref(), Some("custom-acp"));
}

#[test]
fn resolve_task_execution_acp_agent_respects_explicit_internal_selection() {
    let task = Task {
        acp_agent: Some("internal".to_string()),
        ..Default::default()
    };

    let acp_agent = resolve_task_execution_acp_agent(&task);

    assert_eq!(acp_agent, None);
}

#[test]
fn resolve_task_execution_acp_agent_does_not_fallback_when_missing() {
    let task = Task::default();

    let acp_agent = resolve_task_execution_acp_agent(&task);

    assert_eq!(acp_agent, None);
}

#[test]
fn resolve_task_execution_delegate_agent_drops_default_main_for_acp_agent() {
    let task = Task {
        agent: Some(desktop_task_model::TASK_AGENT_MAIN.to_string()),
        acp_agent: Some("codex".to_string()),
        ..Default::default()
    };

    let delegate_agent = resolve_task_execution_delegate_agent(&task);

    assert_eq!(delegate_agent, None);
}

#[test]
fn resolve_task_execution_delegate_agent_keeps_explicit_agent_for_acp_agent() {
    let task = Task {
        agent: Some("reviewer-agent".to_string()),
        acp_agent: Some("codex".to_string()),
        ..Default::default()
    };

    let delegate_agent = resolve_task_execution_delegate_agent(&task);

    assert_eq!(delegate_agent.as_deref(), Some("reviewer-agent"));
}

#[test]
fn resolve_task_execution_delegate_agent_uses_main_without_acp_agent() {
    let task = Task::default();

    let delegate_agent = resolve_task_execution_delegate_agent(&task);

    assert_eq!(delegate_agent.as_deref(), Some(desktop_task_model::TASK_AGENT_MAIN));
}

}
