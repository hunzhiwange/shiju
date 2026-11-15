use super::*;
use crate::*;
use crate::test_support::*;
use serde_json::{Value, json};

#[test]
fn read_config_fingerprint_reports_missing_and_existing_file() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime should build");
    runtime.block_on(async {
        let env = ProviderTestEnv::new().await;
        let missing = read_config_fingerprint();
        assert_eq!(missing.path, env.config_path().to_string_lossy());
        assert!(!missing.exists);

        env.write_config(config_with_default(Value::Null, json!({}))).await;
        let existing = read_config_fingerprint();
        assert!(existing.exists);
        assert!(existing.len > 0);
        assert!(existing.modified_nanos > 0);
    });
}

#[tokio::test]
async fn config_fingerprint_change_invalidates_cached_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_model("zz-test-provider", "alpha").await.is_ok());

    std::thread::sleep(std::time::Duration::from_millis(5));
    write_json(
        &env.config_path(),
        &config_with_default(
            Value::Null,
            json!({
                "zz-test-provider": {
                    "models": {
                        "beta": {"status": "active"}
                    }
                }
            }),
        ),
    );

    assert!(get_model("zz-test-provider", "alpha").await.is_err());
    assert!(get_model("zz-test-provider", "beta").await.is_ok());
}

#[tokio::test]
async fn config_fingerprint_deletion_invalidates_cached_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_model("zz-test-provider", "alpha").await.is_ok());

    std::fs::remove_file(env.config_path()).expect("config file should be removed");

    assert!(get_model("zz-test-provider", "alpha").await.is_err());
    let fingerprint = read_config_fingerprint();
    assert_eq!(fingerprint.path, env.config_path().to_string_lossy());
    assert!(!fingerprint.exists);
    assert_eq!(fingerprint.len, 0);
    assert_eq!(fingerprint.modified_nanos, 0);
}
