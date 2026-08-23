//! 验证工作流应用的持久化、运行和 SSE 消费接口。
//!
//! 覆盖未知事件保留、空帧跳过、回调停止、摘要回退及工作流失败状态的错误映射。

use serde_json::json;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER};
use vw_api_types::workflow::{WorkflowRecordUpsertBody, WorkflowResumeRequest};

use super::{
    GatewayWorkflowDefaultStreamRequest, GatewayWorkflowResponseMode, GatewayWorkflowStreamError,
    GatewayWorkflowStreamEvent, GatewayWorkflowStreamRequest,
};
use crate::client::test_support;

fn take_scoped_request(server: &test_support::TestServer) -> test_support::RecordedRequest {
    let request = server.take_request();
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("100000000"));
    request
}

fn workflow_record() -> serde_json::Value {
    json!({
        "uuid": "wf-1",
        "name": "Deploy",
        "description": "Ship workflow",
        "workflow_yaml": "steps: []",
        "created_at_ms": 1,
        "updated_at_ms": 2
    })
}

#[tokio::test]
async fn workflow_client_uses_application_and_stream_routes() {
    let stream = sse(json!({
        "event":"workflow_finished", "task_id":"run-1", "workflow_run_id":"run-1",
        "data":{"status":"succeeded"}
    }));
    let server = test_support::server_raw(vec![
        (200, json!([{"uuid":"wf/one","name":"Deploy","description":"Ship workflow","created_at_ms":1,"updated_at_ms":2}]).to_string()),
        (200, workflow_record().to_string()),
        (200, workflow_record().to_string()),
        (200, workflow_record().to_string()),
        (200, json!({"uuid":"wf/one","deleted":true}).to_string()),
        (200, json!({"run_id":"run-1","status":"succeeded"}).to_string()),
        (200, stream.clone()),
        (200, stream),
    ]);
    let upsert = WorkflowRecordUpsertBody {
        uuid: Some("wf/one".to_string()),
        name: "Deploy".to_string(),
        description: "Ship workflow".to_string(),
        workflow_yaml: "steps: []".to_string(),
    };
    let stream_request = workflow_stream_request("run saved");
    let default_request = GatewayWorkflowDefaultStreamRequest {
        query: "run inline".to_string(),
        application_uuid: None,
        application_workflow: Some("steps: []".to_string()),
        inputs: Default::default(),
        response_mode: GatewayWorkflowResponseMode::Streaming,
        user: "user-1".to_string(),
        conversation_id: None,
    };

    server.client().workflow_applications_list().await.expect("list");
    server.client().workflow_application_get("wf/one").await.expect("get");
    server.client().workflow_application_create(&upsert).await.expect("create");
    server.client().workflow_application_update("wf/one", &upsert).await.expect("update");
    server.client().workflow_application_delete("wf/one").await.expect("delete");
    server
        .client()
        .workflow_resume(&WorkflowResumeRequest {
            run_id: "run-1".to_string(),
            form_token: "token-1".to_string(),
            form_values: Default::default(),
            action: None,
        })
        .await
        .expect("resume");
    let mut default_events = Vec::new();
    server
        .client()
        .stream_default_workflow_application(&default_request, |event| {
            default_events.push(event);
            true
        })
        .await
        .expect("default stream");
    let mut saved_events = Vec::new();
    server
        .client()
        .stream_workflow_application("wf/one", &stream_request, |event| {
            saved_events.push(event);
            true
        })
        .await
        .expect("saved stream");
    assert_eq!(default_events.len(), 1);
    assert_eq!(saved_events.len(), 1);

    let paths = (0..8).map(|_| take_scoped_request(&server)).collect::<Vec<_>>();
    assert_eq!(paths[0].path, "/v1/workflow/applications");
    assert_eq!(paths[1].path, "/v1/workflow/applications/wf%2Fone");
    assert_eq!(paths[2].method, "POST");
    assert_eq!(paths[3].path, "/v1/workflow/applications/wf%2Fone");
    assert_eq!(paths[4].method, "DELETE");
    assert_eq!(paths[5].path, "/v1/workflow/resume");
    assert_eq!(paths[6].path, "/v1/workflow/applications/chat-messages");
    assert_eq!(paths[6].body["application_workflow"], "steps: []");
    assert_eq!(paths[7].path, "/v1/workflow/applications/wf%2Fone/chat-messages");
    assert!(paths[7].body.get("application_uuid").is_none());
    server.join();
}

#[test]
fn workflow_stream_parser_preserves_unknown_events_and_defaults_missing_fields() {
    let unknown = json!({"event": "custom_progress", "percent": 25});
    assert_eq!(
        super::parse_workflow_stream_event(unknown.clone()),
        GatewayWorkflowStreamEvent::Unknown {
            event_type: Some("custom_progress".to_string()),
            payload: unknown,
        }
    );
    assert_eq!(
        super::parse_workflow_stream_event(json!({"event": "message_end"})),
        GatewayWorkflowStreamEvent::MessageEnd {
            id: String::new(),
            task_id: String::new(),
            message_id: String::new(),
            conversation_id: String::new(),
            metadata: json!({}),
        }
    );
    assert_eq!(
        super::parse_workflow_stream_event(json!({"event": "error", "status": u64::MAX})),
        GatewayWorkflowStreamEvent::Error(GatewayWorkflowStreamError {
            task_id: String::new(),
            status: 0,
            code: String::new(),
            message: String::new(),
        })
    );
}

#[tokio::test]
async fn workflow_api_routes_application_crud_methods() {
    let server = test_support::server(vec![
        (
            200,
            json!([{
                "uuid": "wf-1",
                "name": "Deploy",
                "description": "Ship workflow",
                "created_at_ms": 1,
                "updated_at_ms": 2
            }]),
        ),
        (200, workflow_record()),
        (200, workflow_record()),
        (200, workflow_record()),
        (200, json!({"uuid": "wf-1", "deleted": true})),
    ]);
    let body = WorkflowRecordUpsertBody {
        uuid: Some("wf-1".to_string()),
        name: "Deploy".to_string(),
        description: "Ship workflow".to_string(),
        workflow_yaml: "steps: []".to_string(),
    };

    assert_eq!(server.client().workflow_applications_list().await.expect("list")[0].uuid, "wf-1");
    assert_eq!(server.client().workflow_application_get("wf-1").await.expect("get").name, "Deploy");
    assert_eq!(
        server.client().workflow_application_create(&body).await.expect("create").uuid,
        "wf-1"
    );
    assert_eq!(
        server
            .client()
            .workflow_application_update("wf-1", &body)
            .await
            .expect("update")
            .workflow_yaml,
        "steps: []"
    );
    assert!(server.client().workflow_application_delete("wf-1").await.expect("delete").deleted);

    assert_eq!(take_scoped_request(&server).path, "/v1/workflow/applications");
    assert_eq!(take_scoped_request(&server).path, "/v1/workflow/applications/wf-1");
    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/workflow/applications");
    assert_eq!(request.body["uuid"], "wf-1");
    let request = take_scoped_request(&server);
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/workflow/applications/wf-1");
    assert_eq!(request.body["name"], "Deploy");
    let request = take_scoped_request(&server);
    assert_eq!(request.method, "DELETE");
    assert_eq!(request.path, "/v1/workflow/applications/wf-1");
    assert_eq!(request.body, json!({}));
    server.join();
}

#[tokio::test]
async fn workflow_streaming_api_routes_uuid_chat_messages_and_parses_events() {
    let server = test_support::server_raw(vec![(
        200,
        [
            sse(json!({
                "event": "workflow_started",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"id": "run-1"}
            })),
            sse(json!({
                "event": "node_started",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"node_id": "start", "title": "Start"}
            })),
            sse(json!({
                "event": "node_finished",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"node_id": "start", "status": "succeeded"}
            })),
            sse(json!({
                "event": "text_chunk",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"text": "hello"}
            })),
            sse(json!({
                "event": "workflow_finished",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"status": "succeeded"}
            })),
            sse(json!({
                "event": "message_end",
                "id": "run-1",
                "task_id": "run-1",
                "message_id": "run-1",
                "conversation_id": "conv-1",
                "metadata": {}
            })),
            sse(json!({
                "event": "error",
                "task_id": "run-1",
                "status": 400,
                "code": "workflow_run_error",
                "message": "bad node"
            })),
        ]
        .join(""),
    )]);
    let request = GatewayWorkflowStreamRequest {
        query: "preview".to_string(),
        inputs: [("topic".to_string(), json!("rust"))].into(),
        response_mode: GatewayWorkflowResponseMode::Streaming,
        user: "user-1".to_string(),
        conversation_id: Some("conv-1".to_string()),
    };
    let mut events = Vec::new();

    server
        .client()
        .stream_workflow_application("wf-1", &request, |event| {
            events.push(event);
            true
        })
        .await
        .expect("workflow stream");

    assert_eq!(
        events,
        vec![
            GatewayWorkflowStreamEvent::WorkflowStarted {
                task_id: "run-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                data: json!({"id": "run-1"}),
            },
            GatewayWorkflowStreamEvent::NodeStarted {
                task_id: "run-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                data: json!({"node_id": "start", "title": "Start"}),
            },
            GatewayWorkflowStreamEvent::NodeFinished {
                task_id: "run-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                data: json!({"node_id": "start", "status": "succeeded"}),
            },
            GatewayWorkflowStreamEvent::TextChunk {
                task_id: "run-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                text: "hello".to_string(),
                data: json!({"text": "hello"}),
            },
            GatewayWorkflowStreamEvent::WorkflowFinished {
                task_id: "run-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                data: json!({"status": "succeeded"}),
            },
            GatewayWorkflowStreamEvent::MessageEnd {
                id: "run-1".to_string(),
                task_id: "run-1".to_string(),
                message_id: "run-1".to_string(),
                conversation_id: "conv-1".to_string(),
                metadata: json!({}),
            },
            GatewayWorkflowStreamEvent::Error(GatewayWorkflowStreamError {
                task_id: "run-1".to_string(),
                status: 400,
                code: "workflow_run_error".to_string(),
                message: "bad node".to_string(),
            }),
        ]
    );

    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/workflow/applications/wf-1/chat-messages");
    assert_eq!(request.body["query"], "preview");
    assert_eq!(request.body["inputs"]["topic"], "rust");
    assert_eq!(request.body["response_mode"], "streaming");
    assert_eq!(request.body["user"], "user-1");
    assert_eq!(request.body["conversation_id"], "conv-1");
    assert_eq!(request.header(PLATFORM_ID_HEADER), Some("10000"));
    assert_eq!(request.header(COMPANY_ID_HEADER), Some("100000000"));
    server.join();
}

#[tokio::test]
async fn workflow_client_runs_saved_application_by_uuid() {
    let server = test_support::server_raw(vec![(
        200,
        [
            sse(json!({
                "event": "workflow_started",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"id": "run-1"}
            })),
            sse(json!({
                "event": "text_chunk",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"text": "draft "}
            })),
            sse(json!({
                "event": "message",
                "id": "run-1",
                "task_id": "run-1",
                "message_id": "msg-1",
                "conversation_id": "conv-1",
                "answer": "recap accepted",
                "created_at": 10
            })),
            sse(json!({
                "event": "workflow_finished",
                "task_id": "run-1",
                "workflow_run_id": "run-1",
                "data": {"status": "succeeded"}
            })),
        ]
        .join(""),
    )]);
    let request = GatewayWorkflowStreamRequest {
        query: "Run the review workflow".to_string(),
        inputs: [("task_id".to_string(), json!("T-review"))].into(),
        response_mode: GatewayWorkflowResponseMode::Streaming,
        user: "staff-reviewer".to_string(),
        conversation_id: None,
    };

    let outcome = server
        .client()
        .run_workflow_application("wf-review", &request)
        .await
        .expect("workflow application run");

    assert_eq!(outcome.summary, "recap accepted");
    assert_eq!(outcome.workflow_run_id.as_deref(), Some("run-1"));
    assert_eq!(outcome.conversation_id.as_deref(), Some("conv-1"));
    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/workflow/applications/wf-review/chat-messages");
    assert_eq!(request.body["query"], "Run the review workflow");
    assert_eq!(request.body["inputs"]["task_id"], "T-review");
    assert_eq!(request.body["response_mode"], "streaming");
    assert_eq!(request.body["user"], "staff-reviewer");
    server.join();
}

#[tokio::test]
async fn workflow_streaming_returns_error_for_failed_http_response() {
    let server = test_support::server_raw(vec![(404, json!({"message": "missing"}).to_string())]);
    let request = workflow_stream_request("preview");

    let error = server
        .client()
        .stream_workflow_application("missing-workflow", &request, |_| true)
        .await
        .expect_err("failed response");

    assert!(error.contains("404"));
    assert!(error.contains("missing"));
    let request = take_scoped_request(&server);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/workflow/applications/missing-workflow/chat-messages");
    server.join();
}

#[tokio::test]
async fn workflow_streaming_skips_empty_payload_and_stops_when_callback_returns_false() {
    let server = test_support::server_raw(vec![(
        200,
        [
            "data:\n\n".to_string(),
            sse(json!({
                "event": "workflow_started",
                "task_id": "run-early",
                "workflow_run_id": "run-early",
                "data": {}
            })),
            sse(json!({
                "event": "error",
                "task_id": "run-early",
                "status": 400,
                "code": "should_not_be_delivered",
                "message": "late event"
            })),
        ]
        .join(""),
    )]);
    let request = workflow_stream_request("preview");
    let mut events = Vec::new();

    server
        .client()
        .stream_workflow_application("wf-early", &request, |event| {
            events.push(event);
            false
        })
        .await
        .expect("stopped stream");

    assert_eq!(
        events,
        vec![GatewayWorkflowStreamEvent::WorkflowStarted {
            task_id: "run-early".to_string(),
            workflow_run_id: "run-early".to_string(),
            data: json!({}),
        }]
    );
    server.join();
}

#[tokio::test]
async fn workflow_streaming_returns_parse_error_for_invalid_json_payload() {
    let server = test_support::server_raw(vec![(200, "data: not-json\n\n".to_string())]);
    let request = workflow_stream_request("preview");

    let error = server
        .client()
        .stream_workflow_application("wf-bad-json", &request, |_| true)
        .await
        .expect_err("invalid json");

    assert!(error.contains("expected ident"));
    server.join();
}

#[tokio::test]
async fn workflow_client_uses_trimmed_text_summary_when_message_answer_is_empty() {
    let server = test_support::server_raw(vec![(
        200,
        [
            sse(json!({
                "event": "node_started",
                "task_id": "run-text",
                "workflow_run_id": "run-text",
                "data": {}
            })),
            sse(json!({
                "event": "text_chunk",
                "task_id": "run-text",
                "workflow_run_id": "",
                "data": {"text": "  draft "}
            })),
            sse(json!({
                "event": "text_chunk",
                "task_id": "run-text",
                "workflow_run_id": "",
                "data": {"text": "summary  "}
            })),
            sse(json!({
                "event": "message",
                "id": "msg-text",
                "task_id": "run-text",
                "message_id": "msg-text",
                "conversation_id": "",
                "answer": "   ",
                "created_at": 20
            })),
        ]
        .join(""),
    )]);
    let request = workflow_stream_request("summarize");

    let outcome = server.client().run_workflow_application("wf-text", &request).await.expect("run");

    assert_eq!(outcome.summary, "draft summary");
    assert_eq!(outcome.workflow_run_id.as_deref(), Some("run-text"));
    assert_eq!(outcome.conversation_id, None);
    server.join();
}

#[tokio::test]
async fn workflow_client_uses_default_summary_and_message_end_conversation() {
    let server = test_support::server_raw(vec![(
        200,
        sse(json!({
            "event": "message_end",
            "id": "msg-default",
            "task_id": "run-default",
            "message_id": "msg-default",
            "conversation_id": "conv-default",
            "metadata": {}
        })),
    )]);
    let request = workflow_stream_request("summarize");

    let outcome =
        server.client().run_workflow_application("wf-default", &request).await.expect("run");

    assert_eq!(outcome.summary, "workflow completed");
    assert_eq!(outcome.workflow_run_id, None);
    assert_eq!(outcome.conversation_id.as_deref(), Some("conv-default"));
    server.join();
}

#[tokio::test]
async fn workflow_client_returns_error_when_workflow_finished_with_failed_status() {
    let server = test_support::server_raw(vec![(
        200,
        [
            sse(json!({
                "event": "workflow_finished",
                "task_id": "run-failed",
                "workflow_run_id": "run-failed",
                "data": {"status": "failed"}
            })),
            sse(json!({
                "event": "message",
                "id": "late",
                "task_id": "run-failed",
                "message_id": "late",
                "conversation_id": "late",
                "answer": "late",
                "created_at": 30
            })),
        ]
        .join(""),
    )]);
    let request = workflow_stream_request("run");

    let error = server
        .client()
        .run_workflow_application("wf-failed", &request)
        .await
        .expect_err("failed workflow");

    assert_eq!(error, "workflow finished with status failed");
    server.join();
}

#[tokio::test]
async fn workflow_client_returns_stream_error_event_message() {
    let server = test_support::server_raw(vec![(
        200,
        sse(json!({
            "event": "error",
            "task_id": "run-error",
            "status": 422,
            "code": "invalid_inputs",
            "message": "topic is required"
        })),
    )]);
    let request = workflow_stream_request("run");

    let error = server
        .client()
        .run_workflow_application("wf-error", &request)
        .await
        .expect_err("stream error");

    assert_eq!(error, "invalid_inputs: topic is required");
    server.join();
}

fn workflow_stream_request(query: &str) -> GatewayWorkflowStreamRequest {
    GatewayWorkflowStreamRequest {
        query: query.to_string(),
        inputs: Default::default(),
        response_mode: GatewayWorkflowResponseMode::Streaming,
        user: "user-1".to_string(),
        conversation_id: None,
    }
}

fn sse(payload: serde_json::Value) -> String {
    format!("data: {payload}\n\n")
}
