use super::*;
use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use iced_runtime::futures::futures::StreamExt;
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

#[tokio::test(flavor = "multi_thread")]
async fn shiju_model_failures_retry_after_ten_seconds_and_ignore_superseded_attempts() {
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    let id = repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company", "测试公司", "founder", "enabled").unwrap(),
            SecretSkey::new("login-secret").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    let generation = app.account.begin_model_sync();
    let token_retry = token_loaded(&mut app, generation, id, Err("gateway starting".into()));
    let config_retry = configured(&mut app, generation, Err("gateway starting".into()));
    let mut token_stream = iced_runtime::task::into_stream(token_retry).unwrap();
    let mut config_stream = iced_runtime::task::into_stream(config_retry).unwrap();
    let started = std::time::Instant::now();
    let (token_message, config_message) = tokio::time::timeout(Duration::from_secs(20), async {
        tokio::join!(token_stream.next(), config_stream.next())
    })
    .await
    .expect("both failures must schedule a retry");
    assert!(started.elapsed() >= Duration::from_secs(10));

    // 新一轮同步后，两个失败阶段的延迟消息都不能重新配置旧账号。
    app.account.begin_model_sync();
    for action in [token_message, config_message] {
        let Some(iced_runtime::Action::Output(message)) = action else {
            panic!("expected a retry message");
        };
        assert!(matches!(
            message,
            Message::Account(AccountMessage::ResetModels { generation: value }) if value == generation
        ));
        let Message::Account(message) = message else { unreachable!() };
        assert_eq!(crate::app::message::account::update(&mut app, message).units(), 0);
    }
    assert_eq!(configured(&mut app, generation, Err("gateway starting".into())).units(), 0);
}
