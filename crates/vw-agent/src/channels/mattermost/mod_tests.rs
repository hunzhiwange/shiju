use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn one_shot_http_server(status_line: &'static str, body: &'static str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0; 4096];
        let _ = stream.read(&mut buffer).await.unwrap();
        let response = format!(
            "{status_line}\r\ncontent-length: {}\r\ncontent-type: application/json\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    format!("http://{addr}")
}

/// 启动一个持续应答的 HTTP 服务器，捕获收到的首个请求文本。
///
/// 与 `one_shot_http_server` 不同，它接受多个连接（打字循环会重复发送），
/// 并通过返回的接收端暴露第一个请求的原始字节，便于断言请求体。
async fn recording_http_server(
    status_line: &'static str,
    body: &'static str,
) -> (String, tokio::sync::mpsc::Receiver<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            let mut buffer = vec![0; 4096];
            let n = stream.read(&mut buffer).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..n]).to_string();
            let _ = tx.send(request).await;
            let response = format!(
                "{status_line}\r\ncontent-length: {}\r\ncontent-type: application/json\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });
    (format!("http://{addr}"), rx)
}

fn channel_at(base_url: String) -> MattermostChannel {
    MattermostChannel::new(base_url, "token".to_string(), None, vec!["*".to_string()], true, false)
}

#[test]
fn constructor_trims_base_url_and_keeps_runtime_flags() {
    let channel = MattermostChannel::new(
        "https://mattermost.example.com///".to_string(),
        "token".to_string(),
        Some("chan-1".to_string()),
        vec!["alice".to_string()],
        false,
        true,
    );

    assert_eq!(channel.base_url, "https://mattermost.example.com");
    assert_eq!(channel.channel_id.as_deref(), Some("chan-1"));
    assert_eq!(channel.allowed_users, vec!["alice"]);
    assert!(!channel.thread_replies);
    assert!(channel.mention_only);
}

#[test]
fn group_reply_sender_override_rejects_empty_and_honors_wildcard() {
    let specific = MattermostChannel::new(
        "https://mattermost.example.com".to_string(),
        "token".to_string(),
        None,
        vec!["*".to_string()],
        true,
        true,
    )
    .with_group_reply_allowed_senders(vec![
        " user-2 ".to_string(),
        String::new(),
        "user-1".to_string(),
        "user-1".to_string(),
    ]);

    assert_eq!(specific.group_reply_allowed_sender_ids, vec!["user-1", "user-2"]);
    assert!(specific.is_group_sender_trigger_enabled(" user-1 "));
    assert!(!specific.is_group_sender_trigger_enabled(" "));

    let wildcard = specific.with_group_reply_allowed_senders(vec!["*".to_string()]);
    assert!(wildcard.is_group_sender_trigger_enabled("anyone"));
}

#[tokio::test]
async fn send_posts_root_level_message() {
    let base_url = one_shot_http_server("HTTP/1.1 201 Created", "{}").await;
    let channel = MattermostChannel::new(
        base_url,
        "token".to_string(),
        None,
        vec!["*".to_string()],
        true,
        false,
    );

    channel.send(&SendMessage::new("hello", "channel-1")).await.unwrap();
}

#[tokio::test]
async fn send_reports_sanitized_error_status() {
    let base_url = one_shot_http_server(
        "HTTP/1.1 500 Internal Server Error",
        r#"{"message":"token sk-secret should be hidden"}"#,
    )
    .await;
    let channel = MattermostChannel::new(
        base_url,
        "token".to_string(),
        None,
        vec!["*".to_string()],
        true,
        false,
    );

    let error =
        channel.send(&SendMessage::new("hello", "channel-1:root-1")).await.unwrap_err().to_string();

    assert!(error.contains("Mattermost post failed"));
    assert!(error.contains("500 Internal Server Error"));
}

#[tokio::test]
async fn send_includes_root_id_for_thread_recipient() {
    let (base_url, mut rx) = recording_http_server("HTTP/1.1 201 Created", "{}").await;
    let channel = channel_at(base_url);

    channel.send(&SendMessage::new("threaded reply", "channel-1:root-9")).await.unwrap();

    let request = rx.recv().await.unwrap();
    assert!(request.contains("/api/v4/posts"));
    assert!(request.contains("\"root_id\":\"root-9\""));
    assert!(request.contains("\"channel_id\":\"channel-1\""));
}

#[test]
fn is_user_allowed_rejects_empty_list_and_non_matches() {
    let deny_all = MattermostChannel::new(
        "https://mm.example.com".to_string(),
        "token".to_string(),
        None,
        Vec::new(),
        true,
        false,
    );
    assert!(!deny_all.is_user_allowed("anyone"));

    let specific = MattermostChannel::new(
        "https://mm.example.com".to_string(),
        "token".to_string(),
        None,
        vec!["alice".to_string()],
        true,
        false,
    );
    assert!(specific.is_user_allowed("alice"));
    assert!(!specific.is_user_allowed("bob"));
}

#[test]
fn parse_post_drops_unauthorized_user() {
    let channel = MattermostChannel::new(
        "https://mm.example.com".to_string(),
        "token".to_string(),
        None,
        vec!["alice".to_string()],
        true,
        false,
    );
    let post = json!({
        "id": "p1",
        "user_id": "mallory",
        "message": "hi",
        "create_at": 1_600_000_000_000_i64,
        "root_id": ""
    });

    let msg = channel.parse_mattermost_post(&post, "bot", "botname", 1_500_000_000_000_i64, "chan");
    assert!(msg.is_none());
}

#[tokio::test]
async fn health_check_true_on_success() {
    let base_url = one_shot_http_server("HTTP/1.1 200 OK", "{}").await;
    let channel = channel_at(base_url);
    assert!(channel.health_check().await);
}

#[tokio::test]
async fn health_check_false_on_error_status() {
    let base_url = one_shot_http_server("HTTP/1.1 401 Unauthorized", "{}").await;
    let channel = channel_at(base_url);
    assert!(!channel.health_check().await);
}

#[tokio::test]
async fn health_check_false_when_unreachable() {
    // 绑定后立刻丢弃监听端口，使连接被拒绝。
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let channel = channel_at(format!("http://{addr}"));
    assert!(!channel.health_check().await);
}

#[tokio::test]
async fn get_bot_identity_parses_id_and_username() {
    let base_url =
        one_shot_http_server("HTTP/1.1 200 OK", r#"{"id":"bot-id-1","username":"mybot"}"#).await;
    let channel = channel_at(base_url);

    let (id, username) = channel.get_bot_identity().await;
    assert_eq!(id, "bot-id-1");
    assert_eq!(username, "mybot");
}

#[tokio::test]
async fn get_bot_identity_defaults_missing_username_only() {
    let base_url = one_shot_http_server("HTTP/1.1 200 OK", r#"{"id":"bot-id"}"#).await;
    let channel = channel_at(base_url);

    assert_eq!(channel.get_bot_identity().await, ("bot-id".to_string(), String::new()));
}

#[tokio::test]
async fn get_bot_identity_returns_empty_on_failure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let channel = channel_at(format!("http://{addr}"));

    let (id, username) = channel.get_bot_identity().await;
    assert!(id.is_empty());
    assert!(username.is_empty());
}

#[tokio::test]
async fn start_typing_posts_event_then_stop_aborts() {
    let (base_url, mut rx) = recording_http_server("HTTP/1.1 200 OK", "{}").await;
    let channel = channel_at(base_url);

    channel.start_typing("channel-7:root-3").await.unwrap();

    let request = rx.recv().await.unwrap();
    assert!(request.contains("/api/v4/users/me/typing"));
    assert!(request.contains("\"channel_id\":\"channel-7\""));
    assert!(request.contains("\"parent_id\":\"root-3\""));

    // 句柄应已保存，stop_typing 取出并中止。
    assert!(channel.typing_handle.lock().is_some());
    channel.stop_typing("channel-7:root-3").await.unwrap();
    assert!(channel.typing_handle.lock().is_none());
}

#[tokio::test]
async fn start_typing_without_parent_omits_parent_id() {
    let (base_url, mut rx) = recording_http_server("HTTP/1.1 200 OK", "{}").await;
    let channel = channel_at(base_url);

    channel.start_typing("channel-only").await.unwrap();

    let request = rx.recv().await.unwrap();
    assert!(request.contains("\"channel_id\":\"channel-only\""));
    assert!(!request.contains("parent_id"));

    channel.stop_typing("channel-only").await.unwrap();
}

#[tokio::test]
async fn stop_typing_is_idempotent_without_active_handle() {
    let channel = channel_at("https://mm.example.com".to_string());
    // 没有活动句柄时调用应安全返回。
    channel.stop_typing("anything").await.unwrap();
    assert!(channel.typing_handle.lock().is_none());
}

#[test]
fn name_reports_mattermost() {
    let channel = channel_at("https://mm.example.com".to_string());
    assert_eq!(channel.name(), "mattermost");
}

#[tokio::test]
async fn listen_requires_channel_id() {
    let channel = MattermostChannel::new(
        "https://mm.example.com".to_string(),
        "token".to_string(),
        None,
        vec!["*".to_string()],
        true,
        false,
    );
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let err = channel.listen(tx).await.unwrap_err().to_string();
    assert!(err.contains("channel_id required"));
}
