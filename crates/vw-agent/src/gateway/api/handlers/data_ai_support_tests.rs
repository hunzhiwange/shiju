use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::http::StatusCode;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use vw_api_types::data::{
    AiDataConnectionKind, AiDataCountMode, AiDataPageDto, AiDataQueryKind, AiDataQueryResponse,
    AiDataReportSourceDto, AiDataSourceMode,
};

#[derive(Default)]
struct ScriptedProvider {
    responses: Mutex<Vec<Result<String, String>>>,
    calls: Mutex<Vec<String>>,
}

impl ScriptedProvider {
    fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().map(Ok).collect()),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn with_results(responses: Vec<Result<&str, &str>>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|result| result.map(str::to_string).map_err(str::to_string))
                    .collect(),
            ),
            calls: Mutex::new(Vec::new()),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for ScriptedProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.lock().unwrap().push(format!(
            "system={} model={model} temperature={temperature} message={message}",
            system_prompt.unwrap_or_default()
        ));
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Ok("fallback".to_string());
        }
        responses.remove(0).map_err(anyhow::Error::msg)
    }
}

fn state(provider: Arc<dyn Provider>) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider,
        model: "test-model".to_string(),
        temperature: 0.1,
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
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn connection(id: &str, kind: AiDataConnectionKind) -> AiDataConnectionDto {
    AiDataConnectionDto {
        id: id.to_string(),
        name: id.to_string(),
        kind,
        description: None,
        enabled: true,
        read_only: true,
        base_url: Some("https://api.example.test".to_string()),
        connection_url: None,
        sqlite_path: None,
        default_path: None,
        auth_token: None,
        headers: BTreeMap::new(),
        schema_hint: None,
        updated_at_ms: 1,
        last_used_ms: None,
    }
}

fn report() -> AiDataReportDto {
    AiDataReportDto {
        id: "report-1".to_string(),
        name: "Report".to_string(),
        slug: "report".to_string(),
        data_source: AiDataSourceMode::Normal,
        default_source_key: Some("source-a".to_string()),
        report_config: json!({"modules": [{"show": false}, {"show": true}]}),
        sources: vec![AiDataReportSourceDto {
            source_key: "source-a".to_string(),
            connection_id: "conn-a".to_string(),
            query_kind: AiDataQueryKind::Sql,
            sql: Some("SELECT 1".to_string()),
            count_sql: None,
            cube_query: None,
            http_method: "GET".to_string(),
            http_path: None,
            http_body: None,
            append_pagination: true,
        }],
        updated_at_ms: 1,
    }
}

#[test]
fn resolve_ai_context_prefers_report_source_then_direct_connection() {
    let connections = vec![connection("conn-a", AiDataConnectionKind::Sqlite)];
    let reports = vec![report()];

    let (resolved_connection, resolved_report, source) = resolve_ai_context(
        &connections,
        &reports,
        &AiDataAiQueryRequest {
            prompt: "how many".to_string(),
            report_id: Some("report".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(resolved_connection.id, "conn-a");
    assert_eq!(resolved_report.unwrap().report_config["modules"].as_array().unwrap().len(), 1);
    assert_eq!(source.unwrap()["source_key"], "source-a");

    let (resolved_connection, resolved_report, source) = resolve_ai_context(
        &connections,
        &reports,
        &AiDataAiQueryRequest {
            prompt: "direct".to_string(),
            connection_id: Some("conn-a".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(resolved_connection.id, "conn-a");
    assert!(resolved_report.is_none());
    assert!(source.is_none());
}

#[test]
fn resolve_ai_context_reports_precise_errors() {
    let connections = vec![connection("other", AiDataConnectionKind::Sqlite)];
    let err = resolve_ai_context(
        &connections,
        &[report()],
        &AiDataAiQueryRequest {
            prompt: "bad report".to_string(),
            report_id: Some("report".to_string()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("不存在的连接"));

    let err = resolve_ai_context(
        &[],
        &[],
        &AiDataAiQueryRequest { prompt: "missing".to_string(), ..Default::default() },
    )
    .unwrap_err();
    assert!(err.message.contains("report_id 或 connection_id"));

    let err = resolve_ai_context(
        &[],
        &[],
        &AiDataAiQueryRequest {
            prompt: "missing".to_string(),
            connection_id: Some("missing".to_string()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.message.contains("连接不存在"));
}

#[test]
fn parse_execution_plan_accepts_plain_and_embedded_json() {
    let plain = parse_execution_plan(
        r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT 1","http_method":"GET"}"#,
    )
    .unwrap();
    assert_eq!(plain.connection_id, "conn-a");

    let embedded = parse_execution_plan(
        r#"模型输出如下：
        {"connection_id":"conn-a","query_kind":"http","http_method":"POST","http_path":"/items"}
        "#,
    )
    .unwrap();
    assert_eq!(embedded.query_kind, AiDataQueryKind::Http);
    assert_eq!(embedded.http_method, "POST");

    let err = parse_execution_plan("no-json").unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("执行计划 JSON"));
}

#[test]
fn parse_execution_plan_skips_json_with_wrong_schema_before_valid_plan() {
    let plan = parse_execution_plan(
        r#"先给出元数据 {"kind":"draft"}，再给出计划
        {"connection_id":"conn-a","query_kind":"sql","sql":"SELECT 1"}"#,
    )
    .unwrap();

    assert_eq!(plan.connection_id, "conn-a");
    assert_eq!(plan.query_kind, AiDataQueryKind::Sql);
    assert_eq!(plan.sql.as_deref(), Some("SELECT 1"));

    let err =
        parse_execution_plan(r#"{"connection_id":"conn-a","query_kind":"unknown"}"#).unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("执行计划 JSON"));
}

#[tokio::test]
async fn plan_and_summary_use_provider_prompts() {
    let provider = Arc::new(ScriptedProvider::new(vec![
        r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT 1"}"#.to_string(),
        "一共有 1 条".to_string(),
    ]));
    let app_state = state(provider.clone());
    let connection = connection("conn-a", AiDataConnectionKind::Sqlite);

    let raw = plan_with_model(
        &app_state,
        &AiDataAiQueryRequest {
            prompt: "count rows".to_string(),
            params: BTreeMap::from([("tenant".to_string(), json!("t1"))]),
            ..Default::default()
        },
        &connection,
        Some(&report()),
        Some(&json!({"source_key": "source-a"})),
        Some(&json!({"tables": []})),
    )
    .await
    .unwrap();
    assert!(raw.contains("SELECT 1"));

    let plan = parse_execution_plan(&raw).unwrap();
    let answer = summarize_result(
        &app_state,
        "count rows",
        &plan,
        &AiDataQueryResponse {
            page: AiDataPageDto {
                per_page: 1,
                current_page: 1,
                total_page: 1,
                total_record: 1,
                from: 1,
                to: 1,
            },
            items: vec![json!({"answer": 1})],
            report_config: None,
            next_cursor: None,
            has_next_page: false,
            debug: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(answer, "一共有 1 条");
    assert_eq!(provider.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn plan_and_summary_map_provider_failures_with_operation_context() {
    let connection = connection("conn-a", AiDataConnectionKind::Sqlite);
    let plan_provider = Arc::new(ScriptedProvider::with_results(vec![Err("planner offline")]));
    let err = plan_with_model(
        &state(plan_provider),
        &AiDataAiQueryRequest { prompt: "count rows".to_string(), ..Default::default() },
        &connection,
        None,
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("AI 规划失败"));
    assert!(err.message.contains("planner offline"));

    let summary_provider = Arc::new(ScriptedProvider::with_results(vec![Err("summary offline")]));
    let plan =
        parse_execution_plan(r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT 1"}"#)
            .unwrap();
    let err = summarize_result(
        &state(summary_provider),
        "count rows",
        &plan,
        &AiDataQueryResponse {
            page: AiDataPageDto {
                per_page: 0,
                current_page: 1,
                total_page: 0,
                total_record: 0,
                from: 0,
                to: 0,
            },
            items: vec![],
            report_config: None,
            next_cursor: None,
            has_next_page: false,
            debug: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("AI 总结失败"));
    assert!(err.message.contains("summary offline"));
}

#[tokio::test]
async fn summarize_result_limits_prompt_sample_to_twenty_items() {
    let provider = Arc::new(ScriptedProvider::new(vec!["summary".to_string()]));
    let plan = parse_execution_plan(
        r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT value FROM answers"}"#,
    )
    .unwrap();
    let items = (0..25).map(|row| json!({"row": row})).collect();

    let answer = summarize_result(
        &state(provider.clone()),
        "summarize rows",
        &plan,
        &AiDataQueryResponse {
            page: AiDataPageDto {
                per_page: 25,
                current_page: 1,
                total_page: 1,
                total_record: 25,
                from: 1,
                to: 25,
            },
            items,
            report_config: None,
            next_cursor: None,
            has_next_page: false,
            debug: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(answer, "summary");
    let calls = provider.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].contains(r#""row": 19"#));
    assert!(!calls[0].contains(r#""row": 20"#));
}

#[tokio::test]
async fn handle_ai_query_executes_sqlite_plan_and_rejects_connection_switching() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ai.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE TABLE answers (value INTEGER); INSERT INTO answers VALUES (42);")
        .unwrap();

    let provider = Arc::new(ScriptedProvider::new(vec![
        r#"{"connection_id":"","query_kind":"sql","sql":"SELECT value FROM answers"}"#.to_string(),
        "答案是 42".to_string(),
    ]));
    let app_state = state(provider);
    let sqlite = AiDataConnectionDto {
        sqlite_path: Some(db_path.to_string_lossy().to_string()),
        base_url: None,
        ..connection("conn-a", AiDataConnectionKind::Sqlite)
    };

    let Json(response) = handle_ai_query(
        State(app_state),
        AiDataSettings::default(),
        vec![sqlite],
        vec![],
        AiDataAiQueryRequest {
            prompt: "answer?".to_string(),
            connection_id: Some("conn-a".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(response.plan.connection_id, "conn-a");
    assert_eq!(response.result.items, vec![json!({"value": 42})]);
    assert_eq!(response.answer.as_deref(), Some("答案是 42"));
    assert!(response.raw_model_response.unwrap().contains("SELECT value"));

    let provider = Arc::new(ScriptedProvider::new(vec![
        r#"{"connection_id":"other","query_kind":"sql","sql":"SELECT 1"}"#.to_string(),
    ]));
    let err = handle_ai_query(
        State(state(provider)),
        AiDataSettings::default(),
        vec![connection("conn-a", AiDataConnectionKind::Sqlite)],
        vec![],
        AiDataAiQueryRequest {
            prompt: "switch".to_string(),
            connection_id: Some("conn-a".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("未授权"));
}

#[tokio::test]
async fn handle_ai_query_keeps_query_result_when_summary_fails() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("summary-failure.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE TABLE answers (value INTEGER); INSERT INTO answers VALUES (7);")
        .unwrap();

    let provider = Arc::new(ScriptedProvider::with_results(vec![
        Ok(r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT value FROM answers"}"#),
        Err("summary unavailable"),
    ]));
    let sqlite = AiDataConnectionDto {
        sqlite_path: Some(db_path.to_string_lossy().to_string()),
        base_url: None,
        ..connection("conn-a", AiDataConnectionKind::Sqlite)
    };

    let Json(response) = handle_ai_query(
        State(state(provider)),
        AiDataSettings::default(),
        vec![sqlite],
        vec![],
        AiDataAiQueryRequest {
            prompt: "answer?".to_string(),
            connection_id: Some("conn-a".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(response.result.items, vec![json!({"value": 7})]);
    assert!(response.answer.is_none());
    assert!(response.raw_model_response.unwrap().contains("SELECT value"));
}

#[tokio::test]
async fn handle_ai_query_maps_runtime_rejection_to_bad_request() {
    let provider = Arc::new(ScriptedProvider::new(vec![
        r#"{"connection_id":"conn-a","query_kind":"sql","sql":"SELECT 1"}"#.to_string(),
    ]));
    let disabled = AiDataConnectionDto {
        enabled: false,
        ..connection("conn-a", AiDataConnectionKind::Sqlite)
    };

    let err = handle_ai_query(
        State(state(provider)),
        AiDataSettings::default(),
        vec![disabled],
        vec![],
        AiDataAiQueryRequest {
            prompt: "query disabled connection".to_string(),
            connection_id: Some("conn-a".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    assert!(err.message.contains("已被禁用"));
}
