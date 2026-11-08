//! 验证应用启动任务、初始导航和错误恢复流程。
//!
//! 启动测试区分首次任务与后续调用，确保初始化动作不会被重复安排。

#[test]
fn startup_tests_module_is_wired() {
    assert!(module_path!().ends_with("startup_tests"));
}

#[test]
fn build_startup_task_constructs_native_bootstrap_task() {
    let (mut app, _task) = crate::app::App::new();

    let task = super::build_startup_task(&mut app);

    let _ = std::hint::black_box(task);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn app_new_does_not_wait_for_gateway_configuration() {
    use std::{sync::mpsc, time::Duration};

    crate::app::config::with_settings_gateway(|requests, responses| {
        let (constructed, completion) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let (app, task) = crate::app::App::new();
            let starting = app.startup_state == crate::app::state::NativeStartupState::Starting;
            let has_task = iced_runtime::task::into_stream(task).is_some();
            let _ = constructed.send((starting, has_task));
        });

        let initial_state = completion.recv_timeout(Duration::from_secs(3));
        let request = requests.try_recv().ok();
        // Release any pending HTTP request before joining, including on the expected failure.
        drop(responses);
        thread.join().unwrap();

        assert_eq!(initial_state.expect("App::new waited for a Gateway response"), (true, true));
        assert!(request.is_none(), "App::new contacted Gateway: {request:?}");
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_startup_backfills_once_and_retries_only_failed_reads() {
    use crate::app::{
        App, Message,
        message::{SettingsMessage, ViewMessage, settings::QuickMessagesMessage},
        state::{AcpHistoryReplayMode, ChatSendBehavior, NativeStartupState},
    };
    use iced::advanced::{layout, renderer::Headless, widget::Tree};
    use iced::{Font, Pixels, Size, window};
    use iced_runtime::{Action, futures::futures::StreamExt, task::into_stream};
    use serde_json::json;
    use std::{collections::HashMap, time::Duration};

    let timeout = Duration::from_secs(5);
    crate::app::config::with_settings_gateway(|requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let (mut app, _) = App::new();
        let window = window::Id::unique();
        let renderer = runtime
            .block_on(<iced::Renderer as Headless>::new(
                Font::DEFAULT,
                Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("headless renderer");
        let layout_window = |app: &App| {
            let mut view = app.view_window(window);
            let mut tree = Tree::new(view.as_widget());
            view.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(1200.0, 700.0)),
            )
        };
        assert!(layout_window(&app).bounds().height > 0.0);
        let retry = app.update(Message::StartupCliServiceBootstrapped(Err("unavailable".into())));
        assert!(into_stream(retry).is_some());
        assert_eq!(app.startup_state, NativeStartupState::Starting);
        assert!(requests.try_recv().is_err());

        let mut actions = into_stream(app.update(Message::StartupCliServiceBootstrapped(Ok(()))))
            .expect("service readiness schedules startup reads");
        assert_eq!(app.startup_state, NativeStartupState::Ready);
        assert!(into_stream(app.update(Message::StartupCliServiceBootstrapped(Ok(())))).is_none());
        let (messages, mut output) = tokio::sync::mpsc::unbounded_channel();
        let driver = runtime.spawn(async move {
            while let Some(action) = actions.next().await {
                if let Action::Output(message) = action {
                    messages.send(message).unwrap();
                }
            }
        });

        let first_request = requests.recv_timeout(timeout).expect("startup makes real HTTP reads");
        // A response is withheld while the main interface lays out and handles local input.
        assert!(layout_window(&app).bounds().height > 0.0);
        assert!(app.archived_session_ids.is_empty());
        let save = app.update(Message::View(ViewMessage::ToggleGitDiffSummary));
        let _ = app.update(Message::Settings(SettingsMessage::QuickMessages(
            QuickMessagesMessage::TextChanged(0, "local draft".into()),
        )));
        let _ = app.update(Message::Settings(SettingsMessage::ProjectEnableWorktreeToggled(
            "/local".into(),
            true,
        )));

        let mut counts = HashMap::new();
        let mut reply = |request: String| {
            let line = request.lines().next().unwrap();
            let (method, target) = line.split_once(' ').unwrap();
            let path = target.split_whitespace().next().unwrap().split('?').next().unwrap();
            *counts.entry(path.to_owned()).or_insert(0) += 1;
            let body = match (method, path) {
                ("GET", "/v1/desktop/preferences") => None,
                ("GET", "/v1/global/config") => Some(json!({
                    "gateway": {"port": 19876},
                    "app_ui": {"system_settings": {
                        "app_theme": iced::Theme::Dark.to_string(),
                        "editor_font_size": 18.0,
                        "editor_auto_line_height": true,
                        "terminal_shell": "zsh",
                        "terminal_font_size": 16.0,
                        "project_worktree_enabled": {"/remote": true},
                    }}
                })),
                ("GET", "/v1/config") => {
                    Some(json!({"heartbeat": {"enabled": false, "interval_minutes": 43}}))
                }
                ("GET", "/v1/global/config/acp") => Some(json!({"codex": {}})),
                ("GET", "/v1/agent" | "/v1/session/ui-previews" | "/v1/session") => Some(json!([])),
                ("GET", "/v1/desktop/external-apps") => Some(json!({})),
                ("GET", "/v1/tools") => Some(json!({"items": [{
                    "identity": {"namespace": null, "name": "file_read"},
                    "source": "builtin", "exposure": "direct", "id": "file_read",
                    "display_name": "Read", "description": "Read a file",
                    "input_schema": {"type": "object"}, "read_only": true
                }]})),
                ("GET", "/v1/session/scope") => Some(json!("fixture-scope")),
                ("PUT", "/v1/session/scope") => {
                    assert_eq!(
                        crate::app::config::request_json_body(&request),
                        json!({"scope": "fixture-scope"})
                    );
                    Some(json!(true))
                }
                ("GET", "/v1/session/archived") => Some(json!(["archive-1"])),
                _ => Some(json!({})), // Ancillary startup requests do not provide config data.
            };
            responses.send(body.map(|value| value.to_string())).unwrap();
        };
        reply(first_request);
        // The real startup task starts one config batch and one home-session load.
        for _ in 1..12 {
            reply(requests.recv_timeout(timeout).expect("remaining startup request"));
        }
        runtime.block_on(async { tokio::time::timeout(timeout, driver).await }).unwrap().unwrap();
        while let Ok(message) = output.try_recv() {
            if matches!(
                message,
                Message::StartupGatewaySnapshotLoaded(..) | Message::StartupSessionsLoaded(..)
            ) {
                let _ = app.update(message);
            }
        }

        assert_eq!(counts.get("/v1/global/config"), Some(&1));
        assert_eq!(counts.get("/v1/session/archived"), Some(&1));
        assert_eq!(counts.get("/v1/tools"), Some(&1));
        assert!(app.archived_session_ids.contains("archive-1"));
        assert_eq!(app.gateway_settings.port, 19876);
        assert_eq!(app.heartbeat_settings.interval_minutes, 43);
        assert_eq!(app.acp_agents, ["codex"]);
        assert_eq!(app.agents_settings.available_tools, ["file_read"]);
        assert_eq!(app.app_theme.to_string(), iced::Theme::Dark.to_string());
        assert_eq!(app.current_font_size, 18.0);
        assert!((app.current_line_height - 25.2).abs() < 0.001);
        assert!(app.auto_adjust_line_height);
        assert!(matches!(app.terminal.shell, crate::app::Shell::Zsh));
        assert_eq!(app.terminal.font_size, 16.0);
        assert_eq!(app.quick_messages[0].text, "local draft");
        assert_eq!(app.project_worktree_enabled.get("/local"), Some(&true));
        assert_eq!(app.project_worktree_enabled.get("/remote"), Some(&true));

        let mut actions = into_stream(save).expect("edited preference saves through Iced");
        let driver = runtime.spawn(async move { actions.next().await.unwrap() });
        let request = requests.recv_timeout(timeout).unwrap();
        assert!(request.starts_with("PATCH /v1/desktop/preferences "));
        assert_eq!(
            crate::app::config::request_json_body(&request),
            json!({"show_git_diff_summary": true})
        );
        responses.send(Some("{}".into())).unwrap();
        let Action::Output(saved) = runtime
            .block_on(async { tokio::time::timeout(timeout, driver).await })
            .unwrap()
            .unwrap()
        else {
            panic!("expected save completion");
        };
        let Message::DesktopPreferencesSaved(_, Ok(())) = &saved else {
            panic!("expected successful save");
        };
        let _ = app.update(saved);

        let health_key = crate::app::message::gateway_health::server_health_key(
            &app.gateway_client_settings.servers[0],
        )
        .unwrap();
        let healthy = || Message::GatewayHealthChecked(vec![(health_key.clone(), true)]);
        let mut retry = into_stream(app.update(healthy())).expect("retry failed preferences only");
        let driver = runtime.spawn(async move { retry.next().await.expect("retry output") });
        let request = requests.recv_timeout(timeout).unwrap();
        assert!(request.starts_with("GET /v1/desktop/preferences "));
        responses
            .send(Some(
                json!({
                    "model": "remote-model",
                    "acp_agent": "codex",
                    "acp_history_strategy": "recent",
                    "acp_history_recent_count": 9,
                    "chat_send_behavior": "guide",
                    "show_git_diff_summary": false,
                    "show_git_diff_highlight": true,
                    "web_bookmarks": [{"title":"Remote","url":"https://example.test"}],
                    "task_board_worktree_pixel_office": true,
                    "task_board_clear_prompt_after_create": false
                })
                .to_string(),
            ))
            .unwrap();
        let action = runtime
            .block_on(async { tokio::time::timeout(timeout, driver).await })
            .unwrap()
            .unwrap();
        let Action::Output(message) = action else {
            panic!("expected startup snapshot");
        };
        let _ = app.update(message);
        assert_eq!(app.model, "remote-model");
        assert_eq!(app.acp_agent.as_deref(), Some("codex"));
        assert_eq!(app.task_board_last_acp_agent.as_deref(), Some("codex"));
        assert_eq!(app.acp_history_mode, AcpHistoryReplayMode::Recent);
        assert_eq!(app.acp_recent_count, 9);
        let runtime_state = app.current_session_runtime();
        assert_eq!(runtime_state.acp_history_mode, AcpHistoryReplayMode::Recent);
        assert_eq!(runtime_state.acp_recent_count, 9);
        assert_eq!(app.chat_send_behavior, ChatSendBehavior::Guide);
        assert!(app.show_git_diff_summary);
        assert!(app.show_git_diff_highlight);
        assert_eq!(app.web_bookmarks[0].title, "Remote");
        assert!(app.task_board_worktree_pixel_office);
        assert!(!app.task_board_clear_prompt_after_create);
        assert_eq!(app.update(healthy()).units(), 0);
        assert!(requests.try_recv().is_err());
    });
}
