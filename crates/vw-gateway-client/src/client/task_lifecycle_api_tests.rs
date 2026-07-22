use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};
use vw_api_types::task_lifecycle::{
    TaskLifecycleStatusDto, TaskLifecycleStatusRoleDto, TaskLifecycleWorkflowDto,
    TaskLifecycleWorkflowUpsertBody,
};

use crate::client::test_support;

fn take_scoped_request(server: &test_support::TestServer) -> test_support::RecordedRequest {
    let request = server.take_request();
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("100000000"));
    request
}

#[tokio::test]
async fn task_lifecycle_client_uses_expected_paths() {
    let server = test_support::server(vec![
        (200, json!([workflow_json("support_review", "Support review")])),
        (200, workflow_json("support_review", "Support review")),
        (200, workflow_json("support_review", "Support review")),
        (200, workflow_json("support_review", "Support triage")),
        (200, json!({"id": "support_review", "deleted": true})),
        (200, workflow_json("dev_lifecycle", "开发生命周期")),
    ]);
    let create = TaskLifecycleWorkflowUpsertBody {
        workflow: custom_workflow("support_review", "Support review"),
    };
    let update = TaskLifecycleWorkflowUpsertBody {
        workflow: custom_workflow("support_review", "Support triage"),
    };

    let workflows = server.client().task_lifecycle_workflows_list().await.expect("list");
    assert_eq!(workflows[0].id, "support_review");
    assert_eq!(
        server.client().task_lifecycle_workflows_get("support_review").await.expect("get").label,
        "Support review"
    );
    assert_eq!(
        server.client().task_lifecycle_workflows_create(&create).await.expect("create").id,
        "support_review"
    );
    assert_eq!(
        server
            .client()
            .task_lifecycle_workflows_update("support_review", &update)
            .await
            .expect("update")
            .label,
        "Support triage"
    );
    assert!(
        server
            .client()
            .task_lifecycle_workflows_delete("support_review")
            .await
            .expect("delete")
            .deleted
    );
    assert_eq!(
        server.client().task_lifecycle_workflows_reset("dev_lifecycle").await.expect("reset").id,
        "dev_lifecycle"
    );

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows");
    assert_eq!(request.body, serde_json::Value::Null);

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows/support_review");
    assert_eq!(request.body, serde_json::Value::Null);

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows");
    assert_eq!(request.body["workflow"]["id"], "support_review");
    assert_eq!(request.body["workflow"]["statuses"][1]["next"], "done");

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows/support_review");
    assert_eq!(request.body["workflow"]["label"], "Support triage");

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows/support_review");
    assert_eq!(request.body, json!({}));

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/task-lifecycle/workflows/dev_lifecycle/reset");
    assert_eq!(request.body, json!({}));
    server.join();
}

fn workflow_json(id: &str, label: &str) -> serde_json::Value {
    serde_json::to_value(custom_workflow(id, label)).expect("workflow json")
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
        auto_execution_candidate: role == TaskLifecycleStatusRoleDto::Executing,
        occupies_execution_slot: role == TaskLifecycleStatusRoleDto::Executing,
        is_scheduling_backlog: role == TaskLifecycleStatusRoleDto::Queued,
        next: next.map(str::to_string),
        transitions: Vec::new(),
    }
}
