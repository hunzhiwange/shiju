use super::{ApiInfo, AuthService, Info};

#[test]
fn auth_service_new_preserves_state_dir_and_encryption_flag() {
    let dir = tempfile::tempdir().expect("temp dir should exist");
    let service = AuthService::new(dir.path(), true);

    assert_eq!(service.state_dir.as_path(), dir.path());
    assert!(service.secrets_encrypt);
}

#[tokio::test]
async fn auth_service_token_getters_return_none_until_oauth_is_configured() {
    let dir = tempfile::tempdir().expect("temp dir should exist");
    let service = AuthService::new(dir.path(), false);

    assert_eq!(service.get_valid_gemini_access_token(None).await.unwrap(), None);
    assert_eq!(service.get_valid_openai_access_token(None).await.unwrap(), None);
}

#[test]
fn auth_store_facade_sets_gets_lists_and_removes_unique_provider() {
    let provider = format!("vw-agent-auth-test-{}", std::process::id());
    let previous = super::get(&provider);
    let info = Info::Api(ApiInfo { key: "test-key".to_string() });

    let result = (|| {
        super::set(&provider, &info)?;
        match super::get(&provider) {
            Some(Info::Api(api)) => assert_eq!(api.key, "test-key"),
            other => panic!("expected api auth info, got {other:?}"),
        }
        assert!(super::all().contains_key(&provider));
        super::remove(&provider)?;
        assert!(super::get(&provider).is_none());
        Ok::<(), std::io::Error>(())
    })();

    if let Some(previous) = previous {
        let _ = super::set(&provider, &previous);
    }

    result.expect("auth store facade should read and write through global path");
}

#[test]
fn auth_service_from_config_uses_empty_state_and_plaintext_secrets() {
    let config = crate::app::agent::config::Config::default();
    let service = AuthService::from_config(&config);

    assert!(service.state_dir.as_os_str().is_empty());
    assert!(!service.secrets_encrypt);
}

#[tokio::test]
async fn auth_service_profile_getters_return_none_until_oauth_is_configured() {
    let dir = tempfile::tempdir().expect("temp dir should exist");
    let service = AuthService::new(dir.path(), false);

    assert_eq!(service.get_gemini_profile(None).await.unwrap(), None);
    assert!(service.get_profile("openai", Some("default")).await.unwrap().is_none());
}
