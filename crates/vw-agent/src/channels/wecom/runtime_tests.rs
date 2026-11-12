use super::*;
use runtime::{WeComRuntimeScope, connected};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
    time::timeout,
};
use tokio_tungstenite::{WebSocketStream, accept_async};
use vw_api_types::id::TenantScope;

type Socket = WebSocketStream<TcpStream>;

fn scope() -> WeComRuntimeScope {
    let mut config = crate::config::Config::default();
    config.config_path = std::env::temp_dir().join(Uuid::new_v4().to_string()).join("config.json");
    WeComRuntimeScope::from_config(&config, TenantScope::default()).unwrap()
}

async fn start(
    scope: WeComRuntimeScope,
    secret: &str,
) -> (WeComChannel, Socket, JoinHandle<anyhow::Result<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let channel = WeComChannel::new_for_test(
        "bot".into(),
        secret.into(),
        format!("ws://{}", listener.local_addr().unwrap()),
    )
    .with_runtime_scope(scope);
    let clone = channel.clone();
    let (tx, _rx) = mpsc::channel(8);
    let task = tokio::spawn(async move { clone.listen(tx).await });
    let (stream, _) = timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
    let mut socket = accept_async(stream).await.unwrap();
    let frame = next(&mut socket).await;
    assert_eq!(frame["body"]["secret"], secret);
    assert!(connected(channel.runtime_scope.as_ref().unwrap()).is_err());
    socket
        .send(Message::Text(json!({"headers":frame["headers"], "errcode":0}).to_string().into()))
        .await
        .unwrap();
    timeout(Duration::from_secs(2), async {
        while channel.outbound.read().await.is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    (channel, socket, task)
}

async fn next(socket: &mut Socket) -> Value {
    let frame = timeout(Duration::from_secs(2), socket.next()).await.unwrap().unwrap().unwrap();
    serde_json::from_str(frame.into_text().unwrap().as_ref()).unwrap()
}

#[tokio::test]
async fn wecom_runtime_registers_authenticated_listener_connection() {
    let scope = scope();
    let (_channel, mut socket, task) = start(scope.clone(), "secret-a").await;
    let handle = connected(&scope).expect("authenticated listener must be discoverable");
    let send = tokio::spawn(async move { handle.send_proactive("chat", "result").await });
    let frame = next(&mut socket).await;
    assert_eq!(frame["cmd"], "aibot_send_msg");
    assert_eq!(frame["body"]["chatid"], "chat");
    socket
        .send(Message::Text(json!({"headers":frame["headers"], "errcode":0}).to_string().into()))
        .await
        .unwrap();
    send.await.unwrap().unwrap();
    task.abort();
    let _ = task.await;
    assert!(connected(&scope).is_err());
}

#[test]
fn wecom_runtime_rejects_disconnected_with_daemon_guidance() {
    let error = connected(&scope()).err().unwrap().to_string();
    assert!(error.contains("daemon"));
}

#[tokio::test]
async fn wecom_runtime_isolates_tenant_store_workspace_and_instance_owner() {
    let scope = scope();
    let (_channel, _socket, task) = start(scope.clone(), "secret-a").await;
    assert!(connected(&scope).is_ok());
    for other in [
        WeComRuntimeScope::from_config(&crate::config::Config::default(), TenantScope::default())
            .unwrap(),
        {
            let mut other = scope.clone();
            other.tenant = TenantScope::new("other", "company").unwrap();
            other
        },
        {
            let mut other = scope.clone();
            other.workspace.push("other");
            other
        },
        {
            let mut other = scope.clone();
            other.config_store.push("other");
            other
        },
    ] {
        assert!(connected(&other).is_err());
    }
    assert!(runtime::Registration::claim(scope.clone()).is_err());
    let mut other_scope = scope.clone();
    other_scope.tenant = TenantScope::new("other", "company").unwrap();
    let (_other, mut other_socket, other_task) = start(other_scope.clone(), "secret-b").await;
    let other_handle = connected(&other_scope).unwrap();
    let send =
        tokio::spawn(
            async move { other_handle.send_proactive("other-chat", "other-result").await },
        );
    let frame = next(&mut other_socket).await;
    assert_eq!(frame["body"]["chatid"], "other-chat");
    other_socket
        .send(Message::Text(json!({"headers":frame["headers"],"errcode":0}).to_string().into()))
        .await
        .unwrap();
    send.await.unwrap().unwrap();
    other_task.abort();
    let _ = other_task.await;
    assert!(connected(&other_scope).is_err());
    assert!(connected(&scope).is_ok());
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn wecom_runtime_disconnect_and_credentials_reload_invalidate_old_handle() {
    let scope = scope();
    let (_channel, mut socket, task) = start(scope.clone(), "secret-a").await;
    let old = connected(&scope).unwrap();
    socket.close(None).await.unwrap();
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    assert!(connected(&scope).is_err());
    let (_new, mut socket, task) = start(scope.clone(), "secret-b").await;
    assert!(old.send_proactive("chat", "stale").await.is_err());
    assert!(
        timeout(Duration::from_millis(100), socket.next()).await.is_err(),
        "stale send must not replay on new connection"
    );
    assert!(connected(&scope).is_ok());
    task.abort();
    let _ = task.await;
    assert!(connected(&scope).is_err());
}

#[test]
fn wecom_runtime_old_generation_drop_preserves_new_generation() {
    let scope = scope();
    let old = runtime::Registration::claim(scope.clone()).unwrap();
    // Force the cleanup race: a stale guard must never remove a replacement owner.
    runtime::registry().lock().unwrap().remove(&scope);
    let new = runtime::Registration::claim(scope.clone()).unwrap();
    drop(old);
    assert!(runtime::Registration::claim(scope.clone()).is_err());
    drop(new);
    assert!(runtime::Registration::claim(scope).is_ok());
}

#[tokio::test]
async fn wecom_runtime_rejects_duplicate_listener_before_opening_socket() {
    let scope = scope();
    let (channel, _socket, task) = start(scope.clone(), "secret").await;
    let (tx, _rx) = mpsc::channel(1);
    let error = timeout(Duration::from_secs(1), channel.listen(tx)).await.unwrap().unwrap_err();
    assert!(error.to_string().contains("已有企微监听器"));
    assert!(connected(&scope).is_ok());
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn wecom_runtime_failed_authentication_releases_owner_without_publication() {
    let scope = scope();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let channel = WeComChannel::new_for_test(
        "bot".into(),
        "secret".into(),
        format!("ws://{}", listener.local_addr().unwrap()),
    )
    .with_runtime_scope(scope.clone());
    let (tx, _rx) = mpsc::channel(1);
    let task = tokio::spawn(async move { channel.listen(tx).await });
    let (stream, _) = timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
    let mut socket = accept_async(stream).await.unwrap();
    let frame = next(&mut socket).await;
    assert!(connected(&scope).is_err());
    socket
        .send(Message::Text(json!({"headers":frame["headers"],"errcode":40001}).to_string().into()))
        .await
        .unwrap();
    timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    assert!(connected(&scope).is_err());
    assert!(runtime::Registration::claim(scope).is_ok());
}
