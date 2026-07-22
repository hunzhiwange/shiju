//! Qdrant 向量数据库内存后端测试模块
//!
//! 本模块提供针对 `QdrantMemory` 实现的单元测试，验证记忆存储与检索的核心功能：
//! - 记忆类别与字符串表示之间的双向转换
//! - 记忆负载（MemoryPayload）的 JSON 序列化行为
//!
//! # 测试范围
//!
//! 1. **类别映射测试**：验证 `MemoryCategory` 枚举与字符串之间的正确转换
//! 2. **序列化测试**：验证 `MemoryPayload` 结构体的 JSON 序列化输出
//!
//! # 依赖关系
//!
//! - 依赖父模块中的 `QdrantMemory`、`MemoryCategory` 和 `MemoryPayload` 类型
//! - 使用 `serde_json` 进行序列化验证

use super::*;
use crate::app::agent::memory::embeddings::EmbeddingProvider;
use async_trait::async_trait;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::any;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct StaticEmbedding {
    dims: usize,
    vector: Vec<f32>,
}

impl StaticEmbedding {
    fn new(vector: Vec<f32>) -> Self {
        Self { dims: vector.len(), vector }
    }
}

#[async_trait]
impl EmbeddingProvider for StaticEmbedding {
    fn name(&self) -> &str {
        "static"
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| self.vector.clone()).collect())
    }
}

#[derive(Clone)]
struct EmptyEmbedding;

#[async_trait]
impl EmbeddingProvider for EmptyEmbedding {
    fn name(&self) -> &str {
        "empty"
    }

    fn dimensions(&self) -> usize {
        0
    }

    async fn embed(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| Vec::new()).collect())
    }
}

#[derive(Clone)]
struct FailingEmbedding;

#[async_trait]
impl EmbeddingProvider for FailingEmbedding {
    fn name(&self) -> &str {
        "failing"
    }

    fn dimensions(&self) -> usize {
        1
    }

    async fn embed(&self, _texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        anyhow::bail!("embedding unavailable")
    }
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    query: Option<String>,
    api_key: Option<String>,
    content_type: Option<String>,
    body: serde_json::Value,
}

#[derive(Clone)]
struct MockResponse {
    status: StatusCode,
    body: String,
}

struct MockQdrant {
    base_url: String,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: tokio::task::JoinHandle<()>,
}

impl Drop for MockQdrant {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

impl MockQdrant {
    async fn spawn(responses: Vec<MockResponse>) -> Self {
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(MockState {
            responses: Arc::clone(&responses),
            requests: Arc::clone(&requests),
        });
        let app = Router::new().fallback(any(record_qdrant_request)).with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    async fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().await.clone()
    }
}

struct MockState {
    responses: Arc<Mutex<VecDeque<MockResponse>>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

async fn record_qdrant_request(
    State(state): State<Arc<MockState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let method = req.method().to_string();
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let query = uri.query().map(str::to_string);
    let api_key =
        req.headers().get("api-key").and_then(|value| value.to_str().ok()).map(str::to_string);
    let content_type =
        req.headers().get("content-type").and_then(|value| value.to_str().ok()).map(str::to_string);
    let bytes = to_bytes(req.into_body(), 1024 * 1024).await.unwrap();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };

    state.requests.lock().await.push(RecordedRequest {
        method,
        path,
        query,
        api_key,
        content_type,
        body,
    });

    let response = state
        .responses
        .lock()
        .await
        .pop_front()
        .unwrap_or_else(|| MockResponse { status: StatusCode::OK, body: "{}".to_string() });

    (response.status, response.body)
}

fn json_response(body: serde_json::Value) -> MockResponse {
    MockResponse { status: StatusCode::OK, body: body.to_string() }
}

fn status_response(status: StatusCode, body: &str) -> MockResponse {
    MockResponse { status, body: body.to_string() }
}

fn qdrant(base_url: &str, embedder: Arc<dyn EmbeddingProvider>) -> QdrantMemory {
    QdrantMemory::new_lazy(base_url, "memories", Some("secret-key".into()), embedder)
}

fn unused_local_url() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}")
}

/// 测试 `category_to_str` 方法对已知类别的映射
///
/// # 测试场景
///
/// 验证 `QdrantMemory::category_to_str` 方法能够正确处理：
/// - 标准类别：`Core`、`Daily`、`Conversation`
/// - 自定义类别：`Custom(String)` 变体
///
/// # 预期行为
///
/// - `MemoryCategory::Core` 应映射为字符串 `"core"`
/// - `MemoryCategory::Daily` 应映射为字符串 `"daily"`
/// - `MemoryCategory::Conversation` 应映射为字符串 `"conversation"`
/// - `MemoryCategory::Custom("notes")` 应映射为字符串 `"notes"`
#[test]
fn category_to_str_maps_known_categories() {
    // 验证标准类别的字符串映射
    assert_eq!(QdrantMemory::category_to_str(&MemoryCategory::Core), "core");
    assert_eq!(QdrantMemory::category_to_str(&MemoryCategory::Daily), "daily");
    assert_eq!(QdrantMemory::category_to_str(&MemoryCategory::Conversation), "conversation");

    // 验证自定义类别的字符串映射（提取内部值）
    assert_eq!(QdrantMemory::category_to_str(&MemoryCategory::Custom("notes".into())), "notes");
}

/// 测试 `parse_category` 方法对已知和自定义值的解析
///
/// # 测试场景
///
/// 验证 `QdrantMemory::parse_category` 方法能够正确将字符串反向转换为 `MemoryCategory`：
/// - 标准字符串：`"core"`、`"daily"`、`"conversation"`
/// - 自定义字符串：任何非标准值应转换为 `Custom` 变体
///
/// # 预期行为
///
/// - 字符串 `"core"` 应解析为 `MemoryCategory::Core`
/// - 字符串 `"daily"` 应解析为 `MemoryCategory::Daily`
/// - 字符串 `"conversation"` 应解析为 `MemoryCategory::Conversation`
/// - 字符串 `"custom_notes"` 应解析为 `MemoryCategory::Custom("custom_notes")`
#[test]
fn parse_category_maps_known_and_custom_values() {
    // 验证标准字符串的解析
    assert_eq!(QdrantMemory::parse_category("core"), MemoryCategory::Core);
    assert_eq!(QdrantMemory::parse_category("daily"), MemoryCategory::Daily);
    assert_eq!(QdrantMemory::parse_category("conversation"), MemoryCategory::Conversation);

    // 验证自定义字符串的解析（包装为 Custom 变体）
    assert_eq!(
        QdrantMemory::parse_category("custom_notes"),
        MemoryCategory::Custom("custom_notes".into())
    );
}

/// 测试 `MemoryPayload` 结构体的完整序列化
///
/// # 测试场景
///
/// 验证包含所有字段的 `MemoryPayload` 实例能够正确序列化为 JSON：
/// - `key`: 记忆项的唯一标识
/// - `content`: 记忆内容文本
/// - `category`: 记忆类别字符串
/// - `timestamp`: 时间戳（ISO 8601 格式）
/// - `session_id`: 可选的会话 ID（此处为 `Some` 值）
///
/// # 预期行为
///
/// 序列化后的 JSON 字符串应包含所有字段的值，包括 `session_id` 字段
#[test]
fn memory_payload_serializes_correctly() {
    // 构建测试用的记忆负载实例（包含所有字段）
    let payload = MemoryPayload {
        key: "test_key".into(),
        content: "test content".into(),
        category: "core".into(),
        timestamp: "2026-02-20T00:00:00Z".into(),
        session_id: Some("session-1".into()), // 会话 ID 存在
    };

    // 执行 JSON 序列化
    let json = serde_json::to_string(&payload).unwrap();

    // 验证关键字段在 JSON 输出中存在
    assert!(json.contains("test_key"));
    assert!(json.contains("test content"));
    assert!(json.contains("session-1")); // 包含会话 ID
}

/// 测试 `MemoryPayload` 在 `session_id` 为 `None` 时的序列化行为
///
/// # 测试场景
///
/// 验证当 `session_id` 字段为 `None` 时，序列化后的 JSON 应跳过该字段，
/// 而不是输出 `"session_id": null`。这依赖于 `#[serde(skip_serializing_if = "Option::is_none")]` 属性。
///
/// # 预期行为
///
/// - 当 `session_id` 为 `None` 时，JSON 输出不应包含 `session_id` 字段
/// - 其他字段应正常序列化
///
/// # 技术要点
///
/// 此行为确保了：
/// 1. JSON 输出更加简洁，避免冗余的 `null` 值
/// 2. 与 Qdrant 向量数据库的 payload 格式保持一致
#[test]
fn memory_payload_skips_none_session_id() {
    // 构建测试用的记忆负载实例（session_id 为 None）
    let payload = MemoryPayload {
        key: "test_key".into(),
        content: "test content".into(),
        category: "core".into(),
        timestamp: "2026-02-20T00:00:00Z".into(),
        session_id: None, // 会话 ID 不存在
    };

    // 执行 JSON 序列化
    let json = serde_json::to_string(&payload).unwrap();

    // 验证 session_id 字段在 JSON 输出中不存在
    assert!(!json.contains("session_id"));
}

#[test]
fn new_lazy_trims_base_url_and_request_sets_headers() {
    let memory = qdrant("http://127.0.0.1:6333/", Arc::new(StaticEmbedding::new(vec![1.0])));

    let request = memory.request(reqwest::Method::POST, "/collections").build().unwrap();

    assert_eq!(request.url().as_str(), "http://127.0.0.1:6333/collections");
    assert_eq!(request.headers().get("api-key").unwrap(), "secret-key");
    assert_eq!(request.headers().get("content-type").unwrap(), "application/json");
}

#[test]
fn request_without_api_key_omits_authentication_header() {
    let memory = QdrantMemory::new_lazy(
        "http://127.0.0.1:6333",
        "memories",
        None,
        Arc::new(StaticEmbedding::new(vec![1.0])),
    );

    let request = memory.request(reqwest::Method::GET, "/collections").build().unwrap();

    assert!(request.headers().get("api-key").is_none());
    assert_eq!(request.headers().get("content-type").unwrap(), "application/json");
}

#[tokio::test]
async fn ensure_collection_skips_network_for_zero_dimensional_embedder() {
    let memory = qdrant("http://127.0.0.1:1", Arc::new(EmptyEmbedding));

    memory.ensure_collection().await.unwrap();
}

#[tokio::test]
async fn ensure_collection_short_circuits_when_collection_exists() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({"result": {}}))]).await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.1, 0.2, 0.3])));

    memory.ensure_collection().await.unwrap();

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/collections/memories");
    assert_eq!(requests[0].api_key.as_deref(), Some("secret-key"));
}

#[tokio::test]
async fn ensure_collection_creates_collection_after_404() {
    let server = MockQdrant::spawn(vec![
        status_response(StatusCode::NOT_FOUND, "missing"),
        json_response(serde_json::json!({"result": true})),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.1, 0.2, 0.3])));

    memory.ensure_collection().await.unwrap();

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(requests[1].path, "/collections/memories");
    assert_eq!(requests[1].body["vectors"]["size"], 3);
    assert_eq!(requests[1].body["vectors"]["distance"], "Cosine");
}

#[tokio::test]
async fn ensure_collection_reports_check_and_creation_errors() {
    let server =
        MockQdrant::spawn(vec![status_response(StatusCode::INTERNAL_SERVER_ERROR, "nope")]).await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.1])));
    let err = memory.ensure_collection().await.unwrap_err().to_string();
    assert!(err.contains("Qdrant collection check failed"));

    let server = MockQdrant::spawn(vec![
        status_response(StatusCode::NOT_FOUND, "missing"),
        status_response(StatusCode::BAD_REQUEST, "bad vectors"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.1])));
    let err = memory.ensure_collection().await.unwrap_err().to_string();
    assert!(err.contains("Qdrant collection creation failed"));
}

#[tokio::test]
async fn new_initializes_collection_immediately() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({"result": {}}))]).await;

    let memory = QdrantMemory::new(
        &server.base_url,
        "memories",
        Some("secret-key".into()),
        Arc::new(StaticEmbedding::new(vec![1.0])),
    )
    .await
    .unwrap();

    assert_eq!(memory.name(), "qdrant");
    assert_eq!(server.requests().await.len(), 1);
}

#[tokio::test]
async fn ensure_collection_reports_connection_errors() {
    let memory = qdrant(&unused_local_url(), Arc::new(StaticEmbedding::new(vec![0.1])));

    let err = memory.ensure_collection().await.unwrap_err().to_string();

    assert!(err.contains("Qdrant connection failed"));
}

#[tokio::test]
async fn ensure_initialized_checks_collection_only_once() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({"result": {}}))]).await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.1])));

    memory.ensure_initialized().await.unwrap();
    memory.ensure_initialized().await.unwrap();

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/collections/memories");
}

#[tokio::test]
async fn store_deletes_existing_key_then_upserts_payload() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({"result": {"status": "deleted"}})),
        json_response(serde_json::json!({"result": {"status": "upserted"}})),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.25, 0.75])));

    memory
        .store("favorite_language", "Rust", MemoryCategory::Core, Some("session-1"))
        .await
        .unwrap();

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].path, "/collections/memories/points/delete");
    assert_eq!(requests[1].query.as_deref(), Some("wait=true"));
    assert_eq!(requests[1].body["filter"]["must"][0]["match"]["value"], "favorite_language");
    assert_eq!(requests[2].path, "/collections/memories/points");
    assert_eq!(requests[2].query.as_deref(), Some("wait=true"));
    assert_eq!(requests[2].body["points"][0]["vector"], serde_json::json!([0.25, 0.75]));
    assert_eq!(requests[2].body["points"][0]["payload"]["key"], "favorite_language");
    assert_eq!(requests[2].body["points"][0]["payload"]["session_id"], "session-1");
}

#[tokio::test]
async fn store_rejects_empty_embeddings_before_upsert() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({"result": {}}))]).await;
    let memory =
        QdrantMemory::new_lazy(&server.base_url, "memories", None, Arc::new(EmptyEmbedding));

    let err =
        memory.store("key", "content", MemoryCategory::Daily, None).await.unwrap_err().to_string();

    assert!(err.contains("non-zero dimensional embeddings"));
    assert_eq!(server.requests().await.len(), 0);
}

#[tokio::test]
async fn store_surfaces_upsert_errors() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({"result": {"status": "deleted"}})),
        status_response(StatusCode::INTERNAL_SERVER_ERROR, "upsert down"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.5])));

    let err =
        memory.store("key", "content", MemoryCategory::Core, None).await.unwrap_err().to_string();

    assert!(err.contains("Qdrant upsert failed"));
}

#[tokio::test]
async fn store_propagates_embedding_errors_before_mutating_points() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({"result": {}}))]).await;
    let memory = qdrant(&server.base_url, Arc::new(FailingEmbedding));

    let err =
        memory.store("key", "content", MemoryCategory::Core, None).await.unwrap_err().to_string();

    assert!(err.contains("embedding unavailable"));
    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/collections/memories");
}

#[tokio::test]
async fn store_continues_upsert_when_removing_existing_key_fails() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        status_response(StatusCode::INTERNAL_SERVER_ERROR, "delete down"),
        json_response(serde_json::json!({"result": {"status": "upserted"}})),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.5])));

    memory.store("key", "content", MemoryCategory::Core, None).await.unwrap();

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1].path, "/collections/memories/points/delete");
    assert_eq!(requests[2].path, "/collections/memories/points");
}

#[tokio::test]
async fn recall_searches_with_session_filter_and_maps_payloads() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": [
                {
                    "id": "point-1",
                    "score": 0.9,
                    "payload": {
                        "key": "k1",
                        "content": "content 1",
                        "category": "custom",
                        "timestamp": "2026-01-01T00:00:00Z",
                        "session_id": "session-1"
                    }
                },
                {"id": {"unexpected": true}, "score": 0.1, "payload": {"key": "bad", "content": "bad", "category": "core", "timestamp": "t"}}
            ]
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![0.4, 0.6])));

    let entries = memory.recall("query", 5, Some("session-1")).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "point-1");
    assert_eq!(entries[0].category, MemoryCategory::Custom("custom".into()));
    assert_eq!(entries[0].score, Some(0.9));
    let requests = server.requests().await;
    assert_eq!(requests[1].path, "/collections/memories/points/search");
    assert_eq!(requests[1].body["filter"]["must"][0]["match"]["value"], "session-1");
    assert_eq!(requests[1].body["limit"], 5);
}

#[tokio::test]
async fn empty_recall_query_falls_back_to_list() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": {
                "points": [{
                    "id": 42,
                    "payload": {
                        "key": "daily",
                        "content": "note",
                        "category": "daily",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                }]
            }
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    let entries = memory.recall("   ", 10, None).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "42");
    assert_eq!(entries[0].category, MemoryCategory::Daily);
    assert_eq!(server.requests().await[1].path, "/collections/memories/points/scroll");
}

#[tokio::test]
async fn empty_recall_embedding_falls_back_to_filtered_list() {
    let server = MockQdrant::spawn(vec![json_response(serde_json::json!({
        "result": {"points": []}
    }))])
    .await;
    let memory =
        QdrantMemory::new_lazy(&server.base_url, "memories", None, Arc::new(EmptyEmbedding));

    let entries = memory.recall("query", 5, Some("session-1")).await.unwrap();

    assert!(entries.is_empty());
    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/collections/memories/points/scroll");
    assert_eq!(requests[0].body["filter"]["must"][0]["match"]["value"], "session-1");
}

#[tokio::test]
async fn recall_skips_missing_payload_and_maps_numeric_point_ids() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": [
                {"id": "missing-payload", "score": 0.1},
                {
                    "id": 23,
                    "score": 0.8,
                    "payload": {
                        "key": "numeric",
                        "content": "content",
                        "category": "core",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                }
            ]
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    let entries = memory.recall("query", 10, None).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "23");
    assert_eq!(entries[0].score, Some(0.8));
}

#[tokio::test]
async fn get_returns_none_for_empty_or_payloadless_results() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({"result": {"points": []}})),
        json_response(serde_json::json!({
            "result": {"points": [{"id": "payloadless"}]}
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    assert!(memory.get("missing").await.unwrap().is_none());
    assert!(memory.get("payloadless").await.unwrap().is_none());
}

#[tokio::test]
async fn get_maps_numeric_ids_and_rejects_non_scalar_ids() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": {
                "points": [{
                    "id": 41,
                    "payload": {
                        "key": "numeric",
                        "content": "content",
                        "category": "daily",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                }]
            }
        })),
        json_response(serde_json::json!({
            "result": {
                "points": [{
                    "id": {"unexpected": true},
                    "payload": {
                        "key": "invalid",
                        "content": "content",
                        "category": "core",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                }]
            }
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    let numeric = memory.get("numeric").await.unwrap().unwrap();
    let invalid = memory.get("invalid").await.unwrap();

    assert_eq!(numeric.id, "41");
    assert_eq!(numeric.category, MemoryCategory::Daily);
    assert!(invalid.is_none());
}

#[tokio::test]
async fn get_list_forget_count_and_health_use_expected_endpoints() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": {
                "points": [{
                    "id": "point-get",
                    "payload": {
                        "key": "key",
                        "content": "content",
                        "category": "conversation",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                }]
            }
        })),
        json_response(serde_json::json!({
            "result": {
                "points": [{
                    "id": "point-list",
                    "payload": {
                        "key": "key2",
                        "content": "content2",
                        "category": "core",
                        "timestamp": "2026-01-02T00:00:00Z",
                        "session_id": "session-2"
                    }
                }]
            }
        })),
        json_response(serde_json::json!({"result": {"status": "deleted"}})),
        json_response(serde_json::json!({"result": {"points_count": 17}})),
        status_response(StatusCode::OK, "{}"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    let entry = memory.get("key").await.unwrap().unwrap();
    assert_eq!(entry.category, MemoryCategory::Conversation);
    let listed = memory.list(Some(&MemoryCategory::Core), Some("session-2")).await.unwrap();
    assert_eq!(listed[0].session_id.as_deref(), Some("session-2"));
    assert!(memory.forget("key").await.unwrap());
    assert_eq!(memory.count().await.unwrap(), 17);
    assert!(memory.health_check().await);

    let requests = server.requests().await;
    assert_eq!(requests[1].body["filter"]["must"][0]["key"], "key");
    assert_eq!(requests[2].body["filter"]["must"][0]["match"]["value"], "core");
    assert_eq!(requests[2].body["filter"]["must"][1]["match"]["value"], "session-2");
    assert_eq!(requests[5].path, "/");
}

#[tokio::test]
async fn list_skips_points_without_usable_payload_or_id() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({
            "result": {
                "points": [
                    {"id": "missing-payload"},
                    {"id": {"bad": true}, "payload": {"key": "bad", "content": "bad", "category": "core", "timestamp": "t"}},
                    {"id": 7, "payload": {"key": "ok", "content": "ok", "category": "core", "timestamp": "t"}}
                ]
            }
        })),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));

    let entries = memory.list(None, None).await.unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "7");
}

#[tokio::test]
async fn count_defaults_missing_points_count_to_zero_and_reports_errors() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        json_response(serde_json::json!({"result": {}})),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    assert_eq!(memory.count().await.unwrap(), 0);

    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        status_response(StatusCode::INTERNAL_SERVER_ERROR, "count down"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    let err = memory.count().await.unwrap_err().to_string();
    assert!(err.contains("Qdrant collection info failed"));
}

#[tokio::test]
async fn qdrant_operations_surface_http_errors() {
    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        status_response(StatusCode::BAD_GATEWAY, "search down"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    let err = memory.recall("query", 1, None).await.unwrap_err().to_string();
    assert!(err.contains("Qdrant search failed"));

    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        status_response(StatusCode::BAD_GATEWAY, "scroll down"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    let err = memory.get("key").await.unwrap_err().to_string();
    assert!(err.contains("Qdrant scroll failed"));

    let server = MockQdrant::spawn(vec![
        json_response(serde_json::json!({"result": {}})),
        status_response(StatusCode::BAD_GATEWAY, "delete down"),
    ])
    .await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    let err = memory.forget("key").await.unwrap_err().to_string();
    assert!(err.contains("Qdrant delete failed"));
}

#[tokio::test]
async fn health_check_returns_false_for_http_and_connection_failures() {
    let server =
        MockQdrant::spawn(vec![status_response(StatusCode::SERVICE_UNAVAILABLE, "down")]).await;
    let memory = qdrant(&server.base_url, Arc::new(StaticEmbedding::new(vec![1.0])));
    assert!(!memory.health_check().await);

    let memory = qdrant(&unused_local_url(), Arc::new(StaticEmbedding::new(vec![1.0])));
    assert!(!memory.health_check().await);
}
