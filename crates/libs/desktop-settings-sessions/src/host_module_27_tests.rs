use crate::archived_chats::*;
use crate::state::ArchivedChatsState;
use session_info::Info;
use vw_api_transport::id::TenantScope;
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
        let iced_runtime::Action::Output(message) = action else {
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
    let iced_runtime::Action::Output(message) = action else {
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
        let iced_runtime::Action::Output(message) = output else {
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
