//! 会话请求与排队输入的唯一数据所有者。
use session_ui_types::ChatMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcpHistoryReplayMode {
    Discard,
    Full,
    Summary,
    Recent,
}

impl AcpHistoryReplayMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discard => "discard",
            Self::Full => "full",
            Self::Summary => "summary",
            Self::Recent => "recent",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "discard" => Self::Discard,
            "full" => Self::Full,
            "recent" => Self::Recent,
            "summary" => Self::Summary,
            _ => Self::Discard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatSendBehavior {
    Queue,
    StopAndSend,
    Guide,
}

impl ChatSendBehavior {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::StopAndSend => "stop_send",
            Self::Guide => "guide",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "stop_send" | "stop-and-send" | "stop_and_send" | "stop" => Self::StopAndSend,
            "guide" => Self::Guide,
            _ => Self::Queue,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Queue => "添加到队列",
            Self::StopAndSend => "停止并发送",
            Self::Guide => "通过消息引导",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Queue => "发送到队列，保持当前排队逻辑。",
            Self::StopAndSend => "立刻停止当前请求，并开始发送。",
            Self::Guide => "等待当前一轮工具完成，下一轮请求前。",
        }
    }
}

/// Agent 请求信息
///
/// 表示一个正在处理或待处理的 Agent 请求，
/// 包含请求的唯一标识、会话信息、查询内容和历史消息。
///
/// `Hash` 只使用 `id`：iced `Subscription::run_with` 靠哈希保活流；
/// 若把 `history` 等字段纳入哈希，状态抖动会取消 SSE，表现为刚显示
/// 「正在规划」就报「未收到网关完成信号」。
#[derive(Debug, Clone)]
pub struct AgentRequest {
    /// 请求的唯一标识符
    pub id: u64,
    /// 所属会话的标识符
    pub session: String,
    /// 用户的查询文本
    pub query: String,
    /// 工作区的根目录路径（可选）
    pub root: Option<String>,
    /// 要使用的模型标识符（可选，使用默认模型）
    pub model: Option<String>,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub acp_test: bool,
    pub acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub acp_agent: Option<String>,
    pub acp_allowed_tools: Option<Vec<String>>,
    pub agent: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub require_agent_tool: bool,
    pub acp_force_new_session: bool,
    pub acp_history_mode: AcpHistoryReplayMode,
    pub acp_recent_count: usize,
    /// None lets the gateway use the established session's stored permission.
    pub full_access_enabled: Option<bool>,
    pub resume_history_only: bool,
    pub workflow_mode_enabled: bool,
    pub goal_mode_enabled: bool,
    /// 对话历史消息列表
    pub history: Vec<ChatMessage>,
}

impl std::hash::Hash for AgentRequest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.id, state);
    }
}

impl PartialEq for AgentRequest {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AgentRequest {}

/// 请求队列项
///
/// 表示队列中等待处理的请求项，
/// 记录了请求的创建时间、查询内容和配置信息。
#[derive(Debug, Clone)]
pub struct QueueItem {
    /// 请求创建的时间戳（毫秒）
    pub created_ms: u64,
    /// 用户的查询文本
    pub query: String,
    /// 当前请求附带的本地附件路径列表
    pub attachments: Vec<String>,
    /// 工作区的根目录路径（可选）
    pub root: Option<String>,
    /// 要使用的模型标识符（可选）
    pub model: Option<String>,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub acp_test: bool,
    pub acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub acp_agent: Option<String>,
    pub acp_allowed_tools: Option<Vec<String>>,
    pub agent: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub require_agent_tool: bool,
    pub acp_force_new_session: bool,
    pub acp_history_mode: AcpHistoryReplayMode,
    pub acp_recent_count: usize,
    pub full_access_enabled: bool,
    pub send_behavior: ChatSendBehavior,
    pub request_history_override: Option<Vec<ChatMessage>>,
    pub resume_history_only: bool,
    pub workflow_mode_enabled: bool,
    pub goal_mode_enabled: bool,
}


/// 显式重置后同步在途与待发送请求的历史快照。
pub fn reset_history_snapshots(
    active_request: &mut Option<std::sync::Arc<AgentRequest>>,
    queue: &mut [QueueItem],
    retained_chat: &[ChatMessage],
) {
    if let Some(request) = active_request.as_mut() {
        std::sync::Arc::make_mut(request).history = retained_chat.to_vec();
    }
    for item in queue {
        if item.resume_history_only || item.request_history_override.is_some() {
            item.request_history_override = Some(retained_chat.to_vec());
        }
    }
}

/// 队列操作只处理会话输入；复制、编辑与发送由宿主映射到对应效果。
pub enum QueueAction {
    Remove(usize),
    Edit(usize),
    Guide(usize),
    Up(usize),
    Down(usize),
    Copy(usize),
}

pub enum QueueEffect {
    None,
    Edit(String),
    Send(QueueItem),
    Copy(String),
}

pub fn update_queue(queue: &mut Vec<QueueItem>, action: QueueAction) -> QueueEffect {
    match action {
        QueueAction::Remove(i) => {
            if i < queue.len() { queue.remove(i); }
        }
        QueueAction::Edit(i) => {
            if i < queue.len() { return QueueEffect::Edit(queue.remove(i).query); }
        }
        QueueAction::Guide(i) => {
            if i < queue.len() {
                let mut item = queue.remove(i);
                item.send_behavior = ChatSendBehavior::StopAndSend;
                return QueueEffect::Send(item);
            }
        }
        QueueAction::Up(i) => {
            if i > 0 && i < queue.len() { queue.swap(i, i - 1); }
        }
        QueueAction::Down(i) => {
            if i + 1 < queue.len() { queue.swap(i, i + 1); }
        }
        QueueAction::Copy(i) => {
            if let Some(item) = queue.get(i)
                && !item.query.is_empty()
            { return QueueEffect::Copy(item.query.clone()); }
        }
    }
    QueueEffect::None
}

/// 会话发送、排队与请求计时的局部运行态。
#[derive(Debug, Clone, Default)]
pub struct RequestLifecycle {
    /// 请求队列
    pub queue: Vec<QueueItem>,
    /// 当前活跃的 Agent 请求
    pub active_agent_request: Option<std::sync::Arc<AgentRequest>>,
    /// 是否正在处理请求
    pub is_requesting: bool,
    /// 提交动画计数器
    pub submit_anim: u8,
    /// 本轮请求开始的毫秒时间戳，用于状态栏展示运行时长；0 表示未在请求中
    pub request_started_ms: u64,
    /// 请求开始到结束的单调时钟耗时；持续计入步骤间停顿。
    pub processing_timer: runtime_time::ActiveTimer,
    /// 仅累计 LLM StepStart 到 StepFinish 的单调时钟耗时
    pub llm_timer: runtime_time::ActiveTimer,
}

impl RequestLifecycle {
    pub fn begin_timing(&mut self, started_ms: u64, now: web_time::Instant) {
        self.is_requesting = true;
        self.request_started_ms = started_ms;
        self.processing_timer.reset();
        self.processing_timer.start_at(now);
        self.llm_timer.reset();
        self.submit_anim = 0;
    }

    pub fn clear_request(&mut self) {
        self.is_requesting = false;
        self.submit_anim = 0;
        self.active_agent_request = None;
    }

    /// 返回是否仍需调度下一帧。
    pub fn submit_tick(&mut self, is_requesting: bool) -> bool {
        if !is_requesting {
            self.submit_anim = 0;
            return false;
        }
        self.submit_anim = (self.submit_anim + 1) % 3;
        true
    }
}

impl QueueItem {
    pub fn into_request(
        self,
        id: u64,
        session: String,
        query: String,
        history: Vec<ChatMessage>,
        full_access_enabled: Option<bool>,
    ) -> AgentRequest {
        AgentRequest {
            id, session, query, history, full_access_enabled,
            root: self.root,
            model: self.model,
            reasoning_effort: self.reasoning_effort,
            acp_test: self.acp_test,
            acp_model_source: self.acp_model_source,
            acp_agent: self.acp_agent,
            acp_allowed_tools: self.acp_allowed_tools,
            agent: self.agent,
            allowed_tools: self.allowed_tools,
            require_agent_tool: self.require_agent_tool,
            acp_force_new_session: self.acp_force_new_session,
            acp_history_mode: self.acp_history_mode,
            acp_recent_count: self.acp_recent_count,
            resume_history_only: self.resume_history_only,
            workflow_mode_enabled: self.workflow_mode_enabled,
            goal_mode_enabled: self.goal_mode_enabled,
        }
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;

pub enum DispatchKind {
    StopDraft,
    Replace,
    Guide,
    Queue,
    Start,
}

impl RequestLifecycle {
    pub fn dispatch_kind(&self, behavior: ChatSendBehavior, has_active_draft: bool) -> DispatchKind {
        match behavior {
            ChatSendBehavior::StopAndSend if self.is_requesting && has_active_draft => DispatchKind::StopDraft,
            ChatSendBehavior::StopAndSend => DispatchKind::Replace,
            ChatSendBehavior::Guide if self.is_requesting => DispatchKind::Guide,
            ChatSendBehavior::Queue if self.is_requesting => DispatchKind::Queue,
            ChatSendBehavior::Guide | ChatSendBehavior::Queue => DispatchKind::Start,
        }
    }
}
