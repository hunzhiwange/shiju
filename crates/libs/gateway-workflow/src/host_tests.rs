// 覆盖工作流 API 的路由、请求转换、运行时调用与流式响应边界。

// 测试同时覆盖阻塞与 SSE 两种响应模式，以及缺失输入时的短路失败。
use super::server_test_source::*;
use super::{GatewayWorkflowToolProvider, workflow_runtime, workflow_tool_call_error_text, workflow_tool_runtime_context, workflow_request_root};
use crate::app::agent::config::Config;
use crate::app::agent::gateway::state::AppState;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::observability::NoopObserver;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::tools::ToolCallError;
use crate::global_database::GlobalDatabase;
use crate::global_database::config::DatabaseProvider;
use crate::workflow::{
    WorkflowToolProvider,
    WorkflowToolRequest,
};
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
};
use parking_lot::Mutex;
use rusqlite::params;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use uuid::Uuid;
use vw_api_transport::TenantScope;
use vw_api_workflow::{
    WorkflowRecordUpsertBody, WorkflowRunRequest, WorkflowRunResponse, WorkflowRunStatus,
};

// 固定 provider 让处理器测试只观察请求编排，不引入真实模型或网络差异。
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
    let _router = router::<AppState>(RuntimeFactory(workflow_runtime));
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
        Extension(RuntimeFactory(workflow_runtime)),
        Extension(tenant.clone()),
        Json(VibeWindowChatRequest { user: "  ".to_string(), ..valid.clone() }),
    )
    .await
    .expect_err("user is required before runtime");
    assert_eq!(empty_user.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty_user.to_string(), "user is required");

    let empty_query = chat_messages_inner(
        test_state(),
        RuntimeFactory(workflow_runtime),
        tenant.clone(),
        VibeWindowChatRequest { query: " ".to_string(), ..valid.clone() },
    )
    .await
    .expect_err("query is required before runtime");
    assert_eq!(empty_query.status, StatusCode::BAD_REQUEST);
    assert_eq!(empty_query.to_string(), "query is required");

    let path_error = workflow_application_chat_messages(
        State(test_state()),
        Extension(RuntimeFactory(workflow_runtime)),
        Extension(tenant.clone()),
        Path(" ".to_string()),
        Json(valid.clone()),
    )
    .await
    .expect_err("blank path uuid must fail");
    assert_eq!(path_error.to_string(), "workflow uuid is required");

    let body_override = workflow_application_chat_messages(
        State(test_state()),
        Extension(RuntimeFactory(workflow_runtime)),
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
        workflow_applications_list(Extension(tenant.clone()), None).await.expect("list workflows");
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
        RuntimeFactory(workflow_runtime),
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
        RuntimeFactory(workflow_runtime),
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
        tenant: TenantScope::default(),
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

    let Json(record_a) =
        workflow_application_get(Extension(tenant_a.clone()), Path(uuid.clone()), None)
            .await
            .expect("get tenant A workflow");
    let Json(record_b) = workflow_application_get(Extension(tenant_b.clone()), Path(uuid), None)
        .await
        .expect("get tenant B workflow");
    assert_eq!(record_a.workflow_yaml, "tenant: a");
    assert_eq!(record_b.workflow_yaml, "tenant: b");

    let Json(updated_a) = workflow_application_update(
        Extension(tenant_a.clone()),
        Path(record_a.uuid.clone()),
        HeaderMap::new(),
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
    let missing_a =
        workflow_application_get(Extension(tenant_a), Path(record_b.uuid.clone()), None)
            .await
            .expect_err("tenant A workflow should be deleted");
    assert_eq!(missing_a.status, StatusCode::NOT_FOUND);
    let Json(record_b) = workflow_application_get(Extension(tenant_b), Path(record_b.uuid), None)
        .await
        .expect("tenant B workflow survives tenant A delete");
    assert_eq!(record_b.workflow_yaml, "tenant: b");
}

#[tokio::test(flavor = "multi_thread")]
async fn workflow_uuid_hydration_is_isolated_by_tenant_and_ignores_legacy_table() {
    let home = tempfile::tempdir().expect("temp home");
    let database = GlobalDatabase::for_home(home.path()).expect("global database");
    assert_eq!(database.path(), config_runtime_paths::global_database_path(home.path()));
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

    crate::workflow::store::create_record(
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
    crate::workflow::store::create_record(
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

#[tokio::test]
async fn task_pool_actions_explicit_message_requires_authorized_target() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_string_lossy().into_owned();
    let mut request = WorkflowRunRequest {
        inputs: BTreeMap::from([
            (WORKFLOW_INPUT_FULL_ACCESS.to_string(), json!(true)),
            (WORKFLOW_INPUT_ROOT.to_string(), json!(root)),
            ("query".to_string(), json!("hello")),
        ]),
        ..WorkflowRunRequest::default()
    };

    assert_eq!(workflow_request_root(&request).as_deref(), Some(root.as_str()));

    let ctx = workflow_tool_runtime_context(&request);
    let tool_use_context = ctx.tool_use_context();
    assert!(!tool_use_context.full_access_enabled());
    assert!(!tool_use_context.bypass_non_cli_approval_for_turn());
    assert_eq!(ctx.root.as_deref(), Some(root.as_str()));

    let provider = GatewayWorkflowToolProvider { ctx, tenant: TenantScope::default() };
    let result = provider
        .call(WorkflowToolRequest {
            provider: "shiju".into(),
            tool_name: "send_message_to_thread".into(),
            action: None,
            credential_id: None,
            inputs: BTreeMap::from([
                ("thread_id".into(), json!("unavailable")),
                ("prompt".into(), json!("fixture")),
            ]),
        })
        .await;
    assert!(result.is_err(), "an unbound workflow must not send to a thread");
    strip_workflow_runtime_inputs(&mut request);
    assert!(!request.inputs.contains_key(WORKFLOW_INPUT_FULL_ACCESS));
    assert!(!request.inputs.contains_key(WORKFLOW_INPUT_ROOT));
    assert_eq!(request.inputs.get("query"), Some(&json!("hello")));
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

    assert_eq!(workflow_request_root(&request), None);
    assert!(!ctx.tool_use_context().full_access_enabled());
    assert!(!ctx.tool_use_context().bypass_non_cli_approval_for_turn());
    assert_eq!(ctx.root, None);
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
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

// 使用生产 client router；仅替换模型传输，保留认证、租户、YAML 和执行器。
struct TalkingHeadApiProvider {
    barrier: tokio::sync::Barrier,
    calls: Mutex<Vec<(String, String)>>,
    fail_model: bool,
}

#[async_trait::async_trait]
impl Provider for TalkingHeadApiProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("unexpected transport")
    }

    async fn chat(
        &self,
        request: model_types::ChatRequest<'_>,
        model: &str,
        _: f64,
    ) -> anyhow::Result<model_types::ChatResponse> {
        assert_eq!(request.messages.len(), 1);
        assert!(request.tools.is_none());
        let prompt = &request.messages[0].content;
        self.calls.lock().push((model.to_string(), prompt.clone()));
        assert!(!prompt.contains("UNREFERENCED_SENTINEL"));
        assert!(!prompt.contains("HISTORY_SENTINEL"));
        let text = if prompt.contains("Return only a short brief") {
            assert_eq!(model, "deepseek/deepseek-flash");
            if self.fail_model {
                anyhow::bail!("requested model unavailable: deepseek/deepseek-flash")
            }
            "BRIEF_SENTINEL".to_string()
        } else {
            assert!(prompt.contains("BRIEF_SENTINEL"));
            assert!(!prompt.contains("KEYWORD_RESULT_SENTINEL"));
            assert!(!prompt.contains("VISUAL_RESULT_SENTINEL"));
            tokio::time::timeout(Duration::from_secs(5), self.barrier.wait())
                .await
                .expect("both LLM branches must enter before either returns");
            if prompt.contains("keyword visual segments") {
                assert_eq!(model, "deepseek/deepseek-flash");
                json!([{"anchor":"A","start":0,"end":1,"visual_type":"keyword","assets":["KEYWORD_RESULT_SENTINEL"]}]).to_string()
            } else {
                assert_eq!(model, "visual-provider/visual-model");
                json!([
                    {"anchor":"B","start":1,"end":2,"visual_type":"b_roll","assets":["VISUAL_RESULT_SENTINEL"]},
                    {"anchor":"C","start":2,"end":3,"visual_type":"motion_graphic","assets":["diagram"]}
                ]).to_string()
            }
        };
        Ok(model_types::ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }
}

fn talking_head_api_fixture(fail_model: bool) -> (axum::Router, Arc<TalkingHeadApiProvider>) {
    let provider = Arc::new(TalkingHeadApiProvider {
        barrier: tokio::sync::Barrier::new(2),
        calls: Mutex::new(vec![]),
        fail_model,
    });
    let mut state = test_state();
    state.provider = provider.clone();
    state.pairing = Arc::new(PairingGuard::new(true, &["talking-head-test-skey".to_string()]));
    let app = crate::gateway::bootstrap::client_router(
        state.clone(),
        axum::Router::new(),
        axum::Router::new(),
    )
    .with_state(state);
    (app, provider)
}

fn talking_head_api_body(mode: &str) -> serde_json::Value {
    let yaml = include_str!("../../workflow-engine/testdata/workflow/talking_head.yaml");
    // 第二个配置使用独立视觉模型，其余节点保留样例模型。
    let yaml = yaml.replacen(
        "type: llm\n          model: {name: deepseek/deepseek-flash}\n          prompt_template:\n            - role: user\n              text: 'Return only a JSON array of dynamic",
        "type: llm\n          model: {name: visual-provider/visual-model}\n          prompt_template:\n            - role: user\n              text: 'Return only a JSON array of dynamic", 1,
    );
    assert!(yaml.contains("visual-provider/visual-model"));
    json!({
        "application_workflow":yaml,"query":"HISTORY_SENTINEL","user":"test-user",
        "response_mode":mode,"conversation_id":"test-conversation",
        "inputs":{
            "transcript":"A B C","style":"concise","requirements":"three visual types",
            "timestamps":[{"word":"A","start":0,"end":1},{"word":"B","start":1,"end":2},{"word":"C","start":2,"end":3}],
            "unreferenced":"UNREFERENCED_SENTINEL"
        }
    })
}

async fn talking_head_api_request(
    app: axum::Router,
    path: &str,
    body: serde_json::Value,
    company: &str,
    authenticated: bool,
) -> (StatusCode, String) {
    use tower::ServiceExt;
    let mut request = axum::http::Request::post(path)
        .header("content-type", "application/json")
        .header(vw_api_transport::PLATFORM_ID_HEADER, "talking-head-api")
        .header(vw_api_transport::COMPANY_ID_HEADER, company);
    if authenticated {
        request = request.header("authorization", "Bearer talking-head-test-skey");
    }
    tokio::time::timeout(Duration::from_secs(15), async {
        let response = app
            .oneshot(request.body(axum::body::Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    })
    .await
    .expect("API must complete")
}

fn talking_head_api_events(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|line| serde_json::from_str(line).expect("SSE JSON"))
        .collect()
}

#[tokio::test]
async fn talking_head_api_blocking_returns_validated_plan() {
    let (app, provider) = talking_head_api_fixture(false);
    let (status, body) = talking_head_api_request(
        app,
        "/v1/workflow/applications/chat-messages",
        talking_head_api_body("blocking"),
        "blocking",
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let response: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(response["conversation_id"], "test-conversation");
    let answer: serde_json::Value =
        serde_json::from_str(response["answer"].as_str().unwrap()).unwrap();
    assert_eq!(answer["duration"], 3);
    assert_eq!(answer["segments"].as_array().unwrap().len(), 3);
    assert_eq!(provider.calls.lock().len(), 3);
}

#[tokio::test]
async fn talking_head_api_streaming_reports_nodes_and_join() {
    let (app, provider) = talking_head_api_fixture(false);
    let (status, body) = talking_head_api_request(
        app,
        "/v1/workflow/applications/chat-messages",
        talking_head_api_body("streaming"),
        "streaming",
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let events = talking_head_api_events(&body);
    assert_eq!(events.first().unwrap()["event"], "workflow_started");
    assert_eq!(events.last().unwrap()["event"], "message_end");
    let finished: Vec<_> = events.iter().filter(|e| e["event"] == "node_finished").collect();
    assert_eq!(finished.len(), 6);
    for node in &finished {
        assert_eq!(node["data"]["status"], "succeeded");
    }
    let join = events
        .iter()
        .position(|e| e["event"] == "node_started" && e["data"]["node_id"] == "validate")
        .unwrap();
    for id in ["keywords", "visuals"] {
        let branch = events
            .iter()
            .position(|e| e["event"] == "node_finished" && e["data"]["node_id"] == id)
            .unwrap();
        assert!(branch < join);
        assert!(
            events[branch]["data"]["outputs"]["text"].as_str().unwrap().contains("RESULT_SENTINEL")
        );
    }
    let done = events.iter().find(|e| e["event"] == "workflow_finished").unwrap();
    assert_eq!(done["data"]["status"], "succeeded");
    let answer = events
        .iter()
        .find(|e| {
            e["event"] == "text_chunk"
                && e["data"]["from_variable_selector"] == json!(["answer", "text"])
        })
        .map(|e| &e["data"]["text"])
        .or_else(|| events.iter().find(|e| e["event"] == "message").map(|e| &e["answer"]))
        .expect("validated answer");
    let plan: serde_json::Value = serde_json::from_str(answer.as_str().unwrap()).unwrap();
    assert_eq!(plan["segments"].as_array().unwrap().len(), 3);
    assert_eq!(provider.calls.lock().len(), 3);
}

#[tokio::test]
async fn talking_head_api_streaming_preserves_model_error_without_fallback() {
    let (app, provider) = talking_head_api_fixture(true);
    let (status, body) = talking_head_api_request(
        app,
        "/v1/workflow/applications/chat-messages",
        talking_head_api_body("streaming"),
        "model-error",
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let events = talking_head_api_events(&body);
    let failed = events
        .iter()
        .find(|e| e["event"] == "node_finished" && e["data"]["node_id"] == "plan")
        .unwrap();
    assert_eq!(failed["data"]["status"], "failed");
    assert!(failed["data"]["error"].as_str().unwrap().contains("requested model unavailable"));
    let done = events.iter().find(|e| e["event"] == "workflow_finished").unwrap();
    assert_eq!(done["data"]["status"], "failed");
    assert!(done["data"]["error"].as_str().unwrap().contains("requested model unavailable"));
    assert!(
        !events.iter().any(|e| e["event"] == "node_started" && e["data"]["node_id"] == "answer")
    );
    assert_eq!(provider.calls.lock().len(), 1);
}

#[tokio::test]
async fn talking_head_api_saved_sample_requires_auth_and_isolates_tenants() {
    let (app, provider) = talking_head_api_fixture(false);
    let company = Uuid::new_v4().to_string();
    let mut body = talking_head_api_body("blocking");
    let save = json!({"name":"Talking head test", "workflow_yaml":body["application_workflow"]});
    let (status, _) = talking_head_api_request(
        app.clone(),
        "/v1/workflow/applications",
        save.clone(),
        &company,
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, saved) =
        talking_head_api_request(app.clone(), "/v1/workflow/applications", save, &company, true)
            .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let uuid = saved["uuid"].as_str().unwrap();
    body.as_object_mut().unwrap().remove("application_workflow");
    let path = format!("/v1/workflow/applications/{uuid}/chat-messages");
    let (status, _) =
        talking_head_api_request(app.clone(), &path, body.clone(), "other-company", true).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(provider.calls.lock().is_empty());
    let (status, response) = talking_head_api_request(app, &path, body, &company, true).await;
    // 清理仅本测试创建的数据，即使后面的断言失败也不留下样例记录。
    let _ = workflow_application_delete(
        Extension(TenantScope::new("talking-head-api", &company).unwrap()),
        Path(uuid.to_string()),
    )
    .await
    .unwrap();
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(provider.calls.lock().len(), 3);
}

// 显式手工运行；普通回归不会使用真实模型或产生模型费用。
#[tokio::test]
#[ignore = "requires a configured local gateway and incurs real model usage"]
async fn talking_head_api_live_configured_gateway_returns_validated_plans() {
    use futures_util::StreamExt;

    let config = crate::config::get_blocking();
    println!("live upstream host={} port={}", config.gateway.host, config.gateway.port);
    let mut state = test_state();
    state.provider = Arc::new(crate::providers::session_llm::SessionLlmProvider::new());
    state.pairing = Arc::new(PairingGuard::new(true, &["talking-head-test-skey".to_string()]));
    let app = crate::gateway::bootstrap::client_router(
        state.clone(),
        axum::Router::new(),
        axum::Router::new(),
    )
    .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::builder().timeout(Duration::from_secs(120)).build().unwrap();
    for (mode, visual_model) in [
        ("streaming", "deepseek/deepseek-flash"),
        ("blocking", "deepseek/deepseek-flash"),
        ("streaming", "shiju/deepseek-v4.1-flash"),
    ] {
        let mut body = talking_head_api_body(mode);
        body["application_workflow"] = json!(
            body["application_workflow"]
                .as_str()
                .unwrap()
                .replace("visual-provider/visual-model", visual_model)
        );
        body["query"] = json!("生成口播剪辑方案");
        body["inputs"] = json!({
            "transcript":"今天介绍本地工作流。模型并行生成关键词和画面建议。最后校验剪辑方案。",
            "timestamps":[
                {"word":"今天介绍本地工作流。","start":0,"end":3},
                {"word":"模型并行生成关键词和画面建议。","start":3,"end":7},
                {"word":"最后校验剪辑方案。","start":7,"end":10}
            ],
            "style":"简洁科技风", "requirements":"含三种画面类型；仅返回要求的 JSON，不要 Markdown 围栏。"
        });
        let start = std::time::Instant::now();
        let response = client
            .post(format!("http://{address}/v1/workflow/applications/chat-messages"))
            .header("authorization", "Bearer talking-head-test-skey")
            .header(vw_api_transport::PLATFORM_ID_HEADER, "talking-head-api-live")
            .header(vw_api_transport::COMPANY_ID_HEADER, "local-acceptance")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let text = if mode == "streaming" {
            let mut chunks = response.bytes_stream();
            let mut bytes = Vec::new();
            let mut parsed = 0;
            while let Some(chunk) = chunks.next().await {
                bytes.extend_from_slice(&chunk.unwrap());
                while let Some(relative) = bytes[parsed..].iter().position(|byte| *byte == b'\n') {
                    let end = parsed + relative;
                    if let Some(payload) = bytes[parsed..end].strip_prefix(b"data: ") {
                        let event: serde_json::Value = serde_json::from_slice(payload).unwrap();
                        if event["event"] == "node_started" || event["event"] == "node_finished" {
                            println!(
                                "live event={} node={} observed_seconds={:.3}",
                                event["event"],
                                event["data"]["node_id"],
                                start.elapsed().as_secs_f64()
                            );
                        }
                    }
                    parsed = end + 1;
                }
            }
            String::from_utf8(bytes).unwrap()
        } else {
            response.text().await.unwrap()
        };
        if mode == "blocking" {
            let response: serde_json::Value = serde_json::from_str(&text).unwrap();
            let plan: serde_json::Value =
                serde_json::from_str(response["answer"].as_str().unwrap()).unwrap();
            assert_eq!(plan["duration"], 10);
            assert!(plan["segments"].as_array().unwrap().len() >= 3);
        } else {
            let events = talking_head_api_events(&text);
            for event in events.iter().filter(|e| e["event"] == "node_finished") {
                let data = &event["data"];
                println!(
                    "live node={} status={} input_json_chars={} elapsed_seconds={}",
                    data["node_id"],
                    data["status"],
                    data["inputs"].to_string().chars().count(),
                    data["elapsed_time"]
                );
            }
            let done = events.iter().find(|e| e["event"] == "workflow_finished").unwrap();
            assert_eq!(done["data"]["status"], "succeeded", "{}", done["data"]["error"]);
            let nodes: Vec<_> = events.iter().filter(|e| e["event"] == "node_finished").collect();
            assert_eq!(nodes.len(), 6);
            for node in nodes {
                assert_eq!(node["data"]["status"], "succeeded");
            }
            let answer = events
                .iter()
                .find(|e| {
                    e["event"] == "text_chunk"
                        && e["data"]["from_variable_selector"] == json!(["answer", "text"])
                })
                .unwrap();
            let plan: serde_json::Value =
                serde_json::from_str(answer["data"]["text"].as_str().unwrap()).unwrap();
            assert_eq!(plan["duration"], 10);
            assert!(plan["segments"].as_array().unwrap().len() >= 3);
        }
        println!(
            "live mode={mode} plan_model=deepseek/deepseek-flash visual_model={visual_model} elapsed_seconds={:.3} validated=true",
            start.elapsed().as_secs_f64()
        );
    }
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn video_edit_template_uuid_hydration_reads_tenant_record_and_rejects_override() {
    use crate::workflow::templates::{VIDEO_EDIT_TEMPLATE_UUID, VideoEditTemplateSeed};
    let home = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    let tenant = test_tenant("template-hydration");
    let seed = VideoEditTemplateSeed::production().unwrap();
    let snapshot = seed.load(database.clone(), tenant.clone()).await.unwrap();
    let mut request = WorkflowRunRequest {
        workflow_uuid: Some(VIDEO_EDIT_TEMPLATE_UUID.into()),
        ..Default::default()
    };
    hydrate_workflow_yaml_from_uuid(database.clone(), tenant.clone(), &mut request).await.unwrap();
    assert_eq!(request.workflow_yaml.as_deref(), Some(snapshot.record.workflow_yaml.as_str()));
    let error = hydrate_workflow_yaml_from_uuid(database, tenant, &mut request).await.unwrap_err();
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
}
