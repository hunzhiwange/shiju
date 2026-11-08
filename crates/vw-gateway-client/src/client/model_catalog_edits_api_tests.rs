use super::{AgentAppsReadError, test_support};
use serde_json::json;
use vw_api_types::{agent_apps::AppId, id::TenantScope, model_catalog::*};

const TOKEN: &str = "test-editor-token";

fn target() -> (ModelCatalogProject, ScopedModelCatalogIdentity) {
    (
        ModelCatalogProject { project_id: "project".into(), directory: "/work/模型".into() },
        ScopedModelCatalogIdentity {
            scope: TenantScope::new("platform", "company").unwrap(),
            deployment: ModelCatalogIdentity {
                target_host_id: "mac".into(),
                app_id: AppId::Codex,
                provider_id: "saved".into(),
                endpoint: "https://example.test/v1".into(),
            },
            model_id: "vendor/Model".into(),
        },
    )
}

#[tokio::test]
async fn catalog_api_edits_sdk_sends_typed_requests_with_project_and_tenant() {
    let (project, identity) = target();
    let edits = ModelCatalogEditRequest {
        deployment: identity.deployment.clone(),
        expected_revision: "first".into(),
        edits: vec![ModelCatalogEdit::Upsert {
            model_id: identity.model_id.clone(),
            display_alias: Some("展示".into()),
            metadata: Box::default(),
        }],
        default_change: None,
    };
    let defaults = ModelCatalogDefaultRequest {
        deployment: identity.deployment.clone(),
        expected_revision: "second".into(),
        change: ModelCatalogDefaultChange::Set { model_id: identity.model_id.clone() },
    };
    let preview = ModelCatalogFillPreviewRequest { identity: identity.clone() };
    let commit = ModelCatalogFillCommitRequest {
        identity: identity.clone(),
        expected_revision: "third".into(),
    };
    let server = test_support::server_with_tenant_scope(
        vec![
            (200, json!({"revision":"second","changed":true})),
            (200, json!({"revision":"third","changed":true})),
            (200, json!({"identity":identity,"revision":"third","proposed":{"contextWindow":256}})),
            (200, json!({"revision":"fourth","changed":true})),
        ],
        identity.scope.clone(),
    );
    let client = server.client();
    assert_eq!(
        client.edit_model_catalog(&project, &edits, TOKEN).await.unwrap().revision,
        "second"
    );
    assert_eq!(
        client.set_model_catalog_default(&project, &defaults, TOKEN).await.unwrap().revision,
        "third"
    );
    let result = client.preview_model_catalog_fill(&project, &preview, TOKEN).await.unwrap();
    assert_eq!(result.identity, identity);
    assert_eq!(result.proposed.context_window, Some(256));
    assert!(client.commit_model_catalog_fill(&project, &commit, TOKEN).await.unwrap().changed);
    for (resource, body) in [
        ("edits", json!(edits)),
        ("defaults", json!(defaults)),
        ("fill-preview", json!(preview)),
        ("fill-commit", json!(commit)),
    ] {
        let sent = server.take_request();
        assert_eq!(sent.method, "POST");
        assert_eq!(
            sent.path,
            format!(
                "/v1/model-catalog/{resource}?projectId=project&directory=%2Fwork%2F%E6%A8%A1%E5%9E%8B"
            )
        );
        assert_eq!(sent.header("x-agent-apps-token"), Some(TOKEN));
        assert_eq!(sent.header("x-company-id"), Some("company"));
        assert_eq!(sent.body, body);
    }
    server.join();
}

#[tokio::test]
async fn catalog_api_edits_sdk_preserves_conflict_status_without_private_payload() {
    let (project, identity) = target();
    let server = test_support::server(vec![(409, json!({"private":TOKEN}))]);
    let request = ModelCatalogFillCommitRequest { identity, expected_revision: "stale".into() };
    let error =
        server.client().commit_model_catalog_fill(&project, &request, TOKEN).await.unwrap_err();
    assert_eq!(error, AgentAppsReadError::Http { status: 409, body: serde_json::Value::Null });
    server.join();
}
