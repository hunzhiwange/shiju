use super::*;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn mock_server(
    responses: Vec<(&'static str, u16, &'static str)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut responses: VecDeque<_> = responses
        .into_iter()
        .map(|(path, status, body)| (path.to_string(), status, body.to_string()))
        .collect();

    let handle = tokio::spawn(async move {
        let mut seen = Vec::new();
        while let Some((expected_path, status, body)) = responses.pop_front() {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("")
                .to_string();
            seen.push(path.clone());
            assert_eq!(path, expected_path);

            let status_text = match status {
                200 => "OK",
                404 => "Not Found",
                _ => "Error",
            };
            let response = format!(
                "HTTP/1.1 {status} {status_text}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
        seen
    });

    (format!("http://{addr}"), handle)
}

fn channel(homeserver: String, room_id: &str) -> MatrixChannel {
    MatrixChannel::new(
        homeserver,
        "token".to_string(),
        room_id.to_string(),
        vec!["@user:server".to_string()],
    )
}

fn channel_with_hints(
    homeserver: String,
    owner_hint: Option<&str>,
    device_id_hint: Option<&str>,
) -> MatrixChannel {
    MatrixChannel::new_with_session_hint(
        homeserver,
        "token".to_string(),
        "!room:server".to_string(),
        vec!["@user:server".to_string()],
        owner_hint.map(str::to_string),
        device_id_hint.map(str::to_string),
    )
}

fn channel_with_store(homeserver: String, store_dir: PathBuf) -> MatrixChannel {
    MatrixChannel::new_with_session_hint_and_vibewindow_dir(
        homeserver,
        "token".to_string(),
        "!room:server".to_string(),
        vec!["@user:server".to_string()],
        None,
        None,
        Some(store_dir),
    )
}

#[tokio::test]
async fn target_room_id_returns_direct_room_ids_without_http() {
    let ch = channel("http://127.0.0.1:9".to_string(), "!room:server");

    assert_eq!(ch.target_room_id().await.unwrap(), "!room:server");
}

#[tokio::test]
async fn target_room_id_accepts_matrix_to_room_id_permalink_without_http() {
    let ch =
        channel("http://127.0.0.1:9".to_string(), "https://matrix.to/#/!room%3Aserver?via=server");

    assert_eq!(ch.target_room_id().await.unwrap(), "!room:server");
}

#[tokio::test]
async fn resolve_room_id_accepts_direct_room_ids() {
    let ch = channel("http://127.0.0.1:9".to_string(), "  !room:server  ");

    assert_eq!(ch.resolve_room_id().await.unwrap(), "!room:server");
}

#[tokio::test]
async fn resolve_room_alias_fetches_directory_endpoint_and_caches_target_room() {
    let (homeserver, handle) = mock_server(vec![(
        "/_matrix/client/v3/directory/room/%23alias%3Aserver",
        200,
        r#"{"room_id":"!resolved:server"}"#,
    )])
    .await;
    let ch = channel(homeserver, "#alias:server");

    assert_eq!(ch.target_room_id().await.unwrap(), "!resolved:server");
    assert_eq!(ch.target_room_id().await.unwrap(), "!resolved:server");
    assert_eq!(handle.await.unwrap().len(), 1);
}

#[tokio::test]
async fn resolve_room_alias_accepts_bare_alias_reference() {
    let (homeserver, handle) = mock_server(vec![(
        "/_matrix/client/v3/directory/room/%23vibewindow-scheduler%3Alocalhost",
        200,
        r#"{"room_id":"!resolved:localhost"}"#,
    )])
    .await;
    let ch = channel(homeserver, "  vibewindow-scheduler:localhost  ");

    assert_eq!(ch.target_room_id().await.unwrap(), "!resolved:localhost");
    assert_eq!(handle.await.unwrap().len(), 1);
}

#[tokio::test]
async fn resolve_room_alias_accepts_matrix_to_permalink() {
    let (homeserver, handle) = mock_server(vec![(
        "/_matrix/client/v3/directory/room/%23alias%3Aserver",
        200,
        r#"{"room_id":"!resolved:server"}"#,
    )])
    .await;
    let ch = channel(homeserver, "https://matrix.to/#/#alias:server");

    assert_eq!(ch.target_room_id().await.unwrap(), "!resolved:server");
    assert_eq!(handle.await.unwrap().len(), 1);
}

#[tokio::test]
async fn resolve_room_alias_reports_sanitized_error_body() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/directory/room/%23missing%3Aserver",
        500,
        r#"{"error":"token syt_secret_value failed"}"#,
    )])
    .await;
    let ch = channel(homeserver, "#missing:server");

    let err = ch.resolve_room_id().await.expect_err("alias failure should error");

    assert!(err.to_string().contains("Matrix room alias resolution failed"));
    assert!(!err.to_string().contains("syt_secret_value"));
}

#[tokio::test]
async fn resolve_room_alias_rejects_malformed_success_payload() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/directory/room/%23alias%3Aserver",
        200,
        r#"{"unexpected":true}"#,
    )])
    .await;
    let ch = channel(homeserver, "#alias:server");

    assert!(ch.resolve_room_id().await.is_err());
}

#[tokio::test]
async fn resolve_room_id_rejects_invalid_room_reference() {
    let ch = channel("http://127.0.0.1:9".to_string(), "plain-room");

    let err = ch.resolve_room_id().await.expect_err("invalid room reference");

    assert!(err.to_string().contains("must start with"));
}

#[tokio::test]
async fn get_my_user_id_reads_whoami_response() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    assert_eq!(ch.get_my_user_id().await.unwrap(), "@bot:server");
}

#[tokio::test]
async fn get_my_user_id_reports_sanitized_whoami_errors() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        401,
        r#"{"error":"token syt_secret_value failed"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    let err = ch.get_my_user_id().await.expect_err("whoami failure should error");
    let message = err.to_string();

    assert!(message.contains("Matrix whoami failed"));
    assert!(!message.contains("syt_secret_value"));
}

#[tokio::test]
async fn ensure_room_supported_accepts_accessible_unencrypted_room() {
    let (homeserver, _handle) = mock_server(vec![
        ("/_matrix/client/v3/rooms/%21room%3Aserver/joined_members", 200, r#"{"joined":{}}"#),
        (
            "/_matrix/client/v3/rooms/%21room%3Aserver/state/m.room.encryption",
            404,
            r#"{"errcode":"M_NOT_FOUND"}"#,
        ),
    ])
    .await;
    let ch = channel(homeserver, "!room:server");

    ch.ensure_room_supported("!room:server").await.unwrap();
}

#[tokio::test]
async fn ensure_room_supported_accepts_accessible_encrypted_room() {
    let (homeserver, _handle) = mock_server(vec![
        ("/_matrix/client/v3/rooms/%21room%3Aserver/joined_members", 200, r#"{"joined":{}}"#),
        (
            "/_matrix/client/v3/rooms/%21room%3Aserver/state/m.room.encryption",
            200,
            r#"{"algorithm":"m.megolm.v1.aes-sha2"}"#,
        ),
    ])
    .await;
    let ch = channel(homeserver, "!room:server");

    ch.ensure_room_supported("!room:server").await.unwrap();
}

#[tokio::test]
async fn ensure_room_supported_errors_when_access_check_fails() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/rooms/%21room%3Aserver/joined_members",
        403,
        r#"{"error":"forbidden"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    let err = ch.ensure_room_supported("!room:server").await.expect_err("access failure");

    assert!(err.to_string().contains("room access check failed"));
}

#[tokio::test]
async fn ensure_room_supported_errors_when_encryption_check_fails() {
    let (homeserver, _handle) = mock_server(vec![
        ("/_matrix/client/v3/rooms/%21room%3Aserver/joined_members", 200, r#"{"joined":{}}"#),
        (
            "/_matrix/client/v3/rooms/%21room%3Aserver/state/m.room.encryption",
            500,
            r#"{"error":"token syt_secret_value failed"}"#,
        ),
    ])
    .await;
    let ch = channel(homeserver, "!room:server");

    let err = ch.ensure_room_supported("!room:server").await.expect_err("encryption failure");
    let message = err.to_string();

    assert!(message.contains("room encryption check failed"));
    assert!(!message.contains("syt_secret_value"));
}

#[tokio::test]
async fn matrix_client_restores_from_whoami_identity() {
    let (homeserver, handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    let client = ch.matrix_client().await.unwrap();
    let cached = ch.matrix_client().await.unwrap();

    assert_eq!(client.user_id().map(ToString::to_string), Some("@bot:server".to_string()));
    assert_eq!(client.device_id().map(ToString::to_string), Some("DEVICE".to_string()));
    assert_eq!(cached.user_id().map(ToString::to_string), Some("@bot:server".to_string()));
    assert_eq!(handle.await.unwrap().len(), 1);
}

#[tokio::test]
async fn matrix_client_prefers_whoami_identity_over_mismatched_hints() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let ch = channel_with_hints(homeserver, Some("@hint:server"), Some("HINTDEVICE"));

    let client = ch.matrix_client().await.unwrap();

    assert_eq!(client.user_id().map(ToString::to_string), Some("@bot:server".to_string()));
    assert_eq!(client.device_id().map(ToString::to_string), Some("DEVICE".to_string()));
}

#[tokio::test]
async fn matrix_client_uses_device_hint_when_whoami_omits_device_id() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server"}"#,
    )])
    .await;
    let ch = channel_with_hints(homeserver, None, Some("HINTDEVICE"));

    let client = ch.matrix_client().await.unwrap();

    assert_eq!(client.user_id().map(ToString::to_string), Some("@bot:server".to_string()));
    assert_eq!(client.device_id().map(ToString::to_string), Some("HINTDEVICE".to_string()));
}

#[tokio::test]
async fn matrix_client_errors_when_whoami_omits_device_id_without_hint() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    let err = ch.matrix_client().await.expect_err("missing device id should error");

    assert!(err.to_string().contains("whoami response did not include device_id"));
}

#[tokio::test]
async fn matrix_client_falls_back_to_session_hints_when_whoami_fails() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        500,
        r#"{"error":"homeserver unavailable"}"#,
    )])
    .await;
    let ch = channel_with_hints(homeserver, Some("@hint:server"), Some("HINTDEVICE"));

    let client = ch.matrix_client().await.unwrap();

    assert_eq!(client.user_id().map(ToString::to_string), Some("@hint:server".to_string()));
    assert_eq!(client.device_id().map(ToString::to_string), Some("HINTDEVICE".to_string()));
}

#[tokio::test]
async fn matrix_client_returns_whoami_error_without_complete_hints() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        500,
        r#"{"error":"token syt_secret_value failed"}"#,
    )])
    .await;
    let ch = channel_with_hints(homeserver, Some("@hint:server"), None);

    let err = ch.matrix_client().await.expect_err("incomplete hints should not mask whoami error");
    let message = err.to_string();

    assert!(message.contains("Matrix whoami failed"));
    assert!(!message.contains("syt_secret_value"));
}

#[tokio::test]
async fn matrix_client_errors_when_whoami_user_id_is_invalid() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"bot","device_id":"DEVICE"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");

    assert!(ch.matrix_client().await.is_err());
}

#[tokio::test]
async fn matrix_client_initializes_persistent_store_dir() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let temp = tempdir().unwrap();
    let ch = channel_with_store(homeserver, temp.path().to_path_buf());

    let client = ch.matrix_client().await.unwrap();

    assert_eq!(client.user_id().map(ToString::to_string), Some("@bot:server".to_string()));
    assert!(temp.path().join("state/matrix").is_dir());
}

#[tokio::test]
async fn matrix_client_errors_when_store_dir_cannot_be_created() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("state"), b"not a directory").unwrap();
    let ch = channel_with_store(homeserver, temp.path().to_path_buf());

    let err = ch.matrix_client().await.expect_err("store directory creation should fail");

    assert!(err.to_string().contains("persistent store directory"));
}

#[tokio::test]
async fn log_e2ee_diagnostics_handles_restored_and_logged_out_clients() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/account/whoami",
        200,
        r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
    )])
    .await;
    let ch = channel(homeserver, "!room:server");
    let restored = ch.matrix_client().await.unwrap();
    let logged_out =
        MatrixSdkClient::builder().homeserver_url("http://127.0.0.1:9").build().await.unwrap();

    ch.log_e2ee_diagnostics(&restored).await;
    ch.log_e2ee_diagnostics(&logged_out).await;

    assert!(restored.user_id().is_some());
    assert!(logged_out.user_id().is_none());
}
// 本文件验证 Matrix 房间引用解析、别名查询缓存与错误脱敏。
