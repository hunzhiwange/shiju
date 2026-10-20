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
}

/// 流式事件枚举
///
/// 该枚举定义了 LLM 流式响应中可能出现的所有事件类型。
/// 用于在流式处理过程中传递增量文本、推理内容、工具调用和完成状态。
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 文本增量事件，包含新生成的文本片段
    Delta(String),
    /// 推理增量事件，包含模型的推理过程文本（用于思维链模型）
    ReasoningDelta(String),
    /// 工具调用事件，包含需要执行的工具调用列表
    ToolCalls(Vec<ToolCall>),
    /// 远端 ACP 代理已经执行的工具生命周期更新。
    ///
    /// 该事件只用于展示，不能再次作为本地工具调用执行。
    RemoteToolUpdate {
        /// ACP 工具调用的稳定标识。
        call_id: String,
        /// 工具名称。
        name: String,
        /// 是否为该调用的初始事件。
        is_initial: bool,
        /// 已净化的结构化生命周期载荷。
        payload: Value,
    },
    /// 完成事件，包含结束原因和令牌使用统计
    Done {
        /// 完成原因（如"stop"、"length"等）
        finish_reason: Option<String>,
        /// 令牌使用统计（输入和输出令牌数）
        usage: models::TokenUsage,
    },
    /// 错误事件，包含错误详情
    Error(message::AssistantError),
    /// 完整消息事件，包含所有消息的完整列表
    FullMessages(Vec<serde_json::Value>),
}

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

/// 工具调用结构
///
/// 该结构体表示 LLM 请求执行的工具调用，包含工具调用的所有必要信息。
/// 在函数调用场景中，LLM 会返回此类结构以请求执行特定工具。
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// 工具调用的唯一标识符，用于关联调用和结果
    pub id: String,
    /// 要调用的工具名称
    pub name: String,
    /// 工具参数的 JSON 字符串表示
    pub arguments: String,
}

/// LLM 错误类型枚举
///
/// 该枚举定义了 LLM 会话处理过程中可能出现的所有错误类型。
/// 实现了 Display 和 Error trait 以支持错误处理和显示。
#[derive(Debug)]
pub enum Error {
    /// 提供商未找到错误，包含未找到的提供商标识符
    ProviderNotFound(String),
    /// HTTP 请求错误，封装 reqwest 错误
    Http(reqwest::Error),
    /// 请求被中断（用户取消或超时）
    Aborted,
    /// API 错误，包含助手级别的错误详情
    Api(message::AssistantError),
}

/// 实现 Display trait 以提供人类可读的错误信息
impl std::fmt::Display for Error {
    /// 格式化错误信息
    ///
    /// # 参数
    /// - `f` - 格式化器
    ///
    /// # 返回值
    /// 格式化结果，成功返回 Ok(())
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // 提供商未找到：显示提供商标识符
            Error::ProviderNotFound(id) => write!(f, "provider not found: {}", id),
            // HTTP 错误：直接显示原始错误信息
            Error::Http(e) => write!(f, "{}", e),
            // 中断错误：显示简短的标识
            Error::Aborted => write!(f, "aborted"),
            // API 错误：尝试序列化为 JSON，失败则显示占位符
            Error::Api(message::AssistantError::Unknown { message }) => {
                write!(f, "{}", message.strip_prefix("acp: ").unwrap_or(message))
            }
            Error::Api(e) => {
                write!(f, "{}", serde_json::to_string(e).unwrap_or_else(|_| "?".to_string()))
            }
        }
    }
}

/// 实现 Error trait 以支持标准错误处理
impl std::error::Error for Error {}

/// 实现 From trait 以自动转换 reqwest::Error
impl From<reqwest::Error> for Error {
    /// 将 reqwest 错误转换为 LLM 错误
    ///
    /// # 参数
    /// - `value` - reqwest 错误实例
    ///
    /// # 返回值
    /// 转换后的 Error::Http 变体
    fn from(value: reqwest::Error) -> Self {
        Error::Http(value)
    }
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
