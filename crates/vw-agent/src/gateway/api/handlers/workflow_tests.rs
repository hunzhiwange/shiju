use super::{
    GatewayWorkflowToolProvider, VibeWindowChatRequest, VibeWindowResponseMode, VibeWindowSseEvent,
    WORKFLOW_INPUT_FULL_ACCESS, WORKFLOW_INPUT_ROOT, WorkflowSseContext,
    apply_path_chat_workflow_uuid, chat_messages_inner, chat_request_to_workflow_request,
    compact_sse_node_outputs, hydrate_workflow_yaml_from_uuid, node_finished_data,
    node_started_data, node_started_data_from_event, node_status, normalized_optional_string,
    router, strip_workflow_runtime_inputs, text_chunk_data, truncate_sse_node_output,
    validate_path_workflow_uuid, workflow_answer, workflow_application_chat_messages,
    workflow_application_create, workflow_application_delete, workflow_application_get,
    workflow_application_update, workflow_applications_chat_messages, workflow_applications_list,
    workflow_delta_should_stream_text, workflow_finished_data, workflow_request_full_access,
    workflow_request_root, workflow_response_to_chat_response, workflow_response_to_sse_events,
    workflow_response_to_sse_response, workflow_runtime, workflow_sse_output_preview,
    workflow_started_data, workflow_status, workflow_tool_call_error_text,
    workflow_tool_runtime_context,
};
use crate::app::agent::config::Config;
use crate::app::agent::gateway::state::AppState;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::ToolCallError;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::workflow::{
    WorkflowNodeDeltaEvent, WorkflowNodeStartedEvent, WorkflowRunEvent, WorkflowToolProvider,
    WorkflowToolRequest,
};
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::StatusCode,
};
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use uuid::Uuid;
use vw_api_types::TenantScope;
use vw_api_types::workflow::{
    WorkflowNodeRunDto, WorkflowNodeRunStatus, WorkflowRecordUpsertBody, WorkflowRunRequest,
    WorkflowRunResponse, WorkflowRunStatus,
};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

#[test]
fn workflow_router_is_wired() {
    let _router = router();
}

#[tokio::test]
async fn workflow_chat_handlers_reject_missing_user_or_query_before_runtime() {
    let tenant = test_tenant("chat-validation");
    let valid = VibeWindowChatRequest {
        query: "hello".to_string(),
        user: "user-1".to_string(),
        application_workflow: Some("workflow: inline".to_string()),
        ..VibeWindowChatRequest::default()
    };

    let empty_user = workflow_applications_chat_messages(
        State(test_state()),
        Extension(tenant.clone()),
        Json(VibeWindowChatRequest { user: "  ".to_string(), ..valid.clone() }),
    )
    .await
    .expect_err("user is required before runtime");
    assert_eq!(empty_user.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty_user.to_string(), "user is required");

    let empty_query = chat_messages_inner(
        test_state(),
        tenant.clone(),
        VibeWindowChatRequest { query: " ".to_string(), ..valid.clone() },
    )
    .await
    .expect_err("query is required before runtime");
    assert_eq!(empty_query.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty_query.to_string(), "query is required");

    let path_error = workflow_application_chat_messages(
        State(test_state()),
        Extension(tenant.clone()),
        Path(" ".to_string()),
        Json(valid.clone()),
    )
    .await
    .expect_err("blank path uuid must fail");
    assert_eq!(path_error.to_string(), "workflow uuid is required");

    let body_override = workflow_application_chat_messages(
        State(test_state()),
        Extension(tenant),
        Path("app-1".to_string()),
        Json(VibeWindowChatRequest { application_uuid: Some("body-app".to_string()), ..valid }),
    )
    .await
    .expect_err("path uuid must reject body uuid");
    assert!(body_override.to_string().contains("application_uuid"));
}

#[tokio::test]
async fn workflow_handlers_cover_list_and_uuid_hydration_short_circuits() {
    let tenant = test_tenant("list-and-hydrate");
    let Json(created) = workflow_application_create(
        Extension(tenant.clone()),
        Json(WorkflowRecordUpsertBody {
            uuid: Some(Uuid::new_v4().to_string()),
            name: "Listed".to_string(),
            description: String::new(),
            workflow_yaml: "workflow: listed".to_string(),
        }),
    )
    .await
    .expect("create workflow");

    let Json(records) =
        workflow_applications_list(Extension(tenant.clone())).await.expect("list workflows");
    assert!(records.iter().any(|record| record.uuid == created.uuid));

    let mut no_uuid = WorkflowRunRequest::default();
    hydrate_workflow_yaml_from_uuid(
        GlobalDatabase::for_home(tempfile::tempdir().unwrap().path()).unwrap(),
        tenant.clone(),
        &mut no_uuid,
    )
    .await
    .expect("missing uuid should be a no-op");
    assert_eq!(no_uuid.workflow_yaml, None);

    let mut already_hydrated = WorkflowRunRequest {
        workflow_uuid: Some(created.uuid),
        workflow_yaml: Some("workflow: inline".to_string()),
        ..WorkflowRunRequest::default()
    };
    hydrate_workflow_yaml_from_uuid(
        GlobalDatabase::for_home(tempfile::tempdir().unwrap().path()).unwrap(),
        tenant,
        &mut already_hydrated,
    )
    .await
    .expect("existing yaml should skip database lookup");
    assert_eq!(already_hydrated.workflow_yaml.as_deref(), Some("workflow: inline"));
}

#[tokio::test]
async fn workflow_streaming_mode_returns_sse_response_without_blocking_on_run() {
    let response = chat_messages_inner(
        test_state(),
        test_tenant("streaming"),
        VibeWindowChatRequest {
            query: "hello".to_string(),
            application_workflow: Some("not: executable".to_string()),
            response_mode: VibeWindowResponseMode::Streaming,
            user: "user-1".to_string(),
            conversation_id: Some("conv-1".to_string()),
            ..VibeWindowChatRequest::default()
        },
    )
    .await
    .expect("streaming response should be created before run completion");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn workflow_blocking_mode_reports_missing_uuid_from_store() {
    let error = chat_messages_inner(
        test_state(),
        test_tenant("blocking-missing"),
        VibeWindowChatRequest {
            query: "hello".to_string(),
            application_uuid: Some(Uuid::new_v4().to_string()),
            response_mode: VibeWindowResponseMode::Blocking,
            user: "user-1".to_string(),
            ..VibeWindowChatRequest::default()
        },
    )
    .await
    .expect_err("missing uuid should fail during hydration");

    assert_eq!(error.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn workflow_runtime_and_gateway_tool_provider_execute_successful_tool_call() {
    let request = WorkflowRunRequest {
        inputs: BTreeMap::from([(WORKFLOW_INPUT_FULL_ACCESS.to_string(), json!(true))]),
        ..WorkflowRunRequest::default()
    };
    let runtime =
        workflow_runtime(&test_state(), test_tenant("runtime"), &request).expect("runtime");
    assert!(runtime.knowledge_provider.is_some());
    assert!(runtime.tool_provider.is_some());

    let provider = GatewayWorkflowToolProvider {
        ctx: workflow_tool_runtime_context(&WorkflowRunRequest::default()),
    };
    let result = provider
        .call(WorkflowToolRequest {
            provider: "vibewindow".to_string(),
            tool_name: "sleep".to_string(),
            action: None,
            credential_id: None,
            inputs: BTreeMap::from([("duration_ms".to_string(), json!(0))]),
        })
        .await
        .expect("sleep tool should run");

    assert!(result.text.as_deref().unwrap_or_default().contains("Slept for 0 ms"));
    assert!(result.files.is_empty());
}

#[test]
fn workflow_response_to_sse_response_wraps_static_events() {
    let response = workflow_response_to_sse_response(
        WorkflowRunResponse {
            answer: Some("answer".to_string()),
            ..workflow_response("run-static")
        },
        Some("conv-static".to_string()),
    );

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn workflow_handlers_keep_shared_uuid_isolated_by_request_tenant() {
    let uuid = Uuid::new_v4().to_string();
    let tenant_a = TenantScope::new("platform-handler", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-handler", "company-b").expect("tenant B");

    for (tenant, name, yaml) in
        [(tenant_a.clone(), "Tenant A", "tenant: a"), (tenant_b.clone(), "Tenant B", "tenant: b")]
    {
        let Json(created) = workflow_application_create(
            Extension(tenant),
            Json(WorkflowRecordUpsertBody {
                uuid: Some(uuid.clone()),
                name: name.to_string(),
                description: String::new(),
                workflow_yaml: yaml.to_string(),
            }),
        )
        .await
        .expect("create scoped workflow");
        assert_eq!(created.name, name);
    }

    let Json(record_a) = workflow_application_get(Extension(tenant_a.clone()), Path(uuid.clone()))
        .await
        .expect("get tenant A workflow");
    let Json(record_b) = workflow_application_get(Extension(tenant_b.clone()), Path(uuid))
        .await
        .expect("get tenant B workflow");
    assert_eq!(record_a.workflow_yaml, "tenant: a");
    assert_eq!(record_b.workflow_yaml, "tenant: b");

    let Json(updated_a) = workflow_application_update(
        Extension(tenant_a.clone()),
        Path(record_a.uuid.clone()),
        Json(WorkflowRecordUpsertBody {
            uuid: None,
            name: "Tenant A Updated".to_string(),
            description: String::new(),
            workflow_yaml: "tenant: a-updated".to_string(),
        }),
    )
    .await
    .expect("update tenant A workflow");
    assert_eq!(updated_a.workflow_yaml, "tenant: a-updated");

    let Json(deleted_a) =
        workflow_application_delete(Extension(tenant_a.clone()), Path(record_a.uuid))
            .await
            .expect("delete tenant A workflow");
    assert!(deleted_a.deleted);
    let missing_a = workflow_application_get(Extension(tenant_a), Path(record_b.uuid.clone()))
        .await
        .expect_err("tenant A workflow should be deleted");
    assert_eq!(missing_a.status, StatusCode::NOT_FOUND);
    let Json(record_b) = workflow_application_get(Extension(tenant_b), Path(record_b.uuid))
        .await
        .expect("tenant B workflow survives tenant A delete");
    assert_eq!(record_b.workflow_yaml, "tenant: b");
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table() {
    let home = tempfile::tempdir().expect("temp home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    assert_eq!(database.path(), vw_config_types::paths::global_database_path(home.path()));
    assert_workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table(database).await;

    let fixture = crate::global_database::test_support::postgres_test_database_unit!();
    assert_workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table(
        fixture.database().clone(),
    )
    .await;
    fixture.cleanup().expect("clean PostgreSQL test database");
}

async fn assert_workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table(
    database: GlobalDatabase,
) {
    let uuid = Uuid::new_v4().to_string();
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-a", "company-b").expect("tenant B");
    create_legacy_workflow_for_hydration(&database, &uuid);

    let mut missing_request =
        WorkflowRunRequest { workflow_uuid: Some(uuid.clone()), ..WorkflowRunRequest::default() };
    let missing =
        hydrate_workflow_yaml_from_uuid(database.clone(), tenant_a.clone(), &mut missing_request)
            .await
            .expect_err("legacy table must be invisible");
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    super::super::workflow_store::create_record(
        database.clone(),
        tenant_a.clone(),
        WorkflowRecordUpsertBody {
            uuid: Some(uuid.clone()),
            name: "Tenant A".to_string(),
            description: String::new(),
            workflow_yaml: "tenant: a".to_string(),
        },
    )
    .await
    .expect("tenant A workflow");
    super::super::workflow_store::create_record(
        database.clone(),
        tenant_b.clone(),
        WorkflowRecordUpsertBody {
            uuid: Some(uuid.clone()),
            name: "Tenant B".to_string(),
            description: String::new(),
            workflow_yaml: "tenant: b".to_string(),
        },
    )
    .await
    .expect("tenant B workflow with shared uuid");

    let mut tenant_a_request =
        WorkflowRunRequest { workflow_uuid: Some(uuid.clone()), ..WorkflowRunRequest::default() };
    hydrate_workflow_yaml_from_uuid(database.clone(), tenant_a, &mut tenant_a_request)
        .await
        .expect("hydrate tenant A workflow");
    assert_eq!(tenant_a_request.workflow_yaml.as_deref(), Some("tenant: a"));

    let mut tenant_b_request =
        WorkflowRunRequest { workflow_uuid: Some(uuid.clone()), ..WorkflowRunRequest::default() };
    hydrate_workflow_yaml_from_uuid(database.clone(), tenant_b, &mut tenant_b_request)
        .await
        .expect("hydrate tenant B workflow");
    assert_eq!(tenant_b_request.workflow_yaml.as_deref(), Some("tenant: b"));

    let legacy_yaml = legacy_workflow_yaml(&database, &uuid);
    assert_eq!(legacy_yaml, "legacy: true");
}

fn create_legacy_workflow_for_hydration(database: &GlobalDatabase, uuid: &str) {
    match database.provider() {
        DatabaseProvider::Sqlite => {
            let connection = database.connect_sqlite().expect("legacy SQLite database");
            connection
                .execute_batch(
                    "CREATE TABLE dify_workflows (
                         uuid TEXT PRIMARY KEY NOT NULL,
                         name TEXT NOT NULL,
                         description TEXT NOT NULL,
                         workflow_yaml TEXT NOT NULL,
                         created_at_ms INTEGER NOT NULL,
                         updated_at_ms INTEGER NOT NULL
                     );",
                )
                .expect("legacy SQLite schema");
            connection
                .execute(
                    "INSERT INTO dify_workflows \
                     (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms) \
                     VALUES (?1, 'Legacy', '', 'legacy: true', 1, 1)",
                    params![uuid],
                )
                .expect("legacy SQLite workflow");
        }
        DatabaseProvider::Postgres => {
            let mut connection = database.connect_postgres().expect("legacy PostgreSQL database");
            connection
                .batch_execute(
                    "CREATE TABLE dify_workflows (
                         uuid TEXT PRIMARY KEY NOT NULL,
                         name TEXT NOT NULL,
                         description TEXT NOT NULL,
                         workflow_yaml TEXT NOT NULL,
                         created_at_ms BIGINT NOT NULL,
                         updated_at_ms BIGINT NOT NULL
                     );",
                )
                .expect("legacy PostgreSQL schema");
            connection
                .execute(
                    "INSERT INTO dify_workflows
                     (uuid, name, description, workflow_yaml, created_at_ms, updated_at_ms)
                     VALUES ($1, 'Legacy', '', 'legacy: true', 1, 1)",
                    &[&uuid],
                )
                .expect("legacy PostgreSQL workflow");
        }
    }
}

fn legacy_workflow_yaml(database: &GlobalDatabase, uuid: &str) -> String {
    match database.provider() {
        DatabaseProvider::Sqlite => database
            .connect_sqlite()
            .expect("legacy SQLite table connection")
            .query_row(
                "SELECT workflow_yaml FROM dify_workflows WHERE uuid = ?1",
                params![uuid],
                |row| row.get(0),
            )
            .expect("legacy SQLite row remains"),
        DatabaseProvider::Postgres => database
            .connect_postgres()
            .expect("legacy PostgreSQL table connection")
            .query_one("SELECT workflow_yaml FROM dify_workflows WHERE uuid = $1", &[&uuid])
            .expect("legacy PostgreSQL row remains")
            .get(0),
    }
}

#[test]
fn workflow_runtime_inputs_enable_full_access_without_start_input_leak() {
    let mut request = WorkflowRunRequest {
        inputs: BTreeMap::from([
            (WORKFLOW_INPUT_FULL_ACCESS.to_string(), json!(true)),
            (WORKFLOW_INPUT_ROOT.to_string(), json!("/tmp/workflow-root")),
            ("query".to_string(), json!("hello")),
        ]),
        ..WorkflowRunRequest::default()
    };

    assert!(workflow_request_full_access(&request));
    assert_eq!(workflow_request_root(&request).as_deref(), Some("/tmp/workflow-root"));

    let ctx = workflow_tool_runtime_context(&request);
    let tool_use_context = ctx.tool_use_context();
    assert!(tool_use_context.full_access_enabled());
    assert!(tool_use_context.bypass_non_cli_approval_for_turn());
    assert_eq!(ctx.root.as_deref(), Some("/tmp/workflow-root"));

    strip_workflow_runtime_inputs(&mut request);
    assert!(!request.inputs.contains_key(WORKFLOW_INPUT_FULL_ACCESS));
    assert!(!request.inputs.contains_key(WORKFLOW_INPUT_ROOT));
    assert_eq!(request.inputs.get("query"), Some(&json!("hello")));
}

#[test]
fn chat_response_maps_workflow_answer_and_conversation() {
    let response = WorkflowRunResponse {
        run_id: "run-1".to_string(),
        status: WorkflowRunStatus::Succeeded,
        answer: None,
        outputs: BTreeMap::from([("answer".to_string(), json!("ok"))]),
        nodes: Vec::new(),
        error: None,
        pause: None,
    };

    let chat = workflow_response_to_chat_response(&response, Some("conv-1".to_string()));

    assert_eq!(chat.task_id, "run-1");
    assert_eq!(chat.conversation_id, "conv-1");
    assert_eq!(chat.answer, "ok");
    assert_eq!(chat.mode, "advanced-chat");
}

#[test]
fn streaming_events_keep_conversation_id() {
    let response = WorkflowRunResponse {
        run_id: "run-1".to_string(),
        status: WorkflowRunStatus::Succeeded,
        answer: Some("done".to_string()),
        outputs: BTreeMap::new(),
        nodes: Vec::new(),
        error: None,
        pause: None,
    };

    let events = workflow_response_to_sse_events(response, Some("conv-1".to_string()));

    assert!(events.iter().any(|event| matches!(
        event,
        VibeWindowSseEvent::Message { conversation_id, .. } if conversation_id == "conv-1"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        VibeWindowSseEvent::MessageEnd { conversation_id, .. } if conversation_id == "conv-1"
    )));
}

#[test]
fn chat_request_to_workflow_request_accepts_uuid_and_strips_max_steps() {
    let body = VibeWindowChatRequest {
        query: "hello".to_string(),
        application_uuid: Some(" app-1 ".to_string()),
        application_workflow: None,
        inputs: BTreeMap::from([
            ("max_steps".to_string(), json!(12)),
            ("topic".to_string(), json!("rust")),
        ]),
        response_mode: VibeWindowResponseMode::Blocking,
        user: "user-1".to_string(),
        conversation_id: None,
        files: None,
        auto_generate_name: None,
    };

    let request = chat_request_to_workflow_request(body).expect("request should convert");

    assert_eq!(request.workflow_uuid.as_deref(), Some("app-1"));
    assert_eq!(request.workflow_yaml, None);
    assert_eq!(request.query.as_deref(), Some("hello"));
    assert_eq!(request.max_steps, 12);
    assert_eq!(request.inputs, BTreeMap::from([("topic".to_string(), json!("rust"))]));
}

#[test]
fn chat_request_to_workflow_request_rejects_legacy_input_fields() {
    for key in ["application_uuid", "application_workflow", "workflow_uuid", "workflow_yaml"] {
        let body = VibeWindowChatRequest {
            query: "hello".to_string(),
            application_uuid: Some("app-1".to_string()),
            inputs: BTreeMap::from([(key.to_string(), json!("legacy"))]),
            user: "user-1".to_string(),
            ..VibeWindowChatRequest::default()
        };

        let error = chat_request_to_workflow_request(body).expect_err("legacy key should fail");

        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
        assert!(error.to_string().contains(key));
    }
}

#[test]
fn chat_request_to_workflow_request_requires_uuid_or_yaml_and_defaults_steps() {
    let missing_workflow = VibeWindowChatRequest {
        query: "hello".to_string(),
        user: "user-1".to_string(),
        ..VibeWindowChatRequest::default()
    };

    let error =
        chat_request_to_workflow_request(missing_workflow).expect_err("workflow is required");

    assert_eq!(error.to_string(), "application_uuid or application_workflow is required");

    let invalid_max_steps = VibeWindowChatRequest {
        query: "hello".to_string(),
        application_workflow: Some("yaml".to_string()),
        inputs: BTreeMap::from([("max_steps".to_string(), json!("many"))]),
        user: "user-1".to_string(),
        ..VibeWindowChatRequest::default()
    };

    let request =
        chat_request_to_workflow_request(invalid_max_steps).expect("invalid max uses default");

    assert_eq!(request.max_steps, 200);
    assert!(!request.inputs.contains_key("max_steps"));
}

#[test]
fn path_workflow_uuid_is_trimmed_and_blocks_body_overrides() {
    assert_eq!(validate_path_workflow_uuid(" app-1 ").expect("valid uuid"), "app-1");
    assert_eq!(
        validate_path_workflow_uuid("   ").expect_err("blank uuid").to_string(),
        "workflow uuid is required"
    );

    let mut body = VibeWindowChatRequest {
        query: "hello".to_string(),
        user: "user-1".to_string(),
        ..VibeWindowChatRequest::default()
    };

    apply_path_chat_workflow_uuid(&mut body, " app-2 ").expect("path uuid should apply");

    assert_eq!(body.application_uuid.as_deref(), Some("app-2"));

    for (field, body) in [
        (
            "application_uuid",
            VibeWindowChatRequest {
                application_uuid: Some("body-app".to_string()),
                ..VibeWindowChatRequest::default()
            },
        ),
        (
            "application_workflow",
            VibeWindowChatRequest {
                application_workflow: Some("yaml".to_string()),
                ..VibeWindowChatRequest::default()
            },
        ),
        (
            "inputs.workflow_yaml",
            VibeWindowChatRequest {
                inputs: BTreeMap::from([("workflow_yaml".to_string(), json!("yaml"))]),
                ..VibeWindowChatRequest::default()
            },
        ),
    ] {
        let mut body = body;
        let error =
            apply_path_chat_workflow_uuid(&mut body, "app-1").expect_err("override should fail");

        assert!(error.to_string().contains(field));
    }
}

#[test]
fn path_workflow_uuid_rejects_legacy_input_aliases_and_allows_blank_inline_yaml() {
    for key in ["application_uuid", "workflow_uuid", "application_workflow"] {
        let mut body = VibeWindowChatRequest {
            inputs: BTreeMap::from([(key.to_string(), json!("legacy"))]),
            ..VibeWindowChatRequest::default()
        };

        let error = apply_path_chat_workflow_uuid(&mut body, "app-1")
            .expect_err("legacy alias should fail");

        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert!(error.to_string().contains(key));
    }

    let mut body = VibeWindowChatRequest {
        application_workflow: Some("   ".to_string()),
        ..VibeWindowChatRequest::default()
    };

    apply_path_chat_workflow_uuid(&mut body, " app-2 ")
        .expect("blank inline workflow should not override the path uuid");

    assert_eq!(body.application_uuid.as_deref(), Some("app-2"));
}

#[test]
fn optional_string_normalization_drops_blank_values() {
    assert_eq!(normalized_optional_string(Some(" value ".to_string())).as_deref(), Some("value"));
    assert_eq!(normalized_optional_string(Some("   ".to_string())), None);
    assert_eq!(normalized_optional_string(None), None);
}

#[test]
fn runtime_inputs_default_to_restricted_context() {
    let request = WorkflowRunRequest {
        inputs: BTreeMap::from([
            (WORKFLOW_INPUT_FULL_ACCESS.to_string(), json!("true")),
            (WORKFLOW_INPUT_ROOT.to_string(), json!("   ")),
        ]),
        ..WorkflowRunRequest::default()
    };

    let ctx = workflow_tool_runtime_context(&request);

    assert!(!workflow_request_full_access(&request));
    assert_eq!(workflow_request_root(&request), None);
    assert!(!ctx.tool_use_context().full_access_enabled());
    assert!(!ctx.tool_use_context().bypass_non_cli_approval_for_turn());
    assert_eq!(ctx.root, None);
}

#[test]
fn workflow_answer_prefers_explicit_answer_then_known_outputs() {
    let explicit = WorkflowRunResponse {
        answer: Some("explicit".to_string()),
        outputs: BTreeMap::from([("answer".to_string(), json!("output answer"))]),
        ..workflow_response("run-1")
    };
    let from_text = WorkflowRunResponse {
        outputs: BTreeMap::from([("text".to_string(), json!("text answer"))]),
        ..workflow_response("run-2")
    };
    let from_result = WorkflowRunResponse {
        outputs: BTreeMap::from([("result".to_string(), json!("result answer"))]),
        ..workflow_response("run-3")
    };

    assert_eq!(workflow_answer(&explicit), "explicit");
    assert_eq!(workflow_answer(&from_text), "text answer");
    assert_eq!(workflow_answer(&from_result), "result answer");
    assert_eq!(workflow_answer(&workflow_response("run-4")), "");
}

#[test]
fn output_preview_uses_known_text_keys_before_json_fallback() {
    let outputs = BTreeMap::from([
        ("answer".to_string(), json!("  answer text  ")),
        ("data".to_string(), json!("data text")),
    ]);
    let json_outputs = BTreeMap::from([("count".to_string(), json!(2))]);

    assert_eq!(workflow_sse_output_preview(&outputs), "answer text");
    assert!(workflow_sse_output_preview(&json_outputs).contains("\"count\": 2"));
}

#[test]
fn compact_outputs_preserve_usage_and_mark_truncation() {
    let long_answer = "a".repeat(super::WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS + 1);
    let outputs = BTreeMap::from([
        ("answer".to_string(), json!(long_answer)),
        ("usage".to_string(), json!({"total_tokens": 9})),
    ]);

    let compacted = compact_sse_node_outputs(&outputs);

    assert_eq!(compacted["truncated"], json!(true));
    assert_eq!(compacted["usage"], json!({"total_tokens": 9}));
    assert_eq!(compacted["answer"], compacted["text"]);
    assert_eq!(compacted["result"], compacted["text"]);
}

#[test]
fn truncation_keeps_short_text_and_splits_long_text() {
    let (short, short_truncated) = truncate_sse_node_output("短文本");
    let long =
        format!("{}{}", "头".repeat(super::WORKFLOW_SSE_NODE_OUTPUT_MAX_CHARS), "尾".repeat(10));

    let (truncated, long_truncated) = truncate_sse_node_output(&long);

    assert_eq!(short, "短文本");
    assert!(!short_truncated);
    assert!(long_truncated);
    assert!(truncated.contains("已截断"));
    assert!(truncated.starts_with('头'));
    assert!(truncated.ends_with('尾'));
}

#[test]
fn node_and_workflow_statuses_match_protocol_values() {
    assert_eq!(workflow_status(&WorkflowRunStatus::Running), "running");
    assert_eq!(workflow_status(&WorkflowRunStatus::Paused), "running");
    assert_eq!(workflow_status(&WorkflowRunStatus::Succeeded), "succeeded");
    assert_eq!(workflow_status(&WorkflowRunStatus::Failed), "failed");
    assert_eq!(node_status(&WorkflowNodeRunStatus::Paused), "running");
    assert_eq!(node_status(&WorkflowNodeRunStatus::Succeeded), "succeeded");
    assert_eq!(node_status(&WorkflowNodeRunStatus::Failed), "failed");
}

#[test]
fn node_data_maps_started_and_finished_payloads() {
    let node = workflow_node(
        "node-1",
        "llm",
        WorkflowNodeRunStatus::Failed,
        BTreeMap::from([("answer".to_string(), json!("node answer"))]),
    );

    let started = node_started_data(&node, 2, 111);
    let finished = node_finished_data(&node, 2, 111);

    assert_eq!(started.id, "node-1-started");
    assert_eq!(started.inputs, Some(json!({"input": "value"})));
    assert_eq!(finished.id, "node-1-finished");
    assert_eq!(finished.status, "failed");
    assert_eq!(finished.error.as_deref(), Some("boom"));
    assert_eq!(finished.elapsed_time, Some(1.5));
    assert_eq!(finished.outputs.expect("outputs")["text"], json!("node answer"));
}

#[test]
fn event_node_started_data_omits_runtime_inputs() {
    let event = WorkflowNodeStartedEvent {
        node_id: "node-1".to_string(),
        node_type: "answer".to_string(),
        title: "Answer".to_string(),
        index: 3,
    };

    let data = node_started_data_from_event(&event, 222);

    assert_eq!(data.id, "node-1-started");
    assert_eq!(data.node_type, "answer");
    assert_eq!(data.title, "Answer");
    assert_eq!(data.index, 3);
    assert_eq!(data.inputs, None);
}

#[test]
fn workflow_started_and_finished_data_match_protocol_shape() {
    let response = WorkflowRunResponse {
        status: WorkflowRunStatus::Failed,
        outputs: BTreeMap::from([("text".to_string(), json!("failed output"))]),
        nodes: vec![
            workflow_node("a", "start", WorkflowNodeRunStatus::Succeeded, BTreeMap::new()),
            workflow_node("b", "end", WorkflowNodeRunStatus::Failed, BTreeMap::new()),
        ],
        error: Some("failed".to_string()),
        ..workflow_response("run-1")
    };

    let started = workflow_started_data("run-1", 10);
    let finished = workflow_finished_data(&response, 10);

    assert_eq!(started.id, "run-1");
    assert_eq!(started.workflow_id, "vibewindow-workflow");
    assert_eq!(started.sequence_number, Some(1));
    assert_eq!(finished.id, "run-1");
    assert_eq!(finished.status, "failed");
    assert_eq!(finished.error.as_deref(), Some("failed"));
    assert_eq!(finished.total_steps, 2);
    assert_eq!(finished.outputs.expect("outputs")["text"], json!("failed output"));
}

#[test]
fn delta_streaming_rules_only_emit_supported_text_once() {
    let final_llm = workflow_delta("final_response", "llm", false);
    let replacement_llm = workflow_delta("final_response", "llm", true);
    let answer = workflow_delta("answer-1", "answer", false);

    assert!(workflow_delta_should_stream_text(&final_llm, false));
    assert!(workflow_delta_should_stream_text(&final_llm, true));
    assert!(!workflow_delta_should_stream_text(&replacement_llm, false));
    assert!(!workflow_delta_should_stream_text(&workflow_delta("other", "llm", false), false));
    assert!(workflow_delta_should_stream_text(&answer, false));
    assert!(!workflow_delta_should_stream_text(&answer, true));
    assert!(!workflow_delta_should_stream_text(&workflow_delta("code", "tool", false), false));
}

#[test]
fn text_chunk_uses_node_text_selector() {
    let event = workflow_delta("answer-1", "answer", false);

    let data = text_chunk_data(&event);

    assert_eq!(data.text, "delta text");
    assert_eq!(data.from_variable_selector, vec!["answer-1", "text"]);
}

#[test]
fn sse_events_include_node_lifecycle_message_and_end() {
    let response = WorkflowRunResponse {
        answer: None,
        outputs: BTreeMap::from([("answer".to_string(), json!("final answer"))]),
        nodes: vec![workflow_node(
            "node-1",
            "answer",
            WorkflowNodeRunStatus::Succeeded,
            BTreeMap::from([("answer".to_string(), json!("node answer"))]),
        )],
        ..workflow_response("run-1")
    };

    let events = workflow_response_to_sse_events(response, None);

    assert_eq!(events.len(), 6);
    assert!(matches!(events[0], VibeWindowSseEvent::WorkflowStarted { .. }));
    assert!(matches!(events[1], VibeWindowSseEvent::NodeStarted { .. }));
    assert!(matches!(events[2], VibeWindowSseEvent::NodeFinished { .. }));
    assert!(matches!(
        &events[3],
        VibeWindowSseEvent::Message { answer, .. } if answer == "final answer"
    ));
    assert!(matches!(events[4], VibeWindowSseEvent::WorkflowFinished { .. }));
    assert!(matches!(events[5], VibeWindowSseEvent::MessageEnd { .. }));
}

#[test]
fn sse_events_skip_empty_message() {
    let events = workflow_response_to_sse_events(workflow_response("run-1"), None);

    assert!(!events.iter().any(|event| matches!(event, VibeWindowSseEvent::Message { .. })));
    assert_eq!(events.len(), 3);
}

#[tokio::test]
async fn sse_context_ignores_node_events_until_run_starts() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_event(&tx, WorkflowRunEvent::NodeStarted(workflow_started_event("node-1")));
    context.send_event(&tx, WorkflowRunEvent::NodeDelta(workflow_delta("answer", "answer", false)));
    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });

    let _ = rx.recv().await.expect("workflow started event");
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn sse_context_emits_node_started_and_finished_after_run_starts() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });
    context.send_event(&tx, WorkflowRunEvent::NodeStarted(workflow_started_event("node-1")));
    context.send_event(
        &tx,
        WorkflowRunEvent::NodeFinished(crate::workflow::WorkflowNodeFinishedEvent {
            node: workflow_node(
                "node-1",
                "answer",
                WorkflowNodeRunStatus::Succeeded,
                BTreeMap::from([("answer".to_string(), json!("done"))]),
            ),
            index: 1,
        }),
    );

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 3);
}

#[tokio::test]
async fn sse_context_streams_progress_delta_then_emits_distinct_finished_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });
    context.send_event(&tx, WorkflowRunEvent::NodeDelta(workflow_delta("answer", "answer", false)));
    context.send_finished(
        &tx,
        WorkflowRunResponse {
            answer: Some("final answer".to_string()),
            ..workflow_response("run-1")
        },
    );

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 5);
}

#[tokio::test]
async fn sse_context_streams_delta_then_suppresses_duplicate_finished_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });
    context.send_event(&tx, WorkflowRunEvent::NodeDelta(workflow_delta("answer", "answer", false)));
    context.send_finished(
        &tx,
        WorkflowRunResponse {
            answer: Some("delta text".to_string()),
            ..workflow_response("run-1")
        },
    );

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 4);
}

#[tokio::test]
async fn sse_context_streams_final_llm_then_suppresses_finished_message() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });
    context.send_event(
        &tx,
        WorkflowRunEvent::NodeDelta(workflow_delta("final_response", "llm", false)),
    );
    context.send_finished(
        &tx,
        WorkflowRunResponse {
            answer: Some("final answer".to_string()),
            ..workflow_response("run-1")
        },
    );

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 4);
}

#[tokio::test]
async fn sse_context_finished_emits_message_when_no_text_was_streamed() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(Some("conv-1".to_string()));

    context.send_finished(
        &tx,
        WorkflowRunResponse {
            answer: Some("final answer".to_string()),
            ..workflow_response("run-1")
        },
    );

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 3);
}

#[tokio::test]
async fn sse_context_error_uses_current_run_or_default_task() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut context = WorkflowSseContext::new(None);

    context.send_error(&tx, 400, "request_error", "bad request");
    context.send_event(&tx, WorkflowRunEvent::WorkflowStarted { run_id: "run-1".to_string() });
    context.send_error(&tx, 500, "run_error", "failed");

    let event_count = drain_sse_count(&mut rx);

    assert_eq!(event_count, 3);
}

#[test]
fn tool_call_error_text_keeps_denied_or_failed_message_only() {
    let denied = ToolCallError::Denied { message: "denied".to_string(), permission_request: None };
    let failed = ToolCallError::Failed("failed".to_string());

    assert_eq!(workflow_tool_call_error_text(denied), "denied");
    assert_eq!(workflow_tool_call_error_text(failed), "failed");
}

fn workflow_response(run_id: &str) -> WorkflowRunResponse {
    WorkflowRunResponse {
        run_id: run_id.to_string(),
        status: WorkflowRunStatus::Succeeded,
        answer: None,
        outputs: BTreeMap::new(),
        nodes: Vec::new(),
        error: None,
        pause: None,
    }
}

fn workflow_node(
    node_id: &str,
    node_type: &str,
    status: WorkflowNodeRunStatus,
    outputs: BTreeMap<String, serde_json::Value>,
) -> WorkflowNodeRunDto {
    WorkflowNodeRunDto {
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        title: "Node".to_string(),
        status,
        inputs: BTreeMap::from([("input".to_string(), json!("value"))]),
        outputs,
        selected_handle: None,
        error: Some("boom".to_string()),
        elapsed_ms: 1500,
    }
}

fn workflow_started_event(node_id: &str) -> WorkflowNodeStartedEvent {
    WorkflowNodeStartedEvent {
        node_id: node_id.to_string(),
        node_type: "answer".to_string(),
        title: "Answer".to_string(),
        index: 1,
    }
}

fn workflow_delta(node_id: &str, node_type: &str, replace: bool) -> WorkflowNodeDeltaEvent {
    WorkflowNodeDeltaEvent {
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        title: "Delta".to_string(),
        index: 1,
        text: "delta text".to_string(),
        replace,
    }
}

fn drain_sse_count(rx: &mut mpsc::UnboundedReceiver<axum::response::sse::Event>) -> usize {
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    count
}

fn test_tenant(suffix: &str) -> TenantScope {
    TenantScope::new("platform-workflow", format!("company-{suffix}")).expect("tenant")
}

fn test_state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.25,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
    }
}
