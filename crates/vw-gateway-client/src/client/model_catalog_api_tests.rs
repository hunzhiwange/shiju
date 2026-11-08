use super::{AgentAppsReadError, test_support};
use serde_json::json;
use vw_api_types::{
    agent_apps::AppId,
    common::PaginationRequest,
    id::TenantScope,
    model_catalog::{ModelCatalogIdentity, ModelCatalogProject, ModelCatalogQuery},
};

const TOKEN: &str = "model-catalog-test-token";

#[tokio::test]
async fn catalog_api_read_sdk_sends_project_host_and_pagination() {
    let server = test_support::server_with_tenant_scope(
        vec![(200, json!({"revision":"test","items":[]}))],
        TenantScope::new("platform", "company").unwrap(),
    );
    let project = ModelCatalogProject {
        project_id: "project-one".into(),
        directory: "/work/项目 A".into(),
    };
    let query = ModelCatalogQuery {
        deployment: ModelCatalogIdentity {
            target_host_id: "mac".into(),
            app_id: AppId::Codex,
            provider_id: "provider".into(),
            endpoint: "https://fixture.test/v1".into(),
        },
        search: Some("vendor/model +".into()),
        page: Some(PaginationRequest { cursor: Some("2".into()), limit: Some(5) }),
    };
    let response = server.client().model_catalog(&project, &query, TOKEN).await.unwrap();
    assert_eq!(response.revision, "test");
    let request = server.take_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.header("x-agent-apps-token"), Some(TOKEN));
    assert_eq!(request.header("x-company-id"), Some("company"));
    assert!(!request.path.contains(TOKEN));
    assert_eq!(
        request.path,
        "/v1/model-catalog?projectId=project-one&directory=%2Fwork%2F%E9%A1%B9%E7%9B%AE+A&targetHostId=mac&appId=codex&providerId=provider&endpoint=https%3A%2F%2Ffixture.test%2Fv1&search=vendor%2Fmodel+%2B&cursor=2&limit=5"
    );
    server.join();
}

#[tokio::test]
async fn catalog_api_read_sdk_keeps_status_and_discards_untrusted_error_payload() {
    let server = test_support::server(vec![(403, json!({"credential":TOKEN}))]);
    let project =
        ModelCatalogProject { project_id: "project-one".into(), directory: "/work/a".into() };
    let error = server
        .client()
        .model_catalog_templates(&project, "mac", AppId::Codex, TOKEN)
        .await
        .unwrap_err();
    assert_eq!(error, AgentAppsReadError::Http { status: 403, body: serde_json::Value::Null });
    assert!(!error.to_string().contains(TOKEN));
    server.join();
}
