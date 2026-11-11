//! Real operation HTTP cancellation around the existing irreversible gate.
use super::*;
use axum::{
    Extension,
    body::{Body, to_bytes},
    http::Request,
    middleware,
    routing::get,
};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId};
#[tokio::test]
async fn aggregation_t42_processing_cancel_waits_for_worker_and_preserves_published_result() {
    let root = tempfile::tempdir().unwrap();
    let policy = root.path().join("policy.json");
    std::fs::create_dir(root.path().join("private")).unwrap();
    std::fs::set_permissions(root.path().join("private"), std::fs::Permissions::from_mode(0o700))
        .unwrap();
    let token = "aggregation-t42-operation-bearer-token-32";
    let apps = json!([{"project_id":"project","app_id":"claude"}]);
    std::fs::write(&policy,json!({"principals":[{"id":"actor","token_sha256":hex::encode(Sha256::digest(token)),"grants":[{
        "platform_id":"p","company_id":"c","host_id":"host","provider_config_read":true,"provider_config_write":true,
        "aggregation_read":apps,"aggregation_write":apps,"routing_manage":true,"routing_apps":apps}]}],
        "hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
    std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    let state = State {
        policy: Some(policy.clone()),
        editor: Editor::new(Some(policy)),
        operations: Default::default(),
    };
    let owner = Owner {
        project: ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from("project"))
            .unwrap(),
        host: "host".into(),
        app: AppId::Claude,
        principal: "actor".into(),
    };
    let router = Router::new()
        .route(
            "/app-providers/operations/{id}",
            get(super::super::read).delete(super::super::cancel),
        )
        .layer(Extension(state.clone()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    for published in [false, true] {
        let (ready, wait) = tokio::sync::oneshot::channel();
        let (resume, receive) = std::sync::mpsc::channel();
        let path = root.path().join(if published { "after" } else { "before" });
        let output = path.clone();
        let initial = state
            .aggregation_exit_start(
                owner.clone(),
                format!("cancel-{published}"),
                [u8::from(published); 32],
                move || {
                    if published {
                        vw_agent_apps::providers::operations::publication().unwrap();
                    }
                    ready.send(()).unwrap();
                    receive.recv().unwrap();
                    if vw_agent_apps::providers::operations::publication().is_err() {
                        return AggregationOutcome::Failed {
                            status: 409,
                            code: AggregationErrorCode::RevisionConflict,
                            recovery: RecoveryState::None,
                        };
                    }
                    std::fs::write(output, b"restored").unwrap();
                    AggregationOutcome::Exited {
                        state: AggregationState {
                            revision: "settled".into(),
                            mode: AggregationMode::Direct,
                            attached: false,
                            default_provider_id: None,
                            direct_provider_id: None,
                            members: vec![],
                            notices: vec![],
                            recovery: RecoveryState::Restored,
                        },
                    }
                },
            )
            .unwrap();
        wait.await.unwrap();
        let uri = format!(
            "/app-providers/operations/{}?targetHostId=host&projectId=project",
            initial["operationId"].as_str().unwrap()
        );
        let request = |method: &str| {
            Request::builder()
                .method(method)
                .uri(&uri)
                .header(PLATFORM_ID_HEADER, "p")
                .header(COMPANY_ID_HEADER, "c")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };
        let response = router.clone().oneshot(request("DELETE")).await.unwrap();
        assert_eq!(response.status(), 200);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
        assert_eq!(body["status"], "running");
        assert_eq!(body["cancellationRequested"], true);
        assert!(body.get("aggregationOutcome").is_none());
        resume.send(()).unwrap();
        let mut terminal = None;
        for _ in 0..300 {
            let response = router.clone().oneshot(request("GET")).await.unwrap();
            assert_eq!(response.status(), 200);
            let body: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap())
                    .unwrap();
            if matches!(body["status"].as_str(), Some("completed" | "failed" | "cancelled")) {
                terminal = Some(body);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let terminal = terminal.expect("worker must settle");
        assert_eq!(
            terminal["status"],
            if published { "completed" } else { "cancelled" },
            "{terminal}"
        );
        assert_eq!(path.exists(), published);
        if published {
            assert_eq!(terminal["aggregationOutcome"]["kind"], "exited");
        } else {
            assert!(terminal.get("aggregationOutcome").is_none());
        }
    }
}
