//! Agent 端点测试模块
//!
//! 本模块包含对 gateway agent 端点的集成测试，主要验证认证和授权行为。
//!
//! # 测试范围
//!
//! - Bearer Token 认证机制
//! - 配对（Pairing）启用状态下的访问控制
//! - 未授权请求的拒绝行为

use super::*;
use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

/// 测试配对启用时 agent 端点要求 Bearer Token
///
/// # 测试场景
///
/// 当系统启用了配对（Pairing）功能时，agent 端点必须要求请求携带有效的 Bearer Token。
/// 如果请求未提供认证信息，应返回 401 UNAUTHORIZED 状态码。
///
/// # 验证点
///
/// - 未提供认证令牌的请求应被拒绝
/// - 响应状态码应为 `StatusCode::UNAUTHORIZED` (401)
///
/// # 实现细节
///
/// 1. 创建模拟的 Provider 和 Memory 实现
/// 2. 配置 AppState，启用配对功能并设置已配对令牌
/// 3. 发送不带认证头的请求到 agent 处理器
/// 4. 验证返回状态码为 401
#[tokio::test]
async fn agent_endpoint_requires_bearer_token_when_pairing_enabled() {
    // 创建模拟的 Provider 实现，用于测试
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl;

    // 创建模拟的 Memory 实现
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 定义用于测试的已配对令牌
    let paired_token = "zc_test_token".to_string();

    // 构建应用状态，模拟完整的 gateway 配置
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        webhook_secret_hash: None,
        // 启用配对功能，并设置已配对令牌列表
        pairing: Arc::new(PairingGuard::new(true, std::slice::from_ref(&paired_token))),
        trust_forwarded_headers: false,
        // 配置速率限制器（100 请求/秒的阈值）
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        // 配置幂等性存储（300秒过期，最多1000条记录）
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        // 各通道配置均为 None（本测试不需要）
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        // 使用空操作观察器（不记录日志）
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        // 工具注册表为空（本测试不需要工具）
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    // 构造请求体，包含简单的测试消息
    let body: Result<Json<AgentBody>, JsonRejection> =
        Ok(Json(AgentBody { message: "hello".into() }));

    // 调用 agent 处理器，模拟不带认证头的 HTTP 请求
    let unauthorized: Response = crate::app::agent::gateway::agent::handle_agent(
        State(state),
        test_connect_info(),
        HeaderMap::new(), // 空的头部，不包含认证信息
        body,
    )
    .await
    .into_response();

    // 验证：未授权请求应返回 401 状态码
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
}
