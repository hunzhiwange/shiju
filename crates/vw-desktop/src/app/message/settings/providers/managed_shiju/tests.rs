use super::*;
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use vw_gateway_client::ShijuCredential;

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
    auth::store::set_to(
        &path,
        "other",
        &auth::Info::Api(auth::ApiInfo { key: "other-secret".to_string() }),
    )
    .unwrap();
    save_token(&path, &token()).unwrap();
    assert!(
        matches!(auth::store::get_from(&path, "shiju"), Some(auth::Info::Api(info)) if info.key == "issued-model-secret")
    );
    assert!(
        matches!(auth::store::get_from(&path, "other"), Some(auth::Info::Api(info)) if info.key == "other-secret")
    );
    auth::store::remove_from(&path, "shiju").unwrap();
    assert!(auth::store::get_from(&path, "shiju").is_none());
    assert!(auth::store::get_from(&path, "other").is_some());
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

#[test]
fn shiju_model_token_results_after_account_switch_or_logout_are_ignored() {
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    let id = repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company", "测试公司", "founder", "enabled").unwrap(),
            SecretSkey::new("login-secret").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository.clone());
    let stale = app.account.begin_model_sync();
    let current = app.account.begin_model_sync();
    assert_eq!(token_loaded(&mut app, stale, id.clone(), Ok(token())).units(), 0);
    assert_eq!(
        token_loaded(&mut app, current, "other-account".to_string(), Ok(token())).units(),
        0
    );
    repository.sign_out();
    app.account.replace_repository(repository);
    assert_eq!(token_loaded(&mut app, current, id, Ok(token())).units(), 0);
    assert_eq!(reset(&mut app, stale).units(), 0);
}

#[test]
fn shiju_model_connect_without_login_never_opens_manual_key_dialog() {
    let (mut app, _) = App::new();
    let _ = crate::app::message::settings::providers::update(
        &mut app,
        crate::app::message::SettingsMessage::ProviderConnectOpen("shiju".to_string()),
    );
    assert!(app.provider_settings.connect_modal.is_none());
    assert!(super::super::models::current_provider_api_key("shiju").is_none());
}
