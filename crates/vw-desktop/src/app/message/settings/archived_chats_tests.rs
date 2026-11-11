use super::*;
use vw_gateway_client::vw_api_types::id::TenantScope;

fn identity(company: &str) -> Identity {
    Identity {
        gateway: "http://127.0.0.1:1".into(),
        tenant: TenantScope::new("10000", company).unwrap(),
        auth_fingerprint: [0; 32],
    }
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
