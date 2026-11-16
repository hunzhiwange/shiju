//! 会话运行状态；渲染缓存由聊天视图所有者提供具体类型。
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use iced::widget::text_editor;
use desktop_chat_tools::state::{SessionToolSelectorState, SessionToolSelectorTab};
use crate::message::chat::request::AcpHistoryReplayMode;
use crate::message::chat::stream::tail::StreamTailState;

static GUIDE_HANDOFF_REQUEST_IDS: LazyLock<Mutex<HashSet<u64>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub fn mark_pending_guide_handoff(request_id: u64) {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.insert(request_id);
}

pub fn take_pending_guide_handoff(request_id: u64) -> bool {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.remove(&request_id)
}

pub fn clear_pending_guide_handoff(request_id: u64) {
    let mut pending =
        GUIDE_HANDOFF_REQUEST_IDS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pending.remove(&request_id);
}

#[derive(Debug, Clone)]
pub struct SessionDraft {
    pub project_path: String,
    pub directory: String,
    pub history_len: usize,
}

/// 会话运行时状态
///
/// 管理单个会话的运行时状态，包括请求状态、输入编辑器、
/// 模型配置、任务模式等会话特定的信息。
#[derive(Debug, Clone)]
pub struct SessionRuntimeState<RenderState> {
    pub request_lifecycle: crate::message::chat::request::RequestLifecycle,
    pub draft: Option<SessionDraft>,
    /// Gateway authority: None is a regular session; false is an unstarted fork.
    pub fork_execution_started: Option<bool>,
    /// Lock a submitted fork until a fresh gateway read, including cancellation/errors.
    pub fork_request_submitted: bool,
    /// Do not let inherited binding loads overwrite an explicit fork selection.
    pub fork_agent_selected: bool,
    /// 计划运行子会话由网关提交历史，桌面仅维护实时投影。
    pub is_plan_run_session: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub mcp_app_panels: Vec<desktop_web_runtime::mcp_app_panel::McpAppPanel>,
    /// 是否有未查看的成功响应
    pub has_unseen_success: bool,
    /// 当前请求的恢复事件；不进入模型上下文。
    pub model_recovery: Option<(u64, gateway_chat_stream::GatewayModelRecoveryEvent)>,
    /// 尚未提交到聊天 UI 的流式文本增量。
    pub pending_stream_delta: String,
    /// 是否已经为流式文本增量安排了合并刷新。
    pub stream_delta_flush_scheduled: bool,
    pub stream_save: crate::message::chat::stream::save::StreamSaveState,
    pub pending_compaction_status_index: Option<usize>,
    /// 当前 assistant 尾消息的增量解析、高度和后台预热代次。
    pub stream_tail_state: StreamTailState,
    /// 活动 assistant 正文的 stable-prefix/mutable-tail 增量投影。
    pub stream_render_state: RenderState,
    /// 输入编辑器内容
    pub input_editor: text_editor::Content,
    /// 当前选择的模型标识符
    pub model: String,
    /// 是否自动选择模型
    pub auto_model: bool,
    /// 是否等待从历史步骤恢复该会话自己的模型。
    pub restore_model_from_history: bool,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub tool_selector: SessionToolSelectorState,
    /// 是否启用任务模式
    pub task_mode_enabled: bool,
    /// 是否将本轮聊天包装为临时工作流执行
    pub workflow_mode_enabled: bool,
    /// 是否持续执行并独立验收，直到达到目标
    pub goal_mode_enabled: bool,
    /// 任务模式的优先级
    pub task_mode_priority: String,
    /// 任务模式的模型标识符
    pub task_mode_model: String,
    /// 任务模式使用的 ACP 智能体
    pub task_mode_executor: Option<String>,
    /// 任务模式使用的推理强度；`None` 表示沿用模型或 ACP 智能体默认值
    pub task_mode_reasoning_effort:
        Option<vw_api_chat_identity::ReasoningEffortDto>,
    /// 任务模式的任务拆分策略
    pub task_mode_task_split_mode: desktop_task_model::TaskSplitMode,
    /// 任务模式的子任务列表
    pub task_mode_subtasks: Vec<String>,
    /// 子任务的编辑器内容列表
    pub task_mode_subtask_editors: Vec<text_editor::Content>,
    pub agent: Option<String>,
    pub agent_label: Option<String>,
    pub acp_agent: Option<String>,
    pub acp_model_source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub acp_options_state: crate::message::chat::acp_options::AcpOptionsState,
    pub acp_history_mode: AcpHistoryReplayMode,
    pub acp_recent_count: usize,
    pub full_access_enabled: bool,
    pub full_access_initialized: bool,
    pub full_access_saving: bool,
    pub full_access_save_failed: bool,
    /// 用户操作后不再采用智能体默认值。
    pub full_access_overridden: bool,
    pub last_effective_acp_agent: Option<String>,
    pub acp_rebuild_required: bool,
}

impl<RenderState: Default> Default for SessionRuntimeState<RenderState> {
    fn default() -> Self {
        Self::new()
    }
}

impl<RenderState: Default> SessionRuntimeState<RenderState> {
    pub fn visible_session_tool_selector_tab(&self) -> SessionToolSelectorTab {
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
    pub fn with_defaults(model: String, auto_model: bool) -> Self {
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
            stream_render_state: RenderState::default(),
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
            task_mode_task_split_mode: desktop_task_model::TaskSplitMode::FollowSystem,
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
    pub fn new() -> Self {
        Self::with_defaults("auto".to_string(), true)
    }
}

#[cfg(test)]
mod agent_tests;

pub fn collect_agent_subscription_requests<RenderState>(
    active_session_id: &Option<String>,
    runtime_states: &std::collections::HashMap<String, SessionRuntimeState<RenderState>>,
) -> Vec<std::sync::Arc<crate::message::chat::request::AgentRequest>> {
    use std::sync::Arc;
    let mut requests = Vec::new();
    if let Some(active_id) = active_session_id
        && let Some(request) = runtime_states
            .get(active_id)
            .and_then(|runtime| runtime.request_lifecycle.active_agent_request.as_ref().map(Arc::clone))
    {
        requests.push(request);
    }
    for (session_id, runtime) in runtime_states {
        if active_session_id.as_ref() == Some(session_id) {
            continue;
        }
        if let Some(request) = runtime.request_lifecycle.active_agent_request.as_ref().map(Arc::clone) {
            requests.push(request);
        }
    }
    requests
}

impl<R> SessionRuntimeState<R> {
    pub fn open_draft(&mut self, project_path: String, directory: String) {
        self.input_editor = text_editor::Content::new();
        self.fork_execution_started = None;
        self.fork_request_submitted = false;
        self.fork_agent_selected = false;
        self.acp_options_state.source_pending = None;
        self.acp_options_state.source_request_id = None;
        self.draft = Some(SessionDraft { project_path, directory, history_len: 0 });
    }
    pub fn apply_open_execution_state(&mut self, started: Option<bool>) {
        self.fork_execution_started = started;
        if started != Some(false) { self.fork_agent_selected = false; }
        if !self.request_lifecycle.is_requesting && self.request_lifecycle.active_agent_request.is_none() {
            self.fork_request_submitted = false;
        }
    }
}

impl<R> SessionRuntimeState<R> {
    pub fn restore_open_binding(&mut self, binding: &vw_api_chat_identity::SessionAgentIdentity) {
        if !self.fork_agent_selected {
            self.agent = (binding.agent != desktop_settings_agents::state::MAIN_AGENT_KEY).then(|| binding.agent.clone());
            self.acp_agent = binding.acp_agent.clone();
            self.agent_label = binding.label.clone();
        }
    }
}
