//! Gateway HTTP API 的统一错误类型。
//!
//! 本模块把内部错误映射成稳定的 HTTP 状态码和 JSON 错误体，避免各个处理器
//! 重复拼装响应，也避免把底层错误结构直接暴露给客户端。

use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;

#[derive(Debug)]
/// Gateway 处理器返回给 HTTP 层的错误。
///
/// `status` 决定响应状态码，`message` 会作为 JSON `error` 字段返回给客户端。
/// 调用方应避免传入密钥、原始令牌或敏感载荷。
pub struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    /// 构造 `400 Bad Request` 错误。
    ///
    /// # 参数
    ///
    /// * `message` - 面向客户端的错误摘要，不应包含敏感信息。
    ///
    /// # 返回值
    ///
    /// 返回可由 Axum 转换为 HTTP 响应的 API 错误。
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: message.into() }
    }

    /// 构造 `404 Not Found` 错误。
    ///
    /// # 参数
    ///
    /// * `message` - 资源不存在或不可见时返回的错误摘要。
    ///
    /// # 返回值
    ///
    /// 返回可由 Axum 转换为 HTTP 响应的 API 错误。
    pub fn not_found(message: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.into() }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self { status: StatusCode::CONFLICT, message: message.into() }
    }

    /// 构造 `500 Internal Server Error` 错误。
    ///
    /// # 参数
    ///
    /// * `message` - 内部失败摘要；调用方需要先脱敏再传入。
    ///
    /// # 返回值
    ///
    /// 返回可由 Axum 转换为 HTTP 响应的 API 错误。
    pub fn internal(message: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: message.into() }
    }

    /// 构造 `501 Not Implemented` 错误。
    ///
    /// 用于对外协议已预留但当前运行时尚未实现的能力。
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_IMPLEMENTED, message: message.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // 统一使用 {"error": "..."}，让桌面端和外部客户端拥有稳定解析路径。
        let body = serde_json::json!({ "error": self.message });
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
