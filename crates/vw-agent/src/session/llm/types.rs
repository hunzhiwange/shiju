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

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
pub use model_types::ToolCallKind;

pub use llm_runtime::types::{AgentInfo, StreamInput, PromptStreamEvent};
pub use llm_runtime::StreamEvent;

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
