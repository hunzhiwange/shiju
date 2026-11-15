//! 交互式提问与用户回答相关类型。
//!
//! 当代理在执行过程中需要用户确认、输入文本或从多个选项中决策时，
//! 会使用本模块中的类型在前后端之间传递问题与回答结果。
//!
//! 支持的交互形态包括：
//! - 审批确认
//! - 单选或多选决策
//! - 自由文本输入

use crate::common::TimestampMs;
use crate::id::{QuestionId, SessionId};
use serde::{Deserialize, Serialize};

/// 提问类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// 需要用户审批通过或拒绝。
    Approval,
    /// 需要用户从候选项中选择。
    Choice,
    /// 需要用户输入自由文本。
    Input,
}

/// 提问当前状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    /// 尚未回答。
    Pending,
    /// 已完成回答。
    Resolved,
    /// 已超时失效。
    Expired,
}

/// 提问选项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionOptionDto {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 提问详情。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionDto {
    pub id: QuestionId,
    pub session_id: SessionId,
    pub kind: QuestionKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub options: Vec<QuestionOptionDto>,
    #[serde(default)]
    pub multiple: bool,
    pub status: QuestionStatus,
    pub created_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<TimestampMs>,
}

/// 列出问题响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListQuestionsResponse {
    pub items: Vec<QuestionDto>,
}

/// 列出问题请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ListQuestionsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<QuestionStatus>,
}

/// 回复问题请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReplyQuestionRequest {
    #[serde(default)]
    pub selected_option_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// 拒绝问题请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RejectQuestionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 问题解决结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionResolutionDto {
    pub resolved_at_ms: TimestampMs,
    #[serde(default)]
    pub selected_option_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// 解决问题响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveQuestionResponse {
    pub question: QuestionDto,
    pub resolution: QuestionResolutionDto,
}
