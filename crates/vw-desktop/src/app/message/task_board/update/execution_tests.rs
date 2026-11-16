#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("execution_tests"));
}

#[tokio::test]
async fn task_pool_desktop_non_git_can_start_and_retry() {
    use iced_runtime::futures::futures::StreamExt;
    let gateway = DesktopRecordingGateway::start();
    let directory = tempfile::tempdir().unwrap();
    let (mut app, _) = crate::app::App::new();
    app.navigation_tabs.project_path = Some(directory.path().to_string_lossy().into_owned());
    app.workspace_state.project_is_git_repo = false;
    let prior_review_deadline = app.task_board.task_board_next_auto_review_tick_at_ms;
    let _ = super::update(&mut app, super::TaskBoardMessage::StartExecution);
    assert_eq!(app.task_board.task_board_next_auto_review_tick_at_ms, prior_review_deadline);
    assert!(app.task_board.task_board_executor_running);
    for status in [crate::app::task::TaskStatus::Pending, crate::app::task::TaskStatus::Failed] {
        let mut task = crate::app::task::Task::new(1);
        task.status = status;
        task.id = "desktop-task".into();
        app.task_board.task_board_tasks = vec![task];
        let command = super::update(
            &mut app,
            super::TaskBoardMessage::ExecuteTask { task_id: "desktop-task".into() },
        );
        let mut stream = iced_runtime::task::into_stream(command).unwrap();
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await.unwrap();
        match result {
            Some(iced_runtime::Action::Output(crate::app::Message::TaskBoard(
                super::TaskBoardMessage::AgentSchedulerStatusLoaded { result, .. },
            ))) => {
                result.expect("scheduler response must deserialize");
            }
            _ => panic!("expected scheduler status output"),
        }
    }
    app.task_board.task_board_settings.max_concurrent = 7;
    let tick = super::update(&mut app, super::TaskBoardMessage::ExecutionTick);
    let mut stream = iced_runtime::task::into_stream(tick).unwrap();
    for _ in 0..2 {
        tokio::time::timeout(std::time::Duration::from_secs(10), stream.next()).await.unwrap();
    }
    let requests = gateway.finish();
    assert!(
        requests
            .iter()
            .any(|(_, body)| body.get("desired_capacity") == Some(&serde_json::json!(7)))
    );
    let commands: Vec<_> =
        requests.iter().filter(|(_, body)| body.get("command").is_some()).collect();
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].1["command"], "start");
    assert_eq!(commands[1].1["command"], "retry");
    assert!(requests.iter().all(|(path, _)| !path.contains("/git/")));
}

struct DesktopRecordingGateway {
    _directory: tempfile::TempDir,
    previous: Option<std::ffi::OsString>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<Vec<(String, serde_json::Value)>>>,
}

impl DesktopRecordingGateway {
    fn start() -> Self {
        use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};
        let directory = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("VIBEWINDOW_CONFIG_DIR");
        unsafe {
            std::env::set_var("VIBEWINDOW_CONFIG_DIR", directory.path());
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut config = GatewayClientSystemSettingsConfig::default();
        config.set_servers(
            vec![GatewayClientServerConfig {
                id: "test".into(),
                name: "test".into(),
                host: "127.0.0.1".into(),
                port: listener.local_addr().unwrap().port(),
                bearer_token: "test-token".into(),
                ..Default::default()
            }],
            "test".into(),
        );
        std::fs::write(
            directory.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = stop.clone();
        let handle = std::thread::spawn(move || {
            use serde_json::json;
            use std::io::{Read, Write};
            let mut requests = Vec::new();
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                };
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                let (headers, body) = loop {
                    let n = stream.read(&mut buffer).unwrap();
                    assert!(n > 0);
                    bytes.extend_from_slice(&buffer[..n]);
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_string();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            break (
                                headers,
                                serde_json::from_slice::<serde_json::Value>(
                                    &bytes[end + 4..end + 4 + length],
                                )
                                .unwrap_or(json!({})),
                            );
                        }
                    }
                };
                let path =
                    headers.lines().next().unwrap().split_whitespace().nth(1).unwrap().to_string();
                let response = if path.contains("projects/resolve") {
                    json!({"project": {"id":"project-test", "name":"test", "directory":body["directory"], "display_path":"test", "status":"ready", "created_at_ms":1,"updated_at_ms":1,"git":{"is_repo":false,"has_uncommitted_changes":false}}})
                } else if body.get("desired_capacity").is_some() {
                    json!({"idle_count":7,"target_idle_count":7})
                } else if body.get("command").is_some() {
                    json!({"accepted":true})
                } else if body.get("task_id").is_some() {
                    json!({"task_id":"desktop-task", "task":{"id":"desktop-task", "title":"test", "kind":"finite", "status":"failed", "created_at_ms":1,"updated_at_ms":1}})
                } else {
                    json!({"project_id":"project-test","tasks":[]})
                };
                requests.push((path, body));
                let response = response.to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", response.len(), response).unwrap();
            }
            requests
        });
        Self { _directory: directory, previous, stop, handle: Some(handle) }
    }
    fn finish(mut self) -> Vec<(String, serde_json::Value)> {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        self.handle.take().unwrap().join().unwrap()
    }
}
impl Drop for DesktopRecordingGateway {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var("VIBEWINDOW_CONFIG_DIR", value) },
            None => unsafe { std::env::remove_var("VIBEWINDOW_CONFIG_DIR") },
        }
    }
}

#[test]
fn task_pool_acceptance_capacity_and_employee_interactions_match_baseline() {
    let project = tempfile::tempdir().unwrap();
    let (mut app, _) = crate::app::App::new();
    app.navigation_tabs.project_path = Some(project.path().display().to_string());
    app.task_board.task_board_worktree_desired_capacity = Some(17);
    let expanded = app.task_board.task_board_worktree_panel_expanded;
    let _ = crate::app::message::task_board::update(
        &mut app,
        super::TaskBoardMessage::ToggleWorktreePanelExpanded,
    );
    assert_eq!(app.task_board.task_board_worktree_panel_expanded, !expanded);
    for pixel in [false, true] {
        let _ = crate::app::message::task_board::update(
            &mut app,
            super::TaskBoardMessage::ToggleWorktreePixelOffice(pixel),
        );
        assert_eq!(app.task_board.task_board_worktree_pixel_office, pixel);
        assert_eq!(app.task_board.task_board_worktree_desired_capacity, Some(17));
    }
    let _ = crate::app::message::task_board::update(
        &mut app,
        super::TaskBoardMessage::CleanAllWorktreesPressed,
    );
    assert_eq!(app.task_board.task_board_worktree_manual_confirm_kind, Some("cleanup"));
    let _ = crate::app::message::task_board::update(
        &mut app,
        super::TaskBoardMessage::CancelWorktreeManualConfirm,
    );
    assert_eq!(app.task_board.task_board_worktree_manual_confirm_kind, None);
    assert_eq!(app.task_board.task_board_worktree_desired_capacity, Some(17));
    assert!(!project.path().join(".git").exists());
    assert_eq!(std::fs::read_dir(project.path()).unwrap().count(), 0);
}
