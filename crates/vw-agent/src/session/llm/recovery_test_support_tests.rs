use super::RecoveryConfig;

const KEY: &str = "VW_MODEL_RECOVERY_TEST_KEY";

#[tokio::test]
async fn recovery_config_updates_credentials_and_restores_distinct_environment_snapshots() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _outer = RecoveryConfig::new("https://example.test/v1", Some("process-key")).await;
    crate::env::set(KEY, "startup-key");

    {
        let _fixture = RecoveryConfig::new("https://example.test/v1", Some("fixture-key")).await;
        assert_eq!(std::env::var(KEY).unwrap(), "fixture-key");
        assert_eq!(crate::env::get(KEY).as_deref(), Some("fixture-key"));
        let provider =
            crate::provider::provider::get_provider("model-recovery").await.unwrap().unwrap();
        assert_eq!(provider.key.as_deref(), Some("fixture-key"));
    }
    assert_eq!(std::env::var(KEY).unwrap(), "process-key");
    assert_eq!(crate::env::get(KEY).as_deref(), Some("startup-key"));

    {
        let _fixture = RecoveryConfig::new("https://example.test/v1", None).await;
        assert!(std::env::var_os(KEY).is_none());
        assert!(crate::env::get(KEY).is_none());
        let provider =
            crate::provider::provider::get_provider("model-recovery").await.unwrap().unwrap();
        assert!(provider.key.is_none());
    }
    assert_eq!(std::env::var(KEY).unwrap(), "process-key");
    assert_eq!(crate::env::get(KEY).as_deref(), Some("startup-key"));
}
