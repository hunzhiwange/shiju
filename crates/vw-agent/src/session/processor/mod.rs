//! 会话处理器模块
//!
//! 本模块实现代理的核心会话处理循环，负责协调大语言模型(LLM)调用、工具执行、
//! 状态管理等关键流程。主要职责包括：
//!
//! - 将用户请求转换为LLM可处理的消息格式
//! - 执行多步推理循环，直至任务完成或达到步数上限
//! - 管理工具调用与结果记录
//! - 处理文档请求的预取与上下文注入
//! - 持久化会话工件以供后续分析
//!
//! # 模块结构
//!
//! - `llm_messages`: 消息格式转换，将会话消息转为LLM输入格式
//! - `llm_runner`: LLM调用执行器，支持重试与错误处理
//! - `planning`: 任务规划相关逻辑
//! - `prefetch`: 上下文预取，如文档列表读取
//! - `prompting`: 提示构建逻辑
//! - `helpers`: 工具权限、预览与ACP判定等辅助函数
//! - `artifacts`: 会话工件持久化逻辑
//! - `runner`: 会话主循环编排入口
//! - `todo_updates`: TODO状态更新逻辑
//! - `todos`: TODO任务管理与状态补丁构建
//! - `tools_exec`: 工具执行与记录
//! - `types`: 公共类型定义（Request、StreamEvent等）
//! - `utils`: 辅助工具函数

pub(crate) use super::session::{Role, Session};

mod artifacts;
pub(crate) mod context_governance;
mod helpers;
pub(crate) mod llm_messages;
pub(crate) mod llm_runner;
#[cfg(test)]
pub(crate) use llm_runner::{
    ChannelCancelStream, enqueue_test_llm_stream, register_channel_cancel_stream,
    take_test_llm_messages, test_llm_stream_guard,
};
mod planning;
mod prefetch;
mod prompting;
mod recovery;
mod runner;
mod todo_updates;
mod todos;
mod tools_exec;
mod turn_policy;
mod types;
mod utils;

pub(crate) use prefetch::app_session_scope_from_root_async;
pub(crate) use types::ToolSessionState;

#[cfg(test)]
pub(crate) use helpers::allowed_tool_ids_for_request_from_specs;
pub(crate) use helpers::{allowed_tool_ids, should_execute_structured_tool_calls_locally};
pub use runner::run;
pub use types::{
    CompactionPhase, ModelRecovery, RecoveryPhase, Request, StreamEvent, ToolCallStatus, ToolEvent,
    ToolEventKind,
};

/// 测试模块
///
/// 包含会话处理器的单元测试用例。
#[cfg(test)]
use crate::app::agent::tools::todo;

/// 单元测试模块
///
/// 测试文件位于 `tests.rs`，包含针对会话处理逻辑的各类测试。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
