use serde_json::Value;
use std::{fmt, time::Duration};
use tool_types::ToolCallResult;
use vw_api_tools::PermissionRequestDto;

/// Claude Tools V2 运行时错误。
#[derive(Debug, Clone, PartialEq)]
pub enum ToolCallError {
    /// 调用被权限或安全策略拒绝。
    Denied { message: String, permission_request: Option<PermissionRequestDto> },
    /// 调用失败。
    Failed(String),
}

impl ToolCallError {
    pub fn denied(message: impl Into<String>) -> Self {
        Self::Denied { message: message.into(), permission_request: None }
    }

    pub fn denied_with_permission_request(
        message: impl Into<String>,
        permission_request: PermissionRequestDto,
    ) -> Self {
        Self::Denied { message: message.into(), permission_request: Some(permission_request) }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Denied { message, .. } | Self::Failed(message) => message,
        }
    }

    pub fn permission_request(&self) -> Option<&PermissionRequestDto> {
        match self {
            Self::Denied { permission_request, .. } => permission_request.as_ref(),
            Self::Failed(_) => None,
        }
    }
}

impl fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Denied { message, .. } | Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ToolCallError {}

/// 已执行工具调用的结构化结果。
#[derive(Debug, Clone)]
pub struct ExecutedToolCall {
    /// 规范化后的工具 ID。
    pub tool_name: String,
    /// 最终执行所使用的输入。
    pub input: Value,
    /// 工具的结构化返回值。
    pub result: ToolCallResult,
    /// 工具执行耗时。
    pub duration: Duration,
}

/// 写回对话历史的工具结果条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultHistoryEntry {
    /// 实际执行的工具名称。
    pub tool_name: String,
    /// 原生 tool call id。
    pub tool_call_id: Option<String>,
    /// 面向模型的文本输出。
    pub output: String,
}
