//! 图标库四个固定操作的共享契约，不提供通用 RPC。
//!
//! 列表直接返回 `Vec<CustomIconEntry>`，改名返回 `CustomIconEntry`，
//! 删除成功返回空 HTTP 响应（204）。条目 ID 来自路由路径，不放进改名请求。

use serde::{Deserialize, Serialize};

/// 用户上传的图标；仅包含业务字段，不承诺迁移旧版存储数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomIconEntry {
    pub id: String,
    pub name: String,
    /// 与供应商自定义图标相同的图片 data URI。
    pub data: String,
    /// Unix 毫秒时间，与上游 `createdAt` 单位一致。
    pub created_at: i64,
}

/// 新增图标请求；图片格式、大小及名称由业务层验证。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCustomIconRequest {
    pub name: String,
    pub data: String,
}

/// 重复上传返回原条目，且不更改原名称。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCustomIconResult {
    pub icon: CustomIconEntry,
    pub existed: bool,
}

/// 修改路由指定图标的名称。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameCustomIconRequest {
    pub name: String,
}

/// 本领域稳定错误码；网关分别映射到 400、404、409、503。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAppsErrorCode {
    InvalidInput,
    NotFound,
    Conflict,
    StorageUnavailable,
}

/// 图标 API 专用错误响应，不改变其他 API 的错误结构。
///
/// 不携带错误源或任意详情。调用方必须使用安全的业务消息，禁止将 SQL、
/// 原始载荷、令牌或内部路径填入 `message`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAppsErrorResponse {
    pub code: AgentAppsErrorCode,
    pub message: String,
}

mod catalog;
pub use catalog::*;

#[cfg(test)]
mod catalog_tests;

#[cfg(test)]
mod installations_tests;

mod installations;
pub use installations::*;

mod actions;
pub use actions::*;
#[cfg(test)]
mod actions_tests;

mod preferences;
pub use preferences::*;
#[cfg(test)]
mod preferences_tests;

mod launch;
pub use launch::*;
#[cfg(test)]
mod launch_tests;

mod read;
pub use read::*;

pub mod aggregation;
#[cfg(test)]
mod aggregation_tests;

mod prompts;
pub use prompts::*;
