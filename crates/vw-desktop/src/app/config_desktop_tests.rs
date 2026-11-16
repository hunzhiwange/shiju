//! 验证桌面配置的默认值、文件转换和兼容性处理。
//!
//! 临时目录中的桌面配置覆盖缺失、旧版与完整文件，确认默认合并不丢字段。

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use tempfile::TempDir;
use config_ui::{GatewayClientServerConfig, GatewayClientSystemSettingsConfig};


#[derive(Debug, Clone)]
pub(crate) struct RecordedRequest {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) body: String,
}

pub(crate) struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &OsStr) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
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

pub(crate) struct TestServer {
    pub(crate) port: u16,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: Option<JoinHandle<()>>,
}

impl TestServer {
    pub(crate) fn start(
        expected_requests: usize,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    ) -> Self {
        Self::start_matching(expected_requests, |_| true, handler)
    }

    pub(crate) fn start_matching(
        expected_requests: usize,
        matches: impl Fn(&RecordedRequest) -> bool + Send + 'static,
        handler: impl Fn(&RecordedRequest) -> (u16, &'static str) + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let port = listener.local_addr().expect("test server address should be available").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let handler = Arc::new(handler);
        let handle = std::thread::spawn(move || {
            let mut remaining = expected_requests;
            for stream in listener.incoming() {
                let mut stream = stream.expect("request stream should open");
                let request = read_http_request(&mut stream);
                if !matches(&request) {
                    write_http_response(&mut stream, 500, "{}");
                    continue;
                }
                let (status, body) = handler(&request);
                server_requests.lock().expect("requests lock should not be poisoned").push(request);
                write_http_response(&mut stream, status, body);
                remaining -= 1;
                if remaining == 0 {
                    break;
                }
            }
        });

        Self { port, requests, handle: Some(handle) }
    }

    pub(crate) fn take_requests(mut self) -> Vec<RecordedRequest> {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("test server should finish");
        }
        self.requests.lock().expect("requests lock should not be poisoned").clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.handle.take();
    }
}

fn read_http_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let read = stream.read(&mut chunk).expect("request should be readable");
        assert!(read > 0, "request should contain headers");
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("request header terminator should exist")
        + 4;
    let header_text =
        String::from_utf8(buffer[..header_end].to_vec()).expect("headers should be utf8");
    let mut lines = header_text.lines();
    let request_line = lines.next().expect("request line should exist");
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().expect("method should exist").to_string();
    let path = request_parts.next().expect("path should exist").to_string();
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content length should parse"))
        })
        .unwrap_or(0);

    while buffer.len() < header_end + content_length {
        let read = stream.read(&mut chunk).expect("body should be readable");
        assert!(read > 0, "request body ended early");
        buffer.extend_from_slice(&chunk[..read]);
    }

    let body = String::from_utf8(buffer[header_end..header_end + content_length].to_vec())
        .expect("body should be utf8");
    RecordedRequest { method, path, body }
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "ERROR" };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("response should write");
}

fn env_lock() -> MutexGuard<'static, ()> {
    crate::app::TEST_PROCESS_STATE_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

pub(crate) fn temp_home() -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let lock = env_lock();
    let home = tempfile::tempdir().expect("temp home should be created");
    let guard = EnvGuard::set_os("HOME", home.path().as_os_str());
    (lock, home, guard)
}

fn gateway_config(port: u16) -> GatewayClientSystemSettingsConfig {
    let mut config = GatewayClientSystemSettingsConfig::default();
    config.set_servers(
        vec![GatewayClientServerConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            bearer_token: "test-token".to_string(),
            ..GatewayClientServerConfig::default()
        }],
        "test".to_string(),
    );
    config
}

fn configure_gateway(port: u16) -> (MutexGuard<'static, ()>, TempDir, EnvGuard) {
    let home = temp_home();
    set_gateway_port(port);
    home
}

pub(crate) fn set_gateway_port(port: u16) {
    super::super::system_settings::save_gateway_client_bootstrap_config(&gateway_config(port))
        .unwrap();
}


#[test]
fn app_update_source_patch_and_restart_read_use_gateway_preferences() {
    use desktop_release::UpdateSource;
    let stored = Arc::new(Mutex::new(serde_json::json!({"theme":"dark"})));
    let server_state = stored.clone();
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.path, "/v1/desktop/preferences");
        if request.method == "PATCH" {
            let patch: serde_json::Value = serde_json::from_str(&request.body).unwrap();
            assert_eq!(patch, serde_json::json!({"app_update_source":"github"}));
            server_state.lock().unwrap()["app_update_source"] = patch["app_update_source"].clone();
        } else {
            assert_eq!(request.method, "GET");
            assert_eq!(server_state.lock().unwrap()["app_update_source"], "github");
        }
        (200, r#"{"app_update_source":"github","theme":"dark"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    super::super::gateway::run_gateway_call(super::save_app_config_async(
        crate::app::desktop_update::source_patch(UpdateSource::Github),
    ))
    .unwrap();
    let config = super::load_app_config();
    assert_eq!(
        crate::app::desktop_update::DesktopUpdate::from_config(&config).source,
        Some(UpdateSource::Github)
    );
    assert_eq!(config["theme"], "dark");
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn app_update_source_gateway_save_failure_reaches_unsaved_toast() {
    use crate::app::{Message, message::ViewMessage};
    use iced_runtime::futures::futures::StreamExt;
    use desktop_release::UpdateSource;
    let mut app = crate::app::App::new().0;
    let server = TestServer::start(1, |request| {
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/v1/desktop/preferences");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&request.body).unwrap(),
            serde_json::json!({"app_update_source":"github"})
        );
        (500, r#"{"error":"preferences unavailable"}"#)
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let task = crate::app::message::view::update(
        &mut app,
        ViewMessage::AppUpdateSourceSelected(UpdateSource::Github),
    );
    let result = super::super::gateway::run_gateway_call(async {
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        match stream.next().await {
            Some(iced_runtime::Action::Output(Message::View(
                ViewMessage::AppUpdateSourceSaved(result),
            ))) => Ok(result),
            _ => Err("missing persistence result".into()),
        }
    })
    .unwrap();
    assert!(result.is_err());
    let _ = crate::app::message::view::update(&mut app, ViewMessage::AppUpdateSourceSaved(result));
    assert_eq!(app.desktop_update.source, Some(UpdateSource::Github));
    assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("未保存"));
    assert_eq!(server.take_requests().len(), 1);
}

#[test]
fn preference_save_failure_reaches_host_toast() {
    let mut app = crate::app::App::new().0;
    let scope = super::preferences_scope(&gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1));
    let _ = app.desktop_preferences_saved(scope, Err("rejected".into()));
    assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("未保存"));
}


async fn collect_preference_messages(
    task: iced::Task<crate::app::Message>,
) -> Vec<crate::app::Message> {
    use iced_runtime::futures::futures::StreamExt;

    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        return Vec::new();
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut messages = Vec::new();
        while let Some(action) = stream.next().await {
            if let iced_runtime::Action::Output(message) = action {
                messages.push(message);
            }
        }
        messages
    })
    .await
    .expect("preference tasks should finish")
}

#[test]
fn project_save_failure_reaches_host_toast() {
    let mut app = crate::app::App::new().0;
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 1);
    let scope = super::ProjectPreferencesScope::new(&endpoint, "/repo/a".into());
    let _ = app.project_chat_preferences_saved(scope, Err("rejected".into()));
    assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("项目聊天偏好未保存"));
}

#[test]
fn send_behavior_change_emits_send_before_preferences_complete() {
    use crate::app::{App, Message, message::{ChatMessage, ViewMessage}, state::ChatSendBehavior};
    use iced_runtime::futures::futures::StreamExt;
    let (_lock, _home, _guard) = temp_home();
    let mut app = App::new().0;
    set_gateway_port(1);
    app.session_state.active_session_id = Some("active".into());
    app.current_session_runtime_mut().request_lifecycle.is_requesting = true;
    let behavior = app.update(Message::View(ViewMessage::SelectChatSendBehavior(ChatSendBehavior::Queue)));
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let mut actions = iced_runtime::task::into_stream(behavior).unwrap();
        let send = tokio::time::timeout(std::time::Duration::from_secs(5), actions.next()).await.unwrap();
        assert!(matches!(send, Some(iced_runtime::Action::Output(Message::Chat(ChatMessage::SendPressed)))));
    });
}

#[test]
fn mini_companion_preferences_round_trip_preserves_other_fields() {
    use crate::app::{App, Message, state::TaskPetAvatarKind};
    use serde_json::{Value, json};
    let server = TestServer::start(2, |request| {
        assert_eq!(request.path, "/v1/desktop/preferences");
        match request.method.as_str() {
            "PATCH" => {
                let patch: Value = serde_json::from_str(&request.body).unwrap();
                assert_eq!(
                    patch,
                    json!({"mini_companion": {"avatar":"none","scale":1.5,"visible":true}})
                );
                (200, "{}")
            }
            "GET" => (
                200,
                r#"{"theme":"dark","mini_companion":{"avatar":"none","scale":1.5,"visible":true}}"#,
            ),
            _ => panic!("unexpected request"),
        }
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut app = App::new().0;
    app.pet_state.task_pet_avatar_kind = TaskPetAvatarKind::None;
    app.pet_state.task_pet_scale = 1.5;
    app.companion_navigation.task_pet_visible = true;
    let save = app.save_task_pet_preferences();
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(save).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::DesktopPreferencesSaved(_, Ok(()))));
    for message in messages {
        let _ = app.update(message);
    }
    let cfg = super::load_app_config();
    assert_eq!(cfg["theme"], "dark");
    let mut restored = App::new().0;
    let _ = restored.update(Message::StartupAppConfigLoaded(Ok(cfg)));
    assert_eq!(restored.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::None);
    assert_eq!(restored.pet_state.task_pet_scale, 1.5);
    assert!(restored.companion_navigation.task_pet_visible);
    assert!(restored.task_pet_window_id.is_some());
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn mini_companion_late_load_keeps_local_edit() {
    use crate::app::{App, Message, state::TaskPetAvatarKind};
    use serde_json::json;
    let (_lock, _home, _guard) = temp_home();
    let mut app = App::new().0;
    app.pet_state.task_pet_avatar_kind = TaskPetAvatarKind::Beauty;
    app.companion_navigation.task_pet_visible = false;
    let _save = app.save_task_pet_preferences();
    let _ = app.update(Message::StartupAppConfigLoaded(Ok(json!({
        "mini_companion": {"avatar":"robot", "scale":2.0, "visible":true}
    }))));
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::Beauty);
    assert_eq!(app.pet_state.task_pet_scale, 1.0);
    assert!(!app.companion_navigation.task_pet_visible);
    assert!(app.task_pet_window_id.is_none());
}

#[test]
fn mini_companion_failed_save_keeps_edit_and_can_retry() {
    use crate::app::{App, Message, message::ViewMessage, state::TaskPetAvatarKind};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let writes = AtomicUsize::new(0);
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.method, "PATCH");
        assert!(request.body.contains("mini_companion"));
        if writes.fetch_add(1, Ordering::SeqCst) == 0 { (500, "{}") } else { (200, "{}") }
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut app = App::new().0;
    let save = crate::app::message::view::update(
        &mut app,
        ViewMessage::TaskPetAvatarSelected(TaskPetAvatarKind::None),
    );
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(save).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::DesktopPreferencesSaved(_, Err(_))));
    for message in messages {
        let _ = app.update(message);
    }
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::None);
    assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("未保存"));
    let retry = app.save_task_pet_preferences();
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(retry).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::DesktopPreferencesSaved(_, Ok(()))));
    for message in messages {
        let _ = app.update(message);
    }
    assert!(app.task_pet_window_id.is_none());
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn mini_companion_old_endpoint_snapshot_cannot_restore_visibility() {
    use crate::app::{App, Message, impls::StartupGatewaySnapshot};
    use gateway_endpoint::GatewayEndpoint;
    let (_lock, _home, _guard) = temp_home();
    let mut app = App::new().0;
    let _load = app.startup_configuration.load(crate::app::config::gateway_client_endpoint()).map(|(generation, snapshot)| crate::app::Message::StartupGatewaySnapshotLoaded(generation, snapshot));
    app.startup_configuration.select_endpoint(&GatewayEndpoint::new("127.0.0.1", 1));
    let snapshot = StartupGatewaySnapshot {
        preferences: Some(Ok(
            serde_json::json!({"mini_companion":{"avatar":"none","scale":2.0,"visible":true}}),
        )),
        system: None,
        agent: None,
        catalog: None,
        tools: None,
        external_apps: None,
    };
    let _ = app.update(Message::StartupGatewaySnapshotLoaded(1, Box::new(snapshot)));
    assert!(!app.companion_navigation.task_pet_visible);
    assert_eq!(app.pet_state.task_pet_scale, 1.0);
    assert!(app.task_pet_window_id.is_none());
}

#[test]
fn mini_companion_queued_edits_save_latest_without_reopening_window() {
    use crate::app::{App, Message, state::TaskPetAvatarKind};
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let writes = AtomicUsize::new(0);
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.method, "PATCH");
        let patch: Value = serde_json::from_str(&request.body).unwrap();
        let first = writes.fetch_add(1, Ordering::SeqCst) == 0;
        assert_eq!(
            patch["mini_companion"],
            if first {
                json!({"avatar":"robot", "scale":1.0, "visible":true})
            } else {
                json!({"avatar":"none", "scale":1.75, "visible":false})
            }
        );
        (200, "{}")
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut app = App::new().0;
    app.companion_navigation.task_pet_visible = true;
    let first = app.save_task_pet_preferences();
    app.pet_state.task_pet_avatar_kind = TaskPetAvatarKind::None;
    assert!(iced_runtime::task::into_stream(app.save_task_pet_preferences()).is_none());
    app.companion_navigation.task_pet_visible = false;
    app.set_task_pet_scale(1.75).unwrap();
    assert!(iced_runtime::task::into_stream(app.save_task_pet_preferences()).is_none());
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(first).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    let next = app.update(messages.into_iter().next().unwrap());
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(next).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::DesktopPreferencesSaved(_, Ok(()))));
    let done = app.update(messages.into_iter().next().unwrap());
    assert!(iced_runtime::task::into_stream(done).is_none());
    assert!(app.task_pet_window_id.is_none());
    assert!(!app.companion_navigation.task_pet_visible);
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn mini_companion_defaults_and_loaded_scale_are_bounded() {
    use crate::app::{App, Message, state::TaskPetAvatarKind};
    use serde_json::json;
    let mut app = App::new().0;
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::Robot);
    assert_eq!(app.pet_state.task_pet_scale, 1.0);
    assert!(!app.companion_navigation.task_pet_visible);
    let _ = app.update(Message::StartupAppConfigLoaded(Ok(json!({
        "mini_companion":{"avatar":"unknown", "scale":99.0, "visible":false}
    }))));
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::Robot);
    assert_eq!(app.pet_state.task_pet_scale, 2.0);
    let _ = app.update(Message::StartupAppConfigLoaded(Ok(json!({
        "mini_companion":{"avatar":"handsome", "scale":-1.0, "visible":false}
    }))));
    assert_eq!(app.pet_state.task_pet_avatar_kind, TaskPetAvatarKind::Handsome);
    assert_eq!(app.pet_state.task_pet_scale, 0.5);
}

#[test]
fn mini_companion_size_failure_keeps_draft_and_retry_commits_once() {
    use crate::app::{
        App, Message,
        message::{ViewMessage, view},
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = AtomicUsize::new(0);
    let server = TestServer::start(2, move |request| {
        assert_eq!(request.method, "PATCH");
        let value: serde_json::Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(value["mini_companion"]["scale"], 1.5);
        if calls.fetch_add(1, Ordering::SeqCst) == 0 { (500, "{}") } else { (200, "{}") }
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut app = App::new().0;
    let _ = view::update(&mut app, ViewMessage::TaskPetSizeOpen);
    let _ = view::update(&mut app, ViewMessage::TaskPetSizePreview(1.5));
    for success in [false, true] {
        let save = view::update(&mut app, ViewMessage::TaskPetSizeSave);
        let duplicate = view::update(&mut app, ViewMessage::TaskPetSizeSave);
        assert!(iced_runtime::task::into_stream(duplicate).is_none());
        let messages = super::super::gateway::run_gateway_call(async {
            Ok(collect_preference_messages(save).await)
        })
        .unwrap();
        assert_eq!(messages.len(), 1);
        assert!(
            matches!(&messages[0], Message::View(ViewMessage::TaskPetSizeSaved(_, result)) if result.is_ok() == success)
        );
        for message in messages {
            let _ = app.update(message);
        }
        assert!(app.task_pet_size_pending.is_none());
        if !success {
            assert_eq!(app.pet_state.task_pet_size_draft, Some(1.5));
            assert_eq!(app.pet_state.task_pet_scale, 1.0);
            assert!(app.toast_state.active_toast.as_ref().unwrap().message.contains("未保存"));
        }
    }
    assert!(app.pet_state.task_pet_size_draft.is_none());
    assert_eq!(app.pet_state.task_pet_scale, 1.5);
    assert_eq!(server.take_requests().len(), 2);
}

#[test]
fn mini_companion_size_queued_save_waits_for_its_own_response() {
    use crate::app::{
        App, Message,
        message::{ViewMessage, view},
    };
    let server = TestServer::start(2, |request| {
        assert_eq!(request.method, "PATCH");
        (200, "{}")
    });
    let (_lock, _home, _guard) = configure_gateway(server.port);
    let mut app = App::new().0;
    let earlier = app.save_desktop_preference("theme", serde_json::json!("dark"));
    let _ = view::update(&mut app, ViewMessage::TaskPetSizeOpen);
    let _ = view::update(&mut app, ViewMessage::TaskPetSizePreview(2.0));
    let queued = view::update(&mut app, ViewMessage::TaskPetSizeSave);
    assert!(iced_runtime::task::into_stream(queued).is_none());
    let messages = super::super::gateway::run_gateway_call(async {
        Ok(collect_preference_messages(earlier).await)
    })
    .unwrap();
    assert_eq!(messages.len(), 1);
    assert!(matches!(&messages[0], Message::DesktopPreferencesSaved(_, Ok(()))));
    for message in messages {
        let next = app.update(message);
        assert_eq!(app.pet_state.task_pet_size_draft, Some(2.0));
        assert_eq!(app.pet_state.task_pet_scale, 1.0);
        let messages = super::super::gateway::run_gateway_call(async {
            Ok(collect_preference_messages(next).await)
        })
        .unwrap();
        assert_eq!(messages.len(), 1);
        for message in messages {
            let _ = app.update(message);
        }
    }
    assert!(app.pet_state.task_pet_size_draft.is_none());
    assert_eq!(app.pet_state.task_pet_scale, 2.0);
    let requests = server.take_requests();
    assert_eq!(requests.len(), 2);
    let patch: serde_json::Value = serde_json::from_str(&requests[1].body).unwrap();
    assert_eq!(patch["mini_companion"]["scale"], 2.0);
}
