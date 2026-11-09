use super::{GatewayClient, test_support};
use crate::{
    AppsUsageCredentialRefs, AppsUsageDisplay, AppsUsageScriptTrial, AppsUsageSettings,
    AppsUsageWriteError, GatewayAuth,
};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::{AppId, AppOperationPhase},
    agent_apps_auth::AuthErrorCode,
    apps_usage::{UsageQueryErrorCode, UsageQueryKind, UsageSnapshotQuery},
    id::{TaskId, TenantScope},
    task::TaskStatus,
};

const TOKEN: &str = "fixture-apps-principal-token";
const KEY_REF: &str = "33333333-3333-4333-8333-333333333333";
const SOURCE: &str = "({ extractor: () => ({remaining: 0, extra: 'draft & ?'}) })";

fn selection() -> UsageSnapshotQuery {
    UsageSnapshotQuery {
        project_id: "project-a".into(),
        target_host_id: "host +&?/#%主机".into(),
        app_id: AppId::Codex,
        provider_id: "provider &accountId=other".into(),
        account_id: Some("account-a".into()),
        kind: None,
    }
}

fn settings(interval: u16) -> AppsUsageSettings {
    AppsUsageSettings {
        template: "general".into(),
        enabled: true,
        refresh_interval_minutes: interval,
        display: AppsUsageDisplay::Used,
        credentials: AppsUsageCredentialRefs {
            api_key_ref: Some(KEY_REF.into()),
            ..Default::default()
        },
    }
}

fn trial() -> AppsUsageScriptTrial {
    AppsUsageScriptTrial {
        selection: selection(),
        template: "custom".into(),
        source: Some(SOURCE.into()),
        timeout_seconds: Some(12),
        credentials: AppsUsageCredentialRefs::default(),
    }
}

fn progress(id: &str, status: &str, phase: &str, host: &str) -> Value {
    json!({
        "operationId": id,
        "status": status,
        "phase": phase,
        "projectId": "project-a",
        "targetHostId": host,
        "correlationId": "trace-a",
        "cancelRequested": false,
        "items": [{
            "appId": "codex",
            "providerId": "provider &accountId=other",
            "accountId": "account-a",
            "kind": "official_subscription",
            "status": "completed",
            "snapshotVersion": "version-a",
            "errorCode": null
        }]
    })
}

#[tokio::test]
async fn usage_usage_write_api_returns_expected_result() {
    let mut selection = selection();
    let host = selection.target_host_id.clone();
    let single_id = "single/id?x=1#%";
    let batch_id = "batch-id";
    let server = test_support::server_with_tenant_scope(
        vec![
            (202, json!({"operationId": single_id, "status": "queued"})),
            (202, json!({"operationId": batch_id, "status": "queued"})),
            (200, progress(single_id, "running", "executing", &host)),
            (200, progress(single_id, "completed", "terminal", &host)),
            (200, progress(batch_id, "cancelled", "terminal", &host)),
            (204, Value::Null),
            (200, json!([{"remaining": 0.0, "planName": "Trial"}])),
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

    selection.kind = Some(UsageQueryKind::OfficialSubscription);
    let single = client.apps_usage_refresh(&selection, TOKEN).await.unwrap();
    assert_eq!(single.status, TaskStatus::Queued);
    let batch = client.apps_usage_refresh_all(&selection.project_id, &host, TOKEN).await.unwrap();
    let current = client.apps_usage_operation(&host, &single.operation_id, TOKEN).await.unwrap();
    assert_eq!(current.operation.phase, AppOperationPhase::Executing);
    assert_eq!(current.correlation_id, "trace-a");
    let event =
        client.apps_usage_operation_events(&host, &single.operation_id, TOKEN).await.unwrap();
    assert_eq!(event.operation.status, TaskStatus::Completed);
    assert_eq!(event.items[0].kind, UsageQueryKind::OfficialSubscription);
    assert_eq!(event.items[0].snapshot_version.as_deref(), Some("version-a"));
    let cancelled =
        client.apps_usage_operation_cancel(&host, &batch.operation_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.operation.status, TaskStatus::Cancelled);
    assert_eq!(cancelled.operation.operation_id, batch.operation_id);

    selection.kind = None;
    selection.account_id = None;
    client.apps_usage_save_settings(&selection, &settings(1440), TOKEN).await.unwrap();
    let packages = client.apps_usage_script_test(&trial(), TOKEN).await.unwrap();
    assert_eq!(packages[0].plan_name.as_deref(), Some("Trial"));
    assert_eq!(packages[0].remaining.map(f64::from), Some(0.0));

    let query = "targetHostId=host+%2B%26%3F%2F%23%25%E4%B8%BB%E6%9C%BA";
    let single_path = "/v1/agent-apps/operations/single%2Fid%3Fx%3D1%23%25";
    let expected = [
        ("POST", "/v1/apps/usage/refresh".into()),
        ("POST", "/v1/apps/usage/refresh-all".into()),
        ("GET", format!("{single_path}?{query}")),
        ("GET", format!("{single_path}/events?{query}")),
        ("DELETE", format!("/v1/agent-apps/operations/batch-id?{query}")),
        ("PUT", "/v1/apps/usage/settings".into()),
        ("POST", "/v1/apps/usage/script-test".into()),
    ];
    let mut bodies = Vec::new();
    for (method, path) in expected {
        let request = server.take_request();
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
        assert_eq!(request.header("authorization"), Some("Bearer gateway-token"));
        assert_eq!(request.header("x-agent-apps-token"), Some(TOKEN));
        assert_eq!(request.header("x-platform-id"), Some("platform-a"));
        assert_eq!(request.header("x-company-id"), Some("company-a"));
        bodies.push(request.body);
    }
    assert_eq!(
        bodies[0],
        json!({
            "projectId": "project-a", "targetHostId": host, "appId": "codex",
            "providerId": "provider &accountId=other", "accountId": "account-a",
            "kind": "official_subscription"
        })
    );
    assert_eq!(bodies[1], json!({"projectId": "project-a", "targetHostId": host}));
    assert!(bodies[2..5].iter().all(Value::is_null));
    assert_eq!(bodies[5]["selection"]["appId"], "codex");
    assert!(bodies[5]["selection"]["accountId"].is_null());
    assert_eq!(
        bodies[5]["settings"],
        json!({
            "template": "general", "enabled": true, "refreshIntervalMinutes": 1440,
            "display": "used", "credentials": {"apiKeyRef": KEY_REF}
        })
    );
    assert_eq!(bodies[6]["selection"]["accountId"], "account-a");
    assert_eq!(bodies[6]["template"], "custom");
    assert_eq!(bodies[6]["source"], SOURCE);
    assert_eq!(bodies[6]["timeoutSeconds"], 12);
    server.join();
}

#[tokio::test]
async fn usage_usage_write_api_rejects_invalid_input() {
    let selection = selection();
    let host = &selection.target_host_id;
    let server = test_support::server_raw(vec![
        (400, json!({
            "code": "invalid_input", "message": "Invalid usage settings", "correlation_id": "trace"
        }).to_string()),
        (504, json!({
            "code": "timeout", "message": "Usage script trial failed", "correlationId": "trial-trace"
        }).to_string()),
        (200, progress("other-id", "cancelled", "terminal", host).to_string()),
        (403, "sensitive proxy payload".into()),
    ]);
    let client = server.client();
    assert_eq!(
        client.apps_usage_refresh(&selection, "invalid\ntoken").await.unwrap_err(),
        AppsUsageWriteError::InvalidPrincipalToken
    );
    assert_eq!(
        client.apps_usage_operation_cancel(host, &TaskId::from(".."), TOKEN).await.unwrap_err(),
        AppsUsageWriteError::InvalidOperationId
    );
    assert!(matches!(
        client.apps_usage_save_settings(&selection, &settings(1441), TOKEN).await,
        Err(AppsUsageWriteError::Http { status: 400, error: Some(error) })
            if error.code == AuthErrorCode::InvalidInput && error.correlation_id == "trace"
    ));
    assert_eq!(
        client.apps_usage_script_test(&trial(), TOKEN).await.unwrap_err(),
        AppsUsageWriteError::ScriptTrial {
            status: 504,
            code: UsageQueryErrorCode::Timeout,
            correlation_id: "trial-trace".into(),
        }
    );
    assert_eq!(
        client.apps_usage_operation_cancel(host, &TaskId::from("my-id"), TOKEN).await.unwrap_err(),
        AppsUsageWriteError::Decode
    );
    assert_eq!(
        client.apps_usage_operation_cancel(host, &TaskId::from("my-id"), TOKEN).await.unwrap_err(),
        AppsUsageWriteError::Http { status: 403, error: None }
    );
    assert_eq!(server.take_request().path, "/v1/apps/usage/settings");
    assert_eq!(server.take_request().path, "/v1/apps/usage/script-test");
    for _ in 0..2 {
        let request = server.take_request();
        assert_eq!(request.method, "DELETE");
        assert!(request.path.starts_with("/v1/agent-apps/operations/my-id?targetHostId="));
    }
    server.join();
}
