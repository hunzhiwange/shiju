//! QQ 渠道 Webhook 处理测试模块
//!
//! 本模块包含 QQ 渠道（QQ 频道机器人）Webhook 端点的集成测试用例。
//! 主要测试以下场景：
//!
//! - 未配置 QQ 渠道时，Webhook 端点应返回 404 Not Found
//! - QQ 渠道 Webhook 验证流程（回调和签名挑战机制）
//!
//! ## 测试覆盖
//!
//! | 测试用例 | 覆盖场景 |
//! |---------|---------|
//! | `qq_webhook_returns_not_found_when_not_configured` | QQ 未配置时的错误处理 |
//! | `qq_webhook_validation_returns_signed_challenge` | QQ 平台回调 URL 验证 |
//!
//! ## QQ Webhook 验证机制说明
//!
//! QQ 频道机器人使用回调 URL 验证机制来确认开发者服务器的有效性。
//! 当配置回调 URL 时，QQ 平台会发送一个 `op=13` 的验证请求，
//! 开发者服务器需要返回 `plain_token` 和对应的 `signature`（签名），
//! 签名通过 Bot secret 和 token 计算得出。

use super::*;
use axum::body::Bytes;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

/// 测试：未配置 QQ 渠道时 Webhook 返回 404 错误
///
/// # 测试场景
///
/// 当应用状态中 `qq` 字段为 `None` 且 `qq_webhook_enabled` 为 `false` 时，
/// 任何发送到 QQ Webhook 端点的请求都应返回 `404 Not Found` 状态码。
///
/// # 验证点
///
/// - 响应状态码应为 `StatusCode::NOT_FOUND` (404)
/// - 不应调用任何 Provider 方法
///
/// # 示例
///
/// ```ignore
/// // 当 QQ 渠道未配置时
/// let response = handle_qq_webhook(...).await;
/// assert_eq!(response.status(), StatusCode::NOT_FOUND);
/// ```
#[tokio::test]
async fn qq_webhook_returns_not_found_when_not_configured() {
    // 创建模拟 Provider 和 Memory 实例
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构建应用状态，关键点：qq 字段为 None，qq_webhook_enabled 为 false
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
        qq: None,                  // QQ 渠道未配置
        qq_webhook_enabled: false, // QQ Webhook 未启用
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    // 调用 QQ Webhook 处理器，发送一个模拟的验证请求体
    // op=13 表示这是一个回调 URL 验证请求
    let response: Response = Box::pin(super::handlers::handle_qq_webhook(
        State(state),
        HeaderMap::new(),
        Bytes::from_static(br#"{"op":13,"d":{"plain_token":"p","event_ts":"1"}}"#),
    ))
    .await
    .into_response();

    // 验证：应返回 404 Not Found，因为 QQ 渠道未配置
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试：QQ Webhook 验证请求返回正确签名的挑战响应
///
/// # 测试场景
///
/// 模拟 QQ 平台发送的回调 URL 验证请求（`op=13`），
/// 验证服务器能够正确计算并返回签名挑战响应。
///
/// # 验证点
///
/// - 响应状态码应为 `StatusCode::OK` (200)
/// - 响应 JSON 中应包含正确的 `plain_token`
/// - 响应 JSON 中应包含正确计算的 `signature`
/// - Provider 不应被调用（验证阶段不涉及 AI 模型交互）
///
/// # 签名计算说明
///
/// 签名通过 HMAC-SHA256 算法计算，使用 Bot AppID 和 Secret 作为密钥材料，
/// 对 `plain_token` 和 `event_ts` 进行签名。
///
/// # 示例
///
/// ```ignore
/// // QQ 平台发送验证请求
/// // 请求体: {"op":13,"d":{"plain_token":"Arq0D5A61EgUu4OxUvOp","event_ts":"1725442341"}}
/// // 期望响应:
/// // {
/// //   "plain_token": "Arq0D5A61EgUu4OxUvOp",
/// //   "signature": "87befc99c42c651b3aac..."
/// // }
/// ```
#[tokio::test]
async fn qq_webhook_validation_returns_signed_challenge() {
    // 创建模拟 Provider，用于验证验证阶段不调用 AI 模型
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 创建 QQ 渠道实例，配置 AppID 和 Secret
    // AppID: 11111111
    // Secret: DG5g3B4j9X2KOErG
    // 允许列表: ["*"] 表示接受所有来源的消息
    let qq =
        Arc::new(QQChannel::new("11111111".into(), "DG5g3B4j9X2KOErG".into(), vec!["*".into()]));

    // 构建应用状态，关键点：qq 字段已配置，qq_webhook_enabled 为 true
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
        qq: Some(qq),             // QQ 渠道已配置
        qq_webhook_enabled: true, // QQ Webhook 已启用
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };

    // 构建请求头，包含 X-Bot-Appid 用于验证请求来源
    let mut headers = HeaderMap::new();
    headers.insert("X-Bot-Appid", HeaderValue::from_static("11111111"));

    // 调用 QQ Webhook 处理器
    // 请求体包含 QQ 平台的验证挑战数据:
    // - op: 13 (验证操作码)
    // - d.plain_token: Arq0D5A61EgUu4OxUvOp (明文令牌，需原样返回)
    // - d.event_ts: 1725442341 (事件时间戳)
    let response: Response = Box::pin(super::handlers::handle_qq_webhook(
        State(state),
        headers,
        Bytes::from_static(
            br#"{"op":13,"d":{"plain_token":"Arq0D5A61EgUu4OxUvOp","event_ts":"1725442341"}}"#,
        ),
    ))
    .await
    .into_response();

    // 验证：应返回 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // 收集响应体并解析为 JSON
    let payload = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();

    // 验证：响应中应包含原样的 plain_token
    assert_eq!(parsed["plain_token"], "Arq0D5A61EgUu4OxUvOp");

    // 验证：响应中应包含正确计算的签名
    // 签名是使用配置的 Secret 对 plain_token 和 event_ts 进行 HMAC-SHA256 计算的结果
    assert_eq!(
        parsed["signature"],
        "87befc99c42c651b3aac0278e71ada338433ae26fcb24307bdc5ad38c1adc2d01bcfcadc0842edac85e85205028a1132afe09280305f13aa6909ffc2d652c706"
    );

    // 验证：Provider 不应被调用（验证阶段不涉及 AI 模型交互）
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
}
