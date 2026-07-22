use super::*;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;
use vw_api_types::TenantScope;
use vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
};

#[tokio::test]
async fn task_lifecycle_default_router_registers_workflow_routes() {
    let response = router::<()>()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/task-lifecycle/workflows")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router response");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_routes_crud_custom_workflows() {
    let home = tempfile::tempdir().expect("temp home");
    let expected_path = vw_config_types::paths::global_database_path(home.path());
    let paths = StorePaths { home: home.path().to_path_buf() };
    let tenant = TenantScope::new("platform-lifecycle", "company-lifecycle").expect("tenant");
    let app =
        Router::new().nest("/v1", router_with_paths::<()>(paths)).layer(Extension(tenant.clone()));

    let list = request_json(&app, Method::GET, "/v1/task-lifecycle/workflows", None).await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(
        list.body
            .as_array()
            .expect("list body")
            .iter()
            .any(|workflow| workflow["id"] == "dev_lifecycle" && workflow["built_in"] == true)
    );

    assert!(expected_path.is_file());
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    assert_eq!(database.path(), expected_path);
    let connection = database.connect().expect("shared database connection");
    let table_name: String = connection
        .query_row(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name = 'workflow_lifecycle_workflows'",
            [],
            |row| row.get(0),
        )
        .expect("task lifecycle table in shared database");
    assert_eq!(table_name, "workflow_lifecycle_workflows");
    drop(connection);

    let builtin_update = request_json(
        &app,
        Method::PUT,
        "/v1/task-lifecycle/workflows/dev_lifecycle",
        Some(json!({ "workflow": custom_workflow("dev_lifecycle", "Override") })),
    )
    .await;
    assert_eq!(builtin_update.status, StatusCode::OK);
    assert_eq!(builtin_update.body["id"], "dev_lifecycle");
    assert_eq!(builtin_update.body["label"], "Override");
    assert_eq!(builtin_update.body["built_in"], true);

    let workflow = custom_workflow("support_review", "Support Review");
    let created = request_json(
        &app,
        Method::POST,
        "/v1/task-lifecycle/workflows",
        Some(json!({ "workflow": workflow })),
    )
    .await;
    assert_eq!(created.status, StatusCode::OK);
    assert_eq!(created.body["id"], "support_review");
    assert!(!created.body.get("built_in").and_then(Value::as_bool).unwrap_or(false));
    let stored_scope: (String, String) = database
        .connect()
        .expect("shared database connection")
        .query_row(
            "SELECT platform_id, company_id FROM workflow_lifecycle_workflows \
             WHERE id = 'support_review'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("stored local workflow scope");
    assert_eq!(
        stored_scope,
        (tenant.platform_id().as_ref().to_string(), tenant.company_id().as_ref().to_string())
    );

    let fetched =
        request_json(&app, Method::GET, "/v1/task-lifecycle/workflows/support_review", None).await;
    assert_eq!(fetched.status, StatusCode::OK);
    assert_eq!(fetched.body["label"], "Support Review");

    let updated = request_json(
        &app,
        Method::PUT,
        "/v1/task-lifecycle/workflows/support_review",
        Some(json!({ "workflow": custom_workflow("support_review", "Updated Review") })),
    )
    .await;
    assert_eq!(updated.status, StatusCode::OK);
    assert_eq!(updated.body["label"], "Updated Review");

    let builtin_delete =
        request_json(&app, Method::DELETE, "/v1/task-lifecycle/workflows/dev_lifecycle", None)
            .await;
    assert_eq!(builtin_delete.status, StatusCode::BAD_REQUEST);
    assert!(builtin_delete.body["error"].as_str().unwrap_or_default().contains("built-in"));

    let reset =
        request_json(&app, Method::POST, "/v1/task-lifecycle/workflows/dev_lifecycle/reset", None)
            .await;
    assert_eq!(reset.status, StatusCode::OK);
    assert_eq!(reset.body["id"], "dev_lifecycle");
    assert_eq!(reset.body["label"], "开发生命周期");

    let deleted =
        request_json(&app, Method::DELETE, "/v1/task-lifecycle/workflows/support_review", None)
            .await;
    assert_eq!(deleted.status, StatusCode::OK);
    assert_eq!(deleted.body, json!({ "id": "support_review", "deleted": true }));

    let missing =
        request_json(&app, Method::GET, "/v1/task-lifecycle/workflows/support_review", None).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn task_lifecycle_routes_keep_shared_id_isolated_by_request_tenant() {
    let home = tempfile::tempdir().expect("temp home");
    let tenant_a = TenantScope::new("platform-lifecycle", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-lifecycle", "company-b").expect("tenant B");
    let paths = StorePaths { home: home.path().to_path_buf() };
    let app_a = Router::new()
        .nest("/v1", router_with_paths::<()>(paths.clone()))
        .layer(Extension(tenant_a));
    let app_b =
        Router::new().nest("/v1", router_with_paths::<()>(paths)).layer(Extension(tenant_b));

    for (app, label) in [(&app_a, "Tenant A"), (&app_b, "Tenant B")] {
        let created = request_json(
            app,
            Method::POST,
            "/v1/task-lifecycle/workflows/shared_review",
            Some(json!({ "workflow": custom_workflow("shared_review", label) })),
        )
        .await;
        assert_eq!(created.status, StatusCode::METHOD_NOT_ALLOWED);

        let created = request_json(
            app,
            Method::POST,
            "/v1/task-lifecycle/workflows",
            Some(json!({ "workflow": custom_workflow("shared_review", label) })),
        )
        .await;
        assert_eq!(created.status, StatusCode::OK);
    }

    let fetched_a =
        request_json(&app_a, Method::GET, "/v1/task-lifecycle/workflows/shared_review", None).await;
    let fetched_b =
        request_json(&app_b, Method::GET, "/v1/task-lifecycle/workflows/shared_review", None).await;
    assert_eq!(fetched_a.body["label"], "Tenant A");
    assert_eq!(fetched_b.body["label"], "Tenant B");

    let reset_a = request_json(
        &app_a,
        Method::POST,
        "/v1/task-lifecycle/workflows/shared_review/reset",
        None,
    )
    .await;
    assert_eq!(reset_a.status, StatusCode::BAD_REQUEST);
    let fetched_b =
        request_json(&app_b, Method::GET, "/v1/task-lifecycle/workflows/shared_review", None).await;
    assert_eq!(fetched_b.body["label"], "Tenant B");
}

struct JsonResponse {
    status: StatusCode,
    body: Value,
}

async fn request_json(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> JsonResponse {
    let mut builder = Request::builder().method(method).uri(uri);
    let request_body = if let Some(body) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        Body::from(serde_json::to_vec(&body).expect("request json"))
    } else {
        Body::empty()
    };
    let response = app.clone().oneshot(builder.body(request_body).expect("request")).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("response body");
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    JsonResponse { status, body }
}

fn custom_workflow(id: &str, label: &str) -> TaskLifecycleWorkflowDto {
    TaskLifecycleWorkflowDto {
        id: id.to_string(),
        label: label.to_string(),
        built_in: false,
        statuses: vec![
            status("queued", "Queued", TaskLifecycleStatusRoleDto::Queued, false, Some("running")),
            status(
                "running",
                "Running",
                TaskLifecycleStatusRoleDto::Executing,
                false,
                Some("done"),
            ),
            status("done", "Done", TaskLifecycleStatusRoleDto::Done, true, None),
        ],
    }
}

fn status(
    key: &str,
    label: &str,
    role: TaskLifecycleStatusRoleDto,
    terminal: bool,
    next: Option<&str>,
) -> TaskLifecycleStatusDto {
    TaskLifecycleStatusDto {
        key: key.to_string(),
        label: label.to_string(),
        role,
        enabled: true,
        terminal,
        auto_execution_candidate: !terminal,
        occupies_execution_slot: role == TaskLifecycleStatusRoleDto::Executing,
        is_scheduling_backlog: role == TaskLifecycleStatusRoleDto::Queued,
        next: next.map(str::to_string),
        transitions: Vec::new(),
    }
}
