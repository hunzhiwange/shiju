use super::*;
use vw_gateway_client::vw_api_types::id::TenantScope;

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

#[test]
fn archived_chats_restore_waits_for_persistence_and_clears_empty_project() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.contains("a"));
    assert_eq!(state.sessions.len(), 2);
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, restored(&current, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.sessions[0].id, "child");
    assert!(state.sessions[0].time.archived.is_some());
    assert!(state.selected_project_id.is_none());
    assert!(!state.operating_ids.contains("a"));
    assert_eq!(state.project_options().len(), 1);
}

#[test]
fn archived_chats_restore_failure_retains_row_and_allows_retry() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Err("fixture unavailable".into())), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.sessions[0].time.archived.is_some());
    assert!(!state.operating_ids.contains("a"));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_rejects_other_endpoint_and_unsolicited_response() {
    let (mut state, current) = restore_state();
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, restored(&current, Ok(saved.clone())), &current);
    assert_eq!(state.sessions.len(), 2);
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let mut other = current.clone();
    other.gateway = "http://127.0.0.1:2".into();
    apply(&mut state, restored(&other, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_duplicate_and_conflicting_delete_do_not_start_requests() {
    let (mut state, current) = restore_state();
    state.delete_confirmation =
        Some(crate::app::state::ArchivedChatsDeleteConfirmation::Chat("a".into()));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.is_empty());
    state.delete_confirmation =
        Some(crate::app::state::ArchivedChatsDeleteConfirmation::Project("p".into()));
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    assert!(state.operating_ids.is_empty());
    state.delete_confirmation = None;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let generation = state.request_generation;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert_eq!(state.request_generation, generation);
    assert!(!state.loading);
    assert_eq!(state.restore_requests.len(), 1);
}

#[test]
fn archived_chats_restore_retry_and_same_endpoint_reset_reject_late_responses() {
    let (mut state, current) = restore_state();
    let mut saved = state.sessions[0].clone();
    saved.time.archived = None;
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Err("failed".into())), &current);
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Ok(saved.clone())), &current);
    assert_eq!(state.sessions.len(), 2);
    assert!(state.operating_ids.contains("a"));
    reset(&mut state);
    state.identity = Some(current.clone());
    state.sessions = vec![info("a", "p", "Archived again", 10)];
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    apply(&mut state, restored(&current, Ok(saved)), &current);
    assert_eq!(state.sessions.len(), 1);
    assert!(state.operating_ids.contains("a"));
}

#[test]
fn archived_chats_restore_success_invalidates_older_project_refreshes() {
    let (mut state, current) = restore_state();
    apply(&mut state, ArchivedChatsMessage::Restore("a".into()), &current);
    let first_generation = state.restore_requests["a"];
    apply(&mut state, ArchivedChatsMessage::Restore("child".into()), &current);
    let second_generation = state.restore_requests["child"];
    let mut first = state.sessions[0].clone();
    first.time.archived = None;
    let mut second = state.sessions[1].clone();
    second.time.archived = None;
    apply(
        &mut state,
        ArchivedChatsMessage::Restored {
            identity: current.clone(),
            generation: first_generation,
            id: "a".into(),
            result: Ok(first),
        },
        &current,
    );
    let refresh_generation = state.request_generation;
    apply(
        &mut state,
        ArchivedChatsMessage::Restored {
            identity: current.clone(),
            generation: second_generation,
            id: "child".into(),
            result: Ok(second),
        },
        &current,
    );
    assert!(state.sessions.is_empty());
    assert!(state.operating_ids.is_empty());
    assert!(
        state.request_generation > refresh_generation,
        "older project snapshots must be invalidated by later successful restores"
    );
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
        vw_gateway_client::GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
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
        let config = vw_config_types::ui::GatewayClientSystemSettingsConfig {
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

fn load(
    state: &mut ArchivedChatsState,
    current: &Identity,
    generation: u64,
    result: Result<Vec<Info>, String>,
) {
    apply(
        state,
        ArchivedChatsMessage::Loaded { identity: current.clone(), generation, result },
        current,
    );
}

#[test]
fn archived_chats_browse_loads_historical_projects() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    load(
        &mut state,
        &identity("100000000"),
        1,
        Ok(vec![info("a", "old-project", "旧项目聊天", 10)]),
    );
    assert!(!state.loading);
    assert_eq!(state.groups()[0].project.name, "old-project");
    assert_eq!(state.groups()[0].rows[0].title, "旧项目聊天");
}

#[test]
fn archived_chats_browse_filters_and_all_three_sorts() {
    let current = identity("100000000");
    let mut state = ArchivedChatsState {
        sessions: vec![
            info("a", "p", "Zulu", 20),
            info("b", "p", "Alpha", 10),
            info("c", "q", "Other", 30),
        ],
        ..Default::default()
    };
    apply(&mut state, ArchivedChatsMessage::ProjectSelected(Some("p".into())), &current);
    assert_eq!(state.groups().len(), 1);
    for (sort, first) in [
        (ArchivedChatsSort::Updated, "a"),
        (ArchivedChatsSort::Created, "b"),
        (ArchivedChatsSort::Title, "b"),
    ] {
        apply(&mut state, ArchivedChatsMessage::SortSelected(sort), &current);
        assert_eq!(state.groups()[0].rows[0].id, first);
    }
    apply(&mut state, ArchivedChatsMessage::SearchChanged("  ZULU ".into()), &current);
    assert_eq!(state.groups()[0].rows.len(), 1);
    apply(&mut state, ArchivedChatsMessage::SearchChanged("missing".into()), &current);
    assert!(state.groups().is_empty());
}

#[test]
fn archived_chats_browse_retries_after_failure() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    let current = identity("100000000");
    load(&mut state, &current, 1, Err("unavailable".into()));
    assert_eq!(state.error.as_deref(), Some("unavailable"));
    assert!(!state.loading);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert!(state.loading);
    assert_eq!(state.request_generation, 2);
    assert!(state.error.is_none());
    load(&mut state, &current, 2, Ok(vec![]));
    assert!(!state.loading);
    assert!(state.error.is_none());
}

#[test]
fn archived_chats_browse_rejects_out_of_order_results() {
    let mut state = ArchivedChatsState::default();
    let current = identity("100000000");
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert_eq!(state.request_generation, 2);
    load(&mut state, &current, 2, Ok(vec![info("new", "p", "New", 2)]));
    load(&mut state, &current, 1, Ok(vec![info("old", "p", "Old", 1)]));
    assert_eq!(state.sessions[0].id, "new");
}

#[test]
fn archived_chats_browse_rejects_other_tenant_and_gateway() {
    let mut state =
        ArchivedChatsState { loading: true, request_generation: 1, ..Default::default() };
    let current = identity("100000001");
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: identity("100000000"),
            generation: 1,
            result: Ok(vec![info("old", "p", "Old", 1)]),
        },
        &current,
    );
    assert!(state.sessions.is_empty());
    assert!(state.loading);
    let mut old_gateway = current.clone();
    old_gateway.gateway = "http://127.0.0.1:2".into();
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: old_gateway,
            generation: 1,
            result: Err("old".into()),
        },
        &current,
    );
    assert!(state.error.is_none());
}

#[test]
fn archived_chats_browse_switch_clears_data_and_pending_actions() {
    let mut state = ArchivedChatsState {
        sessions: vec![info("old", "p", "Old", 1)],
        search: "old".into(),
        selected_project_id: Some("p".into()),
        request_generation: 8,
        loading: true,
        error: Some("old".into()),
        ..Default::default()
    };
    state.operating_ids.insert("old".into());
    state.delete_confirmation =
        Some(crate::app::state::ArchivedChatsDeleteConfirmation::Chat("old".into()));
    reset(&mut state);
    assert!(state.sessions.is_empty());
    assert!(state.selected_project_id.is_none());
    assert!(state.operating_ids.is_empty());
    assert!(state.delete_confirmation.is_none());
    assert!(state.error.is_none());
    assert!(!state.loading);
    assert!(state.request_generation > 8);
}

#[test]
fn archived_chats_browse_refresh_invalidates_previous_identity() {
    let old = identity("100000000");
    let mut state = ArchivedChatsState::default();
    apply(&mut state, ArchivedChatsMessage::Refresh, &old);
    state.sessions = vec![info("old", "p", "Old", 1)];
    let current = identity("100000001");
    apply(&mut state, ArchivedChatsMessage::Refresh, &current);
    assert!(state.sessions.is_empty());
    let generation = state.request_generation;
    apply(
        &mut state,
        ArchivedChatsMessage::Loaded {
            identity: old,
            generation,
            result: Ok(vec![info("old", "p", "Old", 1)]),
        },
        &current,
    );
    assert!(state.loading);
    load(&mut state, &current, generation, Ok(vec![info("new", "q", "New", 2)]));
    assert_eq!(state.sessions[0].id, "new");
    let mut other_auth = current.clone();
    other_auth.auth_fingerprint = [1; 32];
    apply(&mut state, ArchivedChatsMessage::Refresh, &other_auth);
    assert!(state.sessions.is_empty());
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
    let endpoint = vw_gateway_client::GatewayEndpoint::new("127.0.0.1", port)
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
        let config = vw_config_types::ui::GatewayClientSystemSettingsConfig {
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
