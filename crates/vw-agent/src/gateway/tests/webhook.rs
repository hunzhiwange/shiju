//! Webhook 处理器测试模块
//!
//! 本模块包含 webhook 端点的集成测试，覆盖以下功能：
//! - 幂等性处理：确保重复请求不会触发多次 provider 调用
//! - 认证授权：验证公共流量在未配置认证层时被拒绝
//! - 自动保存：验证不同请求使用不同的存储键
//! - 密钥验证：测试 webhook 密钥的各种验证场景
//!
//! 这些测试使用 Mock 组件来隔离测试 webhook 处理逻辑，
//! 不依赖真实的 provider 或 memory 实现。

use super::*;
use axum::Json;
use axum::body::to_bytes;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

/// 测试 webhook 幂等性机制能否正确跳过重复的 provider 调用
///
/// ## 测试场景
/// 使用相同的幂等键发送两次 webhook 请求，验证：
/// - 第一次请求正常处理，provider 被调用
/// - 第二次请求返回幂等响应，provider 不被重复调用
///
/// ## 预期结果
/// - 两次请求都返回 HTTP 200 OK
/// - 第二次响应包含 `{"status": "duplicate", "idempotent": true}`
/// - provider 仅被调用 1 次
#[tokio::test]
async fn webhook_idempotency_skips_duplicate_provider_calls() {
    // 创建 Mock Provider 并克隆引用用于后续调用次数验证
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构建测试用 AppState，包含幂等性存储（5分钟过期，最大1000条记录）
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    };

    // 准备包含幂等键的请求头
    let mut headers = HeaderMap::new();
    headers.insert("X-Idempotency-Key", HeaderValue::from_static("abc-123"));

    // 第一次请求：应该正常处理并调用 provider
    let body = Ok(Json(WebhookBody { message: "hello".into() }));
    let first: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state.clone()),
        test_connect_info(),
        headers.clone(),
        body,
    )
    .await
    .into_response();
    assert_eq!(first.status(), StatusCode::OK);

    // 第二次请求：使用相同的幂等键，应返回幂等响应而不调用 provider
    let body = Ok(Json(WebhookBody { message: "hello".into() }));
    let second: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_connect_info(),
        headers,
        body,
    )
    .await
    .into_response();
    assert_eq!(second.status(), StatusCode::OK);

    // 解析第二次响应体，验证幂等性标记
    let payload = to_bytes(second.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(parsed["status"], "duplicate");
    assert_eq!(parsed["idempotent"], true);

    // 验证 provider 仅被调用一次（第二次请求未触发新调用）
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
}

/// 测试 webhook 在未配置认证层时拒绝公共流量
///
/// ## 测试场景
/// 使用公共连接信息（`test_public_connect_info`）发送 webhook 请求，
/// 且未配置 webhook_secret_hash 或其他认证机制。
///
/// ## 预期结果
/// - 返回 HTTP 401 UNAUTHORIZED
/// - 请求被拒绝，provider 不被调用
///
/// ## 安全考虑
/// 此测试确保在暴露 webhook 端点给公网时，必须配置至少一种认证机制。
#[tokio::test]
async fn webhook_rejects_public_traffic_without_auth_layers() {
    // 创建 Mock Provider（此测试中无需跟踪调用次数）
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl;
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构建 AppState，特别注意：webhook_secret_hash 为 None，表示未配置认证
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    };

    // 使用公共连接信息发送请求（模拟来自公网的流量）
    let body: Result<Json<WebhookBody>, JsonRejection> =
        Ok(Json(WebhookBody { message: "hello".into() }));
    let response: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_public_connect_info(),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    // 验证请求被拒绝（未授权）
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试 webhook 自动保存功能为每个请求生成不同的存储键
///
/// ## 测试场景
/// 启用 auto_save 配置，发送两个不同的 webhook 请求，
/// 使用 TrackingMemory 跟踪实际使用的存储键。
///
/// ## 预期结果
/// - 两个请求都成功处理
/// - 存储了 2 个不同的键
/// - 所有键都以 "webhook_msg_" 前缀开头
/// - provider 被调用 2 次
///
/// ## 目的
/// 确保并发或连续的 webhook 请求不会互相覆盖记忆数据。
#[tokio::test]
async fn webhook_autosave_stores_distinct_keys_per_request() {
    // 创建 Mock Provider 用于验证调用次数
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();

    // 创建 TrackingMemory 用于跟踪存储键的使用情况
    let tracking_impl = Arc::new(TrackingMemory::default());
    let memory: Arc<dyn Memory> = tracking_impl.clone();

    // 构建 AppState，关键配置：auto_save = true
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: true,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    };

    let headers = HeaderMap::new();

    // 第一个请求
    let body1 = Ok(Json(WebhookBody { message: "hello one".into() }));
    let first: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state.clone()),
        test_connect_info(),
        headers.clone(),
        body1,
    )
    .await
    .into_response();
    assert_eq!(first.status(), StatusCode::OK);

    // 第二个请求（内容不同）
    let body2 = Ok(Json(WebhookBody { message: "hello two".into() }));
    let second: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_connect_info(),
        headers,
        body2,
    )
    .await
    .into_response();
    assert_eq!(second.status(), StatusCode::OK);

    // 验证存储了 2 个不同的键
    let keys = tracking_impl.keys.lock().clone();
    assert_eq!(keys.len(), 2);
    assert_ne!(keys[0], keys[1]);

    // 验证键的格式正确
    assert!(keys[0].starts_with("webhook_msg_"));
    assert!(keys[1].starts_with("webhook_msg_"));

    // 验证 provider 被调用 2 次（每个请求一次）
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 2);
}

/// 测试 webhook 密钥哈希的确定性和非空性
///
/// ## 测试场景
/// 对相同的密钥进行两次哈希，对不同的密钥进行哈希。
///
/// ## 预期结果
/// - 相同密钥的哈希值相等
/// - 不同密钥的哈希值不相等
/// - 哈希值长度为 64 字符（SHA-256 的十六进制表示）
///
/// ## 安全考虑
/// 确保密钥哈希函数是确定性的，以便在不同请求间验证密钥。
#[test]
fn webhook_secret_hash_is_deterministic_and_nonempty() {
    // 生成两个不同的测试密钥
    let secret_a = generate_test_secret();
    let secret_b = generate_test_secret();

    // 对相同密钥进行两次哈希
    let one = hash_webhook_secret(&secret_a);
    let two = hash_webhook_secret(&secret_a);

    // 对不同密钥进行哈希
    let other = hash_webhook_secret(&secret_b);

    // 验证确定性：相同输入产生相同输出
    assert_eq!(one, two);

    // 验证不同输入产生不同输出
    assert_ne!(one, other);

    // 验证哈希长度（SHA-256 十六进制表示为 64 字符）
    assert_eq!(one.len(), 64);
}

/// 测试 webhook 密钥验证拒绝缺少密钥头的请求
///
/// ## 测试场景
/// 配置 webhook_secret_hash，但请求中不包含 X-Webhook-Secret 头。
///
/// ## 预期结果
/// - 返回 HTTP 401 UNAUTHORIZED
/// - provider 不被调用
///
/// ## 安全考虑
/// 确保在配置了密钥验证时，缺少密钥头的请求被拒绝。
#[tokio::test]
async fn webhook_secret_hash_rejects_missing_header() {
    // 创建 Mock Provider 并克隆引用用于验证调用次数
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 生成测试密钥
    let secret = generate_test_secret();

    // 构建 AppState，配置 webhook_secret_hash
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: std::sync::Arc::new(parking_lot::Mutex::new(
            crate::app::agent::gateway::state::GatewayChannels {
                webhook_secret_hash: Some(Arc::from(hash_webhook_secret(&secret))),
                whatsapp: None,
                whatsapp_app_secret: None,
                linq: None,
                linq_signing_secret: None,
                nextcloud_talk: None,
                nextcloud_talk_webhook_secret: None,
                wati: None,
                qq: None,
                qq_webhook_enabled: false,
            },
        )),
    };

    // 发送不带 X-Webhook-Secret 头的请求
    let body: Result<Json<WebhookBody>, JsonRejection> =
        Ok(Json(WebhookBody { message: "hello".into() }));
    let response: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_connect_info(),
        HeaderMap::new(),
        body,
    )
    .await
    .into_response();

    // 验证请求被拒绝（未授权）
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 验证 provider 未被调用
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
}

/// 测试 webhook 密钥验证拒绝无效密钥头的请求
///
/// ## 测试场景
/// 配置 webhook_secret_hash，但请求中的 X-Webhook-Secret 头包含错误的密钥。
///
/// ## 预期结果
/// - 返回 HTTP 401 UNAUTHORIZED
/// - provider 不被调用
///
/// ## 安全考虑
/// 确保在配置了密钥验证时，错误的密钥被拒绝。
#[tokio::test]
async fn webhook_secret_hash_rejects_invalid_header() {
    // 创建 Mock Provider 并克隆引用用于验证调用次数
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 生成两个不同的密钥：一个用于配置，一个用于错误的请求
    let valid_secret = generate_test_secret();
    let wrong_secret = generate_test_secret();

    // 构建 AppState，使用 valid_secret 的哈希进行配置
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: std::sync::Arc::new(parking_lot::Mutex::new(
            crate::app::agent::gateway::state::GatewayChannels {
                webhook_secret_hash: Some(Arc::from(hash_webhook_secret(&valid_secret))),
                whatsapp: None,
                whatsapp_app_secret: None,
                linq: None,
                linq_signing_secret: None,
                nextcloud_talk: None,
                nextcloud_talk_webhook_secret: None,
                wati: None,
                qq: None,
                qq_webhook_enabled: false,
            },
        )),
    };

    // 准备包含错误密钥的请求头
    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Secret", HeaderValue::from_str(&wrong_secret).unwrap());

    // 发送请求
    let body: Result<Json<WebhookBody>, JsonRejection> =
        Ok(Json(WebhookBody { message: "hello".into() }));
    let response: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_connect_info(),
        headers,
        body,
    )
    .await
    .into_response();

    // 验证请求被拒绝（未授权）
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 验证 provider 未被调用
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
}

/// 测试 webhook 密钥验证接受有效密钥头的请求
///
/// ## 测试场景
/// 配置 webhook_secret_hash，请求中的 X-Webhook-Secret 头包含正确的密钥。
///
/// ## 预期结果
/// - 返回 HTTP 200 OK
/// - provider 被调用一次
///
/// ## 安全考虑
/// 确保在配置了密钥验证时，正确的密钥被接受并处理请求。
#[tokio::test]
async fn webhook_secret_hash_accepts_valid_header() {
    // 创建 Mock Provider 并克隆引用用于验证调用次数
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 生成测试密钥
    let secret = generate_test_secret();

    // 构建 AppState，使用该密钥的哈希进行配置
    let state = AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: std::sync::Arc::new(parking_lot::Mutex::new(
            crate::app::agent::gateway::state::GatewayChannels {
                webhook_secret_hash: Some(Arc::from(hash_webhook_secret(&secret))),
                whatsapp: None,
                whatsapp_app_secret: None,
                linq: None,
                linq_signing_secret: None,
                nextcloud_talk: None,
                nextcloud_talk_webhook_secret: None,
                wati: None,
                qq: None,
                qq_webhook_enabled: false,
            },
        )),
    };

    // 准备包含正确密钥的请求头
    let mut headers = HeaderMap::new();
    headers.insert("X-Webhook-Secret", HeaderValue::from_str(&secret).unwrap());

    // 发送请求
    let body: Result<Json<WebhookBody>, JsonRejection> =
        Ok(Json(WebhookBody { message: "hello".into() }));
    let response: Response = crate::app::agent::gateway::webhook::handle_webhook(
        State(state),
        test_connect_info(),
        headers,
        body,
    )
    .await
    .into_response();

    // 验证请求成功处理
    assert_eq!(response.status(), StatusCode::OK);

    // 验证 provider 被调用一次
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
}
