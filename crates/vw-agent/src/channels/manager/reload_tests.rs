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
    let mut listeners = ChannelListeners::default();
    listeners.handles = vec![first, second];
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
    let mut listeners = ChannelListeners::default();
    listeners.handles.push(handle);
    drop(listeners);
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

#[test]
fn channel_reload_preserves_explicit_project_selection() {
    let listeners = ChannelListeners::default();
    let mut config = Config::default();
    let state = listeners.configure_project_state(&config);
    let chat = ChannelChatScope {
        tenant: crate::session::ui_store::current_tenant_scope(),
        channel: "telegram".into(),
        instance: "default".into(),
        reply_target: "room".into(),
        thread: None,
        sender: "sender".into(),
    };
    state.lock().unwrap().selections.insert(chat.clone(), "explicit-project".into());
    config
        .channels_config
        .instance_default_projects
        .insert("telegram:default".into(), "new-default".into());
    let reloaded = listeners.configure_project_state(&config);
    assert!(Arc::ptr_eq(&state, &reloaded));
    let reloaded = reloaded.lock().unwrap();
    assert_eq!(reloaded.selections[&chat], "explicit-project");
    assert_eq!(reloaded.config.instance_default_projects["telegram:default"], "new-default");
}

#[tokio::test]
async fn wecom_runtime_manager_stop_and_drop_release_authenticated_connection() {
    use crate::channels::wecom::{
        WeComChannel,
        runtime::{WeComRuntimeScope, connected},
    };
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{accept_async, tungstenite::Message};
    let temp = tempfile::TempDir::new().unwrap();
    let mut config = Config::default();
    config.config_path = temp.path().join("config.json");
    config.workspace_dir = temp.path().into();
    let scope =
        WeComRuntimeScope::from_config(&config, vw_api_types::id::TenantScope::default()).unwrap();
    for stop in [true, false] {
        let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let channel = WeComChannel::new_for_test(
            "bot".into(),
            "secret".into(),
            format!("ws://{}", socket.local_addr().unwrap()),
        )
        .with_runtime_scope(scope.clone());
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let mut listeners = ChannelListeners::default();
        listeners.handles.push(spawn_supervised_listener(
            ConfiguredChannel::new("WeCom", Arc::new(channel)),
            tx,
            1,
            1,
        ));
        let (stream, _) =
            tokio::time::timeout(Duration::from_secs(2), socket.accept()).await.unwrap().unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let subscribe = ws.next().await.unwrap().unwrap().into_text().unwrap();
        let subscribe: serde_json::Value = serde_json::from_str(&subscribe).unwrap();
        ws.send(Message::Text(
            serde_json::json!({"headers":subscribe["headers"],"errcode":0}).to_string().into(),
        ))
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while connected(&scope).is_err() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("daemon listener must publish its authenticated connection");
        let old = connected(&scope).unwrap();
        if stop {
            listeners.stop().await;
        }
        drop(listeners);
        tokio::time::timeout(Duration::from_secs(2), async {
            while connected(&scope).is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(old.send_proactive("chat", "stale").await.is_err());
    }
}

#[tokio::test]
async fn channel_reload_captures_nondefault_tenant_before_poll() {
    use crate::config::{SystemConfigStore, schema::ChannelConfigWatcher};
    use crate::global_database::GlobalDatabase;
    use vw_api_types::id::TenantScope;
    let temp = tempfile::TempDir::new().unwrap();
    let store =
        SystemConfigStore::new(GlobalDatabase::new(temp.path().join("config.sqlite3")).unwrap())
            .unwrap();
    let mut config = Config::default();
    config.config_path = temp.path().join("config.json");
    let tenant = TenantScope::new("tenant", "company").unwrap();
    let payload = |bot: &str| {
        serde_json::json!({"channels_config":{"cli":true,"wecom":{"bot_id":bot,"secret":"fixture"}}})
            .to_string()
    };
    store.upsert(&TenantScope::default(), &payload("wrong-bot")).unwrap();
    store.upsert(&tenant, &payload("correct-bot")).unwrap();
    let mut watcher = crate::session::ui_store::with_tenant_scope(tenant, async {
        ChannelConfigWatcher::from_store(store, &config, false).unwrap()
    })
    .await;
    assert_eq!(watcher.poll(&config).await.unwrap().unwrap().wecom.unwrap().bot_id, "correct-bot");
}
