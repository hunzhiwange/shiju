use serde::{Deserialize, Serialize};
use std::fmt;

/// 选项题中的单个可选项描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionInfo {
    pub label: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// 单个提问项的展示与交互定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub question: String,
    pub header: String,
    pub options: Vec<OptionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<bool>,
}

/// 触发提问的工具调用元信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMeta {
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(rename = "callID")]
    pub call_id: String,
}

/// 可信 MCP 请求的服务器来源及原始受支持 schema；不包含用户回答。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpForm {
    pub server: String,
    pub schema: serde_json::Value,
}

/// 发给前端的问题请求载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_form: Option<McpForm>,
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub questions: Vec<Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolMeta>,
}

/// 单个问题的一组回答值。
pub type Answer = Vec<String>;

/// 用户对整组问题的回复内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply {
    pub answers: Vec<Answer>,
}

/// 在内部发起提问时使用的输入结构。
#[derive(Debug, Clone)]
pub struct AskInput {
    pub session_id: String,
    pub questions: Vec<Info>,
    pub tool: Option<ToolMeta>,
}

/// 在内部提交回答时使用的输入结构。
#[derive(Debug, Clone)]
pub struct ReplyInput {
    pub request_id: String,
    pub answers: Vec<Answer>,
}

/// 表示用户主动关闭或拒绝了当前提问。
#[derive(Debug, Clone)]
pub struct RejectedError;

impl fmt::Display for RejectedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The user dismissed this question")
    }
}

impl std::error::Error for RejectedError {}

#[cfg(test)]
#[path = "question_tests.rs"]
mod question_tests;
