use super::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub(crate) use desktop_settings_agents::state::{
    MAIN_AGENT_KEY, AGENT_PROMPT_SYSTEM_TAB, WORKSPACE_IDENTITY_FILES,
    WorkspaceIdentityFileState, AgentSettingsEntryKind,
};

pub(crate) const AGENT_DETAIL_BASIC_TAB: &str = "basic";
pub(crate) const AGENT_DETAIL_IDENTITY_TAB: &str = "identity";
pub(crate) const AGENT_DETAIL_TOOLS_TAB: &str = "tools";
pub(crate) const AGENT_DETAIL_SKILLS_TAB: &str = "skills";
pub(crate) const AGENT_DETAIL_MCP_TAB: &str = "mcp";
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

pub use desktop_session::message::chat::request::{
    AcpHistoryReplayMode, ChatSendBehavior, AgentRequest, QueueItem,
};

pub(crate) use desktop_session::message::chat::stream::tail::StreamTailState;

#[derive(Debug, Clone)]
pub(crate) struct SessionDraft {
    pub(crate) project_path: String,
    pub(crate) directory: String,
    pub(crate) history_len: usize,
}

/// 会话运行时状态
///
/// 管理单个会话的运行时状态，包括请求状态、输入编辑器、
/// 模型配置、任务模式等会话特定的信息。
#[derive(Debug, Clone)]
pub(crate) struct SessionRuntimeState {
    pub(crate) request_lifecycle: desktop_session::message::chat::request::RequestLifecycle,
    pub(crate) draft: Option<SessionDraft>,
    /// Gateway authority: None is a regular session; false is an unstarted fork.
    pub(crate) fork_execution_started: Option<bool>,
    /// Lock a submitted fork until a fresh gateway read, including cancellation/errors.
    pub(crate) fork_request_submitted: bool,
    /// Do not let inherited binding loads overwrite an explicit fork selection.
    pub(crate) fork_agent_selected: bool,
    /// 计划运行子会话由网关提交历史，桌面仅维护实时投影。
    pub(crate) is_plan_run_session: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) mcp_app_panels: Vec<crate::app::components::mcp_app_panel::McpAppPanel>,
    /// 是否有未查看的成功响应
    pub(crate) has_unseen_success: bool,
    /// 当前请求的恢复事件；不进入模型上下文。
    pub(crate) model_recovery: Option<(u64, gateway_chat_stream::GatewayModelRecoveryEvent)>,
    /// 尚未提交到聊天 UI 的流式文本增量。
    pub(crate) pending_stream_delta: String,
    /// 是否已经为流式文本增量安排了合并刷新。
    pub(crate) stream_delta_flush_scheduled: bool,
    pub(crate) stream_save: desktop_session::message::chat::stream::save::StreamSaveState,
    pub(crate) pending_compaction_status_index: Option<usize>,
    /// 当前 assistant 尾消息的增量解析、高度和后台预热代次。
    pub(crate) stream_tail_state: StreamTailState,
    /// 活动 assistant 正文的 stable-prefix/mutable-tail 增量投影。
    pub(crate) stream_render_state: crate::app::session::stream_render::StreamRenderState,
    /// 输入编辑器内容
    pub(crate) input_editor: text_editor::Content,
    /// 当前选择的模型标识符
    pub(crate) model: String,
    /// 是否自动选择模型
    pub(crate) auto_model: bool,
    /// 是否等待从历史步骤恢复该会话自己的模型。
    pub(crate) restore_model_from_history: bool,
    pub(crate) reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub(crate) tool_selector: SessionToolSelectorState,
    /// 是否启用任务模式
    pub(crate) task_mode_enabled: bool,
    /// 是否将本轮聊天包装为临时工作流执行
    pub(crate) workflow_mode_enabled: bool,
    /// 是否持续执行并独立验收，直到达到目标
    pub(crate) goal_mode_enabled: bool,
    /// 任务模式的优先级
    pub(crate) task_mode_priority: String,
    /// 任务模式的模型标识符
    pub(crate) task_mode_model: String,
    /// 任务模式使用的 ACP 智能体
    pub(crate) task_mode_executor: Option<String>,
    /// 任务模式使用的推理强度；`None` 表示沿用模型或 ACP 智能体默认值
    pub(crate) task_mode_reasoning_effort:
        Option<vw_api_chat_identity::ReasoningEffortDto>,
    /// 任务模式的任务拆分策略
    pub(crate) task_mode_task_split_mode: crate::app::task::TaskSplitMode,
    /// 任务模式的子任务列表
    pub(crate) task_mode_subtasks: Vec<String>,
    /// 子任务的编辑器内容列表
    pub(crate) task_mode_subtask_editors: Vec<text_editor::Content>,
    pub(crate) agent: Option<String>,
    pub(crate) agent_label: Option<String>,
    pub(crate) acp_agent: Option<String>,
    pub(crate) acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub(crate) acp_options_state: desktop_session::message::chat::acp_options::AcpOptionsState,
    pub(crate) acp_history_mode: AcpHistoryReplayMode,
    pub(crate) acp_recent_count: usize,
    pub(crate) full_access_enabled: bool,
    pub(crate) full_access_initialized: bool,
    pub(crate) full_access_saving: bool,
    pub(crate) full_access_save_failed: bool,
    /// 用户操作后不再采用智能体默认值。
    pub(crate) full_access_overridden: bool,
    pub(crate) last_effective_acp_agent: Option<String>,
    pub(crate) acp_rebuild_required: bool,
}

pub(crate) use desktop_session::session::active::{StepUiMeta, UserQuestionMarker};

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
        let message_meta_bytes = self
            .message_meta_texts
            .iter()
            .filter_map(Option::as_ref)
            .map(crate::app::i18n::chat::MessageMeta::cached_bytes);
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

pub(crate) use desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState;

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveSessionViewState {
    pub(crate) updated_ms: u64,
    pub(crate) steps: Vec<ChatSessionStep>,
    pub(crate) calls: Vec<Value>,
    pub(crate) step_llm_raw: HashMap<SessionStepRawKey, Value>,
    pub(crate) trajectory_projection:
        crate::app::components::chat_panel::trajectory::TrajectoryProjection,
    #[cfg(test)]
    pub(crate) trajectory_projection_rebuild_count: usize,
    pub(crate) llm_raw_step_indices: HashSet<SessionStepRawKey>,
    pub(crate) step_index_map: HashMap<u32, StepUiMeta>,
    pub(crate) message_meta_texts: Vec<Option<crate::app::i18n::chat::MessageMeta>>,
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
pub(crate) use desktop_session::message::chat::trajectory::SessionStepRawKey;

pub(crate) use desktop_session::message::chat::input::tool_detail::ToolDetailDialog;

impl Default for SessionRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionRuntimeState {
    pub(crate) fn visible_session_tool_selector_tab(&self) -> SessionToolSelectorTab {
        match self.tool_selector.active_tab() {
            SessionToolSelectorTab::Tools if self.acp_agent.is_some() => {
                SessionToolSelectorTab::Agent
            }
            tab => tab,
        }
    }

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
            request_lifecycle: Default::default(),
            draft: None,
            fork_execution_started: None,
            fork_request_submitted: false,
            fork_agent_selected: false,
            is_plan_run_session: false,
            #[cfg(not(target_arch = "wasm32"))]
            mcp_app_panels: Vec::new(),
            has_unseen_success: false,
            model_recovery: None,
            pending_stream_delta: String::new(),
            stream_delta_flush_scheduled: false,
            stream_save: Default::default(),
            pending_compaction_status_index: None,
            stream_tail_state: StreamTailState::default(),
            stream_render_state: crate::app::session::stream_render::StreamRenderState::default(),
            input_editor: text_editor::Content::new(),
            model,
            auto_model,
            restore_model_from_history: false,
            reasoning_effort: None,
            tool_selector: SessionToolSelectorState::default(),
            task_mode_enabled: false,
            workflow_mode_enabled: false,
            goal_mode_enabled: true,
            task_mode_priority: "999".to_string(),
            task_mode_model,
            task_mode_executor: None,
            task_mode_reasoning_effort: None,
            task_mode_task_split_mode: crate::app::task::TaskSplitMode::FollowSystem,
            task_mode_subtasks,
            task_mode_subtask_editors,
            agent: None,
            agent_label: None,
            acp_agent: None,
            acp_model_source: Default::default(),
            acp_options_state: Default::default(),
            acp_history_mode: AcpHistoryReplayMode::Discard,
            acp_recent_count: 3,
            full_access_enabled: false,
            full_access_overridden: false,
            full_access_initialized: false,
            full_access_saving: false,
            full_access_save_failed: false,
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
