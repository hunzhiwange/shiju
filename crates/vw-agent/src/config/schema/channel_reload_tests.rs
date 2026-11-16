//! 宿主阻塞打开时捕获请求租户与数据库上下文。
use super::*;
use vw_api_transport::id::TenantScope;
use tempfile::TempDir;

fn set_telegram(config: &mut Config, token: &str) {
    config.channels_config.telegram = Some(
        serde_json::from_value(serde_json::json!({
            "bot_token": token, "allowed_users": ["alice"]
        }))
        .unwrap(),
    );
}

#[tokio::test]
async fn channel_reload_blocking_open_uses_callers_persisted_configuration() {
    let tenant =
        TenantScope::new("channel-reload-worker", uuid::Uuid::new_v4().to_string()).unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let temp = TempDir::new().unwrap();
        let mut config = Config {
            config_path: temp.path().join("config.json"),
            workspace_dir: temp.path().join("workspace"),
            ..Config::default()
        };
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        set_telegram(&mut config, "persisted-token");
        crate::config::schema::save_config_for_tenant(&config, &tenant).await.unwrap();
        set_telegram(&mut config, "stale-launch-token");

        let mut source = ChannelConfigWatcher::new(&config, true).await.unwrap();
        let channels = source.load_for_delivery(&config).await.unwrap();
        assert_eq!(channels.telegram.unwrap().bot_token, "persisted-token");
    })
    .await;
}
