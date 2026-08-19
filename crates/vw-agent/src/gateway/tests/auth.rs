//! Bearer Token 认证单元测试模块
//!
//! 本模块为 `gateway/api/auth.rs` 中的 Bearer Token 认证功能提供全面的单元测试覆盖。
//!
//! ## 测试覆盖范围
//!
//! 1. **配对禁用场景**
//!    - 无 Authorization 请求头时的行为（应放行）
//!    - 存在 Bearer Token 时的行为（应忽略并放行）
//!
//! 2. **配对启用 + 有效 Token**
//!    - 提供正确 Bearer Token 时的行为（应返回 HTTP 200 OK）
//!
//! 3. **配对启用 + 无效 Token**
//!    - 缺失 Authorization 请求头（应返回 HTTP 401 Unauthorized）
//!    - Token 值不匹配（应返回 HTTP 401 Unauthorized）
//!    - Authorization 请求头格式错误（应返回 HTTP 401）
//!    - Bearer Token 为空字符串（应返回 HTTP 401）
//!
//! ## 测试策略
//!
//! 使用 Mock 对象模拟 Provider 和 Memory，隔离测试认证逻辑，
//! 确保认证中间件在各种边界条件下的正确行为。

use super::*;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};

// ═══════════════════════════════════════════════════════════════════════════════
// 测试辅助函数
// ═══════════════════════════════════════════════════════════════════════════════

/// 创建一个配对功能禁用的 AppState 测试实例
///
/// 该函数构建一个完整的 AppState，其中 `pairing` 字段被设置为禁用状态。
/// 主要用于测试当配对功能关闭时，认证中间件应该无条件放行所有请求。
///
/// # 返回值
///
/// 返回配置了以下特征的 AppState：
/// - 配对功能禁用（pairing.enabled = false）
/// - 使用 MockProvider 和 MockMemory 作为依赖
/// - 使用默认配置
/// - 禁用自动保存
/// - 无 Webhook 密钥
/// - 使用 NoopObserver 进行可观测性记录
///
/// # 示例
///
/// ```ignore
/// let state = make_state_no_pairing();
/// // 此状态下，所有请求都应通过认证
/// assert!(require_auth(&state, &HeaderMap::new()).is_ok());
/// ```
fn make_state_no_pairing() -> AppState {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    AppState {
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
    }
}

/// 创建一个配对功能启用的 AppState 测试实例
///
/// 该函数构建一个完整的 AppState，其中 `pairing` 字段被设置为启用状态，
/// 并配置指定的 token 作为有效的认证凭证。
///
/// # 参数
///
/// * `token` - 用于配对认证的有效 Bearer Token 字符串
///
/// # 返回值
///
/// 返回配置了以下特征的 AppState：
/// - 配对功能启用（pairing.enabled = true）
/// - 使用指定的 token 作为有效认证凭证
/// - 使用 MockProvider 和 MockMemory 作为依赖
/// - 使用默认配置
/// - 禁用自动保存
/// - 无 Webhook 密钥
/// - 使用 NoopObserver 进行可观测性记录
///
/// # 示例
///
/// ```ignore
/// let state = make_state_with_pairing("my-secret-token");
/// // 此状态下，只有提供正确 token 的请求才能通过认证
/// let headers = bearer_headers("my-secret-token");
/// assert!(require_auth(&state, &headers).is_ok());
/// ```
fn make_state_with_pairing(token: &str) -> AppState {
    let provider: Arc<dyn Provider> = Arc::new(MockProvider::default());
    let memory: Arc<dyn Memory> = Arc::new(MockMemory);

    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider,
        model: "test-model".into(),
        temperature: 0.0,
        mem: memory,
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(true, std::slice::from_ref(&token.to_string()))),
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
    }
}

/// 创建包含 Bearer Token 的 HTTP 请求头
///
/// 该辅助函数用于快速构建包含有效 Bearer Token 的 HeaderMap，
/// 便于在测试中模拟携带认证信息的 HTTP 请求。
///
/// # 参数
///
/// * `token` - Bearer Token 的值
///
/// # 返回值
///
/// 返回包含 Authorization 请求头的 HeaderMap，格式为 `Bearer {token}`
///
/// # Panics
///
/// 如果 token 包含非法的 HTTP header 值字符，将触发 panic。
/// 测试中使用的 token 应确保只包含合法字符。
///
/// # 示例
///
/// ```ignore
/// let headers = bearer_headers("my-token-123");
/// // headers 现在包含: Authorization: Bearer my-token-123
/// ```
fn bearer_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let value = format!("Bearer {token}");
    headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&value).unwrap());
    headers
}

// ═══════════════════════════════════════════════════════════════════════════════
// 测试用例：配对功能禁用场景
// ═══════════════════════════════════════════════════════════════════════════════

/// 测试配对功能禁用时，无 Authorization 请求头的请求应该被允许
///
/// # 测试场景
///
/// - 配对功能禁用（pairing.enabled = false）
/// - HTTP 请求不包含 Authorization 请求头
///
/// # 预期结果
///
/// `require_auth` 应返回 `Ok(())`，表示请求被允许通过
#[test]
fn require_auth_pairing_disabled_always_allows() {
    let state = make_state_no_pairing();
    // 即使没有 Authorization 请求头，也应该成功，因为配对功能已禁用
    let headers = HeaderMap::new();
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    assert!(result.is_ok(), "配对禁用时：没有 token 的请求必须成功");
}

/// 测试配对功能禁用时，携带 Bearer Token 的请求也应该被允许
///
/// # 测试场景
///
/// - 配对功能禁用（pairing.enabled = false）
/// - HTTP 请求包含 Bearer Token
///
/// # 预期结果
///
/// `require_auth` 应返回 `Ok(())`，忽略 Bearer Token 并放行请求。
/// 这验证了配对功能禁用时，认证逻辑的短路行为。
#[test]
fn require_auth_pairing_disabled_ignores_bearer_token() {
    let state = make_state_no_pairing();
    // 即使提供了 Bearer token，配对禁用路径也会短路并放行
    let headers = bearer_headers("some-random-token");
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    assert!(result.is_ok(), "配对禁用时：Bearer token 也必须通过");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 测试用例：配对启用 + 有效 Token
// ═══════════════════════════════════════════════════════════════════════════════

/// 测试配对启用时，提供有效的 Bearer Token 应该被接受
///
/// # 测试场景
///
/// - 配对功能启用（pairing.enabled = true）
/// - 提供的 Bearer Token 与配置的 token 完全匹配
///
/// # 预期结果
///
/// `require_auth` 应返回 `Ok(())`，表示认证成功
#[test]
fn require_auth_valid_bearer_token_returns_ok() {
    let token = "valid-secret-token-abc123";
    let state = make_state_with_pairing(token);
    let headers = bearer_headers(token);
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    assert!(result.is_ok(), "有效的 Bearer token 必须被接受");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 测试用例：配对启用 + 缺失/错误 Token → 401
// ═══════════════════════════════════════════════════════════════════════════════

/// 测试配对启用时，缺少 Authorization 请求头应返回 401
///
/// # 测试场景
///
/// - 配对功能启用（pairing.enabled = true）
/// - HTTP 请求不包含 Authorization 请求头
///
/// # 预期结果
///
/// `require_auth` 应返回 `Err((StatusCode::UNAUTHORIZED, JsonBody))`，
/// 其中：
/// - HTTP 状态码为 401 UNAUTHORIZED
/// - 响应体包含 "error" 字段
/// - "error" 字段不为空
#[test]
fn require_auth_missing_token_returns_401() {
    let token = "secret-token-xyz";
    let state = make_state_with_pairing(token);
    let headers = HeaderMap::new(); // 没有 Authorization 请求头
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    let (status, json_body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "缺失 token 必须返回 HTTP 401，实际返回: {status}"
    );
    let body: serde_json::Value = json_body.0;
    assert!(body.get("error").is_some(), "401 响应体必须包含 'error' 字段");
    let msg = body["error"].as_str().unwrap_or("");
    assert!(!msg.is_empty(), "错误消息不能为空");
}

/// 测试配对启用时，提供错误的 Bearer Token 应返回 401
///
/// # 测试场景
///
/// - 配对功能启用（pairing.enabled = true）
/// - 提供的 Bearer Token 与配置的 token 不匹配
///
/// # 预期结果
///
/// `require_auth` 应返回 `Err((StatusCode::UNAUTHORIZED, JsonBody))`，
/// 其中：
/// - HTTP 状态码为 401 UNAUTHORIZED
/// - 响应体包含 "error" 字段
/// - "error" 字段不为空
#[test]
fn require_auth_wrong_token_returns_401() {
    let token = "correct-token";
    let state = make_state_with_pairing(token);
    let headers = bearer_headers("wrong-token");
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    let (status, json_body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "错误的 token 必须返回 HTTP 401，实际返回: {status}"
    );
    let body: serde_json::Value = json_body.0;
    assert!(body.get("error").is_some(), "401 响应体必须包含 'error' 字段");
    let msg = body["error"].as_str().unwrap_or("");
    assert!(!msg.is_empty(), "错误消息不能为空");
}

/// 测试配对启用时，格式错误的 Authorization 请求头应返回 401
///
/// # 测试场景
///
/// - 配对功能启用（pairing.enabled = true）
/// - Authorization 请求头不以 "Bearer " 前缀开头（例如使用 "Token" 代替）
///
/// # 预期结果
///
/// `require_auth` 应返回 `Err((StatusCode::UNAUTHORIZED, JsonBody))`，
/// 将非 Bearer scheme 视为无效认证方式。
#[test]
fn require_auth_malformed_auth_header_returns_401() {
    // 请求头值不以 "Bearer " 开头 —— 视为缺失/无效
    let token = "correct-token";
    let state = make_state_with_pairing(token);
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Token correct-token"));
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    let (status, json_body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "非 Bearer scheme 必须返回 HTTP 401，实际返回: {status}"
    );
    let body: serde_json::Value = json_body.0;
    assert!(body.get("error").is_some(), "401 响应体必须包含 'error' 字段");
}

/// 测试配对启用时，空的 Bearer Token 应返回 401
///
/// # 测试场景
///
/// - 配对功能启用（pairing.enabled = true）
/// - Authorization 请求头包含 "Bearer " 前缀，但 token 部分为空字符串
///
/// # 预期结果
///
/// `require_auth` 应返回 `Err((StatusCode::UNAUTHORIZED, JsonBody))`，
/// 空的 Bearer Token 应被视为无效凭证。
#[test]
fn require_auth_empty_bearer_returns_401() {
    // "Bearer " 前缀存在，但 token 为空字符串
    let token = "real-token";
    let state = make_state_with_pairing(token);
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer "));
    let result = crate::app::agent::gateway::api::auth::require_auth(&state, &headers);
    let (status, json_body) = result.unwrap_err();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "空的 Bearer token 必须返回 HTTP 401，实际返回: {status}"
    );
    let body: serde_json::Value = json_body.0;
    assert!(body.get("error").is_some(), "401 响应体必须包含 'error' 字段");
}
