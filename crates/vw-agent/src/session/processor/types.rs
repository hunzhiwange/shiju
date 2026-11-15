//! 会话处理器类型定义模块
//!
//! 本模块定义了会话处理过程中使用的核心数据类型，包括请求结构、流式事件和工具会话状态。
//! 这些类型用于在会话处理管道中传递和处理数据。

use crate::agent::loop_::NonCliApprovalContext;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::provider::provider::ModelLimit;
// LLM raw artifacts are carried by the shared StreamEvent.
use crate::session::runtime::SessionToolset;
use crate::session::ui_types as models;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 会话处理请求
///
/// 表示一个完整的会话处理请求，包含查询内容、会话标识、模型配置和历史消息等信息。
/// 该结构体用于启动或继续一个会话处理流程。
#[derive(Debug, Clone)]
pub struct Request {
    /// 流标识符，用于唯一标识一个流式会话
    pub stream: u64,

    /// 会话标识符，用于关联同一会话中的多个请求
    pub session: String,

    /// 用户查询文本，包含用户的问题或指令
    pub query: String,

    /// 根目录路径，可选的工作目录或上下文根路径
    pub root: Option<String>,

    /// 模型标识符，指定要使用的 AI 模型（如 "gpt-4"、"claude-3" 等）
    pub model: Option<String>,

    /// 当前请求使用的 Provider 重试与退避配置快照
    pub reliability: crate::app::agent::config::ReliabilityConfig,

    /// 额外调用选项，用于透传 ACP 等适配器特定参数
    pub options: Value,

    /// 交互式审批管理器
    pub approval: Option<Arc<ApprovalManager>>,

    /// 当前请求所属通道
    pub channel_name: Option<String>,

    /// 非 CLI 审批上下文
    pub non_cli_approval_context: Option<NonCliApprovalContext>,

    /// 当前助手消息 ID（可预分配，用于工具/审批关联）
    pub assistant_message_id: Option<String>,

    /// 历史消息列表，包含之前对话的上下文信息
    pub history: Vec<models::ChatMessage>,

    /// 是否持久化会话工件（如生成的文件、中间结果等）
    pub persist_app_session_artifacts: bool,

    /// 会话工件持久化所属的项目范围 ID
    /// 使用前置计算后传入，避免在 runtime 内同步阻塞获取。
    pub app_session_scope: Option<String>,

    /// 当前提示轮次的显式取消令牌；模型与工具边界共享此令牌。
    pub abort_token: Option<CancellationToken>,

    /// 当前绑定会话的初始工具集合；宿主提供 live catalog 时，每次模型请求捕获其已发布版本。
    pub session_tools: SessionToolset,
    /// Validated inbound skill directory lease for this session only.
    pub session_skills: crate::acp::mounts::SessionSkillSources,
}

/// A complete model reference chosen once for a processor request.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedRequestModel {
    pub(crate) model: String,
    pub(crate) limit: ModelLimit,
}

impl ResolvedRequestModel {
    pub(crate) fn new(model: String, limit: ModelLimit) -> Self {
        Self { model, limit }
    }

    pub(crate) fn step_event_model(&self) -> Option<&str> {
        Some(&self.model)
    }
}

pub use session_port::{
    CompactionPhase, ModelRecovery, RecoveryPhase, ToolCallStatus, ToolEvent, ToolEventKind,
};

pub use session_engine::processor_events::StreamEvent;

/// 工具会话状态
///
/// 跟踪会话中工具执行的状态信息，用于防止重复执行和统计执行次数。
/// 该结构体仅在处理器内部使用。
#[derive(Default)]
pub(crate) struct ToolSessionState {
    /// 已见的工具调用标识符集合，用于去重和避免重复执行
    pub(crate) seen: HashSet<u64>,

    /// 非 TODO 类型的工具运行次数统计
    pub(crate) non_todo_tool_runs: u64,

    /// 当前请求通过 AgentTool 后台启动的直系子会话。
    pub(crate) agent_background_children: Vec<String>,
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
