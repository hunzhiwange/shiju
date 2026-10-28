//! # 项目管理模块
//!
//! 本模块提供项目持久化与运行时管理的核心能力，包括：
//! - 最近项目列表与元数据的加载、保存、迁移与解析
//! - 项目切换时终端、会话、预览、标签页等运行时状态的隔离与恢复
//! - 打开项目时的标签页创建、配置加载、Git分支刷新与文件索引异步启动
//!
//! 主要公开函数：
//! - [`load_recent_projects`]/[`save_recent_projects`]: 最近项目路径列表的读写
//! - [`load_recent_projects_meta`]/[`save_recent_projects_meta`]: 最近项目元数据的读写
//! - [`push_recent_project`]: 将路径加入最近列表（去重、置顶、上限截断、自动持久化）
//! - [`ensure_recent_project`]: 将新路径加入最近列表，已存在路径保持原顺序
//!
//! 主要 App 方法：
//! - [`App::open_project`]: 打开指定路径的项目，完成状态切换、配置加载、会话重载、信息拉取
//! - [`App::open_project_and_index`]: 打开项目并在需要时启动后台文件索引
//! - [`App::switch_project_terminal`]: 切换项目对应的终端状态（按项目隔离、不存在则新建）
//! - [`App::refresh_branches`]: 基于当前项目路径刷新 Git 分支列表与当前分支

use super::RecentProjectMeta;
use super::project_dirs;
use super::{Screen, message};
use crate::app::Message;
#[cfg(target_arch = "wasm32")]
use crate::app::config::load_project_chat_preferences_async;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::load_project_chat_preferences;
use crate::app::{App, AppTab};
use iced::Task;

#[derive(Debug, Clone)]
pub struct ProjectBranchSnapshot {
    pub project_path: String,
    pub selected_branch: Option<String>,
    pub branches: Vec<String>,
}

pub fn save_recent_projects_background(recent: Vec<String>) {
    let _ = std::thread::Builder::new()
        .name("vw-save-recent-projects".to_string())
        .spawn(move || save_recent_projects(&recent));
}

pub fn save_recent_projects_meta_background(v: Vec<RecentProjectMeta>) {
    let _ = std::thread::Builder::new()
        .name("vw-save-recent-projects-meta".to_string())
        .spawn(move || save_recent_projects_meta(&v));
}

#[allow(dead_code)]
fn load_project_branches(path: String) -> ProjectBranchSnapshot {
    let selected_branch = super::components::git_panel::current_branch(&path);
    let branches = super::components::git_panel::list_branches(&path).unwrap_or_default();
    ProjectBranchSnapshot { project_path: path, selected_branch, branches }
}

fn project_chat_preferences_task(path: String) -> Task<Message> {
    #[cfg(target_arch = "wasm32")]
    {
        Task::perform(
            async move {
                let preferences =
                    load_project_chat_preferences_async(&path).await.unwrap_or_else(|err| {
                        tracing::warn!(
                            target: "vw_desktop",
                            error = %err,
                            project_path = %path,
                            "failed to load project chat preferences via gateway"
                        );
                        None
                    });
                (path, preferences)
            },
            |(path, preferences)| Message::ProjectChatPreferencesLoaded(path, preferences),
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = path;
        Task::none()
    }
}

pub(crate) fn project_settings_refresh_task(project_path: String) -> Task<Message> {
    Task::batch(vec![
        Task::done(Message::Settings(message::settings::SettingsMessage::Agents(
            message::settings::AgentsMessage::ProjectChanged(project_path),
        ))),
        Task::done(Message::Settings(message::settings::SettingsMessage::SkillsRefresh)),
    ])
}

/// 返回最近项目列表存储路径
///
/// 该路径基于平台数据目录（`data_local_dir`）下的 `recent_projects.json` 文件。
/// 如果无法获取项目目录（例如平台不支持），则返回 [`None`]。
///
/// # 返回值
///
/// - `Some(PathBuf)` - 指向 `recent_projects.json` 的完整路径
/// - `None` - 无法确定项目数据目录时返回
fn recent_projects_path() -> Option<std::path::PathBuf> {
    // 优先使用平台本地数据目录，确保与 XDG/Data Local 等约定一致
    project_dirs()
        .map(|d: directories::ProjectDirs| d.data_local_dir().join("recent_projects.json"))
}

/// 从磁盘加载最近项目路径列表
///
/// 按优先级依次尝试以下候选路径：
/// 1. `data_local_dir/recent_projects.json`（首选）
/// 2. `data_dir/recent_projects.json`
/// 3. `config_dir/recent_projects.json`
///
/// 找到首个可解析的文件后立即返回；如果文件位于非首选路径，会将其迁移到首选路径。
/// 若所有候选路径均不存在或解析失败，则返回空列表。
///
/// # 返回值
///
/// - `Vec<String>` - 最近打开的项目路径列表（已去重与规范化）
pub fn load_recent_projects() -> Vec<String> {
    // 获取项目目录，失败则返回空列表
    let Some(dirs) = project_dirs() else {
        return vec![];
    };

    // 按优先级定义候选路径：local data > data > config
    let candidate_paths: [std::path::PathBuf; 3] = [
        dirs.data_local_dir().join("recent_projects.json"),
        dirs.data_dir().join("recent_projects.json"),
        dirs.config_dir().join("recent_projects.json"),
    ];

    // 依次尝试读取并解析，成功即返回
    for path in candidate_paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(recent) = parse_recent_projects(&content) else {
            continue;
        };

        // 如果是从旧路径加载，则迁移到首选路径
        if path != dirs.data_local_dir().join("recent_projects.json") {
            save_recent_projects_background(recent.clone());
        }

        return recent;
    }

    // 所有候选路径均未找到可解析内容
    vec![]
}

/// 将最近项目路径列表持久化到磁盘
///
/// 写入到平台 `data_local_dir` 下的 `recent_projects.json` 文件。
/// 如果父目录不存在会自动创建；写入失败时静默忽略错误。
///
/// # 参数
///
/// - `recent` - 要保存的项目路径列表（仅包含路径字符串）
pub fn save_recent_projects(recent: &Vec<String>) {
    // 无法获取目标路径时直接返回
    let Some(path) = recent_projects_path() else {
        return;
    };
    // 确保父目录存在
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 序列化为 JSON；失败时回退为空数组
    let content = serde_json::to_string(recent).unwrap_or_else(|_| "[]".to_string());
    let _ = std::fs::write(path, content);
}

/// 解析最近项目列表内容（兼容多种历史格式）
///
/// 支持以下 JSON 格式：
/// 1. 字符串数组：`["/path1", "/path2"]`
/// 2. 元数据对象数组：`[{"path": "/path1", ...}, ...]`
/// 3. 包含 `recent_projects` 字段的对象：`{"recent_projects": [...]}`
///
/// 解析成功后会对结果进行规范化（去重、去空白）。
///
/// # 参数
///
/// - `content` - 从文件读取的原始 JSON 字符串
///
/// # 返回值
///
/// - `Some(Vec<String>)` - 解析成功且经过规范化的路径列表
/// - `None` - 无法识别的格式或解析失败
fn parse_recent_projects(content: &str) -> Option<Vec<String>> {
    // 尝试解析为简单的字符串数组
    if let Ok(v) = serde_json::from_str::<Vec<String>>(content) {
        return Some(normalize_recent_projects(v));
    }

    // 尝试解析为元数据对象数组（旧版本格式）
    if let Ok(v) = serde_json::from_str::<Vec<RecentProjectMeta>>(content) {
        return Some(normalize_recent_projects(v.into_iter().map(|m| m.path).collect()));
    }

    // 尝试解析为通用 JSON 值，以处理更复杂的结构
    let Ok(v) = serde_json::from_str::<serde_json::Value>(content) else {
        return None;
    };

    let mut out: Vec<String> = vec![];

    match v {
        // 直接数组：逐项提取路径
        serde_json::Value::Array(items) => {
            for item in items {
                match item {
                    serde_json::Value::String(s) => out.push(s),
                    // 对象中尝试提取 "path" 字段
                    serde_json::Value::Object(map) => {
                        if let Some(serde_json::Value::String(path)) = map.get("path") {
                            out.push(path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        // 嵌套对象：从 "recent_projects" 字段提取数组
        serde_json::Value::Object(map) => {
            let Some(serde_json::Value::Array(items)) = map.get("recent_projects") else {
                return None;
            };
            for item in items {
                match item {
                    serde_json::Value::String(s) => out.push(s.clone()),
                    serde_json::Value::Object(map) => {
                        if let Some(serde_json::Value::String(path)) = map.get("path") {
                            out.push(path.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    Some(normalize_recent_projects(out))
}

fn recent_project_dir_name(path: &str) -> Option<&str> {
    let path = path.trim().trim_end_matches(['/', '\\']);
    if path.is_empty() {
        return None;
    }

    path.rsplit(['/', '\\']).find(|part| !part.is_empty())
}

pub(crate) fn is_visible_recent_project_path(path: &str) -> bool {
    recent_project_dir_name(path).is_some_and(|name| !name.starts_with('.'))
}

/// 规范化最近项目列表
///
/// 处理逻辑：
/// - 去除每项首尾空白
/// - 过滤空字符串
/// - 过滤隐藏目录项目
/// - 去重（保留首次出现的顺序）
///
/// # 参数
///
/// - `recent` - 原始路径列表
///
/// # 返回值
///
/// - `Vec<String>` - 经过规范化的路径列表
fn normalize_recent_projects(recent: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(recent.len());
    for p in recent {
        let p = p.trim().to_string();
        // 跳过空项
        if p.is_empty() {
            continue;
        }
        // 隐藏目录通常是系统/运行时临时工作区，不展示为用户最近项目
        if !is_visible_recent_project_path(&p) {
            continue;
        }
        // 跳过重复项（保持首次出现顺序）
        if out.iter().any(|x| x == &p) {
            continue;
        }
        out.push(p);
    }
    out
}

fn normalize_recent_projects_meta(recent: Vec<RecentProjectMeta>) -> Vec<RecentProjectMeta> {
    let mut out: Vec<RecentProjectMeta> = Vec::with_capacity(recent.len());
    for mut meta in recent {
        let path = meta.path.trim().to_string();
        if path.is_empty() || !is_visible_recent_project_path(&path) {
            continue;
        }
        if out.iter().any(|item| item.path == path) {
            continue;
        }

        meta.path = path;
        out.push(meta);
    }
    out
}

impl App {
    /// 切换项目对应的终端状态
    ///
    /// 实现项目级终端隔离：每个项目维护独立的终端会话。
    /// 切换时保存旧项目的终端状态到内存映射，并恢复或创建新项目的终端。
    ///
    /// # 参数
    ///
    /// - `previous_project_path` - 前一个项目的路径（如果有），其终端状态会被保存
    /// - `new_project_path` - 新项目的路径（如果有），会恢复或创建对应的终端状态
    ///
    /// # 返回值
    ///
    /// - `true` - 新项目没有缓存的终端状态，创建了全新终端（通常需要初始化）
    /// - `false` - 新项目有缓存状态并已恢复，或路径相同无需切换
    ///
    /// # 行为说明
    ///
    /// - 如果前后路径相同，直接返回 `false`（无操作）
    /// - 保存旧终端时会保留当前可见性、Shell、主题、字体等设置
    /// - 新项目若已有缓存状态则直接恢复；否则创建以该项目目录为工作目录的新终端
    /// - `wasm32` 目标会跳过主题应用以避免平台差异
    pub fn switch_project_terminal(
        &mut self,
        previous_project_path: Option<String>,
        new_project_path: Option<String>,
    ) -> bool {
        // 路径相同则无需切换
        if previous_project_path == new_project_path {
            return false;
        }

        // 保存当前终端的运行时配置，用于创建占位符或新建终端
        let terminal_is_visible = self.terminal.is_visible;
        let terminal_shell = self.terminal.shell;
        let terminal_theme = self.terminal.theme;
        let terminal_font_family = self.terminal.font_family.clone();
        let terminal_font_size = self.terminal.font_size;
        let terminal_height = self.terminal.height;

        // 如果存在前一个项目，保存其终端状态到映射表
        if let Some(prev_path) = previous_project_path {
            let placeholder = crate::app::TerminalState::blank_with_settings(
                false,
                terminal_shell,
                terminal_theme,
                terminal_font_family.clone(),
                terminal_font_size,
                terminal_height,
            );
            let prev_terminal = std::mem::replace(&mut self.terminal, placeholder);
            self.terminals_by_project.insert(prev_path, prev_terminal);
        }

        // 处理新项目路径
        if let Some(path) = new_project_path {
            // 尝试恢复已有终端状态
            if let Some(state) = self.terminals_by_project.remove(&path) {
                let old = std::mem::replace(&mut self.terminal, state);
                self.terminal.absorb_replaced(old);
                if self.terminal.is_visible {
                    let _ = self
                        .terminal
                        .ensure_default_terminal(Some(std::path::PathBuf::from(&path)));
                }
                #[cfg(not(target_arch = "wasm32"))]
                self.terminal.apply_app_theme(&self.app_theme);
                false
            } else {
                // 没有缓存状态：先占位，仅在面板可见时懒创建 PTY
                let mut created = crate::app::TerminalState::blank_with_settings(
                    terminal_is_visible,
                    terminal_shell,
                    terminal_theme,
                    terminal_font_family,
                    terminal_font_size,
                    terminal_height,
                );
                if created.is_visible {
                    let _ = created.ensure_default_terminal(Some(std::path::PathBuf::from(&path)));
                }
                let old = std::mem::replace(&mut self.terminal, created);
                self.terminal.absorb_replaced(old);
                #[cfg(not(target_arch = "wasm32"))]
                self.terminal.apply_app_theme(&self.app_theme);
                true
            }
        } else {
            // 没有新项目路径，创建空白终端（保持设置）
            let blank = crate::app::TerminalState::blank_with_settings(
                false,
                terminal_shell,
                terminal_theme,
                terminal_font_family,
                terminal_font_size,
                terminal_height,
            );
            let old = std::mem::replace(&mut self.terminal, blank);
            self.terminal.absorb_replaced(old);
            false
        }
    }

    pub(crate) fn reset_task_board_project_state(&mut self) {
        self.project_is_git_repo = false;
        self.task_board_loading = false;
        self.task_board_tasks.clear();
        self.task_board_tasks_pending_persistence.clear();
        self.task_board_selected_tasks.clear();
        self.task_board_bulk_active_status = None;
        self.task_board_manual_merge_queue.clear();
        self.task_board_selected_task = None;
        self.task_board_viewing_logs = None;
        self.task_board_detail_tab = Default::default();
        self.task_board_log_cache.clear();
        self.task_board_editing_task_id = None;
        self.task_board_dragging = None;
        self.task_board_drag_pending = None;
        self.task_board_executor = crate::app::task::TaskExecutorState::new();
        self.task_board_executor_running = false;
        self.task_board_next_scheduler_tick_at_ms = 0;
        self.task_board_next_auto_review_tick_at_ms = 0;
        self.task_board_next_auto_promote_tick_at_ms = 0;
        self.task_board_next_auto_merge_tick_at_ms = 0;
        self.task_board_worktree_desired_capacity = None;
        self.task_board_last_log_flush_at_ms = 0;
        self.task_board_log_scan_cursor = 0;
        self.task_board_timeout_scan_cursor = 0;
        self.task_board_schedule_scan_cursor = 0;
        self.task_board_executor_popover = false;
        self.task_board_bulk_executor_popover = false;
        self.task_board_bulk_model_popover = false;
        self.task_board_model_popover = false;
        self.task_board_worktree_maintenance_in_flight = false;
        self.task_board_worktree_snapshot_loading = false;
        self.task_board_worktree_manual_action_kind = None;
        self.task_board_worktree_manual_confirm_kind = None;
        self.task_board_worktree_action_logs.clear();
        self.task_board_worktree_action_logs_visible_until_ms = None;
        self.task_board_worktree_action_log_rx = None;
        self.task_board_worktree_snapshot = None;
        self.task_board_last_worktree_snapshot_at_ms = 0;
        self.task_board_context_menu = None;
        self.task_board_new_subtask_content.clear();
        self.task_board_logs_editor = iced::widget::text_editor::Content::new();
        self.task_board_logs_auto_scroll = true;
        self.task_board_logs_context_menu_open = false;
        self.task_board_logs_context_menu_pos = None;
        self.task_board_logs_scroll_top_line = 0.0;
        self.task_board_logs_scroll_remainder = 0.0;
        self.task_board_logs_viewport_height = 0.0;
        self.task_board_create_submit_success = false;
        self.task_board_edit_submit_success = false;
        self.task_board_is_import_mode = false;
        self.task_board_column_has_vertical_scrollbar.clear();
    }

    /// 打开项目并在需要时启动后台文件索引
    ///
    /// 组合了项目打开与文件索引的异步任务：
    /// - 如果项目已有文件索引缓存，仅刷新 Git 变更文件
    /// - 如果没有索引缓存，会启动后台任务扫描并索引项目文件
    ///
    /// # 参数
    ///
    /// - `path` - 要打开的项目绝对路径
    ///
    /// # 返回值
    ///
    /// - `Task<Message>` - 包含以下任务的批处理任务：
    ///   - 打开项目的任务（来自 [`App::open_project`]）
    ///   - 文件索引任务（仅当需要时）
    ///   - Git 变更文件刷新任务
    pub fn open_project_and_index(&mut self, path: String) -> Task<Message> {
        let open_task = self.open_project(path.clone());
        #[cfg(target_arch = "wasm32")]
        {
            let index_task = message::project::refresh_file_index(self);
            return Task::batch(vec![open_task, index_task]);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // 检查是否已有索引缓存
            if self.has_file_index(&path) {
                let mut tasks = vec![
                    open_task,
                    Task::done(Message::Git(message::GitMessage::RefreshGitPanelData)),
                ];
                let needs_refresh =
                    self.file_index_cache.get(&path).is_some_and(|files| files.is_empty());
                if needs_refresh {
                    tasks.push(message::project::helpers::refresh_file_index(self));
                }
                return Task::batch(tasks);
            }

            // 启动后台索引任务
            let path_clone = path.clone();
            let index_task = Task::perform(
                async move {
                    message::spawn_blocking_opt(move || Some(super::load_file_index(&path_clone)))
                        .await
                        .unwrap_or_default()
                },
                |result| Message::Project(message::ProjectMessage::FileIndexLoaded(result)),
            );
            Task::batch(vec![
                open_task,
                index_task,
                Task::done(Message::Git(message::GitMessage::RefreshGitPanelData)),
            ])
        }
    }

    /// 删除当前项目后打开剩余列表中的第一个项目；没有项目时返回首页。
    pub fn open_first_recent_project_or_home(&mut self) -> Task<Message> {
        if let Some(path) = self.recent_projects.first().cloned() {
            self.open_project_and_index(path)
        } else {
            Task::done(Message::View(message::ViewMessage::GoHome))
        }
    }

    /// 切换工作模式空间，并把当前聊天作为新空间中的未提交会话继续保留。
    pub fn switch_work_space_preserving_chat(&mut self, path: String) -> Task<Message> {
        if self.project_path.as_deref() == Some(path.as_str()) {
            return Task::none();
        }

        self.cache_active_session_chat();
        let carried_chat = self.chat.clone();
        let carried_usage = self.usage.clone();
        let carried_view_state = self.active_session_view_state.clone();
        let mut carried_runtime = self.current_session_runtime();

        let task = self.open_project_and_index(path);

        carried_runtime.is_requesting = false;
        carried_runtime.request_started_ms = 0;
        carried_runtime.submit_anim = 0;
        carried_runtime.has_unseen_success = false;
        carried_runtime.active_agent_request = None;
        carried_runtime.queue.clear();
        carried_runtime.restore_model_from_history = false;
        self.active_session_id = None;
        *self.current_session_runtime_mut() = carried_runtime;
        self.chat = carried_chat;
        self.chat_message_ids = vec![None; self.chat.len()];
        self.usage = carried_usage;
        self.active_session_view_state = carried_view_state;
        self.active_session_view_state.updated_ms = 0;
        self.invalidate_chat_ui_state();

        task
    }

    /// 打开指定路径的项目
    ///
    /// 执行完整的项目切换流程：
    /// - 管理标签页（关闭 Apps 中间页、创建/切换项目标签页）
    /// - 保存并恢复预览标签页状态
    /// - 加载项目特定的任务看板设置与聊天偏好
    /// - 切换终端状态（按项目隔离）
    /// - 重置项目 ID、重载会话、加载项目信息
    /// - 刷新 Git 分支列表、清空展开的文件节点
    /// - 更新最近项目列表
    ///
    /// # 参数
    ///
    /// - `path` - 要打开的项目绝对路径
    ///
    /// # 返回值
    ///
    /// - `Task<Message>` - 包含会话重载与项目信息加载的批处理任务
    pub fn open_project(&mut self, path: String) -> Task<Message> {
        // 如果 Apps 中间页标签存在，则移除
        if let Some(pos) = self.open_tabs.iter().position(|t| t.id == "apps") {
            self.open_tabs.remove(pos);
        }
        // 创建或切换到项目标签页
        let tab_id = format!("project_{}", path); // 使用路径作为唯一标识符
        if !self.open_tabs.iter().any(|t| t.id == tab_id) {
            let title = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&path)
                .to_string();

            self.open_tabs.push(AppTab {
                id: tab_id.clone(),
                title,
                screen: Screen::Project,
                project_path: Some(path.clone()),
            });
        }
        self.active_tab_id = Some(tab_id);

        // 保存前一个项目的预览标签页与激活路径
        let previous_project_path = self.project_path.clone();
        let project_changed = previous_project_path.as_deref() != Some(path.as_str());
        if let Some(prev_path) = &previous_project_path {
            self.project_preview_tabs
                .insert(prev_path.clone(), std::mem::take(&mut self.preview_tabs));
            self.project_preview_active_path
                .insert(prev_path.clone(), self.active_preview_path.take());
        }
        self.project_path = Some(path.clone());
        if project_changed {
            self.project_is_git_repo = false;
            self.agents_settings.project_path = Some(path.clone());
            self.agents_settings.loading = true;
            self.agents_settings.config_ready = false;
            self.reset_task_board_project_state();
        }

        // 加载项目特定的任务看板设置
        let settings = self
            .recent_projects_meta
            .iter()
            .find(|m| m.path == path)
            .and_then(|m| m.task_board_settings.clone())
            .unwrap_or_else(crate::app::task::models::TaskBoardSettings::new)
            .sanitized();
        self.task_board_settings = settings;

        // 恢复当前项目的预览标签页与激活路径（若存在）
        let tabs = self.project_preview_tabs.remove(&path).unwrap_or_default();
        let active = self.project_preview_active_path.remove(&path).unwrap_or_default();
        self.preview_tabs = tabs;
        self.active_preview_path = active;
        self.preview_tab_menu_path = None;
        self.preview_tab_menu_pos = None;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(manager) = self.lsp_manager.as_mut() {
            manager.prestart_for_project(&path);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !self.lsp_disabled
            && let Some(active_preview_path) = self.active_preview_path.clone()
        {
            let _ =
                crate::app::message::preview::lsp::sync_lsp_for_path(self, &active_preview_path);
        }

        // 加载项目特定的聊天模型偏好
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((model, auto_model, acp_agent)) = load_project_chat_preferences(&path) {
            self.apply_project_chat_preferences(model, auto_model, acp_agent);
        }
        // 切换到该项目的终端状态
        self.switch_project_terminal(previous_project_path, self.project_path.clone());
        let reap_task = crate::app::message::terminal::reap_retired_task(self);
        // 使离开前尚未完成的首次发送失效；重开同一项目保留已解析的身份。
        self.agent_stream_id = self.agent_stream_id.wrapping_add(1);
        if project_changed {
            self.project_id = None;
        }
        let reload_task = self.reload_sessions_for_project(Some(path.clone()));

        // 重置 Git 相关状态
        self.git_changed_files.clear();
        self.git_changed_files_loading = false;
        self.git_changed_files_repo_path = None;
        self.git_worktree_options.clear();
        self.selected_git_worktree_directory = None;
        self.git_worktree_options_loading = false;
        self.git_worktree_options_project_path = None;
        self.git_worktree_menu_open = false;

        // 将路径加入最近项目列表；已存在项目保持原位，避免点击切换时侧栏跳动。
        ensure_recent_project(&mut self.recent_projects, path.clone());
        self.screen = Screen::Project;
        self.clear_expanded_files();
        self.refresh_search_panel_file_cache();
        self.refresh_file_search_cache();

        // 清空分支信息并异步刷新，避免切换项目时阻塞 UI
        self.refresh_branches();

        // 为最近项目列表生成显示名称（使用元数据或路径末段）
        self.recent_projects_edits = {
            self.recent_projects
                .iter()
                .map(|p| {
                    if let Some(m) = self.recent_projects_meta.iter().find(|m| &m.path == p) {
                        m.name.clone()
                    } else {
                        std::path::Path::new(p)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(p)
                            .to_string()
                    }
                })
                .collect()
        };
        let deferred_path = path.clone();
        let deferred_task =
            Task::done(Message::Project(message::ProjectMessage::StartDeferredTasks {
                project_path: deferred_path,
            }));
        let settings_refresh_task = project_settings_refresh_task(path.clone());
        let preferences_task = project_chat_preferences_task(path);
        Task::batch(vec![
            reap_task,
            reload_task,
            deferred_task,
            preferences_task,
            settings_refresh_task,
        ])
    }

    /// 刷新当前项目的 Git 分支信息
    ///
    /// 基于当前 `project_path` 更新：
    /// - `selected_branch`: 当前所在分支名称
    /// - `branches`: 分支列表（清空后由后续逻辑填充）
    pub fn refresh_branches(&mut self) {
        if let Some(path) = &self.project_path {
            let _ = path;
            self.selected_branch = None;
            self.branches.clear();
            self.project_updated_at_ms = None;
        }
    }
}

/// 将项目路径加入最近列表（去重、置顶、截断、持久化）
///
/// 操作步骤：
/// 1. 如果路径已存在，先移除旧位置
/// 2. 将路径插入到列表头部
/// 3. 截断列表至最多 10 项
/// 4. 持久化到磁盘
///
/// # 参数
///
/// - `recent` - 可变的最近项目列表引用
/// - `path` - 要添加的项目路径
pub fn push_recent_project(recent: &mut Vec<String>, path: String) {
    // 移除已存在的同名路径（确保置顶）
    if let Some(pos) = recent.iter().position(|p| p == &path) {
        recent.remove(pos);
    }
    // 插入到头部
    recent.insert(0, path);
    // 限制列表长度为 10
    recent.truncate(10);
    // 后台持久化，避免切换项目时阻塞 UI
    save_recent_projects_background(recent.clone());
}

/// 确保项目路径存在于最近列表，已存在时保持原顺序。
///
/// 该函数用于普通项目切换，避免用户点击已有项目时列表位置变化。
pub fn ensure_recent_project(recent: &mut Vec<String>, path: String) {
    if recent.iter().any(|p| p == &path) {
        return;
    }
    recent.insert(0, path);
    recent.truncate(10);
    save_recent_projects_background(recent.clone());
}

/// 返回最近项目元数据存储路径
///
/// 该路径基于平台数据目录（`data_local_dir`）下的 `recent_projects_meta.json` 文件。
/// 如果无法获取项目目录，则返回 [`None`]。
///
/// # 返回值
///
/// - `Some(PathBuf)` - 指向 `recent_projects_meta.json` 的完整路径
/// - `None` - 无法确定项目数据目录时返回
fn recent_projects_meta_path() -> Option<std::path::PathBuf> {
    // 与 recent_projects.json 保持同一目录
    project_dirs()
        .map(|d: directories::ProjectDirs| d.data_local_dir().join("recent_projects_meta.json"))
}

/// 从磁盘加载最近项目元数据列表
///
/// 按优先级依次尝试以下候选路径：
/// 1. `data_local_dir/recent_projects_meta.json`（首选）
/// 2. `data_dir/recent_projects_meta.json`
/// 3. `config_dir/recent_projects_meta.json`
///
/// 找到首个可解析的文件后立即返回；如果文件位于非首选路径，会将其迁移到首选路径。
/// 若所有候选路径均不存在或解析失败，则返回空列表。
///
/// # 返回值
///
/// - `Vec<RecentProjectMeta>` - 最近项目的元数据列表
pub fn load_recent_projects_meta() -> Vec<RecentProjectMeta> {
    // 获取项目目录，失败则返回空列表
    let Some(dirs) = project_dirs() else {
        return vec![];
    };

    // 按优先级定义候选路径
    let candidate_paths = [
        dirs.data_local_dir().join("recent_projects_meta.json"),
        dirs.data_dir().join("recent_projects_meta.json"),
        dirs.config_dir().join("recent_projects_meta.json"),
    ];

    // 依次尝试读取并解析
    for path in candidate_paths {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Vec<RecentProjectMeta>>(&content) else {
            continue;
        };
        let v = normalize_recent_projects_meta(v);

        // 如果是从旧路径加载，则迁移到首选路径
        if path != dirs.data_local_dir().join("recent_projects_meta.json") {
            save_recent_projects_meta_background(v.clone());
        }

        return v;
    }

    // 所有候选路径均未找到可解析内容
    vec![]
}

/// 将最近项目元数据列表持久化到磁盘
///
/// 写入到平台 `data_local_dir` 下的 `recent_projects_meta.json` 文件。
/// 如果父目录不存在会自动创建；写入失败时静默忽略错误。
///
/// # 参数
///
/// - `v` - 要保存的元数据列表引用
pub fn save_recent_projects_meta(v: &Vec<RecentProjectMeta>) {
    // 无法获取目标路径时直接返回
    let Some(path) = recent_projects_meta_path() else {
        return;
    };
    // 确保父目录存在
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // 序列化为 JSON；失败时回退为空数组
    let content = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
    let _ = std::fs::write(path, content);
}

#[cfg(test)]
#[path = "projects_tests.rs"]
mod projects_tests;
