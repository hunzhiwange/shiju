//! Nextcloud Talk Webhook 处理器测试模块
//!
//! 本模块包含针对 Nextcloud Talk 集成的 webhook 处理器的单元测试。
//! 主要测试以下场景：
//!
//! - 当 Nextcloud Talk 未配置时，webhook 请求返回 404 NOT_FOUND
//! - 当签名验证失败时，webhook 请求返回 401 UNAUTHORIZED
//!
//! ## 测试覆盖范围
//!
//! 1. 配置检查逻辑
//! 2. HMAC-SHA256 签名验证机制
//! 3. 错误处理与响应状态码

use super::*;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};

/// 计算 Nextcloud Talk webhook 签名的十六进制字符串
///
/// Nextcloud Talk 使用 HMAC-SHA256 算法对 webhook 请求进行签名验证，
/// 以确保请求的真实性和完整性。
///
/// # 参数
///
/// * `secret` - webhook 密钥，用于生成 HMAC 签名
/// * `random` - 随机值，从请求头 `X-Nextcloud-Talk-Random` 获取
/// * `body` - 请求体内容，原始 JSON 字符串
///
/// # 返回值
///
/// 返回签名的十六进制字符串表示
///
/// # 签名计算流程
///
/// 1. 将 random 和 body 拼接成 payload
/// 2. 使用 secret 作为密钥初始化 HMAC-SHA256
/// 3. 对 payload 进行签名
/// 4. 将签名结果编码为十六进制字符串
fn compute_nextcloud_signature_hex(secret: &str, random: &str, body: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // 拼接随机值和请求体，形成待签名载荷
    let payload = format!("{random}{body}");

    // 使用密钥初始化 HMAC-SHA256 实例
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();

    // 更新待签名数据
    mac.update(payload.as_bytes());

    // 完成签名计算并转换为十六进制字符串
    hex::encode(mac.finalize().into_bytes())
}

/// 测试 Nextcloud Talk 未配置时 webhook 返回 404
///
/// # 测试场景
///
/// 当应用状态 `AppState` 中的 `nextcloud_talk` 字段为 `None` 时，
/// 表示 Nextcloud Talk 集成未启用。此时任何对 webhook 端点的请求
/// 都应返回 `404 NOT_FOUND` 状态码。
///
/// # 验证点
///
/// - 响应状态码为 NOT_FOUND (404)
/// - 不会调用 provider 进行消息处理
#[tokio::test]
async fn nextcloud_talk_webhook_returns_not_found_when_not_configured() {
    // 创建模拟的 Provider 和 Memory 实例
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 构建应用状态，关键点：nextcloud_talk 字段为 None
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
        nextcloud_talk: None, // 未配置 Nextcloud Talk
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

    // 调用 webhook 处理器，传入空请求头和测试请求体
    let response: Response = Box::pin(super::handlers::handle_nextcloud_talk_webhook(
        State(state),
        HeaderMap::new(),
        Bytes::from_static(br#"{"type":"message"}"#),
    ))
    .await
    .into_response();

    // 断言：应返回 404 NOT_FOUND
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试无效签名时 webhook 返回 401
///
/// # 测试场景
///
/// 当 Nextcloud Talk 已配置，但请求携带的签名与预期签名不匹配时，
/// webhook 应拒绝该请求并返回 `401 UNAUTHORIZED` 状态码。
///
/// # 验证点
///
/// - 响应状态码为 UNAUTHORIZED (401)
/// - Provider 的调用次数为 0（消息未被处理）
///
/// # 签名验证机制
///
/// 请求头中的 `X-Nextcloud-Talk-Signature` 应与使用相同密钥、
/// 随机值和请求体重新计算的签名一致。
#[tokio::test]
async fn nextcloud_talk_webhook_rejects_invalid_signature() {
    // 创建模拟的 Provider（用于验证调用次数）和 Memory 实例
    let provider_impl = Arc::new(MockProvider::default());
    let provider: Arc<dyn Provider> = provider_impl.clone();
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    // 创建 Nextcloud Talk 频道实例
    let channel = Arc::new(NextcloudTalkChannel::new(
        "https://cloud.example.com".into(),
        "app-token".into(),
        vec!["*".into()],
    ));

    // 定义测试用的密钥、随机值和请求体
    let secret = "nextcloud-test-secret";
    let random = "seed-value";
    let body = r#"{"type":"message","object":{"token":"room-token"},"message":{"actorType":"users","actorId":"user_a","message":"hello"}}"#;

    // 计算有效签名（本测试不使用，仅作参考）
    let _valid_signature = compute_nextcloud_signature_hex(secret, random, body);
    // 定义无效签名，用于触发签名验证失败
    let invalid_signature = "deadbeef";

    // 构建应用状态，关键点：nextcloud_talk 已配置，webhook 密钥已设置
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
        nextcloud_talk: Some(channel), // 已配置 Nextcloud Talk
        nextcloud_talk_webhook_secret: Some(Arc::from(secret)), // 已设置密钥
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

    // 构建请求头，携带随机值和无效签名
    let mut headers = HeaderMap::new();
    headers.insert("X-Nextcloud-Talk-Random", HeaderValue::from_str(random).unwrap());
    headers.insert("X-Nextcloud-Talk-Signature", HeaderValue::from_str(invalid_signature).unwrap());

    // 调用 webhook 处理器
    let response: Response = Box::pin(super::handlers::handle_nextcloud_talk_webhook(
        State(state),
        headers,
        Bytes::from(body),
    ))
    .await
    .into_response();

    // 断言：应返回 401 UNAUTHORIZED
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    // 断言：Provider 不应被调用（调用次数为 0）
    assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 0);
}
