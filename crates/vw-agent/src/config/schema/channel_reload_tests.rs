//! 验证持久化设置变化、项目覆盖、租户隔离与解密失败后的恢复。
use super::super::config_save::save_config_to_store;
use super::*;
use tempfile::TempDir;

fn fixture() -> (TempDir, Config, ChannelConfigWatcher) {
    let temp = TempDir::new().unwrap();
    let store =
        SystemConfigStore::new(GlobalDatabase::new(temp.path().join("config.sqlite3")).unwrap())
            .unwrap();
    let mut config = Config::default();
    config.config_path = temp.path().join("vibewindow.json");
    config.workspace_dir = temp.path().to_path_buf();
    let source = ChannelConfigWatcher { store, project: None, last_payload: None };
    (temp, config, source)
}

fn set_telegram(config: &mut Config, token: &str) {
    config.channels_config.telegram = Some(
        serde_json::from_value(serde_json::json!({
            "bot_token": token, "allowed_users": ["alice"]
        }))
        .unwrap(),
    );
}

fn save(source: &ChannelConfigWatcher, config: &Config) {
    save_config_to_store(config, &TenantScope::default(), &source.store).unwrap();
}

#[tokio::test]
async fn channel_reload_detects_add_edit_and_disable_without_unrelated_reconnects() {
    let (_temp, mut config, mut source) = fixture();
    save(&source, &config);
    assert!(source.poll(&config).await.unwrap().is_none());
    let mut saved = config.clone();
    set_telegram(&mut saved, "first-token");
    save(&source, &saved);
    config.channels_config = source.poll(&config).await.unwrap().unwrap();
    assert_eq!(config.channels_config.telegram.as_ref().unwrap().bot_token, "first-token");
    assert!(source.poll(&config).await.unwrap().is_none());

    set_telegram(&mut saved, "second-token");
    saved.channels_config.telegram.as_mut().unwrap().allowed_users = vec!["bob".into()];
    save(&source, &saved);
    config.channels_config = source.poll(&config).await.unwrap().unwrap();
    let telegram = config.channels_config.telegram.as_ref().unwrap();
    assert_eq!(telegram.bot_token, "second-token");
    assert_eq!(telegram.allowed_users, ["bob"]);
    saved.default_temperature = 0.2;
    save(&source, &saved); // 加密密文可以变化，但有效通道配置未变。
    assert!(source.poll(&config).await.unwrap().is_none());

    saved.channels_config.telegram = None;
    save(&source, &saved);
    assert!(source.poll(&config).await.unwrap().unwrap().telegram.is_none());
}

#[tokio::test]
async fn channel_reload_ignores_other_tenants_and_respects_project_overrides() {
    let (_temp, config, mut source) = fixture();
    save(&source, &config);
    let mut saved = config.clone();
    set_telegram(&mut saved, "other-tenant-token");
    save_config_to_store(&saved, &TenantScope::new("other", "company").unwrap(), &source.store)
        .unwrap();
    assert!(source.poll(&config).await.unwrap().is_none());
    set_telegram(&mut saved, "global-token");
    save(&source, &saved);
    source.project = Some(config.workspace_dir.to_str().unwrap().to_owned());
    source.store.upsert_project(&TenantScope::default(), source.project.as_ref().unwrap(),
        r#"{"channels_config":{"telegram":{"bot_token":"project-token","allowed_users":["project-user"]}}}"#).unwrap();
    let channels = source.poll(&config).await.unwrap().unwrap();
    assert_eq!(channels.telegram.unwrap().bot_token, "project-token");
}

#[tokio::test]
async fn channel_reload_rejects_invalid_secrets_and_recovers_on_next_save() {
    let (_temp, config, mut source) = fixture();
    save(&source, &config);
    assert!(source.poll(&config).await.unwrap().is_none());
    let previous = source.last_payload.clone();
    source
        .store
        .upsert(&TenantScope::default(), r#"{"channels_config":{"cli":true,"telegram":{"bot_token":"enc2:not-hex","allowed_users":["alice"]}}}"#)
        .unwrap();
    assert!(source.poll(&config).await.is_err());
    assert_eq!(source.last_payload, previous);
    assert!(config.channels_config.telegram.is_none());
    let mut saved = config.clone();
    set_telegram(&mut saved, "recovered-token");
    save(&source, &saved);
    assert_eq!(
        source.poll(&config).await.unwrap().unwrap().telegram.unwrap().bot_token,
        "recovered-token"
    );
}
