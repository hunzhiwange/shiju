//! Owned session draft, request and history state. Render/cache types follow the existing generic runtime ports.
use crate::message::chat::request::{AcpHistoryReplayMode, ChatSendBehavior};
use iced::widget::{Id, text_editor};
use std::collections::{HashMap, HashSet};

pub struct State<R, S> {
    /// 输入文本（旧版，已废弃）
    pub input_text: String,
    /// 输入编辑器内容
    pub input_editor: text_editor::Content,
    /// 当前选择的模型标识符
    pub model: String,
    /// 是否自动选择模型
    pub auto_model: bool,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub acp_agent: Option<String>,
    pub acp_history_mode: AcpHistoryReplayMode,
    pub acp_recent_count: usize,
    pub chat_send_behavior: ChatSendBehavior,
    pub acp_agents: Vec<String>,
    /// 附加文件列表
    pub files: Vec<String>,
    /// 是否正在处理请求
    pub is_requesting: bool,
    /// 提交按钮动画计数器
    pub submit_anim: u8,
    /// 当前活跃的 Agent 请求
    pub active_agent_request: Option<crate::message::chat::request::AgentRequest>,
    /// Agent 流式响应的唯一标识符
    pub agent_stream_id: u64,
    /// 请求队列
    pub queue: Vec<crate::message::chat::request::QueueItem>,
    /// 问题对话框关联的请求 ID
    pub question_modal_request_id: Option<String>,
    /// 当前问题对话框承载的请求数据
    pub question_modal_request: Option<question_types::Request>,
    /// 问题对话框的答案选项列表（每个问题多个选项）
    pub question_modal_answers: Vec<Vec<String>>,
    /// 问题对话框的自定义答案输入
    pub question_modal_custom: Vec<String>,
    /// 权限对话框关联的请求 ID
    pub permission_modal_request_id: Option<String>,
    /// 当前权限对话框承载的请求数据
    pub permission_modal_request: Option<gateway_interaction::PendingPermissionRequestDto>,
    /// 当前轮询到的全部待审批权限请求
    pub permission_modal_requests: Vec<gateway_interaction::PendingPermissionRequestDto>,
    /// 会话标题上次点击时间（用于双击检测）
    pub session_title_last_click: Option<web_time::Instant>,
    /// 最近一次调用的日志文件路径
    pub last_call_log_path: Option<String>,
    /// 最近一次会话快照文件路径
    pub last_session_snapshot_path: Option<String>,
    /// 会话信息列表
    pub sessions: Vec<session_info::Info>,
    /// 会话预览映射（会话 ID -> 预览文本），缓存避免每帧查 DB
    pub session_previews: HashMap<String, String>,
    /// 当前活跃会话的唯一标识符
    pub active_session_id: Option<String>,
    /// 会话聊天消息缓存（会话 ID -> 消息列表）
    pub session_chat_cache: HashMap<String, super::cache::SharedChatMessages>,
    /// 最近访问会话的可移动视图工作集缓存
    pub session_view_cache: super::view_cache::SessionViewCache<S>,
    /// 当前会话的有界历史页与活动消息覆盖层。
    pub paged_chat_timeline: Option<desktop_session::session::chat_timeline::PagedChatTimeline>,
    /// 会话消息 ID 缓存（会话 ID -> 消息 ID 列表）
    pub session_chat_message_id_cache: HashMap<String, Vec<Option<String>>>,
    /// 会话历史重置边界（会话 ID -> 重置时保留的消息 ID 列表）
    pub session_history_reset_boundaries: desktop_session::session::cache::HistoryResetBoundaries,
    /// 会话运行时状态映射（会话 ID -> 运行时状态）
    pub session_runtime_states: HashMap<String, super::agent::SessionRuntimeState<R>>,
    /// 持久化的会话推理强度；缺省表示使用模型默认值。
    pub session_reasoning_efforts:
        HashMap<String, Option<vw_api_chat_identity::ReasoningEffortDto>>,
    pub last_llm_full_access: Option<bool>,
    /// 已归档的会话 ID 集合
    pub archived_session_ids: HashSet<String>,
    pub chat_context_menu:
        desktop_session::message::chat::input::context_menus::MessageContextMenuState,
    pub input_context_menu:
        desktop_session::message::chat::input::context_menus::InputContextMenuState,
    /// 当前打开的“重置到此点”菜单消息索引
    pub chat_reset_menu_idx: Option<usize>,
    /// 分叉对话框及待提交的新会话输入
    pub chat_fork_dialog: Option<crate::message::chat::fork::ForkSessionDialog>,
    pub input_popovers: desktop_session::message::chat::input::popovers::InputPopoverState,
    pub quick_message_handoff: Option<crate::message::chat::git_handoff::QuickMessageHandoffState>,
    /// 输入编辑器 ID
    pub input_editor_id: Id,
    /// 正在重命名的会话 ID
    pub session_rename_id: Option<String>,
    /// 会话重命名输入值
    pub session_rename_value: String,
}
impl<R: Default, S: super::view_cache::ViewSnapshot> State<R, S> {
    pub fn new(
        cfg: &serde_json::Value,
        acp_agent: Option<String>,
        acp_agents: Vec<String>,
    ) -> Self {
        let cfg_model = cfg
            .get("model")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("auto")
            .to_string();
        let cfg_auto_model =
            cfg.get("auto_model").and_then(|v: &serde_json::Value| v.as_bool()).unwrap_or(true);
        let cfg_acp_history_mode = crate::message::chat::request::AcpHistoryReplayMode::from_str(
            cfg.get("acp_history_strategy")
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("discard"),
        );
        let cfg_acp_recent_count = cfg
            .get("acp_history_recent_count")
            .and_then(|v: &serde_json::Value| v.as_u64())
            .map(|value: u64| value.clamp(1, 20) as usize)
            .unwrap_or(3);
        let cfg_chat_send_behavior = cfg
            .get("chat_send_behavior")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(crate::message::chat::request::ChatSendBehavior::from_str)
            .unwrap_or(crate::message::chat::request::ChatSendBehavior::Queue);

        Self {
            input_text: String::new(),
            input_editor: iced::widget::text_editor::Content::new(),
            model: cfg_model.clone(),
            auto_model: cfg_auto_model,
            reasoning_effort: None,
            acp_agent,
            acp_history_mode: cfg_acp_history_mode,
            acp_recent_count: cfg_acp_recent_count,
            chat_send_behavior: cfg_chat_send_behavior,
            acp_agents,
            files: vec![],
            is_requesting: false,
            submit_anim: 0,
            active_agent_request: None,
            agent_stream_id: 0,
            queue: vec![],
            question_modal_request_id: None,
            question_modal_request: None,
            question_modal_answers: vec![],
            question_modal_custom: vec![],
            permission_modal_request_id: None,
            permission_modal_request: None,
            permission_modal_requests: vec![],
            session_title_last_click: None,
            last_call_log_path: None,
            last_session_snapshot_path: None,
            sessions: Vec::new(),
            session_previews: HashMap::new(),
            active_session_id: None,
            session_chat_cache: HashMap::new(),
            session_view_cache: super::view_cache::SessionViewCache::<S>::new(),
            paged_chat_timeline: None,
            session_chat_message_id_cache: HashMap::new(),
            session_history_reset_boundaries: Default::default(),
            session_runtime_states: std::collections::HashMap::from([(
                "__empty__".to_string(),
                super::agent::SessionRuntimeState::<R>::with_defaults(
                    cfg_model.clone(),
                    cfg_auto_model,
                ),
            )]),
            session_reasoning_efforts:
                desktop_config::config_desktop::session_reasoning_efforts_from_config(&cfg),
            last_llm_full_access: None,
            archived_session_ids: HashSet::new(),
            chat_context_menu: Default::default(),
            input_context_menu: Default::default(),
            chat_reset_menu_idx: None,
            chat_fork_dialog: None,
            input_popovers: Default::default(),
            quick_message_handoff: None,
            input_editor_id: iced::widget::Id::new("input_editor"),
            session_rename_id: None,
            session_rename_value: String::new(),
        }
    }
}

impl<R: Default, S: super::view_cache::ViewSnapshot> State<R, S> {
    pub fn restore_access_preferences(&mut self, cfg: &serde_json::Value) {
        let mut efforts =
            desktop_config::config_desktop::session_reasoning_efforts_from_config(&cfg);
        // 启动配置返回前发生的本地选择（包括默认值）优先。
        efforts.extend(self.session_reasoning_efforts.clone());
        for (id, runtime) in &mut self.session_runtime_states {
            if let Some(effort) = efforts.get(id) {
                runtime.reasoning_effort = *effort;
            }
        }
        self.session_reasoning_efforts = efforts;
        if let Some(full_access) =
            cfg.get("last_llm_full_access").and_then(serde_json::Value::as_bool)
        {
            self.last_llm_full_access = Some(full_access);
            if let Some(runtime) = self.session_runtime_states.get_mut("__empty__")
                && runtime.acp_agent.is_none()
                && !runtime.full_access_overridden
            {
                runtime.full_access_enabled = full_access;
                runtime.full_access_initialized = true;
            }
        }
    }
    pub fn restore_history_preferences(&mut self, cfg: &serde_json::Value) {
        if let Some(value) = cfg.get("chat_send_behavior").and_then(serde_json::Value::as_str) {
            self.chat_send_behavior =
                crate::message::chat::request::ChatSendBehavior::from_str(value);
        }
        let history_mode = cfg
            .get("acp_history_strategy")
            .and_then(serde_json::Value::as_str)
            .map(crate::message::chat::request::AcpHistoryReplayMode::from_str);
        let history_count = cfg
            .get("acp_history_recent_count")
            .and_then(serde_json::Value::as_u64)
            .map(|count| count.clamp(1, 20) as usize);
        if let Some(mode) = history_mode {
            self.acp_history_mode = mode;
        }
        if let Some(count) = history_count {
            self.acp_recent_count = count;
        }
        if history_mode.is_some() || history_count.is_some() {
            for runtime in self.session_runtime_states.values_mut() {
                runtime.acp_history_mode = self.acp_history_mode;
                runtime.acp_recent_count = self.acp_recent_count;
            }
        }
    }
}

impl<R, S> State<R, S> {
    pub fn apply_project_defaults(
        &mut self,
        model: String,
        auto_model: bool,
        acp_agent: Option<String>,
    ) {
        self.model = model;
        self.auto_model = auto_model;
        if self.active_session_id.is_none() {
            self.acp_agent = acp_agent;
        }
    }
}
