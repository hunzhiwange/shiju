use super::*;
use crate::channels::wecom::runtime::WeComRuntimeScope;
use crate::channels::{WeComChannel, traits::Channel};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::time::{Duration, timeout};
use vw_api_types::id::TenantScope;

fn fixture(tmp: &tempfile::TempDir) -> Config {
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("config.json");
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config.channels_config.wecom =
        Some(serde_json::from_value(json!({"bot_id":"bot","secret":"secret"})).unwrap());
    config.channels_config.wecom_webhook = Some(serde_json::from_value(json!({"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture-secret"})).unwrap());
    config
}

fn tenant() -> TenantScope {
    TenantScope::new("cron-wecom-test", uuid::Uuid::new_v4().to_string()).unwrap()
}

async fn save(config: &Config) {
    crate::config::schema::save_config_for_tenant(
        config,
        &crate::session::ui_store::current_tenant_scope(),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn cron_wecom_delivery_configuration_and_reload_fail_closed() {
    crate::session::ui_store::with_tenant_scope(tenant(), async {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = fixture(&tmp);
        let delivery = DeliveryConfig {
            mode: "announce".into(),
            channel: Some("wecom".into()),
            to: Some("user".into()),
            best_effort: false,
        };
        // Missing stored configuration may never reuse an in-memory secret.
        assert!(delivery::validate_saved(&config, &delivery).await.is_err());
        save(&config).await;
        delivery::validate_saved(&config, &delivery).await.unwrap(); // Does not require a live socket.
        for channel in ["wecom", "wecom_webhook"] {
            let mut d = delivery.clone();
            d.channel = Some(channel.into());
            d.to = Some(if channel == "wecom" { "user" } else { "default" }.into());
            let mut broken = config.clone();
            broken.channels_config.unavailable_channels.insert(format!("{channel}:default"));
            assert!(delivery::validate(&broken, &d).is_err());
            d.to = None;
            assert!(delivery::validate(&config, &d).is_err());
            d.mode = "none".into();
            delivery::validate(&broken, &d).unwrap();
        }
        let mut saved = config.clone();
        saved.channels_config.wecom = None;
        save(&saved).await;
        assert!(
            delivery::validate_saved(&config, &delivery)
                .await
                .unwrap_err()
                .to_string()
                .contains("未配置")
        );
        // Real persistent corrupt ciphertext, independently of the launch snapshot.
        let store = crate::config::SystemConfigStore::new(
            crate::global_database::GlobalDatabase::for_home(crate::global::paths().home.clone())
                .unwrap(),
        )
        .unwrap();
        let scope = crate::session::ui_store::current_tenant_scope();
        for (channel, payload) in [
            ("wecom", json!({"channels_config":{"cli":true,"wecom":{"bot_id":"bot","secret":"enc2:broken"}}})),
            (
                "wecom_webhook",
                json!({"channels_config":{"cli":true,"wecom_webhook":{"webhook_url":"enc2:broken"}}}),
            ),
        ] {
            store.upsert(&scope, &payload.to_string()).unwrap();
            let mut d = delivery.clone();
            d.channel = Some(channel.into());
            d.to = Some(if channel == "wecom" { "user" } else { "default" }.into());
            let error = delivery::validate_saved(&config, &d).await.unwrap_err().to_string();
            assert!(error.contains("密钥不可用"), "{error}");
            assert!(!error.contains("enc2:broken"));
        }
        store.upsert(&scope, "invalid-json-fixture").unwrap();
        let error = delivery::validate_saved(&config, &delivery).await.unwrap_err().to_string();
        assert!(error.contains("读取失败"));
        assert!(!error.contains("invalid-json"));
    })
    .await;
}

#[tokio::test]
async fn cron_wecom_delivery_sends_on_authenticated_socket_and_rejects_old_credentials() {
    crate::session::ui_store::with_tenant_scope(tenant(), async {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = fixture(&tmp);
        save(&config).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let scope = WeComRuntimeScope::from_config(
            &config,
            crate::session::ui_store::current_tenant_scope(),
        )
        .unwrap();
        let channel = WeComChannel::new_for_test(
            "bot".into(),
            "secret".into(),
            format!("ws://{}", listener.local_addr().unwrap()),
        )
        .with_runtime_scope(scope.clone());
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let task = tokio::spawn(async move { channel.listen(tx).await });
        let (socket, _) =
            timeout(Duration::from_secs(2), listener.accept()).await.unwrap().unwrap();
        let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
        let frame = socket.next().await.unwrap().unwrap().into_text().unwrap();
        let frame: Value = serde_json::from_str(&frame).unwrap();
        socket
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({"headers":frame["headers"],"errcode":0}).to_string().into(),
            ))
            .await
            .unwrap();
        timeout(Duration::from_secs(2), async {
            while crate::channels::wecom::runtime::connected(&scope).is_err() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let send = async {
            scheduler::deliver_announcement(&config, "wecom", "user-123", "scheduled result").await
        };
        let server = async {
            let frame = timeout(Duration::from_secs(3), socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .into_text()
                .unwrap();
            let frame: Value = serde_json::from_str(&frame).unwrap();
            assert_eq!(frame["cmd"], "aibot_send_msg");
            assert_eq!(frame["body"]["chatid"], "user-123");
            socket
                .send(tokio_tungstenite::tungstenite::Message::Text(
                    json!({"headers":frame["headers"],"errcode":0}).to_string().into(),
                ))
                .await
                .unwrap();
        };
        let (result, _) = tokio::join!(send, server);
        result.unwrap();
        let mut updated = config.clone();
        updated.channels_config.wecom.as_mut().unwrap().secret = "changed-secret".into();
        save(&updated).await;
        let error = scheduler::deliver_announcement(&config, "wecom", "user", "result")
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("重新连接"));
        assert!(!error.contains("changed-secret"));
        assert!(timeout(Duration::from_millis(100), socket.next()).await.is_err());
        task.abort();
        let _ = task.await;
    })
    .await;
}

#[tokio::test]
async fn cron_wecom_delivery_webhook_routes_and_checks_receipt() {
    crate::session::ui_store::with_tenant_scope(tenant(), async {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = fixture(&tmp);
        save(&config).await;
        for code in [0, 40014] {
            let app = axum::Router::new().route(
                "/",
                axum::routing::post(move |axum::Json(body): axum::Json<Value>| async move {
                    assert_eq!(body["msgtype"], "markdown");
                    assert_eq!(body["markdown"]["content"], "scheduled result");
                    axum::Json(json!({"errcode":code}))
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
            let task = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            let result = delivery::TEST_WEBHOOK_ENDPOINT
                .scope(
                    endpoint,
                    scheduler::deliver_announcement(
                        &config,
                        "wecom_webhook",
                        "default",
                        "scheduled result",
                    ),
                )
                .await;
            assert_eq!(result.is_ok(), code == 0);
            task.abort();
            let _ = task.await;
        }
    })
    .await;
}
