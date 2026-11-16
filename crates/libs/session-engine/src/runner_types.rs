//! Typed request data and per-turn tool execution state (one owner).
use model_types::ModelLimit;
use serde_json::Value;
use session_port::ui_types as models;
use std::{collections::HashSet, sync::Arc};
use tokio_util::sync::CancellationToken;
use tool_assembly::runtime::SessionToolset;
#[derive(Debug)]
pub struct Request<A, S> {
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
    pub reliability: agent_config::runtime::ReliabilityConfig,

    /// 额外调用选项，用于透传 ACP 等适配器特定参数
    pub options: Value,

    /// 交互式审批管理器
    pub approval: Option<Arc<A>>,

    /// 当前请求所属通道
    pub channel_name: Option<String>,

    /// 非 CLI 审批上下文
    pub non_cli_approval_context: Option<tool_context::approval::NonCliApprovalContext>,

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
    pub session_skills: S,
}

/// A complete model reference chosen once for a processor request.
#[derive(Debug, Clone)]
pub struct ResolvedRequestModel {
    pub model: String,
    pub limit: ModelLimit,
}

impl ResolvedRequestModel {
    pub fn new(model: String, limit: ModelLimit) -> Self {
        Self { model, limit }
    }

    pub fn step_event_model(&self) -> Option<&str> {
        Some(&self.model)
    }
}

#[derive(Default)]
pub struct ToolSessionState {
    /// 已见的工具调用标识符集合，用于去重和避免重复执行
    pub seen: HashSet<u64>,

    /// 非 TODO 类型的工具运行次数统计
    pub non_todo_tool_runs: u64,

    /// 当前请求通过 AgentTool 后台启动的直系子会话。
    pub agent_background_children: Vec<String>,
}

/// Tool IO result consumed by the runner's protocol assembly.
#[derive(Debug, Clone, Default)]
pub struct ToolProtocolOutput {
    pub success: bool,
    pub content_for_model: String,
    pub extra_messages: Vec<tool_types::ToolExtraMessage>,
}

impl<A, S: Clone> Clone for Request<A, S> {
    fn clone(&self) -> Self {
        Self {
            stream: self.stream,
            session: self.session.clone(),
            query: self.query.clone(),
            root: self.root.clone(),
            model: self.model.clone(),
            reliability: self.reliability.clone(),
            options: self.options.clone(),
            approval: self.approval.clone(),
            channel_name: self.channel_name.clone(),
            non_cli_approval_context: self.non_cli_approval_context.clone(),
            assistant_message_id: self.assistant_message_id.clone(),
            history: self.history.clone(),
            persist_app_session_artifacts: self.persist_app_session_artifacts,
            app_session_scope: self.app_session_scope.clone(),
            abort_token: self.abort_token.clone(),
            session_tools: self.session_tools.clone(),
            session_skills: self.session_skills.clone(),
        }
    }
}
