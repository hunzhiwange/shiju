//! 节点控制网关接口测试模块
//!
//! 本模块包含节点控制（Node Control）功能的集成测试用例，用于验证网关层
//! 对节点控制请求的处理行为。节点控制功能允许外部系统通过 API 对代理节点
//! 进行管理和控制。
//!
//! # 测试覆盖范围
//!
//! - 禁用状态下节点控制请求的处理
//! - 启用状态下节点列表查询的处理
//!
//! # 相关模块
//!
//! - [`super`] - 父模块（网关测试模块）
//! - [`crate::app::agent::gateway::node_control`] - 节点控制业务逻辑实现

use super::*;
use axum::Json;
use axum::body::to_bytes;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

/// 测试节点控制在禁用状态下返回 404 NOT_FOUND
///
/// # 测试场景
///
/// 当网关配置中 `node_control.enabled` 为 `false` 时，所有节点控制请求
/// 应当返回 HTTP 404 NOT_FOUND 状态码，表明该功能未启用。
///
/// # 验证点
///
/// - 请求方法：`node.list`（节点列表查询）
/// - 期望响应：HTTP 404 NOT_FOUND
/// - 配置状态：节点控制功能禁用
///
/// # 实现细节
///
/// 1. 创建使用默认配置的 `AppState`（节点控制默认禁用）
/// 2. 构造一个标准的 `NodeControlRequest` 请求
/// 3. 调用 `handle_node_control` 处理请求
/// 4. 验证响应状态码为 NOT_FOUND
#[tokio::test]
async fn node_control_returns_not_found_when_disabled() {
    // 创建模拟的 Provider 和 Memory 实例
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构造应用状态，使用默认配置（节点控制功能默认禁用）
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
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
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    // 构造节点控制请求，请求方法为 node.list
    let body: Result<Json<NodeControlRequest>, JsonRejection> = Ok(Json(NodeControlRequest {
        method: "node.list".into(),
        node_id: None,
        capability: None,
        arguments: serde_json::Value::Null,
    }));

    // 调用节点控制处理器并获取响应
    let response: Response = crate::app::agent::gateway::node_control::handle_node_control(
        State(state),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    // 验证：当节点控制禁用时，应返回 404 NOT_FOUND
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试节点控制在启用状态下返回节点列表
///
/// # 测试场景
///
/// 当网关配置中 `node_control.enabled` 为 `true` 且配置了允许的节点 ID 列表时，
/// 节点列表查询请求应当成功返回配置的节点信息。
///
/// # 验证点
///
/// - 请求方法：`node.list`（节点列表查询）
/// - 期望响应：HTTP 200 OK
/// - 响应体结构：
///   - `ok`: `true`（请求成功标志）
///   - `method`: `"node.list"`（响应的方法名）
///   - `nodes`: 包含 2 个节点的数组
///
/// # 实现细节
///
/// 1. 创建自定义配置，启用节点控制并设置允许的节点 ID
/// 2. 构造应用状态 `AppState`
/// 3. 发送 `node.list` 请求
/// 4. 验证响应状态码和响应体内容
#[tokio::test]
async fn node_control_list_returns_stub_nodes_when_enabled() {
    // 创建模拟的 Provider 和 Memory 实例
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构造配置：启用节点控制功能，并设置允许的节点 ID 列表
    let mut config = Config::default();
    config.gateway.node_control.enabled = true;
    config.gateway.node_control.allowed_node_ids = vec!["node-1".into(), "node-2".into()];

    // 构造应用状态，使用启用了节点控制的配置
    let state = AppState {
        config: Arc::new(Mutex::new(config)),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
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
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    // 构造节点控制请求，请求方法为 node.list
    let body: Result<Json<NodeControlRequest>, JsonRejection> = Ok(Json(NodeControlRequest {
        method: "node.list".into(),
        node_id: None,
        capability: None,
        arguments: serde_json::Value::Null,
    }));

    // 调用节点控制处理器并获取响应
    let response: Response = crate::app::agent::gateway::node_control::handle_node_control(
        State(state),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    // 验证：当节点控制启用时，应返回 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // 收集响应体并解析为 JSON
    let payload = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();

    // 验证响应体的结构和内容
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["method"], "node.list");
    assert_eq!(parsed["nodes"].as_array().map(|v| v.len()), Some(2));
}
