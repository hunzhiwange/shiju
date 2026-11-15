//! 项目编辑草稿、局部事件与窄输入边界。
use iced::widget::text_editor;
use desktop_ui_controls::color_picker::ColorFormat;
use desktop_task_model::TaskBoardSettings;
pub use super::edit_meta::{RecentProjectMeta, default_recent_project_session_auto_refresh, default_recent_project_session_refresh_interval_seconds};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEditTab {
    General,
    Launch,
    Refresh,
    Scheduling,
}
#[derive(Debug)]
pub struct Editor {
    /// 正在编辑的项目路径
    pub project_edit_path: Option<String>,
    /// 项目设置当前选中的标签页
    pub project_edit_tab: ProjectEditTab,
    /// 项目编辑名称输入
    pub project_edit_name: String,
    /// 项目图标标识符
    pub project_edit_icon: String,
    /// 项目图标是否被悬停
    pub project_edit_icon_hovered: bool,
    /// 项目图标颜色（十六进制）
    pub project_edit_icon_color: String,
    /// 项目图标颜色选择器是否打开
    pub project_edit_icon_color_picker_open: bool,
    /// 项目图标颜色格式
    pub project_edit_icon_color_format: ColorFormat,
    /// 项目启动脚本
    pub project_edit_start_script: String,
    /// 项目启动脚本编辑器内容
    pub project_edit_start_script_editor: text_editor::Content,
    /// 项目是否启用 worktree
    pub project_edit_worktree_enabled: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub project_edit_worktree_dirty: bool,
    /// 项目任务看板设置
    pub project_edit_task_board_settings: desktop_task_model::TaskBoardSettings,
    /// 项目最大并发数输入
    pub project_edit_max_concurrent_input: String,
    /// 项目任务看板自动刷新开关
    pub project_edit_task_board_auto_refresh: bool,
    /// 项目会话自动刷新开关
    pub project_edit_session_auto_refresh: bool,
    /// 项目会话自动刷新间隔秒数输入
    pub project_edit_session_refresh_interval_seconds_input: String,
    /// 项目任务看板自动刷新间隔秒数输入
    pub project_edit_task_board_refresh_interval_seconds_input: String,
    /// 项目任务调度 tick 间隔秒数输入
    pub project_edit_task_board_scheduler_tick_interval_seconds_input: String,
    /// 项目任务池自动执行 tick 间隔秒数输入
    pub project_edit_task_board_auto_promote_tick_interval_seconds_input: String,
    /// 项目失败重试分钟数输入
    pub project_edit_failed_retry_minutes_input: String,
    /// 项目失败最大重试次数输入
    pub project_edit_failed_retry_max_attempts_input: String,
    /// 项目运行超时分钟数输入
    pub project_edit_running_timeout_minutes_input: String,
    /// 项目 PR 提交停滞超时秒数输入
    pub project_edit_pr_submitted_stall_timeout_seconds_input: String,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            project_edit_path: None,
            project_edit_tab: ProjectEditTab::General,
            project_edit_name: String::new(),
            project_edit_icon: String::new(),
            project_edit_icon_hovered: false,
            project_edit_icon_color: String::new(),
            project_edit_icon_color_picker_open: false,
            project_edit_icon_color_format: desktop_ui_controls::color_picker::ColorFormat::Hex,
            project_edit_start_script: String::new(),
            project_edit_start_script_editor: iced::widget::text_editor::Content::new(),
            project_edit_worktree_enabled: false,
            #[cfg(not(target_arch = "wasm32"))]
            project_edit_worktree_dirty: false,
            project_edit_task_board_settings: desktop_task_model::TaskBoardSettings::new(),
            project_edit_max_concurrent_input: String::new(),
            project_edit_task_board_auto_refresh: true,
            project_edit_session_auto_refresh:
                default_recent_project_session_auto_refresh(),
            project_edit_session_refresh_interval_seconds_input: String::new(),
            project_edit_task_board_refresh_interval_seconds_input: String::new(),
            project_edit_task_board_scheduler_tick_interval_seconds_input: String::new(),
            project_edit_task_board_auto_promote_tick_interval_seconds_input: String::new(),
            project_edit_failed_retry_minutes_input: String::new(),
            project_edit_failed_retry_max_attempts_input: String::new(),
            project_edit_running_timeout_minutes_input: String::new(),
            project_edit_pr_submitted_stall_timeout_seconds_input: String::new(),

        }
    }
}
#[derive(Debug, Clone)]
pub enum Event {
    ProjectToolsMenuToggled(String),
    CurrentProjectMenuToggled(String),
    ProjectToolsMenuClosed,
    ProjectEditOpened(String),
    ProjectEditTabSelected(ProjectEditTab),
    ProjectEditNameChanged(String),
    ProjectEditIconChanged(String),
    ProjectEditIconHovered(bool),
    ProjectEditIconPickFile,
    ProjectEditIconFilePicked(Option<String>),
    ProjectEditIconColorChanged(String),
    ProjectEditIconColorPresetSelected(String),
    ProjectEditIconColorPickerToggled,
    ProjectEditIconColorPickerClosed,
    ProjectEditIconColorFormatChanged(ColorFormat),
    ProjectEditStartScriptChanged(String),
    ProjectEditStartScriptEditorAction(text_editor::Action),
    ProjectEditWorktreeToggled(bool),
    ProjectEditTaskBoardAutoRefreshToggled(bool),
    ProjectEditSessionAutoRefreshToggled(bool),
    ProjectEditMaxConcurrentInputChanged(String),
    ProjectEditSessionRefreshIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(String),
    ProjectEditFailedRetryMinutesInputChanged(String),
    ProjectEditFailedRetryMaxAttemptsInputChanged(String),
    ProjectEditRunningTimeoutMinutesInputChanged(String),
    ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(String),
    ProjectEditMaxConcurrentChanged(u32),
    ProjectEditSessionRefreshIntervalSecondsChanged(u64),
    ProjectEditTaskBoardRefreshIntervalSecondsChanged(u64),
    ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(u64),
    ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(u64),
    ProjectEditFailedRetryMinutesChanged(u32),
    ProjectEditFailedRetryMaxAttemptsChanged(u32),
    ProjectEditRunningTimeoutMinutesChanged(u32),
    ProjectEditPrSubmittedStallTimeoutSecondsChanged(u32),
    ProjectEditRecycleWorktreeOnTaskFinishToggled(bool),
    ProjectEditWorkflowIdChanged(String),
    ProjectEditSaved,
    ProjectEditRuntimeSaved(Result<(), String>),
    ProjectEditCanceled,
    RecentRevealPressed(String),
    RecentRemovePressed(String),
    RecentRemoveCanceled,
    RecentRemoveConfirmed(String),
}
/// 项目编辑实际使用的借用；编辑草稿由 Editor 唯一持有。
pub struct State<'a> {
    pub editor: &'a mut Editor,
    pub hovered_recent_project: &'a mut Option<String>,
    pub project_path: &'a mut Option<String>,
    pub project_tools_menu_path: &'a mut Option<String>,
    pub project_worktree_enabled: &'a mut std::collections::HashMap<String, bool>,
    pub recent_project_remove_confirm_path: &'a mut Option<String>,
    pub recent_projects: &'a mut Vec<String>,
    pub recent_projects_edits: &'a mut Vec<String>,
    pub recent_projects_meta: &'a mut Vec<RecentProjectMeta>,
    pub session_menu_anchor: &'a mut Option<iced::Point>,
    pub session_menu_id: &'a mut Option<String>,
    pub task_board_settings: &'a mut TaskBoardSettings,
    pub work_current_project_menu: &'a mut Option<String>,
}
