//! LLM 会话类型定义模块
//!
//! 该模块定义了 LLM（大语言模型）会话处理过程中使用的核心类型和数据结构。
//! 主要包括：
//! - 代理配置信息
//! - 流式输入参数
//! - 流式事件类型
//! - 工具调用结构
//! - 错误处理类型
//!
//! 这些类型用于在 LLM 提供商和会话管理层之间传递数据。

use crate::app::agent::permission::next as permission_next;
use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::tools;
pub use model_types::ToolCallKind;
use crate::session::ui_types as models;
use serde_json::Value;
use std::collections::HashMap;

/// 代理配置信息
///
/// 该结构体封装了代理实例的所有配置信息，包括基本属性、模型参数和权限规则。
/// 在创建 LLM 会话时，这些信息用于定制代理的行为和响应模式。
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// 代理名称，用于标识和日志记录
    pub name: String,
    /// 代理模式，定义代理的运行类型（如"chat"、"research"等）
    pub mode: String,
    /// 自定义系统提示词，用于覆盖默认提示
    pub prompt: Option<String>,
    /// 采样温度参数，控制输出的随机性（0.0-2.0）
    pub temperature: Option<f64>,
    /// Top-p 采样参数，控制词汇选择的多样性（0.0-1.0）
    pub top_p: Option<f64>,
    /// 扩展选项映射，用于传递额外的模型特定参数
    pub options: HashMap<String, Value>,
    /// 权限规则集，定义代理的权限边界
    pub permission: permission_next::Ruleset,
}

/// 流式输入参数
///
/// 该结构体封装了发起 LLM 流式请求所需的所有输入参数。
/// 包括用户信息、会话标识、模型配置、消息历史以及中断信号等。
#[derive(Debug, Clone)]
pub struct StreamInput {
    /// 发起请求的用户信息
    pub user: message::UserInfo,
    /// 会话唯一标识符，用于跟踪和关联请求
    pub session_id: String,
    /// 要使用的模型配置（包括提供商和模型名称）
    pub model: provider::Model,
    /// 代理配置信息
    pub agent: AgentInfo,
    /// 系统消息列表，用于构建提示上下文
    pub system: Vec<String>,
    /// 中断信号接收器，用于提前终止流式请求
    pub abort: Option<tokio::sync::watch::Receiver<bool>>,
    /// 消息历史，包含对话的完整上下文
    pub messages: Vec<Value>,
    /// 是否使用小型模型（用于优化成本和延迟）
    pub small: bool,
    /// 可用工具的映射表，按工具名称索引
    pub tools: HashMap<String, tools::ToolSpec>,
    /// 重试次数，用于错误恢复策略
    pub retries: u64,
    /// processor 已持有总预算时，禁用 stream 和 SDK 内部重试。
    pub retry_in_caller: bool,
}

pub use session_engine::llm_stream::StreamEvent;

/// 提示流事件枚举
///
/// 该枚举定义了提示生成流程中的事件类型。
/// 用于简化的流式提示处理场景。
#[derive(Debug, Clone)]
pub enum PromptStreamEvent {
    /// 文本增量事件，包含生成的文本片段
    Delta(String),
    /// 完成事件，包含令牌使用统计
    Done(models::TokenUsage),
    /// 错误事件，包含错误消息字符串
    Error(String),
}

pub use session_engine::processor_events::ToolCall;

/// LLM 错误类型枚举
///
/// 该枚举定义了 LLM 会话处理过程中可能出现的所有错误类型。
/// 实现了 Display 和 Error trait 以支持错误处理和显示。
pub use llm_runtime::Error;
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

/// 保留目录错误的结构化分类，不根据展示文字猜测可恢复性。
impl From<provider::CatalogError> for message::AssistantError {
    fn from(error: provider::CatalogError) -> Self {
        use crate::{config, storage};
        let io = match &error {
            provider::CatalogError::Configuration(
                config::Error::Io(io) | config::Error::Storage(storage::Error::Io(io)),
            ) => Some(io),
            _ => None,
        };
        let is_retryable = io.is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::Interrupted
                    | std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::UnexpectedEof
            )
        });
        Self::APIError {
            message: error.to_string(),
            status_code: None,
            is_retryable,
            response_headers: None,
            response_body: None,
            metadata: None,
        }
    }
}

pub(crate) fn catalog_error_string(error: provider::CatalogError) -> String {
    serde_json::to_string(&message::AssistantError::from(error))
        .expect("assistant error serializes")
}

pub(crate) use session_engine::replay_safety::prevent_error_replay;
