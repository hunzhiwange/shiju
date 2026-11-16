//! Task-board state ownership and initial values, independent of the desktop App.
use crate::message::{TaskBoardDetailTab, TaskBoardSettingsModalTab};
use iced::widget::{Id, text_editor};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardViewMode {
    #[default]
    Kanban,
}

pub struct State {
    /// 是否显示任务看板
    pub show_task_board: bool,
    /// 任务看板是否正在加载
    pub task_board_loading: bool,
    /// 任务看板当前视图模式
    pub task_board_view_mode: TaskBoardViewMode,
    /// 任务看板任务列表
    pub task_board_tasks: Vec<desktop_task_model::models::Task>,
    /// 本地终态正在持久化、需避免旧网关快照覆盖的任务 ID
    pub task_board_tasks_pending_persistence: HashSet<String>,

    /// 任务看板创建对话框是否打开
    pub task_board_create_modal_open: bool,
    /// 任务看板草稿
    pub task_board_draft: desktop_task_model::models::TaskDraft,
    /// 任务看板上次使用的模型
    pub task_board_last_model: String,
    /// 任务看板上次使用的 ACP 智能体
    pub task_board_last_acp_agent: Option<String>,
    /// 任务看板批量选中的任务 ID 集合
    pub task_board_selected_tasks: HashSet<String>,
    /// 当前启用批量操作的任务状态列
    pub task_board_bulk_active_status: Option<desktop_task_model::models::TaskStatus>,
    /// 任务看板批量设置优先级输入
    pub task_board_bulk_priority_input: String,
    /// 任务看板批量设置模型输入
    pub task_board_bulk_model_input: String,
    /// 任务看板批量设置代理
    pub task_board_bulk_agent: String,
    /// 任务看板批量设置 ACP 智能体
    pub task_board_bulk_acp_agent: Option<String>,
    /// 任务看板批量设置推理强度
    pub task_board_bulk_reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    /// 任务看板批量设置任务拆分模式
    pub task_board_bulk_task_split_mode: desktop_task_model::models::TaskSplitMode,
    /// 任务看板手动批量合并队列
    pub task_board_manual_merge_queue: Vec<String>,
    /// 任务看板选中的任务 ID
    pub task_board_selected_task: Option<String>,
    /// 任务看板正在查看日志的任务
    pub task_board_viewing_logs: Option<desktop_task_model::models::Task>,
    /// 任务看板详情面板当前标签页
    pub task_board_detail_tab: TaskBoardDetailTab,
    /// 任务看板按任务 ID 缓存的内存日志
    pub task_board_log_cache: HashMap<String, Vec<desktop_task_model::models::TaskLogEntry>>,
    /// 任务看板正在编辑的任务 ID
    pub task_board_editing_task_id: Option<String>,
    /// 任务看板拖动中的任务（任务 ID，状态）
    pub task_board_dragging: Option<(String, desktop_task_model::models::TaskStatus)>,
    /// 任务看板待放置的任务（任务 ID，目标状态，位置）
    pub task_board_drag_pending:
        Option<(String, desktop_task_model::models::TaskStatus, iced::Point)>,
    /// 任务看板状态过滤器
    pub task_board_filter_status: Option<desktop_task_model::models::TaskStatus>,
    /// 任务看板优先级过滤器（最小值，最大值）
    pub task_board_filter_priority: Option<(u32, u32)>,
    /// 任务看板是否按优先级排序
    pub task_board_sort_by_priority: bool,
    /// 任务看板是否升序排序
    pub task_board_sort_ascending: bool,
    /// 任务看板设置
    pub task_board_settings: desktop_task_model::models::TaskBoardSettings,
    /// 任务看板执行器状态
    pub task_board_executor: desktop_task_executor::executor::state::TaskExecutorState,
    /// 任务看板执行器是否正在运行
    pub task_board_executor_running: bool,
    /// 任务看板下次刷新时间（毫秒时间戳）
    pub task_board_next_refresh_at_ms: u64,
    /// 任务看板下次调度器触发时间（毫秒时间戳）
    pub task_board_next_scheduler_tick_at_ms: u64,
    /// 任务看板下次自动审核触发时间（毫秒时间戳）
    pub task_board_next_auto_review_tick_at_ms: u64,
    /// 任务看板下次自动晋升触发时间（毫秒时间戳）
    pub task_board_next_auto_promote_tick_at_ms: u64,
    /// 任务看板下次自动合并触发时间（毫秒时间戳）
    pub task_board_next_auto_merge_tick_at_ms: u64,
    /// gateway 任务池调度返回的 worktree 目标容量
    pub task_board_worktree_desired_capacity: Option<u32>,
    /// 任务看板上次日志刷新时间（毫秒时间戳）
    pub task_board_last_log_flush_at_ms: u64,
    /// 任务看板日志扫描游标
    pub task_board_log_scan_cursor: usize,
    /// 任务看板超时扫描游标
    pub task_board_timeout_scan_cursor: usize,
    /// 任务看板调度扫描游标
    pub task_board_schedule_scan_cursor: usize,
    /// 任务看板执行器弹出框是否打开
    pub task_board_executor_popover: bool,
    /// 任务看板批量执行器弹出框是否打开
    pub task_board_bulk_executor_popover: bool,
    /// 任务看板批量模型弹出框是否打开
    pub task_board_bulk_model_popover: bool,
    /// 任务看板 worktree 维护是否进行中
    pub task_board_worktree_maintenance_in_flight: bool,
    /// 任务看板 worktree 快照是否正在加载
    pub task_board_worktree_snapshot_loading: bool,
    /// 任务看板手动 worktree 操作类型（cleanup / merge）
    pub task_board_worktree_manual_action_kind: Option<&'static str>,
    /// 任务看板手动 worktree 二次确认操作类型（cleanup / merge）
    pub task_board_worktree_manual_confirm_kind: Option<&'static str>,
    /// 任务看板手动 worktree 操作日志
    pub task_board_worktree_action_logs: Vec<String>,
    /// 任务看板手动 worktree 操作日志自动隐藏截止时间（毫秒时间戳）
    pub task_board_worktree_action_logs_visible_until_ms: Option<u64>,
    /// 任务看板手动 worktree 操作日志接收器
    pub task_board_worktree_action_log_rx:
        Option<std::sync::mpsc::Receiver<desktop_task_executor::executor::state::TaskLogStream>>,
    /// 任务看板 worktree 池快照
    pub task_board_worktree_snapshot:
        Option<desktop_task_executor::executor::state::WorktreePoolSnapshot>,
    /// 任务看板上次 worktree 快照时间（毫秒时间戳）
    pub task_board_last_worktree_snapshot_at_ms: u64,
    /// 任务看板 worktree 面板是否展开
    pub task_board_worktree_panel_expanded: bool,
    /// 任务看板是否显示像素办公室
    pub task_board_worktree_pixel_office: bool,
    /// 任务看板上下文菜单（任务 ID，X，Y）
    pub task_board_context_menu: Option<(String, f32, f32)>,
    /// 任务看板新建子任务内容
    pub task_board_new_subtask_content: String,
    /// 任务看板描述编辑器
    pub task_board_desc_editor: text_editor::Content,
    /// 任务看板提示词编辑器
    pub task_board_prompt_editor: text_editor::Content,
    /// 任务看板日志查看器（只读，可选择复制）
    pub task_board_logs_editor: text_editor::Content,
    /// 任务看板日志编辑器 ID
    pub task_board_logs_editor_id: Id,
    /// 任务看板日志是否自动跟随到底部
    pub task_board_logs_auto_scroll: bool,
    /// 任务看板日志右键菜单是否打开
    pub task_board_logs_context_menu_open: bool,
    /// 任务看板日志右键菜单位置
    pub task_board_logs_context_menu_pos: Option<(f32, f32)>,
    /// 任务看板日志当前顶部滚动行号
    pub task_board_logs_scroll_top_line: f32,
    /// 任务看板日志滚轮累积的小数行偏移
    pub task_board_logs_scroll_remainder: f32,
    /// 任务看板日志编辑器视口高度
    pub task_board_logs_viewport_height: f32,
    /// 任务看板模型选择弹出框是否打开
    pub task_board_model_popover: bool,
    /// 任务看板创建后是否清除提示词
    pub task_board_clear_prompt_after_create: bool,
    /// 任务看板创建后是否关闭对话框
    pub task_board_close_after_create: bool,
    /// 任务看板编辑后是否关闭详情面板
    pub task_board_close_after_edit: bool,
    /// 任务看板创建是否成功
    pub task_board_create_submit_success: bool,
    /// 任务看板编辑是否成功
    pub task_board_edit_submit_success: bool,
    /// 任务看板设置对话框是否打开
    pub task_board_settings_modal_open: bool,
    /// 任务看板设置对话框当前标签页
    pub task_board_settings_modal_tab: TaskBoardSettingsModalTab,
    /// 任务看板是否为导入模式
    pub task_board_is_import_mode: bool,
    /// 任务看板导入编辑器
    pub task_board_import_editor: text_editor::Content,
    /// 任务看板导入提示词格式
    pub task_board_import_prompt_format: desktop_task_model::models::TaskImportPromptFormat,
    /// 任务看板导入提示词面板是否折叠
    pub task_board_import_prompt_collapsed: bool,
    /// 任务看板每个状态列是否显示纵向滚动条
    pub task_board_column_has_vertical_scrollbar:
        HashMap<desktop_task_model::models::TaskStatus, bool>,
}

impl State {
    pub fn new(cfg: &serde_json::Value, acp_agent: Option<String>, init_ms: u64) -> Self {
        Self {
            // ========== 任务板基础状态 ==========
            show_task_board: false,
            task_board_loading: false,
            task_board_view_mode: TaskBoardViewMode::default(),
            task_board_tasks: Vec::new(),
            task_board_tasks_pending_persistence: std::collections::HashSet::new(),
            task_board_create_modal_open: false,
            task_board_settings_modal_open: false,
            task_board_settings_modal_tab: TaskBoardSettingsModalTab::default(),

            // ========== 任务导入 ==========
            task_board_is_import_mode: false,
            task_board_import_editor: iced::widget::text_editor::Content::new(),
            task_board_import_prompt_format:
                desktop_task_model::models::TaskImportPromptFormat::Json,
            task_board_import_prompt_collapsed: true,
            task_board_column_has_vertical_scrollbar: std::collections::HashMap::new(),

            // ========== 任务草稿和执行 ==========
            task_board_draft: desktop_task_model::models::TaskDraft::default(),
            task_board_last_model: "auto".to_string(),
            task_board_last_acp_agent: acp_agent.clone(),
            task_board_selected_tasks: std::collections::HashSet::new(),
            task_board_bulk_active_status: None,
            task_board_bulk_priority_input: "999".to_string(),
            task_board_bulk_model_input: "auto".to_string(),
            task_board_bulk_agent: desktop_task_model::models::TASK_AGENT_MAIN.to_string(),
            task_board_bulk_acp_agent: acp_agent.clone(),
            task_board_bulk_reasoning_effort: None,
            task_board_bulk_task_split_mode:
                desktop_task_model::models::TaskSplitMode::FollowSystem,
            task_board_manual_merge_queue: Vec::new(),
            task_board_selected_task: None,
            task_board_viewing_logs: None,
            task_board_detail_tab: TaskBoardDetailTab::default(),
            task_board_log_cache: std::collections::HashMap::new(),
            task_board_editing_task_id: None,

            // ========== 任务拖拽和排序 ==========
            task_board_dragging: None,
            task_board_drag_pending: None,
            task_board_filter_status: None,
            task_board_filter_priority: None,
            task_board_sort_by_priority: true,
            task_board_sort_ascending: false,

            // ========== 任务板设置 ==========
            task_board_settings: desktop_task_model::models::TaskBoardSettings::new(),

            // ========== 任务执行器 ==========
            task_board_executor: desktop_task_executor::executor::state::TaskExecutorState::new(),
            task_board_executor_running: false,

            // ========== 任务板定时器 ==========
            // 下次刷新时间（60秒后）
            task_board_next_refresh_at_ms: (init_ms as u64).saturating_add(60_000),
            // 下次调度器tick（1秒后）
            task_board_next_scheduler_tick_at_ms: (init_ms as u64).saturating_add(1_000),
            // 下次自动审查tick（30秒后）
            task_board_next_auto_review_tick_at_ms: (init_ms as u64).saturating_add(30_000),
            // 下次自动提升tick（30秒后）
            task_board_next_auto_promote_tick_at_ms: (init_ms as u64).saturating_add(30_000),
            // 下次自动合并tick（10秒后）
            task_board_next_auto_merge_tick_at_ms: (init_ms as u64).saturating_add(10_000),
            task_board_worktree_desired_capacity: None,
            // 日志刷新和扫描游标
            task_board_last_log_flush_at_ms: 0,
            task_board_log_scan_cursor: 0,
            task_board_timeout_scan_cursor: 0,
            task_board_schedule_scan_cursor: 0,

            // ========== 任务板UI状态 ==========
            task_board_executor_popover: false,
            task_board_bulk_executor_popover: false,
            task_board_bulk_model_popover: false,
            task_board_worktree_maintenance_in_flight: false,
            task_board_worktree_snapshot_loading: false,
            task_board_worktree_manual_action_kind: None,
            task_board_worktree_manual_confirm_kind: None,
            task_board_worktree_action_logs: Vec::new(),
            task_board_worktree_action_logs_visible_until_ms: None,
            task_board_worktree_action_log_rx: None,
            task_board_worktree_snapshot: None,
            task_board_last_worktree_snapshot_at_ms: 0,
            task_board_worktree_panel_expanded: false,
            task_board_worktree_pixel_office: cfg
                .get("task_board_worktree_pixel_office")
                .and_then(|v: &serde_json::Value| v.as_bool())
                .unwrap_or(false),
            task_board_context_menu: None,
            task_board_new_subtask_content: String::new(),
            task_board_desc_editor: iced::widget::text_editor::Content::new(),
            task_board_prompt_editor: iced::widget::text_editor::Content::new(),
            task_board_logs_editor: iced::widget::text_editor::Content::new(),
            task_board_logs_editor_id: iced::widget::Id::new("task_board_logs_editor"),
            task_board_logs_auto_scroll: true,
            task_board_logs_context_menu_open: false,
            task_board_logs_context_menu_pos: None,
            task_board_logs_scroll_top_line: 0.0,
            task_board_logs_scroll_remainder: 0.0,
            task_board_logs_viewport_height: 0.0,
            task_board_model_popover: false,

            // ========== 任务板行为配置 ==========
            task_board_clear_prompt_after_create: cfg
                .get("task_board_clear_prompt_after_create")
                .and_then(|v: &serde_json::Value| v.as_bool())
                .unwrap_or(true),
            task_board_close_after_create: cfg
                .get("task_board_close_after_create")
                .and_then(|v: &serde_json::Value| v.as_bool())
                .unwrap_or(true),
            task_board_close_after_edit: cfg
                .get("task_board_close_after_edit")
                .and_then(|v: &serde_json::Value| v.as_bool())
                .unwrap_or(true),
            task_board_create_submit_success: false,
            task_board_edit_submit_success: false,
        }
    }
}

impl State {
    pub fn apply_preferences(&mut self, cfg: &serde_json::Value) {
        for (key, setting) in [
            ("task_board_worktree_pixel_office", &mut self.task_board_worktree_pixel_office),
            (
                "task_board_clear_prompt_after_create",
                &mut self.task_board_clear_prompt_after_create,
            ),
            ("task_board_close_after_create", &mut self.task_board_close_after_create),
            ("task_board_close_after_edit", &mut self.task_board_close_after_edit),
        ] {
            if let Some(value) = cfg.get(key).and_then(serde_json::Value::as_bool) {
                *setting = value;
            }
        }
    }
}
