//! 通用基础类型，供多个 API 模块复用。
//!
//! 本模块放置不属于单一业务域、但会被多个 API 子模块反复引用的基础结构，主要包括：
//! - 时间表达：[`TimestampMs`]
//! - 通用确认响应：[`OperationAck`]
//! - 分页协议：[`PaginationRequest`] 与 [`PaginatedResponse`]
//! - 可透传的键值映射：[`StringMap`] 与 [`JsonMap`]
//!
//! 这些类型保持足够小和稳定，便于跨 crate 复用。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// 毫秒时间戳的新类型包装，避免直接裸用整数表达时间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimestampMs(pub i64);

/// 表示通用操作是否成功的响应体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationAck {
    /// 操作是否成功。
    pub ok: bool,
    /// 可选的人类可读说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 通用分页请求参数。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginationRequest {
    /// 下一页游标。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// 单页返回条数上限。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// 通用分页响应结构。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// 当前页数据项。
    pub items: Vec<T>,
    /// 下一页游标；为空表示没有更多数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// 字符串键值映射，用于透传环境变量等简单配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct StringMap {
    /// 实际键值对内容。
    #[serde(flatten)]
    pub values: BTreeMap<String, String>,
}

impl StringMap {
    /// 判断映射中是否没有任何键值对。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// 任意 JSON 键值映射，用于承载附加扩展字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct JsonMap {
    /// 实际键值对内容。
    #[serde(flatten)]
    pub values: BTreeMap<String, Value>,
}
