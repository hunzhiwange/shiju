//! 验证连接器账号隔离、取消、必填变量与密码处理。
use super::connectors::*;
use crate::app::{
    App,
    account::{AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount},
    skill_hub::{
        LoadState,
        connectors::{ConnectorForm, ConnectorWork},
    },
};
use serde_json::json;
fn app() -> App {
    let (mut app, _) = App::new();
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new("company-1", "示例公司", "founder", "active").unwrap(),
            SecretSkey::new("credential").unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    app.account.replace_repository(repository);
    app
}
fn connector() -> vw_gateway_client::ShijuConnector {
    serde_json::from_value(json!({"platform_id": 1, "company_platform_id": 7, "platform_code": "sample", "platform_name": "中文连接器", "authorization_available": true, "account_binding_variables": [
        {"name": "token", "type": "password", "description": "密钥", "required": false},
        {"name": "region", "type": "string", "description": "区域", "required": true}],
        "binding": {"third_party_account_id": "账号", "third_party_account_name": "中文名称", "binding_config": {"token": "secret", "region": "华东"}, "status": "enabled"}})).unwrap()
}
#[test]
fn connector_form_redacts_password_and_preserves_optional_blank_secret() {
    let mut form = ConnectorForm::new(connector());
    assert_eq!(form.values["token"], "");
    let request = form.request().unwrap();
    assert!(!request.binding_config.contains_key("token"));
    assert_eq!(request.binding_config["region"], "华东");
    form.values.insert("region".into(), String::new());
    assert!(form.request().unwrap_err().contains("区域"));
    assert!(!format!("{form:?}").contains("华东"));
    let mut connector = connector();
    connector
        .binding
        .as_mut()
        .unwrap()
        .binding_config
        .insert("region".into(), serde_json::Value::Null);
    assert_eq!(ConnectorForm::new(connector).values["region"], "");
}

fn ready_app() -> App {
    let mut app = app();
    let _ = load(&mut app);
    let generation = app.expert_hub.connectors.list_generation;
    let account_id = owner(&app);
    let mut second = connector();
    second.company_platform_id = 8;
    let mut third = connector();
    third.company_platform_id = 9;
    let _ = update(
        &mut app,
        ConnectorMessage::Loaded(generation, account_id, Ok(vec![connector(), second, third])),
    );
    app
}
fn owner(app: &App) -> String {
    app.account.active_account().unwrap().id().to_string()
}
fn authorization(app: &mut App, id: u64) -> u64 {
    app.expert_hub.connectors.begin(
        id,
        ConnectorWork::Authorizing {
            challenge: vw_gateway_client::ShijuConnectorChallenge {
                flow_id: id.to_string().repeat(32),
                authorization_url: "https://provider.example/authorize".into(),
                poll_interval_seconds: 1,
                expires_in_seconds: 60,
            },
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
        },
    )
}
#[test]
fn connector_loading_rejects_wrong_account_and_stale_generation() {
    let mut app = app();
    let account_id = owner(&app);
    let _ = load(&mut app);
    let generation = app.expert_hub.connectors.list_generation;
    for (generation, account) in [(generation - 1, account_id.as_str()), (generation, "other")] {
        let _ = update(
            &mut app,
            ConnectorMessage::Loaded(generation, account.into(), Ok(vec![connector()])),
        );
        assert!(matches!(app.expert_hub.connectors.list, LoadState::Loading { .. }));
    }
    let _ = update(
        &mut app,
        ConnectorMessage::Loaded(generation, account_id.clone(), Ok(vec![connector()])),
    );
    assert!(matches!(app.expert_hub.connectors.list, LoadState::Loaded(_)));
    app.expert_hub.connectors.clear(Some("other".into()));
    let _ =
        update(&mut app, ConnectorMessage::Loaded(generation, account_id, Ok(vec![connector()])));
    assert!(matches!(app.expert_hub.connectors.list, LoadState::Idle));
}
#[test]
fn connector_authorizations_start_and_cancel_independently() {
    let mut app = ready_app();
    let account_id = owner(&app);
    let _ = update(&mut app, ConnectorMessage::Authorize(7));
    let first = app.expert_hub.connectors.work[&7].generation;
    let _ = update(&mut app, ConnectorMessage::Authorize(8));
    let second = app.expert_hub.connectors.work[&8].generation;
    assert_ne!(first, second);
    assert_eq!(app.expert_hub.connectors.work.len(), 2);
    let _ = update(&mut app, ConnectorMessage::CancelAuthorization(7));
    assert!(!app.expert_hub.connectors.work.contains_key(&7));
    assert_eq!(app.expert_hub.connectors.work[&8].generation, second);
    let _ = update(&mut app, ConnectorMessage::Authorize(7));
    let replacement = app.expert_hub.connectors.work[&7].generation;
    let _ = update(
        &mut app,
        ConnectorMessage::Started(first, account_id.clone(), 7, Err("old failure".into())),
    );
    assert_eq!(app.expert_hub.connectors.work[&7].generation, replacement);
    let _ =
        update(&mut app, ConnectorMessage::Started(second, account_id, 8, Err("failure".into())));
    assert!(!app.expert_hub.connectors.work.contains_key(&8));
    assert!(app.expert_hub.connectors.work.contains_key(&7));
}
#[test]
fn connector_approval_refresh_preserves_other_authorization_and_open_form() {
    use vw_gateway_client::{ShijuConnectorAuthorizationStatus as Status, ShijuConnectorPoll};
    let mut app = ready_app();
    let account_id = owner(&app);
    let first = authorization(&mut app, 7);
    let second = authorization(&mut app, 8);
    let mut third = connector();
    third.company_platform_id = 9;
    app.expert_hub.connectors.form = Some(ConnectorForm::new(third));
    let _ = update(
        &mut app,
        ConnectorMessage::Polled(
            first,
            account_id.clone(),
            7,
            Ok(ShijuConnectorPoll { status: Status::Approved }),
        ),
    );
    assert!(!app.expert_hub.connectors.work.contains_key(&7));
    assert_eq!(app.expert_hub.connectors.work[&8].generation, second);
    assert!(app.expert_hub.connectors.form.is_some());
    assert!(app.expert_hub.connectors.list_loading);
    assert!(matches!(app.expert_hub.connectors.list, LoadState::Loaded(_)));
    let _ = update(
        &mut app,
        ConnectorMessage::Polled(
            second,
            account_id,
            8,
            Ok(ShijuConnectorPoll { status: Status::Pending }),
        ),
    );
    assert!(app.expert_hub.connectors.work.contains_key(&8));
}
#[test]
fn connector_modal_close_does_not_cancel_another_authorization() {
    let mut app = ready_app();
    let generation = authorization(&mut app, 7);
    let _ = update(&mut app, ConnectorMessage::Edit(8));
    assert_eq!(app.expert_hub.connectors.form.as_ref().unwrap().connector.company_platform_id, 8);
    let _ = update(&mut app, ConnectorMessage::Cancel);
    assert!(app.expert_hub.connectors.form.is_none());
    assert_eq!(app.expert_hub.connectors.work[&7].generation, generation);
}
#[test]
fn connector_expired_or_denied_flow_only_removes_its_own_wait() {
    use vw_gateway_client::{ShijuConnectorAuthorizationStatus as Status, ShijuConnectorPoll};
    for status in [Status::Denied, Status::Expired] {
        let mut app = ready_app();
        let account_id = owner(&app);
        let first = authorization(&mut app, 7);
        let second = authorization(&mut app, 8);
        let _ = update(
            &mut app,
            ConnectorMessage::Polled(first, account_id, 7, Ok(ShijuConnectorPoll { status })),
        );
        assert!(!app.expert_hub.connectors.work.contains_key(&7));
        assert_eq!(app.expert_hub.connectors.work[&8].generation, second);
    }
    let mut app = ready_app();
    let account_id = owner(&app);
    let first = authorization(&mut app, 7);
    let second = authorization(&mut app, 8);
    if let ConnectorWork::Authorizing { deadline, .. } =
        &mut app.expert_hub.connectors.work.get_mut(&7).unwrap().work
    {
        *deadline = std::time::Instant::now();
    }
    let _ = update(&mut app, ConnectorMessage::Poll(first, account_id, 7));
    assert!(!app.expert_hub.connectors.work.contains_key(&7));
    assert_eq!(app.expert_hub.connectors.work[&8].generation, second);
}
#[test]
fn connector_save_failure_preserves_modal_and_other_waits() {
    let mut app = ready_app();
    let account_id = owner(&app);
    let other = authorization(&mut app, 7);
    let _ = update(&mut app, ConnectorMessage::Edit(8));
    let _ = update(&mut app, ConnectorMessage::Save);
    let saving = app.expert_hub.connectors.work[&8].generation;
    assert!(app.expert_hub.connectors.form_is_saving());
    let _ = update(&mut app, ConnectorMessage::Cancel);
    assert!(app.expert_hub.connectors.form.is_some());
    let _ = update(&mut app, ConnectorMessage::Refresh);
    assert_eq!(app.expert_hub.connectors.work[&8].generation, saving);
    let _ = update(
        &mut app,
        ConnectorMessage::Saved(saving, account_id, 8, Err("connection failed".into())),
    );
    assert!(!app.expert_hub.connectors.form_is_saving());
    assert!(app.expert_hub.connectors.form.is_some());
    assert_eq!(app.expert_hub.connectors.work[&7].generation, other);
}
#[test]
fn connector_logout_clears_all_waits_and_ignores_late_results() {
    use vw_gateway_client::{ShijuConnectorAuthorizationStatus as Status, ShijuConnectorPoll};
    let mut app = ready_app();
    let account_id = owner(&app);
    let first = authorization(&mut app, 7);
    let _ = authorization(&mut app, 8);
    app.expert_hub.connectors.form = Some(ConnectorForm::new(connector()));
    app.expert_hub.clear_for_account(None);
    let _ = update(
        &mut app,
        ConnectorMessage::Polled(
            first,
            account_id,
            7,
            Ok(ShijuConnectorPoll { status: Status::Approved }),
        ),
    );
    assert!(app.expert_hub.connectors.work.is_empty());
    assert!(app.expert_hub.connectors.form.is_none());
    assert!(matches!(app.expert_hub.connectors.list, LoadState::NeedsLogin));
}
#[test]
fn connector_save_and_authorize_are_disabled_without_login() {
    let (mut app, _) = App::new();
    let _ = load(&mut app);
    let _ = update(&mut app, ConnectorMessage::Authorize(7));
    assert!(matches!(app.expert_hub.connectors.list, LoadState::NeedsLogin));
    assert!(app.expert_hub.connectors.work.is_empty());
}
