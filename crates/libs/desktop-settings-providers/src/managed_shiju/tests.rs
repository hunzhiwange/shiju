use super::*;
use gateway_shiju_account::ShijuCredential;
fn token() -> ShijuModelTokenDto {
    ShijuModelTokenDto {
        key: ShijuCredential::new("issued-model-secret").unwrap(),
        endpoint_url: "https://router.shiju.cloud/v1".to_string(),
    }
}

#[test]
fn shiju_model_token_storage_preserves_other_provider_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    auth::set_to(
        &path,
        "other",
        &auth_types::Info::Api(auth_types::ApiInfo { key: "other-secret".to_string() }),
    )
    .unwrap();
    save_token(&path, &token()).unwrap();
    assert!(
        matches!(auth::get_from(&path, "shiju"), Some(auth_types::Info::Api(info)) if info.key == "issued-model-secret")
    );
    assert!(
        matches!(auth::get_from(&path, "other"), Some(auth_types::Info::Api(info)) if info.key == "other-secret")
    );
    auth::remove_from(&path, "shiju").unwrap();
    assert!(auth::get_from(&path, "shiju").is_none());
    assert!(auth::get_from(&path, "other").is_some());
}

#[test]
fn shiju_model_token_storage_rejects_untrusted_endpoint_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    let mut value = token();
    value.endpoint_url = "https://other.example/v1".to_string();
    assert!(save_token(&path, &value).is_err());
    assert!(!path.exists());
}
