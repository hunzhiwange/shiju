use super::{GatewayClient, test_support};
use crate::{AppsUsageDisplay, AppsUsageReadError, GatewayAuth};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthErrorCode,
    apps_usage::{UsageQueryData, UsageQueryKind, UsageQueryOutcome, UsageSnapshotQuery},
    id::TenantScope,
};

fn selection() -> UsageSnapshotQuery {
    UsageSnapshotQuery {
        project_id: "project-a".into(),
        target_host_id: "mac-host".into(),
        app_id: AppId::Codex,
        provider_id: "provider-a".into(),
        account_id: Some("account-a".into()),
        kind: None,
    }
}

#[tokio::test]
async fn usage_usage_read_api_returns_expected_result() {
    let server = test_support::server_with_tenant_scope(
        vec![
            (
                200,
                json!({
                    "capabilities": ["official_subscription"],
                    "snapshot": {
                        "result": {
                            "source": {"kind": "official", "id": "codex"},
                            "queriedAt": 123,
                            "credentialState": "valid",
                            "outcome": {"status": "success", "data": {
                                "kind": "official_subscription",
                                "value": {"subscriptionType": "Pro", "windows": []}
                            }}
                        },
                        "stale": false
                    }
                }),
            ),
            (
                200,
                json!([
                    {"template": "general", "source": "{{apiKey}}"},
                    {"template": "official_subscription", "source": null}
                ]),
            ),
            (
                200,
                json!({
                    "template": "general",
                    "enabled": true,
                    "refreshIntervalMinutes": 60,
                    "display": "used",
                    "credentials": {"apiKeyRef": "33333333-3333-4333-8333-333333333333"}
                }),
            ),
            (200, Value::Null),
        ],
        TenantScope::new("platform-a", "company-a").unwrap(),
    );
    let client = GatewayClient::new(
        server
            .client()
            .endpoint()
            .clone()
            .with_auth(GatewayAuth { skey: Some("gateway-token".into()) }),
    )
    .unwrap();
    let mut selection = selection();
    selection.kind = Some(UsageQueryKind::OfficialSubscription);
    let token = "fixture-apps-principal-token-with-valid-length";
    let response = client.apps_usage_snapshot(&selection, token).await.unwrap();
    assert_eq!(response.capabilities, [UsageQueryKind::OfficialSubscription]);
    let snapshot = response.snapshot.unwrap();
    assert!(!snapshot.stale);
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(plan) } =
        snapshot.result.outcome
    else {
        panic!("expected an official subscription");
    };
    assert_eq!(plan.subscription_type.as_deref(), Some("Pro"));

    selection.kind = None;
    let templates = client.apps_usage_templates(&selection, token).await.unwrap();
    assert_eq!(templates[0].template, "general");
    assert_eq!(templates[0].source.as_deref(), Some("{{apiKey}}"));
    assert_eq!(templates[1].template, "official_subscription");
    assert_eq!(templates[1].source, None);
    let settings = client.apps_usage_settings(&selection, token).await.unwrap().unwrap();
    assert_eq!(settings.template, "general");
    assert!(settings.enabled);
    assert_eq!(settings.refresh_interval_minutes, 60);
    assert_eq!(settings.display, AppsUsageDisplay::Used);
    assert_eq!(
        settings.credentials.api_key_ref.as_deref(),
        Some("33333333-3333-4333-8333-333333333333")
    );
    selection.account_id = None;
    assert!(client.apps_usage_settings(&selection, token).await.unwrap().is_none());

    let scope = "projectId=project-a&targetHostId=mac-host&appId=codex&providerId=provider-a";
    for (resource, suffix) in [
        ("snapshot", "&accountId=account-a&kind=official_subscription"),
        ("templates", "&accountId=account-a"),
        ("settings", "&accountId=account-a"),
        ("settings", ""),
    ] {
        let request = server.take_request();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, format!("/v1/apps/usage/{resource}?{scope}{suffix}"));
        assert_eq!(request.header("authorization"), Some("Bearer gateway-token"));
        assert_eq!(request.header("x-agent-apps-token"), Some(token));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-a"));
        assert_eq!(request.body, Value::Null);
    }
    server.join();
}

#[tokio::test]
async fn usage_usage_read_api_rejects_invalid_input() {
    let server = test_support::server_raw(vec![
        (
            400,
            json!({
                "code": "invalid_input",
                "message": "Invalid usage selector",
                "correlation_id": "usage-trace"
            })
            .to_string(),
        ),
        (
            403,
            json!({
                "code": "forbidden",
                "message": "Usage action forbidden",
                "correlation_id": "access-trace"
            })
            .to_string(),
        ),
        (401, "sensitive proxy response".into()),
        (200, r#"{"unexpected":"sensitive success response"}"#.into()),
    ]);
    let client = server.client();
    let mut selection = selection();
    let token = "fixture-apps-principal-token-with-valid-length";
    assert_eq!(
        client.apps_usage_snapshot(&selection, "bad\ncredential").await.unwrap_err(),
        AppsUsageReadError::InvalidPrincipalToken
    );
    selection.provider_id = "provider &accountId=other".into();
    let error = client.apps_usage_snapshot(&selection, token).await.unwrap_err();
    let AppsUsageReadError::Http { status: 400, error: Some(error) } = error else {
        panic!("expected a structured invalid input error: {error:?}");
    };
    assert_eq!(error.code, AuthErrorCode::InvalidInput);
    assert_eq!(error.correlation_id, "usage-trace");
    assert_eq!(
        server.take_request().path,
        "/v1/apps/usage/snapshot?projectId=project-a&targetHostId=mac-host&appId=codex&providerId=provider+%26accountId%3Dother&accountId=account-a"
    );

    let error = client.apps_usage_settings(&selection, token).await.unwrap_err();
    assert!(matches!(
        error,
        AppsUsageReadError::Http { status: 403, error: Some(error) }
            if error.code == AuthErrorCode::Forbidden && error.correlation_id == "access-trace"
    ));
    assert_eq!(
        client.apps_usage_templates(&selection, token).await.unwrap_err(),
        AppsUsageReadError::Http { status: 401, error: None }
    );
    assert_eq!(
        client.apps_usage_snapshot(&selection, token).await.unwrap_err(),
        AppsUsageReadError::Decode
    );
    server.join();
}
