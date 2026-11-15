//! 会话待办项相关类型。
//!
//! 本模块定义代理在执行过程中维护的待办项结构，便于前端展示：
//! - 当前任务拆解出的待办列表
//! - 每个待办的状态与优先级
//! - 单条待办的增量更新请求与响应

use vw_api_transport::common::TimestampMs;
use vw_api_transport::id::{SessionId, TodoId};
use serde::{Deserialize, Serialize};

/// 待办状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// 尚未开始。
    Pending,
    /// 正在进行。
    InProgress,
    /// 已完成。
    Completed,
    /// 已取消。
    Cancelled,
}

/// 待办优先级。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoPriority {
    /// 低优先级。
    Low,
    /// 中优先级。
    Medium,
    /// 高优先级。
    High,
}

/// 单个待办项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoDto {
    /// 待办 ID。
    pub id: TodoId,
    /// 所属会话 ID。
    pub session_id: SessionId,
    /// 待办内容。
    pub content: String,
    /// 当前状态。
    pub status: TodoStatus,
    /// 优先级。
    pub priority: TodoPriority,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 更新时间。
    pub updated_at_ms: TimestampMs,
}

/// 列出待办响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListTodosResponse {
    /// 待办列表。
    pub items: Vec<TodoDto>,
}

/// 更新待办请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UpdateTodoRequest {
    /// 新内容。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// 新状态。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TodoStatus>,
    /// 新优先级。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<TodoPriority>,
}

/// 更新待办响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateTodoResponse {
    /// 更新后的待办。
    pub todo: TodoDto,
}
