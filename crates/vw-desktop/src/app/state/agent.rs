use super::*;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

pub(crate) const MAIN_AGENT_KEY: &str = "main";

pub(crate) const AGENT_PROMPT_SYSTEM_TAB: &str = "system_prompt";
pub(crate) const AGENT_DETAIL_BASIC_TAB: &str = "basic";
pub(crate) const AGENT_DETAIL_IDENTITY_TAB: &str = "identity";
pub(crate) const AGENT_DETAIL_TOOLS_TAB: &str = "tools";
pub(crate) const AGENT_DETAIL_SKILLS_TAB: &str = "skills";
pub(crate) const AGENT_TOOLS_OPTIONAL_TAB: &str = "optional";
pub(crate) const AGENT_TOOLS_BUILTIN_TAB: &str = "builtin";
static GUIDE_HANDOFF_REQUEST_IDS: Lazy<Mutex<HashSet<u64>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

pub(crate) fn mark_pending_guide_handoff(request_id: u64) {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.insert(request_id);
}

pub(crate) fn take_pending_guide_handoff(request_id: u64) -> bool {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.remove(&request_id)
}

pub(crate) fn clear_pending_guide_handoff(request_id: u64) {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.remove(&request_id);
}

pub(crate) const WORKSPACE_IDENTITY_FILES: [(&str, &str); 7] = [
    ("AGENTS.md", "智能体 AGENTS"),
    ("SOUL.md", "核心逻辑 SOUL"),
    ("TOOLS.md", "工具集 TOOLS"),
    ("IDENTITY.md", "身份机制 IDENTITY"),
    ("USER.md", "用户系统 USER"),
    ("HEARTBEAT.md", "心跳机制 HEARTBEAT"),
    ("MEMORY.md", "记忆系统 MEMORY"),
];

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceIdentityFileState {
    pub(crate) file_name: String,
    pub(crate) label: String,
    pub(crate) editor: text_editor::Content,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) modified_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentSettingsEntryKind {
    Main,
    BuiltinWorker,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcpHistoryReplayMode {
    Discard,
    Full,
    Summary,
    Recent,
}

impl AcpHistoryReplayMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Discard => "discard",
            Self::Full => "full",
            Self::Summary => "summary",
            Self::Recent => "recent",
        }
    }

    pub(crate) fn from_str(value: &str) -> Self {
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
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Queue => "queue",
            Self::StopAndSend => "stop_send",
            Self::Guide => "guide",
        }
    }

    pub(crate) fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "stop_send" | "stop-and-send" | "stop_and_send" | "stop" => Self::StopAndSend,
            "guide" => Self::Guide,
            _ => Self::Queue,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Queue => "添加到队列",
            Self::StopAndSend => "停止并发送",
            Self::Guide => "通过消息引导",
        }
    }

    pub(crate) const fn description(self) -> &'static str {
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
pub(crate) struct AgentRequest {
    /// 请求的唯一标识符
    pub(crate) id: u64,
    /// 所属会话的标识符
    pub(crate) session: String,
    /// 用户的查询文本
    pub(crate) query: String,
    /// 工作区的根目录路径（可选）
    pub(crate) root: Option<String>,
    /// 要使用的模型标识符（可选，使用默认模型）
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    pub(crate) acp_test: bool,
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_allowed_tools: Option<Vec<String>>,
    pub(crate) agent: Option<String>,
    pub(crate) allowed_tools: Option<Vec<String>>,
    pub(crate) require_agent_tool: bool,
    pub(crate) acp_force_new_session: bool,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) full_access_enabled: bool,
    pub(crate) resume_history_only: bool,
    pub(crate) workflow_mode_enabled: bool,
    pub(crate) goal_mode_enabled: bool,
    /// 对话历史消息列表
    pub(crate) history: Vec<ChatMessage>,
}

impl std::hash::Hash for AgentRequest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
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
pub(crate) struct QueueItem {
    /// 请求创建的时间戳（毫秒）
    pub(crate) created_ms: u64,
    /// 用户的查询文本
    pub(crate) query: String,
    /// 当前请求附带的本地附件路径列表
    pub(crate) attachments: Vec<String>,
    /// 工作区的根目录路径（可选）
    pub(crate) root: Option<String>,
    /// 要使用的模型标识符（可选）
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    pub(crate) acp_test: bool,
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_allowed_tools: Option<Vec<String>>,
    pub(crate) agent: Option<String>,
    pub(crate) allowed_tools: Option<Vec<String>>,
    pub(crate) require_agent_tool: bool,
    pub(crate) acp_force_new_session: bool,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) full_access_enabled: bool,
    pub(crate) send_behavior: ChatSendBehavior,
    pub(crate) request_history_override: Option<Vec<ChatMessage>>,
    pub(crate) resume_history_only: bool,
    pub(crate) workflow_mode_enabled: bool,
    pub(crate) goal_mode_enabled: bool,
}

const STREAM_TAIL_BASE_HEIGHT: f32 = 78.0;

#[derive(Debug, Clone)]
pub(crate) struct StreamTailState {
    message_idx: Option<usize>,
    processed_bytes: usize,
    explicit_lines: usize,
    think_count: usize,
    thinking_open: bool,
    incomplete_tag: String,
    tool_match_tail: String,
    tool_count: usize,
    estimated_height: f32,
    ui_generation: u64,
    ui_preparing_generation: Option<u64>,
}

impl Default for StreamTailState {
    fn default() -> Self {
        Self {
            message_idx: None,
            processed_bytes: 0,
            explicit_lines: 1,
            think_count: 0,
            thinking_open: false,
            incomplete_tag: String::new(),
            tool_match_tail: String::new(),
            tool_count: 0,
            estimated_height: STREAM_TAIL_BASE_HEIGHT,
            ui_generation: 0,
            ui_preparing_generation: None,
        }
    }
}

impl StreamTailState {
    pub(crate) fn begin_message(&mut self, message_idx: usize) {
        if self.message_idx == Some(message_idx) {
            return;
        }
        let ui_generation = self.ui_generation;
        *self = Self { message_idx: Some(message_idx), ui_generation, ..Self::default() };
    }

    pub(crate) fn apply_delta(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }
        self.processed_bytes = self.processed_bytes.saturating_add(delta.len());
        self.explicit_lines =
            self.explicit_lines.saturating_add(delta.bytes().filter(|byte| *byte == b'\n').count());
        self.scan_think_tags(delta);
        self.scan_tool_markers(delta);
        self.recalculate_height();
    }

    pub(crate) fn message_idx(&self) -> Option<usize> {
        self.message_idx
    }

    #[cfg(test)]
    pub(crate) fn processed_bytes(&self) -> usize {
        self.processed_bytes
    }

    pub(crate) fn think_count(&self) -> usize {
        self.think_count
    }

    pub(crate) fn thinking_open(&self) -> bool {
        self.thinking_open
    }

    pub(crate) fn estimated_height(&self) -> f32 {
        self.estimated_height
    }

    pub(crate) fn ui_generation(&self) -> u64 {
        self.ui_generation
    }

    pub(crate) fn advance_ui_generation(&mut self) -> u64 {
        self.ui_generation = self.ui_generation.wrapping_add(1);
        self.ui_generation
    }

    pub(crate) fn begin_ui_preparation(&mut self) -> Option<u64> {
        if self.ui_preparing_generation.is_some() {
            return None;
        }
        self.ui_preparing_generation = Some(self.ui_generation);
        self.ui_preparing_generation
    }

    pub(crate) fn finish_ui_preparation(&mut self, generation: u64) {
        if self.ui_preparing_generation == Some(generation) {
            self.ui_preparing_generation = None;
        }
    }

    pub(crate) fn calibrate(
        &mut self,
        message_idx: usize,
        processed_bytes: usize,
        think_count: usize,
        thinking_open: bool,
        estimated_height: f32,
    ) {
        self.message_idx = Some(message_idx);
        self.processed_bytes = processed_bytes;
        self.think_count = think_count;
        self.thinking_open = thinking_open;
        self.estimated_height = estimated_height;
        self.incomplete_tag.clear();
    }

    pub(crate) fn clear_tail(&mut self) {
        let ui_generation = self.ui_generation;
        *self = Self { ui_generation, ..Self::default() };
    }

    fn scan_think_tags(&mut self, delta: &str) {
        let mut input = String::with_capacity(self.incomplete_tag.len() + delta.len());
        input.push_str(&self.incomplete_tag);
        input.push_str(delta);
        self.incomplete_tag.clear();

        let mut search_from = 0;
        while let Some(relative_start) = input[search_from..].find('<') {
            let start = search_from + relative_start;
            let candidate = &input[start..];
            let Some(relative_end) = candidate.find('>') else {
                if possible_incomplete_think_tag(candidate) {
                    self.incomplete_tag.push_str(candidate);
                }
                break;
            };
            let end = start + relative_end + 1;
            match classify_think_tag(&input[start..end]) {
                Some(true) if !self.thinking_open => {
                    self.think_count = self.think_count.saturating_add(1);
                    self.thinking_open = true;
                }
                Some(false) if self.thinking_open => self.thinking_open = false,
                _ => {}
            }
            search_from = end;
        }
    }

    fn scan_tool_markers(&mut self, delta: &str) {
        let previous_tail_len = self.tool_match_tail.len();
        let mut input = String::with_capacity(previous_tail_len + delta.len());
        input.push_str(&self.tool_match_tail);
        input.push_str(delta);
        self.tool_count = self.tool_count.saturating_add(
            input
                .match_indices("tool ")
                .filter(|(start, _)| start.saturating_add("tool ".len()) > previous_tail_len)
                .count(),
        );
        self.tool_match_tail = trailing_chars(&input, "tool ".len() - 1).to_string();
    }

    fn recalculate_height(&mut self) {
        let wrapped_lines = self.processed_bytes.div_ceil(96).max(1);
        let line_count = self.explicit_lines.max(wrapped_lines) as f32;
        self.estimated_height = (line_count * 22.0
            + self.think_count as f32 * 28.0
            + self.tool_count as f32 * 36.0
            + 56.0)
            .clamp(64.0, 720.0);
    }
}

fn possible_incomplete_think_tag(candidate: &str) -> bool {
    ["<think", "<thinking", "</think", "</thinking"].iter().any(|tag| tag.starts_with(candidate))
        || candidate.starts_with("<think")
        || candidate.starts_with("</think")
}

fn classify_think_tag(tag: &str) -> Option<bool> {
    let inner = tag.strip_suffix('>')?;
    if let Some(remainder) = inner.strip_prefix("</think") {
        return think_tag_remainder_is_valid(remainder).then_some(false);
    }
    inner
        .strip_prefix("<think")
        .filter(|remainder| think_tag_remainder_is_valid(remainder))
        .map(|_| true)
}

fn think_tag_remainder_is_valid(remainder: &str) -> bool {
    remainder.is_empty()
        || remainder.chars().next().is_some_and(char::is_whitespace)
        || remainder.strip_prefix("ing").is_some_and(|after| {
            after.is_empty() || after.chars().next().is_some_and(char::is_whitespace)
        })
}

fn trailing_chars(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .rev()
        .nth(max_chars.saturating_sub(1))
        .map_or(value, |(start, _)| &value[start..])
}

/// 会话运行时状态
///
/// 管理单个会话的运行时状态，包括请求状态、输入编辑器、
/// 模型配置、任务模式等会话特定的信息。
#[derive(Debug, Clone)]
pub(crate) struct SessionRuntimeState {
    /// 是否正在处理请求
    pub(crate) is_requesting: bool,
    /// 本轮请求开始的毫秒时间戳，用于状态栏展示运行时长；0 表示未在请求中
    pub(crate) request_started_ms: u64,
    /// 请求开始到结束的单调时钟耗时；持续计入步骤间停顿。
    pub(crate) processing_timer: vw_shared::time::ActiveTimer,
    /// 仅累计 LLM StepStart 到 StepFinish 的单调时钟耗时
    pub(crate) llm_timer: vw_shared::time::ActiveTimer,
    /// 提交动画计数器
    pub(crate) submit_anim: u8,
    /// 是否有未查看的成功响应
    pub(crate) has_unseen_success: bool,
    /// 当前活跃的 Agent 请求
    pub(crate) active_agent_request: Option<Arc<AgentRequest>>,
    /// 尚未提交到聊天 UI 的流式文本增量。
    pub(crate) pending_stream_delta: String,
    /// 是否已经为流式文本增量安排了合并刷新。
    pub(crate) stream_delta_flush_scheduled: bool,
    /// 是否已经为完整会话快照安排了保存刷新。
    pub(crate) stream_save_flush_scheduled: bool,
    /// 当前 assistant 尾消息的增量解析、高度和后台预热代次。
    pub(crate) stream_tail_state: StreamTailState,
    /// 请求队列
    pub(crate) queue: Vec<QueueItem>,
    /// 输入编辑器内容
    pub(crate) input_editor: text_editor::Content,
    /// 当前选择的模型标识符
    pub(crate) model: String,
    /// 是否自动选择模型
    pub(crate) auto_model: bool,
    /// 是否等待从历史步骤恢复该会话自己的模型。
    pub(crate) restore_model_from_history: bool,
    pub(crate) reasoning_effort: Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    pub(crate) tool_selector: SessionToolSelectorState,
    /// 是否启用任务模式
    pub(crate) task_mode_enabled: bool,
    /// 是否将本轮聊天包装为临时工作流执行
    pub(crate) workflow_mode_enabled: bool,
    /// 是否持续执行并独立验收，直到达到目标
    pub(crate) goal_mode_enabled: bool,
    /// 是否要求顶层智能体优先通过 AgentTool 委托实质工作
    pub(crate) subagent_mode_enabled: bool,
    /// 任务模式的优先级
    pub(crate) task_mode_priority: String,
    /// 任务模式的模型标识符
    pub(crate) task_mode_model: String,
    /// 任务模式使用的 ACP 智能体
    pub(crate) task_mode_executor: Option<String>,
    /// 任务模式使用的推理强度；`None` 表示沿用 ACP 智能体默认值
    pub(crate) task_mode_reasoning_effort:
        Option<vw_gateway_client::vw_api_types::ReasoningEffortDto>,
    /// 任务模式的任务拆分策略
    pub(crate) task_mode_task_split_mode: crate::app::task::TaskSplitMode,
    /// 任务模式的子任务列表
    pub(crate) task_mode_subtasks: Vec<String>,
    /// 子任务的编辑器内容列表
    pub(crate) task_mode_subtask_editors: Vec<text_editor::Content>,
    pub(crate) agent: Option<String>,
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) full_access_enabled: bool,
    pub(crate) last_effective_acp_agent: Option<String>,
    pub(crate) acp_rebuild_required: bool,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub(crate) struct StepUiMeta {
    pub(crate) model: Option<String>,
    pub(crate) started_ms: u64,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) display_time_ms: u64,
}

impl From<&ChatSessionStep> for StepUiMeta {
    fn from(step: &ChatSessionStep) -> Self {
        Self {
            model: step.model.clone(),
            started_ms: step.started_ms,
            finished_ms: step.finished_ms,
            display_time_ms: step.finished_ms.unwrap_or(step.started_ms),
        }
    }
}

impl ActiveSessionViewState {
    pub(crate) fn estimated_bytes(&self) -> usize {
        let step_bytes = self.steps.iter().map(|step| {
            step.model.as_deref().map_or(0, str::len)
                + step.start_snapshot_path.as_deref().map_or(0, str::len)
                + step.finish_snapshot_path.as_deref().map_or(0, str::len)
                + step.finish_reason.as_deref().map_or(0, str::len)
        });
        let call_bytes = self.calls.iter().map(json_value_bytes);
        let step_llm_raw_bytes = self.step_llm_raw.values().map(json_value_bytes);
        let step_meta_bytes =
            self.step_index_map.values().map(|meta| meta.model.as_deref().map_or(0, str::len));
        let message_meta_bytes =
            self.message_meta_texts.iter().filter_map(Option::as_deref).map(str::len);
        let user_question_preview_bytes =
            self.user_question_markers.iter().map(|marker| marker.preview.len());

        step_bytes
            .chain(call_bytes)
            .chain(step_llm_raw_bytes)
            .chain(step_meta_bytes)
            .chain(message_meta_bytes)
            .chain(user_question_preview_bytes)
            .sum()
    }
}

fn json_value_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => 0,
        Value::String(text) => text.len(),
        Value::Array(values) => values.iter().map(json_value_bytes).sum(),
        Value::Object(entries) => {
            entries.iter().map(|(key, value)| key.len() + json_value_bytes(value)).sum()
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExploreSummaryAnimationState {
    pub(crate) previous_summary_text: String,
    pub(crate) current_summary_text: String,
    pub(crate) changed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UserQuestionMarker {
    pub(crate) message_idx: usize,
    pub(crate) preview: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveSessionViewState {
    pub(crate) updated_ms: u64,
    pub(crate) steps: Vec<ChatSessionStep>,
    pub(crate) calls: Vec<Value>,
    pub(crate) step_llm_raw: HashMap<SessionStepRawKey, Value>,
    pub(crate) llm_raw_step_indices: HashSet<SessionStepRawKey>,
    pub(crate) step_index_map: HashMap<u32, StepUiMeta>,
    pub(crate) message_meta_texts: Vec<Option<String>>,
    pub(crate) user_question_markers: Vec<UserQuestionMarker>,
    pub(crate) user_question_indexed_chat_len: usize,
    pub(crate) ui_preparing: bool,
    pub(crate) base_ready: bool,
    pub(crate) prepared_chat_ui_chunks: HashSet<usize>,
    pub(crate) preparing_chat_ui_chunks: HashSet<usize>,
    pub(crate) dirty_preparing_chat_ui_chunks: HashSet<usize>,
    pub(crate) last_visited_chat_ui_chunk_start: Option<usize>,
    pub(crate) pinned_chat_ui_chunk_start: Option<usize>,
}

/// 会话原始 LLM 载荷的稳定读取键。
pub(crate) type SessionStepRawKey = (String, String, u32);

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ToolDetailDialog {
    pub(crate) msg_idx: usize,
    pub(crate) tool_idx: usize,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) editor: text_editor::Content,
    pub(crate) editor_id: Id,
    pub(crate) context_menu_open: bool,
    pub(crate) context_menu_pos: Option<(f32, f32)>,
    pub(crate) scroll_top_line: f32,
    pub(crate) scroll_remainder: f32,
    pub(crate) viewport_height: f32,
}

impl Default for SessionRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRuntimeState {
    /// 创建具有指定模型配置的会话运行时状态
    ///
    /// # 参数
    ///
    /// - `model`：模型标识符
    /// - `auto_model`：是否自动选择模型
    ///
    /// # 返回值
    ///
    /// 返回初始化后的 `SessionRuntimeState` 实例
    pub(crate) fn with_defaults(model: String, auto_model: bool) -> Self {
        let task_mode_subtasks = vec![String::new(), String::new(), String::new()];
        let task_mode_subtask_editors =
            task_mode_subtasks.iter().map(|value| text_editor::Content::with_text(value)).collect();
        let task_mode_model = model.clone();
        Self {
            is_requesting: false,
            request_started_ms: 0,
            processing_timer: vw_shared::time::ActiveTimer::default(),
            llm_timer: vw_shared::time::ActiveTimer::default(),
            submit_anim: 0,
            has_unseen_success: false,
            active_agent_request: None,
            pending_stream_delta: String::new(),
            stream_delta_flush_scheduled: false,
            stream_save_flush_scheduled: false,
            stream_tail_state: StreamTailState::default(),
            queue: Vec::new(),
            input_editor: text_editor::Content::new(),
            model,
            auto_model,
            restore_model_from_history: false,
            reasoning_effort: None,
            tool_selector: SessionToolSelectorState::default(),
            task_mode_enabled: false,
            workflow_mode_enabled: false,
            goal_mode_enabled: true,
            subagent_mode_enabled: false,
            task_mode_priority: "999".to_string(),
            task_mode_model,
            task_mode_executor: None,
            task_mode_reasoning_effort: None,
            task_mode_task_split_mode: crate::app::task::TaskSplitMode::FollowSystem,
            task_mode_subtasks,
            task_mode_subtask_editors,
            agent: None,
            acp_agent: None,
            acp_history_mode: AcpHistoryReplayMode::Discard,
            acp_recent_count: 3,
            full_access_enabled: true,
            last_effective_acp_agent: None,
            acp_rebuild_required: false,
        }
    }

    /// 创建默认的会话运行时状态
    ///
    /// 使用 "auto" 模型和启用自动模型选择。
    ///
    /// # 返回值
    ///
    /// 返回默认配置的 `SessionRuntimeState` 实例
    pub(crate) fn new() -> Self {
        Self::with_defaults("auto".to_string(), true)
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;
