//! # 网关中间件模块
//!
//! 本模块提供 HTTP 请求的中间件处理功能，用于网关层的安全验证和跨域控制。
//!
//! ## 功能
//!
//! - **CORS 源验证**：检查请求来源是否在白名单或默认允许列表中
//! - **基本认证**：基于用户名密码的 HTTP Basic 认证中间件
//!
//! ## 使用场景
//!
//! 这些中间件用于保护 API 端点，确保只有经过授权的客户端才能访问服务。
//! 主要用于：
//! - 验证 Web 客户端的来源合法性
//! - 保护敏感 API 端点免受未授权访问
//!
//! ## 安全考虑
//!
//! - 本地开发环境（localhost/127.0.0.1）默认被允许
//! - Tauri 应用的源地址默认被允许
//! - 生产环境应通过白名单严格限制允许的源

use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::Method;
use axum::http::Request;
use axum::http::header;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use base64::Engine;
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

use crate::app::agent::flag;
use crate::app::agent::gateway::ApiError;

/// 检查 CORS（跨源资源共享）源是否被允许
///
/// 验证给定的 HTTP Origin 头值是否在允许列表中。该方法遵循默认允许
/// 本地开发环境和 Tauri 应用的策略，同时支持通过白名单扩展。
///
/// # 参数
///
/// * `origin` - HTTP Origin 头的值，表示请求的来源
/// * `whitelist` - 允许的源地址白名单列表
///
/// # 返回值
///
/// 返回 `true` 表示该源被允许，返回 `false` 表示该源被拒绝
///
/// # 允许规则
///
/// 以下情况返回 `true`：
/// 1. 源地址以 `http://localhost:` 开头（本地开发服务器）
/// 2. 源地址以 `http://127.0.0.1:` 开头（本地回环地址）
/// 3. 源地址为 Tauri 应用的特殊源：
///    - `tauri://localhost`
///    - `http://tauri.localhost`
///    - `https://tauri.localhost`
/// 4. 源地址在白名单中
///
/// # 示例
///
/// ```ignore
/// use axum::http::HeaderValue;
///
/// let origin = HeaderValue::from_static("http://localhost:3000");
/// let whitelist = vec!["https://example.com".to_string()];
/// assert!(cors_origin_allowed(&origin, &whitelist));
///
/// let origin = HeaderValue::from_static("https://malicious.com");
/// assert!(!cors_origin_allowed(&origin, &whitelist));
/// ```
///
/// # 安全说明
///
/// 在生产环境中，应谨慎配置白名单，避免将敏感 API 暴露给不受信任的源。
pub(crate) fn cors_origin_allowed(origin: &HeaderValue, whitelist: &[String]) -> bool {
    // 尝试将 HeaderValue 转换为字符串，失败则拒绝
    let Ok(s) = origin.to_str() else {
        return false;
    };

    // 默认允许本地开发环境的 localhost 源
    if s.starts_with("http://localhost:") {
        return true;
    }

    // 默认允许本地回环地址的源
    if s.starts_with("http://127.0.0.1:") {
        return true;
    }

    // 默认允许 Tauri 桌面应用的特殊源地址
    // Tauri 应用使用这些特殊协议来标识自身
    if s == "tauri://localhost" || s == "http://tauri.localhost" || s == "https://tauri.localhost" {
        return true;
    }

    // 最后检查是否在用户配置的白名单中
    whitelist.iter().any(|x| x == s)
}

/// HTTP Basic 认证中间件
///
/// 对传入的 HTTP 请求执行 Basic 认证验证。如果请求未通过认证，
/// 将返回 401 Unauthorized 响应并设置 WWW-Authenticate 头。
///
/// # 参数
///
/// * `req` - 传入的 HTTP 请求
/// * `next` - 中间件链中的下一个处理程序
///
/// # 返回值
///
/// 返回 HTTP 响应：
/// - 认证成功或无需认证：继续执行请求，返回下游处理程序的响应
/// - 认证失败：返回 401 Unauthorized 响应
///
/// # 认证逻辑
///
/// 1. OPTIONS 请求（CORS 预检）直接放行，无需认证
/// 2. 如果未配置服务器密码（`VIBEWINDOW_SERVER_PASSWORD`），直接放行
/// 3. 从请求头中提取 Authorization 头
/// 4. 解析 Basic 认证凭据（Base64 编码的 `username:password`）
/// 5. 验证用户名和密码是否匹配
///
/// # 环境变量
///
/// - `VIBEWINDOW_SERVER_PASSWORD`：服务器认证密码（必需，否则跳过认证）
/// - `VIBEWINDOW_SERVER_USERNAME`：服务器认证用户名（可选，默认为 "vibewindow"）
///
/// # 响应头
///
/// 认证失败时，响应将包含：
/// - `WWW-Authenticate: Basic realm="vibewindow"` - 提示客户端需要 Basic 认证
///
/// # 示例
///
/// ```ignore
/// use axum::Router;
/// use axum::middleware::from_fn;
///
/// let app = Router::new()
///     .route("/api/protected", get(protected_handler))
///     .layer(from_fn(basic_auth_middleware));
/// ```
///
/// # 安全说明
///
/// Basic 认证将凭据以 Base64 编码形式传输，建议仅在 HTTPS 下使用。
/// 在生产环境中，应确保使用 TLS 加密传输。
pub(crate) async fn basic_auth_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let password = flag::VIBEWINDOW_SERVER_PASSWORD.as_deref();
    let username = flag::VIBEWINDOW_SERVER_USERNAME.as_deref().unwrap_or("vibewindow");

    basic_auth_middleware_with_credentials(req, next, password, username).await
}

/// 从客户端请求头构造并注入租户作用域。
///
/// 除 CORS 预检外，受管客户端路由必须提供且只能提供一个 UTF-8、非空的平台和公司
/// header。缺少或非法 header 不会回退到默认租户，避免请求越过租户边界。
pub(crate) async fn tenant_scope_middleware(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let platform_id = match required_tenant_header(req.headers(), PLATFORM_ID_HEADER) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let company_id = match required_tenant_header(req.headers(), COMPANY_ID_HEADER) {
        Ok(value) => value,
        Err(error) => return error.into_response(),
    };
    let scope = match TenantScope::new(platform_id, company_id) {
        Ok(scope) => scope,
        Err(error) => return ApiError::bad_request(error.to_string()).into_response(),
    };

    req.extensions_mut().insert(scope);
    next.run(req).await
}

fn required_tenant_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, ApiError> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Err(ApiError::bad_request(format!("missing {name} header")));
    };
    if values.next().is_some() {
        return Err(ApiError::bad_request(format!("{name} header must have a single value")));
    }

    let value = value
        .to_str()
        .map_err(|_| ApiError::bad_request(format!("{name} header must be valid UTF-8")))?;
    if value.trim().is_empty() {
        return Err(ApiError::bad_request(format!("{name} header must not be empty")));
    }
    Ok(value)
}

async fn basic_auth_middleware_with_credentials(
    req: Request<axum::body::Body>,
    next: Next,
    password: Option<&str>,
    username: &str,
) -> Response {
    // OPTIONS 请求是 CORS 预检请求，应直接放行以支持跨域访问
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    // 检查是否配置了密码，未配置则跳过认证（开发模式）
    let Some(password) = password else {
        return next.run(req).await;
    };

    // 执行认证验证链：
    // 1. 获取 Authorization 头
    // 2. 转换为字符串
    // 3. 去除 "Basic " 前缀
    // 4. Base64 解码
    // 5. 转换为 UTF-8 字符串
    // 6. 验证格式是否为 "username:password"
    let authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Basic "))
        .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .is_some_and(|decoded| decoded == format!("{}:{}", username, password));

    // 认证成功，继续处理请求
    if authorized {
        return next.run(req).await;
    }

    // 认证失败，构造 401 Unauthorized 响应
    let mut res = ApiError {
        status: axum::http::StatusCode::UNAUTHORIZED,
        message: "unauthorized".to_string(),
    }
    .into_response();

    // 设置 WWW-Authenticate 头，提示客户端需要进行 Basic 认证
    // 浏览器看到此头后会弹出认证对话框
    res.headers_mut()
        .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Basic realm=\"vibewindow\""));

    res
}

#[cfg(test)]
#[path = "middleware_tests.rs"]
mod middleware_tests;
