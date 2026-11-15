use super::*;
use vw_api_transport::id::TenantScope;

fn identity(company: &str) -> Identity {
    Identity {
        gateway: "http://127.0.0.1:1".into(),
        tenant: TenantScope::new("10000", company).unwrap(),
        auth_fingerprint: [0; 32],
    }
}

fn restore_state() -> (ArchivedChatsState, Identity) {
    let current = identity("100000000");
    (
        ArchivedChatsState {
            identity: Some(current.clone()),
            sessions: vec![info("a", "p", "Archived", 10), info("child", "q", "Child", 9)],
            selected_project_id: Some("p".into()),
            request_generation: 0,
            ..Default::default()
        },
        current,
    )
}

fn restored(current: &Identity, result: Result<Info, String>) -> ArchivedChatsMessage {
    ArchivedChatsMessage::Restored {
        identity: current.clone(),
        generation: 1,
        id: "a".into(),
        result,
    }
}













#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn archived_chats_restore_http_persists_only_target_and_retries_failure() {
    use iced::futures::StreamExt;
    use std::io::{Read, Write};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.json");
    let mut child = info("child", "p", "Child", 9);
    child.parent_id = Some("a".into());
    let rows = vec![info("a", "p", "Archived", 10), child];
    std::fs::write(&path, serde_json::to_vec(&rows).unwrap()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(TenantScope::new("10000", "100000001").unwrap());
    let current = Identity::from_endpoint(&endpoint);
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    let server_path = path.clone();
    let server = std::thread::spawn(move || {
        for attempt in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            let (header_end, content_length) = loop {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    let header = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    assert!(
                        header.starts_with("patch /v1/session/a?directory=%2ffixture%2fp "),
                        "{header}"
                    );
                    assert!(header.contains("100000001"));
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length: ")
                                .map(|value| value.parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + content_length {
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            let patch: serde_json::Value =
                serde_json::from_slice(&bytes[header_end..header_end + content_length]).unwrap();
            assert_eq!(patch["time"]["archived"], 0);
            assert!(patch.get("title").is_none_or(serde_json::Value::is_null));
            let (status, body) = if attempt == 0 {
                ("500 Internal Server Error", b"fixture failure".to_vec())
            } else {
                let mut saved: Vec<Info> =
                    serde_json::from_slice(&std::fs::read(&server_path).unwrap()).unwrap();
                saved[0].time.archived = None;
                std::fs::write(&server_path, serde_json::to_vec(&saved).unwrap()).unwrap();
                ("200 OK", serde_json::to_vec(&saved[0]).unwrap())
            };
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            stream.write_all(&body).unwrap();
        }
    });
    let mut state = ArchivedChatsState {
        identity: Some(current.clone()),
        sessions: rows,
        ..Default::default()
    };
    for attempt in 0..2 {
        apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
        let task = restore_task(
            Ok(client.clone()),
            current.clone(),
            state.restore_requests["a"],
            state.sessions[0].clone(),
        );
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let action = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();
        let iced_runtime::Action::Output(Message::Settings(SettingsMessage::ArchivedChats(
            message,
        ))) = action
        else {
            panic!("expected routed restore result");
        };
        assert!(
            matches!(&message, ArchivedChatsMessage::Restored { result, .. } if result.is_ok() == (attempt == 1))
        );
        apply(&mut state, message, &current);
        assert_eq!(state.sessions.len(), if attempt == 0 { 2 } else { 1 });
        assert!(!state.operating_ids.contains("a"));
    }
    server.join().unwrap();
    let saved: Vec<Info> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(saved[0].time.archived.is_none());
    assert!(saved[1].time.archived.is_some());
    assert_eq!(saved[1].parent_id.as_deref(), Some("a"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn archived_chats_restore_routes_sync_caches_without_navigation_and_rejects_late_results() {
    const CHILD: &str = "SHIJU_ARCHIVED_RESTORE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let config = config_ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!(
                "{}::archived_chats_restore_routes_sync_caches_without_navigation_and_rejects_late_results",
                module_path!().split_once("::").unwrap().1,
            )])
            .env(CHILD, "1").env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path()).env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
        return;
    }
    let mut app = App::new().0;
    let current = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    let mut row = info("a", "p", "Archived", 10);
    app.archived_chats = ArchivedChatsState {
        identity: Some(current.clone()),
        sessions: vec![row.clone()],
        ..Default::default()
    };
    app.sessions = vec![row.clone(), info("other", "q", "Other", 8)];
    app.project_sessions.insert("/non-current".into(), vec![row.clone()]);
    app.archived_session_ids.extend(["a".into(), "other".into()]);
    app.active_session_id = Some("other".into());
    app.project_path = Some("/current".into());
    let send =
        |app: &mut App, message| super::super::update(app, SettingsMessage::ArchivedChats(message));
    let _ = send(&mut app, ArchivedChatsMessage::Restore("a".into()));
    assert!(
        iced_runtime::task::into_stream(send(&mut app, ArchivedChatsMessage::Restore("a".into())))
            .is_none()
    );
    let _ = send(&mut app, restored(&current, Err("fixture failure".into())));
    assert!(app.active_toast.is_some());
    assert!(app.sessions[0].time.archived.is_some());
    assert_eq!(app.archived_chats.sessions.len(), 1);
    let _ = send(&mut app, ArchivedChatsMessage::Restore("a".into()));
    let generation = app.archived_chats.restore_requests["a"];
    row.time.archived = None;
    row.title = "Persisted title".into();
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Restored {
            identity: current.clone(),
            generation,
            id: "a".into(),
            result: Ok(row.clone()),
        },
    );
    assert_eq!(app.sessions[0].title, "Persisted title");
    assert!(app.sessions[0].time.archived.is_none());
    assert!(app.project_sessions["/non-current"][0].time.archived.is_none());
    assert!(!app.archived_session_ids.contains("a"));
    assert!(app.archived_session_ids.contains("other"));
    assert_eq!(app.active_session_id.as_deref(), Some("other"));
    assert_eq!(app.project_path.as_deref(), Some("/current"));
    assert!(app.archived_chats.sessions.is_empty());
    let generation = app.archived_chats.request_generation;
    let _ = send(
        &mut app,
        ArchivedChatsMessage::ProjectRefreshed {
            identity: current.clone(),
            generation: generation - 1,
            result: Ok(("/stale".into(), vec![info("a", "p", "Old snapshot", 10)])),
        },
    );
    assert!(!app.project_sessions.contains_key("/stale"));
    let _ = send(
        &mut app,
        ArchivedChatsMessage::ProjectRefreshed {
            identity: current.clone(),
            generation,
            result: Ok(("/uncached".into(), vec![row.clone()])),
        },
    );
    assert!(app.project_sessions["/uncached"][0].time.archived.is_none());
    let mut old = current.clone();
    old.auth_fingerprint = [1; 32];
    let _ = send(
        &mut app,
        ArchivedChatsMessage::ProjectRefreshed {
            identity: old.clone(),
            generation,
            result: Ok(("/foreign".into(), vec![row.clone()])),
        },
    );
    assert!(!app.project_sessions.contains_key("/foreign"));
    row.title = "Foreign".into();
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Restored {
            identity: old,
            generation,
            id: "a".into(),
            result: Ok(row),
        },
    );
    assert_eq!(app.sessions[0].title, "Persisted title");
}

pub(crate) fn info(id: &str, project: &str, title: &str, updated: i64) -> Info {
    serde_json::from_value(serde_json::json!({
        "id": id, "slug": id, "projectID": project, "directory": format!("/fixture/{project}"),
        "title": title, "version": "1",
        "time": {"created": 100 - updated, "updated": updated, "archived": 1}
    }))
    .unwrap()
}

















#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn archived_chats_browse_task_reads_all_projects_over_http() {
    use iced::futures::StreamExt;
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let body = serde_json::to_vec(&vec![
        info("a", "old-project", "旧项目聊天", 10),
        info("b", "other-project", "Other", 20),
    ])
    .unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&buffer[..count]);
        }
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
        stream.write_all(&body).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port)
        .with_tenant_scope(TenantScope::new("10000", "100000001").unwrap());
    let current = Identity::from_endpoint(&endpoint);
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    let mut state = ArchivedChatsState::default();
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    let task = load_task(Ok(client), current.clone(), state.request_generation);
    let mut stream = iced_runtime::task::into_stream(task).unwrap();
    let action = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next())
        .await
        .unwrap()
        .unwrap();
    let iced_runtime::Action::Output(Message::Settings(SettingsMessage::ArchivedChats(message))) =
        action
    else {
        panic!("expected routed archive result");
    };
    apply(&mut state, message, &current);
    assert_eq!(state.groups().len(), 2);
    assert!(!state.loading);
    assert!(state.error.is_none());
    let request = server.join().unwrap().to_lowercase();
    assert!(request.starts_with("get /v1/session "), "{request}");
    assert!(!request.contains("directory="));
    assert!(request.contains("100000001"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn archived_chats_browse_settings_routes_and_gateway_switch() {
    const CHILD: &str = "SHIJU_ARCHIVED_BROWSE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let config = config_ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::archived_chats_browse_settings_routes_and_gateway_switch",
                    module_path!().split_once("::").unwrap().1
                ),
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
        return;
    }
    let mut app = App::new().0;
    let _ = super::super::update(
        &mut app,
        SettingsMessage::SystemTabSelected(
            crate::app::components::system_settings::SystemTab::ArchivedChats,
        ),
    );
    assert!(app.archived_chats.loading);
    let _ = super::super::update(
        &mut app,
        SettingsMessage::ArchivedChats(ArchivedChatsMessage::SearchChanged("fixture".into())),
    );
    assert_eq!(app.archived_chats.search, "fixture");
    app.archived_chats.sessions = vec![info("old", "p", "Old", 1)];
    let generation = app.archived_chats.request_generation;
    let old = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    let _ = super::super::update(
        &mut app,
        SettingsMessage::GatewayClient(super::super::GatewayClientMessage::CompanyIdChanged(
            "100000001".into(),
        )),
    );
    assert!(app.archived_chats.sessions.is_empty());
    assert!(app.archived_chats.request_generation > generation);
    let _ = super::super::update(
        &mut app,
        SettingsMessage::ArchivedChats(ArchivedChatsMessage::Loaded {
            identity: old,
            generation,
            result: Ok(vec![info("old", "p", "Old", 1)]),
        }),
    );
    assert!(app.archived_chats.sessions.is_empty());
}





#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn archived_chats_delete_http_failure_retains_confirmation_then_retry_deletes_tree() {
    use iced::futures::StreamExt;
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        gateway_endpoint::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port());
    let current = Identity::from_endpoint(&endpoint);
    let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("sessions.json");
    let rows = vec![info("a", "p", "父聊天", 10), info("child", "p", "子聊天", 9)];
    std::fs::write(&path, serde_json::to_vec(&rows).unwrap()).unwrap();
    let server_path = path.clone();
    let server = std::thread::spawn(move || {
        for attempt in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 2048];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            let request = String::from_utf8(bytes).unwrap();
            assert!(request.starts_with("DELETE /v1/session/a/archived?directory="), "{request}");
            assert!(request.contains("fixture%2Fp"), "{request}");
            let (status, body) = if attempt == 0 {
                ("500 Internal Server Error", "fixture failure")
            } else {
                std::fs::write(&server_path, b"[]").unwrap();
                ("200 OK", r#"{"deleted_ids":["child","a"]}"#)
            };
            write!(stream, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    let mut state = ArchivedChatsState {
        identity: Some(current.clone()),
        sessions: rows,
        ..Default::default()
    };
    apply(&mut state, ArchivedChatsMessage::DeleteRequested("a".into()), &current);
    for attempt in 0..2 {
        apply(&mut state, ArchivedChatsMessage::DeleteConfirmed, &current);
        let task = delete_task(
            Ok(client.clone()),
            current.clone(),
            state.delete_generation.unwrap(),
            state.delete_target.as_ref().unwrap().row.clone(),
        );
        let mut stream = iced_runtime::task::into_stream(task).unwrap();
        let output = tokio::time::timeout(std::time::Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();
        let iced_runtime::Action::Output(Message::Settings(SettingsMessage::ArchivedChats(
            message,
        ))) = output
        else {
            panic!("expected delete result");
        };
        assert!(
            matches!(&message, ArchivedChatsMessage::Deleted { result, .. } if result.is_ok() == (attempt == 1))
        );
        apply(&mut state, message, &current);
        assert!(state.operating_ids.is_empty());
        assert_eq!(state.sessions.len(), if attempt == 0 { 2 } else { 0 });
        assert_eq!(state.delete_confirmation.is_some(), attempt == 0);
    }
    server.join().unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"[]");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn archived_chats_delete_routes_cleanup_only_after_success() {
    const CHILD: &str = "SHIJU_ARCHIVED_DELETE_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let home = tempfile::tempdir().unwrap();
        let config = config_ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: vec![],
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{}::archived_chats_delete_routes_cleanup_only_after_success",
                    module_path!().split_once("::").unwrap().1
                ),
            ])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
        return;
    }
    let mut app = App::new().0;
    let current = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    let rows = vec![
        info("a", "p", "父聊天", 10),
        info("child", "p", "子聊天", 9),
        info("other", "q", "Other", 8),
    ];
    app.archived_chats = ArchivedChatsState {
        identity: Some(current.clone()),
        sessions: rows.clone(),
        ..Default::default()
    };
    app.archived_chats.sessions[1].parent_id = Some("a".into());
    app.sessions = rows.clone();
    app.project_sessions.insert("/p".into(), rows);
    app.active_session_id = Some("child".into());
    for id in ["a", "child", "other"] {
        app.session_runtime_states.insert(id.into(), Default::default());
        app.session_previews.insert(id.into(), "cached".into());
        app.session_chat_cache.insert(id.into(), crate::app::session::shared_chat_messages(vec![]));
        app.session_chat_message_id_cache.insert(id.into(), vec![]);
        app.archived_session_ids.insert(id.into());
        let snapshot = crate::app::session::view_cache::SessionViewSnapshot::take_from(&mut app);
        let _ = app.session_view_cache.insert(id.into(), snapshot);
        assert!(app.session_view_cache.contains(id));
    }
    let send =
        |app: &mut App, message| super::super::update(app, SettingsMessage::ArchivedChats(message));
    assert!(
        iced_runtime::task::into_stream(send(
            &mut app,
            ArchivedChatsMessage::DeleteRequested("a".into())
        ))
        .is_none()
    );
    assert!(
        iced_runtime::task::into_stream(send(&mut app, ArchivedChatsMessage::DeleteCancelled))
            .is_none()
    );
    assert_eq!(app.sessions.len(), 3);
    let _ = send(&mut app, ArchivedChatsMessage::DeleteRequested("a".into()));
    let _ = send(&mut app, ArchivedChatsMessage::DeleteConfirmed);
    assert!(
        iced_runtime::task::into_stream(send(&mut app, ArchivedChatsMessage::DeleteConfirmed))
            .is_none()
    );
    let generation = app.archived_chats.delete_generation.unwrap();
    assert_eq!(app.sessions.len(), 3);
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Deleted {
            identity: current.clone(),
            generation,
            id: "a".into(),
            result: Err("fixture partial failure".into()),
        },
    );
    assert!(app.active_toast.is_some());
    assert_eq!(app.sessions.len(), 3);
    assert!(app.archived_chats.loading);
    let generation = app.archived_chats.request_generation;
    let rows =
        app.archived_chats.sessions.iter().filter(|row| row.id != "child").cloned().collect();
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Loaded { identity: current.clone(), generation, result: Ok(rows) },
    );
    assert!(app.archived_chats.delete_confirmation.is_some());
    assert_eq!(app.sessions.len(), 2);
    assert!(!app.session_runtime_states.contains_key("child"));
    assert!(!app.session_view_cache.contains("child"));
    assert!(app.active_session_id.is_none());
    let _ = send(&mut app, ArchivedChatsMessage::DeleteConfirmed);
    let generation = app.archived_chats.delete_generation.unwrap();
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Deleted {
            identity: current,
            generation,
            id: "a".into(),
            result: Ok(vec!["child".into(), "a".into()]),
        },
    );
    assert!(app.active_toast.is_some());
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert_eq!(app.sessions.len(), 1);
    assert_eq!(app.project_sessions["/p"].len(), 1);
    for id in ["a", "child"] {
        assert!(!app.session_runtime_states.contains_key(id));
        assert!(!app.session_view_cache.contains(id));
        assert!(!app.session_chat_cache.contains_key(id));
        assert!(!app.session_chat_message_id_cache.contains_key(id));
        assert!(!app.session_previews.contains_key(id));
        assert!(!app.archived_session_ids.contains(id));
    }
    assert!(app.session_runtime_states.contains_key("other"));

    // If the failure refresh also fails, a retry may report only the root:
    // reconcile the previously committed child against a new complete snapshot.
    let current = Identity::from_endpoint(&crate::app::gateway_client_endpoint());
    let root = info("b", "p", "Retry root", 10);
    let mut child = info("nested", "p", "Partial child", 9);
    child.parent_id = Some("b".into());
    app.archived_chats = ArchivedChatsState {
        identity: Some(current.clone()),
        sessions: vec![root.clone(), child.clone()],
        ..Default::default()
    };
    app.sessions.extend([root, child]);
    app.session_runtime_states.insert("nested".into(), Default::default());
    let _ = send(&mut app, ArchivedChatsMessage::DeleteRequested("b".into()));
    let _ = send(&mut app, ArchivedChatsMessage::DeleteConfirmed);
    let generation = app.archived_chats.delete_generation.unwrap();
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Deleted {
            identity: current.clone(),
            generation,
            id: "b".into(),
            result: Err("partial failure".into()),
        },
    );
    let generation = app.archived_chats.request_generation;
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Loaded {
            identity: current.clone(),
            generation,
            result: Err("refresh failure".into()),
        },
    );
    let _ = send(&mut app, ArchivedChatsMessage::DeleteConfirmed);
    let generation = app.archived_chats.delete_generation.unwrap();
    let navigation_generation = app.session_view_cache.navigation_generation;
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Deleted {
            identity: current.clone(),
            generation,
            id: "b".into(),
            result: Ok(vec!["b".into()]),
        },
    );
    assert!(app.archived_chats.loading);
    assert!(app.session_view_cache.navigation_generation > navigation_generation);
    let from_project = app.project_path.clone();
    let from_session = app.active_session_id.clone();
    let _ = crate::app::message::project::update(
        &mut app,
        crate::app::message::project::ProjectMessage::SessionNavigationResolved {
            generation: navigation_generation,
            from_project,
            from_session,
            destination: Some(("/fixture/p".into(), Box::new(info("b", "p", "Deleted", 10)))),
        },
    );
    assert!(app.active_session_id.is_none());
    let generation = app.archived_chats.request_generation;
    let _ = send(
        &mut app,
        ArchivedChatsMessage::Loaded {
            identity: current,
            generation,
            result: Ok(vec![info("other", "q", "Other", 8)]),
        },
    );
    assert_eq!(app.sessions.len(), 1);
    assert!(!app.session_runtime_states.contains_key("nested"));
}


















#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn archived_chats_project_delete_rechecks_archive_before_restricted_http_delete() {
    use std::io::{Read, Write};
    for archived in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut latest = info("a", "p", "Archived", 10);
        if !archived {
            latest.time.archived = None;
        }
        let server = std::thread::spawn(move || {
            let mut requests = vec![];
            let count = if archived { 2 } else { 1 };
            for index in 0..count {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut bytes = vec![];
                let mut buffer = [0; 4096];
                while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(size > 0);
                    bytes.extend_from_slice(&buffer[..size]);
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let body = if index == 0 {
                    serde_json::to_vec(&latest).unwrap()
                } else {
                    serde_json::to_vec(&serde_json::json!({"deleted_ids":["a"]})).unwrap()
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
            requests
        });
        let endpoint = gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port)
            .with_tenant_scope(TenantScope::new("10000", "100000001").unwrap());
        let client = vw_gateway_client::GatewayClient::new(endpoint).unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            project_delete::delete_checked(client, info("a", "p", "Archived", 10)),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(result.is_some(), archived);
        let requests = server.join().unwrap();
        assert!(requests[0].starts_with("GET /v1/session/a?"), "{}", requests[0]);
        if archived {
            assert!(requests[1].starts_with("DELETE /v1/session/a/archived?"), "{}", requests[1]);
        }
    }
}
