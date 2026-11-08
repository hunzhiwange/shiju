use super::{AgentAppsReadError, test_support};
use serde_json::json;
use vw_api_types::{agent_apps::AppId, id::TenantScope, model_catalog::*};

const TOKEN: &str = "model-discovery-test-token";

fn request() -> (ModelCatalogProject, ModelDiscoveryRequest) {
    (
        ModelCatalogProject { project_id: "project".into(), directory: "/work/模型".into() },
        ModelDiscoveryRequest {
            deployment: ModelCatalogIdentity {
                target_host_id: "mac".into(),
                app_id: AppId::Codex,
                provider_id: "saved".into(),
                endpoint: "https://example.test/v1".into(),
            },
            method: ModelDiscoveryMethod::Http,
            account_id: None,
        },
    )
}

#[tokio::test]
async fn catalog_api_discovery_sdk_sends_three_scoped_requests() {
    let (project, request) = request();
    let response = |phase: &str| {
        json!({
            "taskId":"task", "deployment": request.deployment, "state":{"phase":phase}
        })
    };
    let server = test_support::server_with_tenant_scope(
        vec![
            (202, response("queued")),
            (200, response("discovering")),
            (200, response("cancelled")),
        ],
        TenantScope::new("platform", "company").unwrap(),
    );
    let client = server.client();
    let created = client.start_model_discovery(&project, &request, TOKEN).await.unwrap();
    assert_eq!(created.state, ModelDiscoveryState::Queued);
    let read = client.model_discovery(&project, "mac", &created.task_id, TOKEN).await.unwrap();
    assert_eq!(read.state, ModelDiscoveryState::Discovering);
    let cancelled =
        client.cancel_model_discovery(&project, "mac", &created.task_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.state, ModelDiscoveryState::Cancelled);

    for (method, suffix) in [("POST", ""), ("GET", "/task"), ("DELETE", "/task")] {
        let sent = server.take_request();
        assert_eq!(sent.method, method);
        assert_eq!(sent.header("x-agent-apps-token"), Some(TOKEN));
        assert_eq!(sent.header("x-company-id"), Some("company"));
        assert_eq!(
            sent.path,
            format!(
                "/v1/model-catalog/discoveries{suffix}?projectId=project&directory=%2Fwork%2F%E6%A8%A1%E5%9E%8B&targetHostId=mac"
            )
        );
        if method == "POST" {
            assert_eq!(sent.body, serde_json::to_value(&request).unwrap());
        }
    }
    server.join();
}

#[tokio::test]
async fn catalog_api_discovery_sdk_discards_error_payload() {
    let server = test_support::server(vec![(403, json!({"private":TOKEN}))]);
    let (project, request) = request();
    let error = server.client().start_model_discovery(&project, &request, TOKEN).await.unwrap_err();
    assert_eq!(error, AgentAppsReadError::Http { status: 403, body: serde_json::Value::Null });
    server.join();
}
