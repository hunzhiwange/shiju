/// 最近项目元数据
///
/// 存储最近打开项目的元信息，
/// 包括路径、名称、图标和任务看板设置等。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecentProjectMeta {
    /// 项目路径
    pub path: String,
    /// 项目显示名称
    pub name: String,
    /// 任务看板设置（可选）
    #[serde(default)]
    pub task_board_settings: Option<desktop_task_model::models::TaskBoardSettings>,
    /// 会话是否自动刷新（可选）
    #[serde(default = "default_recent_project_session_auto_refresh")]
    pub session_auto_refresh: bool,
    /// 会话自动刷新间隔秒数（可选）
    #[serde(default = "default_recent_project_session_refresh_interval_seconds")]
    pub session_refresh_interval_seconds: u64,
    /// 项目图标标识符（可选）
    #[serde(default)]
    pub icon: Option<String>,
    /// 项目图标颜色（可选）
    #[serde(default)]
    pub icon_color: Option<String>,
    /// Worktree 启动命令（可选）
    #[serde(default)]
    pub worktree_start_command: Option<String>,
}

pub const fn default_recent_project_session_auto_refresh() -> bool {
    true
}

pub const fn default_recent_project_session_refresh_interval_seconds() -> u64 {
    60
}
