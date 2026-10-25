//! 重载和取消必须释放监听器持有的真实 socket，避免端口泄漏与重复消费。
use super::*;
use std::net::SocketAddr;

async fn listener() -> (tokio::task::JoinHandle<()>, SocketAddr) {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    let (ready, started) = tokio::sync::oneshot::channel();
    let handle = tokio::spawn(async move {
        ready.send(()).unwrap();
        let _ = socket.accept().await;
    });
    started.await.unwrap();
    (handle, address)
}

#[tokio::test]
async fn channel_reload_waits_for_old_listeners_to_exit() {
    let (first, first_address) = listener().await;
    let (second, second_address) = listener().await;
    let mut listeners = ChannelListeners { handles: vec![first, second] };
    assert!(tokio::net::TcpListener::bind(first_address).await.is_err());
    listeners.stop().await;
    let _first = tokio::net::TcpListener::bind(first_address).await.unwrap();
    let _second = tokio::net::TcpListener::bind(second_address).await.unwrap();
    assert!(listeners.handles.is_empty());
    listeners.stop().await;
}

#[tokio::test]
async fn channel_runtime_cancellation_aborts_owned_listeners() {
    let (handle, address) = listener().await;
    drop(ChannelListeners { handles: vec![handle] });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(_listener) = tokio::net::TcpListener::bind(address).await {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn channel_reload_starts_from_empty_and_rotates_live_telegram_credentials() {
    use crate::app::agent::config::{SystemConfigStore, schema::ChannelConfigWatcher};
    use crate::global_database::GlobalDatabase;
    use axum::{Json, Router, extract::OriginalUri, routing::any};
    use vw_api_types::id::TenantScope;

    let temp = tempfile::TempDir::new().unwrap();
    let store =
        SystemConfigStore::new(GlobalDatabase::new(temp.path().join("config.sqlite3")).unwrap())
            .unwrap();
    let mut config = Config::default();
    config.config_path = temp.path().join("vibewindow.json");
    config.workspace_dir = temp.path().to_path_buf();
    config.memory.backend = "none".into();
    config.default_provider = Some("ollama".into());
    config.default_model = Some("test-model".into());
    let source = ChannelConfigWatcher::from_store(store.clone(), &config, true).unwrap();
    let (requests, mut received) = tokio::sync::mpsc::unbounded_channel();
    let server_socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", server_socket.local_addr().unwrap());
    let app = Router::new().fallback(any(move |OriginalUri(uri): OriginalUri| {
        let requests = requests.clone();
        async move {
            let _ = requests.send(uri.path().to_owned());
            tokio::time::sleep(Duration::from_millis(50)).await;
            Json(serde_json::json!({"ok":true, "result":[]}))
        }
    }));
    let server = tokio::spawn(async move { axum::serve(server_socket, app).await.unwrap() });
    let mut runtime = tokio::spawn(run_channels_with_source(config.clone(), source));
    for token in ["first-token", "second-token"] {
        config.channels_config.telegram = Some(
            serde_json::from_value(serde_json::json!({
                "bot_token":token, "allowed_users":["alice"], "base_url":base_url
            }))
            .unwrap(),
        );
        store.upsert(&TenantScope::default(), &serde_json::to_string(&config).unwrap()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                tokio::select! {
                    result = &mut runtime => panic!("runtime exited: {result:?}"),
                    path = received.recv() => {
                        if path.unwrap().contains(token) { break; }
                    }
                }
            }
        })
        .await
        .unwrap();
    }
    config.channels_config.telegram = None;
    store.upsert(&TenantScope::default(), &serde_json::to_string(&config).unwrap()).unwrap();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    while received.try_recv().is_ok() {}
    assert!(tokio::time::timeout(Duration::from_millis(200), received.recv()).await.is_err());
    assert!(!runtime.is_finished()); // 无通道时仍保持监测。
    runtime.abort();
    let _ = runtime.await;
    server.abort();
    let _ = server.await;
}
