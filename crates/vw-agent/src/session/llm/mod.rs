//! LLM（大语言模型）会话管理模块
//!
//! 本模块提供与大语言模型交互的核心功能，包括：
//! - 消息构建与管理
//! - 提示词发送与流式响应
//! - 工具调用支持
//! - 日志记录与配置选项
//!
//! ## 主要组件
//!
//! - **messages**: 消息处理，包含工具调用检测等功能
//! - **prompt**: 提示词发送核心逻辑，支持普通模式和工具模式
//! - **stream**: 流式响应处理
//! - **options**: LLM 调用选项配置
//! - **logging**: 日志记录功能
//! - **types**: 核心类型定义（错误、流事件、工具调用等）
//! - **aisdk**: AI SDK 集成（仅非 WASM 平台）
//!
//! ## 使用示例
//!
//! ```ignore
//! use crate::app::agent::session::llm::{send_prompt, stream_prompt_with_tools};
//!
//! // 发送普通提示词
//! let response = send_prompt(prompt, provider).await?;
//!
//! // 发送带工具支持的流式提示词
//! let stream = stream_prompt_with_tools(prompt, provider, tools).await?;
//! ```

mod logging;
mod messages;
mod model_selector;
mod options;
mod prompt;
mod stream;
#[cfg(test)]
mod model_selector_tests;
#[cfg(test)]
mod tests;
mod types;

#[cfg(not(target_arch = "wasm32"))]
mod acp;
#[cfg(not(target_arch = "wasm32"))]
mod aisdk;

pub use messages::has_tool_calls;
pub use model_selector::{
    select_auto_model, AutoModelSelectionRequest, ModelCandidate, ModelReference,
    ModelSelectionDecision, ModelSelectionError, ModelSelectionSource, ModelTaskProfile,
};
#[cfg(test)]
pub(crate) use prompt::model_limit_for_request;
pub(crate) use prompt::stream_chat_with_tools_for_session_with_abort;
pub use prompt::{
    send_prompt, stream_chat_with_tools, stream_chat_with_tools_for_session, stream_prompt,
    stream_prompt_with_tools,
};
pub use stream::stream;
pub use types::{AgentInfo, Error, PromptStreamEvent, StreamEvent, StreamInput, ToolCall};
