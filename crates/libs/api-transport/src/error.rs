//! API 错误响应相关类型。
//!
//! 本模块约定 API 返回错误时的统一外层结构，便于：
//! - 前端统一解析错误码和展示消息
//! - 网关或客户端保留结构化错误详情
//! - 避免不同接口返回彼此不兼容的错误 JSON 形状

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 外层 API 错误响应体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    /// 具体错误详情。
    pub error: ApiErrorDetail,
}

/// 统一的错误详情结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    /// 稳定错误码，便于前后端约定处理逻辑。
    pub code: String,
    /// 人类可读的错误说明。
    pub message: String,
    /// 可选的结构化补充信息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}
