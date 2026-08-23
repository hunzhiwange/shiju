//! 验证识矩账户登录、轮询、资料和密码更新的网关契约。
//!
//! 安全断言确保凭据与密码不会出现在调试输出、请求日志或错误文本中。

use std::sync::{Arc, Mutex};

use serde_json::json;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{Interest, Subscriber};
use tracing::{Event, Metadata};

use super::{
    SHIJU_CREDENTIAL_HEADER, ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest,
    ShijuAccountProfileDto, ShijuAccountUpdateRequest, ShijuAccountUpdateResponseDto,
    ShijuCredential, ShijuLoginChallengeDto, ShijuLoginPollDto,
};
use crate::client::test_support;

#[derive(Clone, Default)]
struct CapturedEvents(Arc<Mutex<Vec<String>>>);

impl CapturedEvents {
    fn joined(&self) -> String {
        self.0.lock().expect("captured events").join("\n")
    }
}

impl Subscriber for CapturedEvents {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        self.0.lock().expect("captured events").push(visitor.fields.join(" "));
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}

    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn clone_span(&self, id: &Id) -> Id {
        id.clone()
    }

    fn try_close(&self, _id: Id) -> bool {
        true
    }
}

#[derive(Default)]
struct EventVisitor {
    fields: Vec<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn shiju_account_client_uses_gateway_paths_without_exposing_secrets() {
    let state = "state-secret-value";
    let credential = ShijuCredential::new("credential-secret-value").expect("credential");
    let server = test_support::server(vec![
        (
            200,
            json!({
                "state": state,
                "authorization_url":
                    "https://auth.shiju.cloud/plugin/login?state=state-secret-value",
                "poll_interval_seconds": 2
            }),
        ),
        (200, json!({"status": "pending"})),
        (
            200,
            json!({
                "company_name": "示例公司",
                "company_status": "enabled",
                "name": "测试用户",
                "account_name": "tester",
                "user_status": "enabled",
                "departments": ["AI 创新组"],
                "roles": ["管理员"]
            }),
        ),
        (
            400,
            json!({
                "message": state,
                "credential": credential.expose()
            }),
        ),
    ]);
    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());

    server.client().shiju_account_login().await.expect("login");
    server.client().shiju_account_login_poll(state).await.expect("poll");
    server.client().shiju_account_profile(&credential).await.expect("profile");
    let error =
        server.client().shiju_account_login_poll(state).await.expect_err("non-success response");
    assert!(!error.contains(state));
    assert!(!error.contains(credential.expose()));

    let login = server.take_request();
    assert_eq!(login.method, "POST");
    assert_eq!(login.path, "/v1/shiju/account/login");
    assert_eq!(login.body, json!({}));

    let poll = server.take_request();
    assert_eq!(poll.method, "POST");
    assert_eq!(poll.path, "/v1/shiju/account/login/poll");
    assert_eq!(poll.body, json!({"state": state}));
    assert!(!poll.path.contains(state));

    let profile = server.take_request();
    assert_eq!(profile.method, "GET");
    assert_eq!(profile.path, "/v1/shiju/account/profile");
    assert_eq!(profile.body, serde_json::Value::Null);
    assert_eq!(profile.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    assert!(!profile.path.contains(credential.expose()));

    let failed_poll = server.take_request();
    assert_eq!(failed_poll.path, "/v1/shiju/account/login/poll");
    let logs = captured.joined();
    assert!(!logs.contains(state));
    assert!(!logs.contains(credential.expose()));
    server.join();
}

#[tokio::test]
async fn shiju_account_contract_decodes_login_poll_and_profile() {
    let server = test_support::server(vec![
        (
            200,
            json!({
                "state": "state-1",
                "authorization_url": "https://auth.shiju.cloud/plugin/login?state=state-1",
                "poll_interval_seconds": 3
            }),
        ),
        (200, json!({"status": "approved", "credential": "approved-secret"})),
        (
            200,
            json!({
                "company_name": "示例公司",
                "company_status": "enabled",
                "name": "刘祥敏",
                "account_name": "liuxiangmin",
                "user_status": "enabled",
                "departments": ["AI创新组"],
                "roles": ["默认角色", "数据管理员"]
            }),
        ),
    ]);

    let challenge: ShijuLoginChallengeDto =
        server.client().shiju_account_login().await.expect("challenge");
    assert_eq!(challenge.state(), "state-1");
    assert_eq!(
        challenge.authorization_url(),
        "https://auth.shiju.cloud/plugin/login?state=state-1"
    );
    assert_eq!(challenge.poll_interval_seconds, 3);

    let poll: ShijuLoginPollDto =
        server.client().shiju_account_login_poll(challenge.state()).await.expect("approved");
    let ShijuLoginPollDto::Approved { credential } = poll else {
        panic!("expected approved poll");
    };
    assert_eq!(credential.expose(), "approved-secret");

    let profile: ShijuAccountProfileDto =
        server.client().shiju_account_profile(&credential).await.expect("profile");
    assert_eq!(profile.company_name, "示例公司");
    assert_eq!(profile.company_status, "enabled");
    assert_eq!(profile.name, "刘祥敏");
    assert_eq!(profile.account_name, "liuxiangmin");
    assert_eq!(profile.user_status, "enabled");
    assert_eq!(profile.departments, ["AI创新组"]);
    assert_eq!(profile.roles, ["默认角色", "数据管理员"]);
    server.join();
}

#[tokio::test]
async fn shiju_account_me_contract_gets_and_updates_with_sensitive_header() {
    let credential = ShijuCredential::new("account-me-secret").expect("credential");
    let server = test_support::server(vec![
        (
            200,
            json!({
                "user_id": "101026",
                "company_id": "10011",
                "account_id": "101026",
                "account_name": "liuxiangmin",
                "name": "刘祥敏2",
                "source_type": "manual",
                "external_user_id": "",
                "allow_console_login": true,
                "status": "enabled",
                "departments": [{
                    "department_id": "101014",
                    "department_name": "AI创新组",
                    "parent_department_id": "101005"
                }],
                "roles": [{
                    "role_id": "102",
                    "role_name": "默认角色",
                    "role_code": "default"
                }]
            }),
        ),
        (
            200,
            json!({
                "user_id": "101026",
                "company_id": "10011",
                "name": "刘祥敏3",
                "status": "enabled"
            }),
        ),
    ]);

    let profile: ShijuAccountMeDto =
        server.client().shiju_account_me(&credential).await.expect("profile");
    assert_eq!(profile.account_name, "liuxiangmin");
    assert_eq!(profile.departments[0].department_name, "AI创新组");
    assert_eq!(profile.roles[0].role_name, "默认角色");

    let request = ShijuAccountUpdateRequest {
        name: "刘祥敏3".to_string(),
        username: "liuxiangmin".to_string(),
    };
    let updated: ShijuAccountUpdateResponseDto =
        server.client().shiju_account_update(&credential, &request).await.expect("update");
    assert_eq!(updated.name, "刘祥敏3");
    assert_eq!(updated.status, "enabled");

    let get = server.take_request();
    assert_eq!(get.method, "GET");
    assert_eq!(get.path, "/v1/shiju/account/me");
    assert_eq!(get.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    assert_eq!(get.body, serde_json::Value::Null);

    let put = server.take_request();
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, "/v1/shiju/account/me");
    assert_eq!(put.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    assert_eq!(put.body, json!({"name": "刘祥敏3", "username": "liuxiangmin"}));
    server.join();
}

#[tokio::test]
async fn shiju_account_password_contract_uses_sensitive_header_and_redacts_secrets() {
    let credential = ShijuCredential::new("password-route-secret").expect("credential");
    let request = ShijuAccountPasswordUpdateRequest {
        old_password: "old-password-123".to_string(),
        new_password: "new-password-456".to_string(),
    };
    let request_debug = format!("{request:?}");
    assert!(!request_debug.contains(&request.old_password));
    assert!(!request_debug.contains(&request.new_password));
    assert!(request_debug.contains("<redacted>"));

    let server = test_support::server(vec![(200, json!({}))]);
    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());
    server
        .client()
        .shiju_account_password_update(&credential, &request)
        .await
        .expect("password update");

    let put = server.take_request();
    assert_eq!(put.method, "PUT");
    assert_eq!(put.path, "/v1/shiju/account/me/password");
    assert_eq!(put.header(SHIJU_CREDENTIAL_HEADER), Some(credential.expose()));
    assert_eq!(
        put.body,
        json!({
            "old_password": "old-password-123",
            "new_password": "new-password-456"
        })
    );
    let events = captured.joined();
    assert!(!events.contains("old-password-123"));
    assert!(!events.contains("new-password-456"));
    server.join();
}

#[tokio::test]
async fn shiju_account_password_returns_gateway_error_message() {
    let message = "用户名或密码错误，请重新输入";
    let server = test_support::server(vec![(401, json!({"error": message}))]);
    let credential = ShijuCredential::new("password-route-secret").expect("credential");
    let request = ShijuAccountPasswordUpdateRequest {
        old_password: "wrong-password-123".to_string(),
        new_password: "new-password-456".to_string(),
    };

    let error = server
        .client()
        .shiju_account_password_update(&credential, &request)
        .await
        .expect_err("password update");

    assert_eq!(error, message);
    let put = server.take_request();
    assert_eq!(put.path, "/v1/shiju/account/me/password");
    server.join();
}

#[test]
fn shiju_account_sensitive_types_redact_debug() {
    assert!(ShijuCredential::new("   ").is_err());

    let credential = ShijuCredential::new("credential-debug-secret").expect("credential");
    let credential_debug = format!("{credential:?}");
    assert!(!credential_debug.contains(credential.expose()));
    assert!(credential_debug.contains("<redacted>"));

    let challenge: ShijuLoginChallengeDto = serde_json::from_value(json!({
        "state": "state-debug-secret",
        "authorization_url":
            "https://auth.shiju.cloud/plugin/login?state=state-debug-secret",
        "poll_interval_seconds": 2
    }))
    .expect("challenge");
    let challenge_debug = format!("{challenge:?}");
    assert!(!challenge_debug.contains(challenge.state()));
    assert!(!challenge_debug.contains(challenge.authorization_url()));
    assert!(challenge_debug.contains("<redacted>"));

    let poll = ShijuLoginPollDto::Approved { credential };
    let poll_debug = format!("{poll:?}");
    assert!(!poll_debug.contains("credential-debug-secret"));
    assert!(poll_debug.contains("<redacted>"));
}
