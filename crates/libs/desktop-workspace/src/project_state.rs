//! Project/session navigation and file-index working state.
use crate::project_message::edit_meta::RecentProjectMeta;
use iced::Point;
use std::collections::{HashMap, HashSet};

pub struct State<Message> {
    /// 当前项目是否为 Git 仓库（网关 ProjectInfo.vcs）
    pub project_is_git_repo: bool,
    /// 当前项目唯一标识符
    pub project_id: Option<String>,
    /// 当前项目最近更新时间（毫秒时间戳）
    pub project_updated_at_ms: Option<u64>,
    /// 最近打开的项目路径列表
    pub recent_projects: Vec<String>,
    /// 文件索引缓存（路径 -> 子文件列表）
    pub file_index_cache: HashMap<String, Vec<String>>,
    /// 文件索引版本号，用于搜索缓存失效判断
    pub file_index_revision: u64,
    /// 文件树缓存（路径 -> 目录树模型）
    pub file_tree_model_cache: HashMap<String, desktop_file_tree::model::FileTreeNode>,
    /// 全局搜索面板文件结果缓存查询
    pub search_panel_file_cache_query: String,
    /// 全局搜索面板文件结果缓存项目路径
    pub search_panel_file_cache_project_path: Option<String>,
    /// 全局搜索面板文件结果缓存索引版本
    pub search_panel_file_cache_revision: u64,
    /// 全局搜索面板文件结果缓存
    pub search_panel_file_cache_results: Vec<String>,
    /// 项目会话映射（项目路径 -> 会话列表）
    pub project_sessions: HashMap<String, Vec<session_info::Info>>,
    /// 项目会话加载计数（用于分页）
    pub project_session_load_counts: HashMap<String, usize>,
    /// 左侧普通会话树的展开会话 ID 集合（不持久化）
    pub session_tree_expanded: HashSet<String>,
    /// 正在加载会话的项目路径集合
    pub project_sessions_loading: HashSet<String>,
    /// 各项目会话列表是否显示纵向滚动条
    pub project_session_has_vertical_scrollbar: HashMap<String, bool>,
    /// 会话菜单的目标会话 ID
    pub session_menu_id: Option<String>,
    /// 会话菜单的锚点位置
    pub session_menu_anchor: Option<Point>,
    /// 项目工具菜单的文件路径
    pub project_tools_menu_path: Option<String>,
    /// 新建会话选择器的项目路径
    pub new_session_picker_project: Option<String>,
    /// 是否正在等待新会话创建完成
    pub new_session_creating: bool,
    /// 新建会话选择器的选项列表（显示名，值）
    pub new_session_picker_options: Vec<(String, String)>,
    /// 项目 worktree 启用状态映射
    pub project_worktree_enabled: HashMap<String, bool>,
    /// 等待保存的 Worktree 更改，按操作顺序提交。
    pub project_worktree_save_queue: std::collections::VecDeque<iced::Task<Message>>,
    pub project_worktree_save_running: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub project_worktree_save_idle: Option<tokio::sync::watch::Sender<bool>>,
    /// 新建会话的上次选择目录
    pub new_session_last_directory: Option<String>,
    /// 新建会话的 worktree 名称
    pub new_session_worktree_name: String,
    /// 新建会话确认删除的目录路径
    pub new_session_confirm_delete_directory: Option<String>,
    /// 新建会话强制删除的目录路径
    pub new_session_force_delete_directory: Option<String>,
    /// 新建会话删除操作的错误信息
    pub new_session_delete_error: Option<String>,
    /// 新建会话确认重置的目录路径
    pub new_session_confirm_reset_directory: Option<String>,
    /// 新建会话重置操作的错误信息
    pub new_session_reset_error: Option<String>,
    /// 最近项目编辑列表
    pub recent_projects_edits: Vec<String>,
    /// 确认删除的最近项目索引
    pub recent_project_delete_confirm_idx: Option<usize>,
    /// 最近项目列表的搜索词
    pub recent_project_query: String,
    /// 确认从列表移除的最近项目路径
    pub recent_project_remove_confirm_path: Option<String>,
    /// 最近项目元数据列表
    pub recent_projects_meta: Vec<RecentProjectMeta>,
    /// 展开的文件树路径列表
    pub file_tree_expanded: Vec<String>,
    /// 展开的文件树路径集合（用于快速查询）
    pub file_tree_expanded_set: HashSet<String>,
    /// 悬停的最近项目路径
    pub hovered_recent_project: Option<String>,
}
impl<Message> State<Message> {
    pub fn new(
        cfg: &serde_json::Value,
        recent_projects: Vec<String>,
        recent_meta: Vec<RecentProjectMeta>,
        cfg_project_worktree_enabled: HashMap<String, bool>,
    ) -> Self {
        let recent_projects_edits = {
            let recent = &recent_projects;
            recent.iter().map(|p| crate::basic::display_name_for_path(&recent_meta, p)).collect()
        };
        let cfg_new_session_last_directory = cfg
            .get("new_session_last_directory")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let cfg_file_tree_expanded = cfg
            .get("file_tree_expanded")
            .and_then(serde_json::Value::as_array)
            .map(|paths| {
                paths
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            project_is_git_repo: false,
            project_id: None,
            project_updated_at_ms: None,
            recent_projects,
            file_index_cache: HashMap::new(),
            file_index_revision: 0,
            file_tree_model_cache: HashMap::new(),
            search_panel_file_cache_query: String::new(),
            search_panel_file_cache_project_path: None,
            search_panel_file_cache_revision: 0,
            search_panel_file_cache_results: Vec::new(),
            project_sessions: std::collections::HashMap::new(),
            project_session_load_counts: std::collections::HashMap::new(),
            session_tree_expanded: std::collections::HashSet::new(),
            project_sessions_loading: std::collections::HashSet::new(),
            project_session_has_vertical_scrollbar: std::collections::HashMap::new(),
            session_menu_id: None,
            session_menu_anchor: None,
            project_tools_menu_path: None,
            new_session_picker_project: None,
            new_session_creating: false,
            new_session_picker_options: Vec::new(),
            project_worktree_enabled: cfg_project_worktree_enabled,
            project_worktree_save_queue: Default::default(),
            project_worktree_save_running: false,
            #[cfg(not(target_arch = "wasm32"))]
            project_worktree_save_idle: None,
            new_session_last_directory: cfg_new_session_last_directory,
            new_session_worktree_name: String::new(),
            new_session_confirm_delete_directory: None,
            new_session_force_delete_directory: None,
            new_session_delete_error: None,
            new_session_confirm_reset_directory: None,
            new_session_reset_error: None,
            recent_projects_edits,
            recent_project_delete_confirm_idx: None,
            recent_project_query: String::new(),
            recent_project_remove_confirm_path: None,
            recent_projects_meta: recent_meta,
            file_tree_expanded: cfg_file_tree_expanded.clone(),
            file_tree_expanded_set: cfg_file_tree_expanded.into_iter().collect(),
            hovered_recent_project: None,
        }
    }
}

impl<Message> State<Message> {
    pub fn apply_preferences(&mut self, cfg: &serde_json::Value) {
        if let Some(paths) = cfg.get("file_tree_expanded").and_then(serde_json::Value::as_array) {
            self.file_tree_expanded =
                paths.iter().filter_map(|value| value.as_str().map(str::to_owned)).collect();
            self.file_tree_expanded_set = self.file_tree_expanded.iter().cloned().collect();
        }
        if let Some(value) = cfg.get("new_session_last_directory") {
            self.new_session_last_directory = value.as_str().map(str::to_owned);
        }
    }
}
