use super::*;
use crate::app::TodoPanelPlacement;
use iced::Task;
use iced::Theme;
use std::collections::{HashMap, HashSet};

mod settings;

fn build_multimodal_settings(
    full_agent_cfg: &vw_config_types::config::Config,
) -> super::state::MultimodalSettingsState {
    let multimodal_cfg = &full_agent_cfg.multimodal;

    super::state::MultimodalSettingsState {
        max_images: multimodal_cfg.max_images.clamp(1, 16) as u32,
        max_image_size_mb: multimodal_cfg.max_image_size_mb.clamp(1, 20) as u32,
        allow_remote_fetch: multimodal_cfg.allow_remote_fetch,
        ocr_enabled: multimodal_cfg.ocr.enabled,
        model_supports_images: multimodal_cfg.ocr.model_supports_images,
        ocr_model: multimodal_cfg
            .ocr
            .model
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string(),
        save_error: None,
    }
}

#[cfg(test)]
#[path = "new/settings_tests.rs"]
mod settings_tests;

#[cfg(test)]
fn build_task_board_settings(_cfg: &serde_json::Value) -> crate::app::task::TaskBoardSettings {
    crate::app::task::TaskBoardSettings::new()
}

impl App {
    pub(crate) fn apply_bootstrap_agent_config(
        &mut self,
        config: &vw_config_types::config::Config,
    ) {
        settings::apply_bootstrap_agent_config(self, config);
    }

    /// 创建并初始化应用程序实例
    ///
    /// 此方法是应用程序的入口点，负责完成所有初始化工作。
    ///
    /// # 初始化流程
    ///
    /// 1. **配置加载**: 从配置文件加载应用程序、系统设置和各子系统配置
    /// 2. **时间戳初始化**: 获取当前时间并格式化为多种表示形式
    /// 3. **UI状态初始化**: 设置终端、主题、编辑器等UI组件的初始状态
    /// 4. **外部应用检测**: 检测系统中可用的外部编辑器和终端应用
    /// 5. **会话初始化**: 加载历史会话和归档会话ID
    /// 6. **子系统配置**: 初始化心跳、定时任务、调度器、安全等子系统配置
    ///
    /// # 返回值
    ///
    /// 返回一个元组：
    /// - `Self`: 完全初始化的应用程序实例
    /// - `Task<Message>`: 初始任务（当前为空任务）
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let (app, task) = App::new();
    /// // app 已经完成所有初始化
    /// ```
    pub fn new() -> (Self, Task<Message>) {
        // 加载应用程序主配置文件
        #[cfg(not(target_arch = "wasm32"))]
        let cfg = config::load_app_config();
        // 加载系统设置配置（包含终端、编辑器等用户偏好设置）
        #[cfg(target_arch = "wasm32")]
        let cfg = serde_json::json!({});
        #[cfg(not(target_arch = "wasm32"))]
        let mut system_settings_cfg = config::load_system_settings_config();
        #[cfg(target_arch = "wasm32")]
        let mut system_settings_cfg = vw_config_types::ui::AppSystemSettingsConfig::default();
        vw_config_types::ui::upgrade_legacy_quick_message_actions(
            &mut system_settings_cfg.quick_messages,
        );
        vw_config_types::ui::append_missing_builtin_general_quick_messages(
            &mut system_settings_cfg.quick_messages,
        );
        vw_config_types::ui::append_missing_builtin_git_quick_messages(
            &mut system_settings_cfg.quick_messages,
        );
        let init_ms = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            }
            #[cfg(target_arch = "wasm32")]
            {
                web_time::SystemTime::now()
                    .duration_since(web_time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            }
        };
        // 解析应用程序配置中的各个选项
        // 终端显示开关
        let cfg_show_terminal =
            cfg.get("show_terminal").and_then(|v: &serde_json::Value| v.as_bool()).unwrap_or(false);
        // 默认使用的AI模型
        let cfg_model = cfg
            .get("model")
            .and_then(|v: &serde_json::Value| v.as_str())
            .unwrap_or("auto")
            .to_string();
        // 是否自动选择模型
        let cfg_auto_model =
            cfg.get("auto_model").and_then(|v: &serde_json::Value| v.as_bool()).unwrap_or(true);
        let cfg_acp_agent = cfg
            .get("acp_agent")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(ToString::to_string);
        let cfg_acp_history_mode = crate::app::state::AcpHistoryReplayMode::from_str(
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
            .map(crate::app::state::ChatSendBehavior::from_str)
            .unwrap_or(crate::app::state::ChatSendBehavior::Queue);
        let cfg_workspace_mode = crate::app::state::WorkspaceMode::from_config(
            cfg.get("workspace_mode").and_then(|value| value.as_str()),
        );
        // 是否启用自动最大化模式
        let cfg_auto_max_mode =
            cfg.get("auto_max_mode").and_then(|v: &serde_json::Value| v.as_bool()).unwrap_or(false);
        // 文件树中已展开的路径列表
        let cfg_file_tree_expanded = cfg
            .get("file_tree_expanded")
            .and_then(|v: &serde_json::Value| v.as_array())
            .map(|arr: &Vec<serde_json::Value>| {
                arr.iter()
                    .filter_map(|e: &serde_json::Value| e.as_str().map(|s: &str| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        // 文件树右键菜单状态（初始为空）
        let file_tree_menu_path = None;
        let file_tree_menu_anchor = None;
        let file_tree_menu_source = None;

        // 解析终端和主题相关配置
        // 终端Shell类型（支持zsh和bash）
        let cfg_shell = match system_settings_cfg.terminal_shell.as_str() {
            "zsh" => Shell::Zsh,
            _ => Shell::Bash,
        };
        // 终端主题（默认跟随系统）
        let cfg_term_theme = TerminalTheme::System;
        // 终端字体配置
        let cfg_term_font_family = system_settings_cfg.terminal_font_family.clone();
        let cfg_term_font_size = system_settings_cfg.terminal_font_size;
        // 应用程序主题（启动时轻量种子内置预设，失败仅 warn）
        if let Some(root) = crate::app::theme_catalog::current_themes_root() {
            if let Err(err) = crate::app::theme_catalog::ensure_bundled_presets(&root) {
                tracing::warn!(
                    target: "vw_desktop",
                    error = %err,
                    "failed to seed bundled theme presets on startup"
                );
            }
        }
        let theme_store = crate::app::theme_catalog::theme_store_from_home();
        let (cfg_app_theme, cfg_active_custom_theme_id) =
            match crate::app::theme_catalog::resolve_app_theme_name(
                &system_settings_cfg.app_theme,
                theme_store.as_ref(),
            ) {
                Some(theme) => {
                    let custom_id = crate::app::theme_catalog::custom_theme_id_from_config(
                        &system_settings_cfg.app_theme,
                    )
                    .map(str::to_string);
                    (theme, custom_id)
                }
                None => (Theme::Light, None),
            };
        // 编辑器主题配置
        let cfg_editor_follow_system_theme = system_settings_cfg.editor_follow_system_theme;
        let cfg_editor_theme = crate::app::theme_catalog::resolve_app_theme_name(
            &system_settings_cfg.editor_theme,
            theme_store.as_ref(),
        )
        .unwrap_or_else(|| cfg_app_theme.clone());

        // 文件管理器配置
        let cfg_file_manager_width = cfg
            .get("file_manager_width")
            .and_then(|v: &serde_json::Value| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(260.0)
            .clamp(180.0, 600.0);
        let cfg_file_manager_show_changes = cfg
            .get("file_manager_show_changes")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(true);
        let cfg_show_file_manager = cfg
            .get("show_file_manager")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(true);
        // Git差异显示配置
        let cfg_show_git_diff_summary = cfg
            .get("show_git_diff_summary")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);
        let cfg_show_git_diff_highlight = cfg
            .get("show_git_diff_highlight")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);
        // 设置面板显示开关
        let cfg_show_settings =
            cfg.get("show_settings").and_then(|v: &serde_json::Value| v.as_bool()).unwrap_or(true);
        // 项目worktree功能开关
        let cfg_project_worktree_enabled = system_settings_cfg.project_worktree_enabled.clone();
        let gateway_client_cfg = config::load_gateway_client_config();

        // 加载各子系统的配置文件
        #[cfg(not(target_arch = "wasm32"))]
        let full_agent_cfg = config::load_full_agent_config();
        #[cfg(target_arch = "wasm32")]
        let full_agent_cfg = vw_config_types::config::Config::default();
        #[cfg(not(target_arch = "wasm32"))]
        let global_acp_cfg = config::load_enabled_acp_config_result().unwrap_or_else(|err| {
            if vw_gateway_client::is_gateway_unavailable_error(&err) {
                tracing::debug!(target: "vw_desktop", error = %err, "failed to load ACP config via gateway");
            } else {
                tracing::warn!(target: "vw_desktop", error = %err, "failed to load ACP config via gateway");
            }
            full_agent_cfg.acp.clone()
        });
        #[cfg(target_arch = "wasm32")]
        let global_acp_cfg = full_agent_cfg.acp.clone();
        let acp_agents = sort_acp_agents(&global_acp_cfg);
        let cfg_acp_agent =
            cfg_acp_agent.filter(|agent| acp_agents.iter().any(|candidate| candidate == agent));
        let heartbeat_cfg = full_agent_cfg.heartbeat.clone();
        let cron_cfg = full_agent_cfg.cron.clone();
        let scheduler_cfg = full_agent_cfg.scheduler.clone();
        let embedding_routes_cfg = full_agent_cfg.embedding_routes.clone();
        let autonomy_cfg = full_agent_cfg.autonomy.clone();
        let memory_cfg = full_agent_cfg.memory.clone();
        let reliability_cfg = full_agent_cfg.reliability.clone();
        let security_cfg = full_agent_cfg.security.clone();
        #[cfg(not(target_arch = "wasm32"))]
        let gateway_cfg_result = config::load_gateway_config_result();
        #[cfg(target_arch = "wasm32")]
        let gateway_cfg_result: Result<_, String> = Ok(full_agent_cfg.gateway.clone());
        let gateway_cfg = gateway_cfg_result.clone().unwrap_or_default();
        let channels_cfg = full_agent_cfg.channels_config.clone();
        let observability_cfg = full_agent_cfg.observability.clone();
        let storage_cfg = full_agent_cfg.storage.clone();
        let browser_cfg = full_agent_cfg.browser.clone();
        let http_request_cfg = full_agent_cfg.http_request.clone();
        let proxy_cfg = full_agent_cfg.proxy.clone();
        let tunnel_cfg = full_agent_cfg.tunnel.clone();
        let hooks_cfg = full_agent_cfg.hooks.clone();
        let runtime_cfg = full_agent_cfg.runtime.clone();
        let skills_cfg = full_agent_cfg.skills.clone();
        let web_search_cfg = full_agent_cfg.web_search.clone();
        let agents_ipc_cfg = full_agent_cfg.agents_ipc.clone();
        let coordination_cfg = full_agent_cfg.coordination.clone();
        let model_routes_cfg = full_agent_cfg.model_routes.clone();
        let query_classification_cfg = full_agent_cfg.query_classification.clone();

        // 新建会话时的默认目录
        let cfg_new_session_last_directory = cfg
            .get("new_session_last_directory")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s: &str| s.to_string());

        let (open_external_platform, open_external_exists, open_external_app) =
            external_apps::resolve_external_apps(&cfg);

        // 初始化会话相关状态
        let sessions = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let cfg_archived_session_ids =
            crate::app::session_gateway::gateway_load_archived_session_ids(None);
        #[cfg(target_arch = "wasm32")]
        let cfg_archived_session_ids = HashSet::new();
        let active_session_id = None;
        let chat = Vec::new();
        let chat_message_editors = Vec::new();
        let usage = models::TokenUsage::default();

        let effective_app_theme = cfg_app_theme;
        let recent_meta = load_recent_projects_meta();

        let cfg_web_bookmarks = load_web_bookmarks(&cfg);

        // 初始化LSP（语言服务器协议）事件通道（仅非WASM平台）
        #[cfg(not(target_arch = "wasm32"))]
        let (lsp_event_sender, lsp_events) = {
            let (event_tx, event_rx) = std::sync::mpsc::channel();
            (Some(event_tx), Some(event_rx))
        };
        #[cfg(not(target_arch = "wasm32"))]
        let lsp_manager = lsp_event_sender.as_ref().cloned().map(LspServiceManager::new);
        let channels_settings = settings_builders::build_channels_settings(&channels_cfg);
        let chat_message_estimated_heights =
            crate::app::components::chat_panel::rough_message_heights(&chat);
        let chat_virtualizer =
            crate::app::components::chat_panel::virtualizer::ChatVirtualizer::from_estimated_heights(
                &chat_message_estimated_heights,
            );
        let chat_height_index =
            crate::app::components::chat_panel::height_index::ChatHeightIndex::from_heights(
                &chat_message_estimated_heights,
            );
        let editor_language = iced_code_editor::i18n::Language::ChineseSimplified;
        let mut git_copy_modal_code_editor = iced_code_editor::CodeEditor::new("", "diff");
        git_copy_modal_code_editor.set_language(editor_language);

        // 创建应用程序实例并初始化所有状态字段
        let mut app = Self {
            // ========== 基础状态 ==========
            screen: Screen::Home,
            workspace_mode: cfg_workspace_mode,
            work_resource_mode: state::WorkResourceMode::default(),
            work_resources_visible: false,
            work_split_ratio: 0.6,
            work_sidebar_width: 292.0,
            work_sidebar_expanded_width: 292.0,
            work_sidebar_collapsed: false,
            work_sidebar_filter_open: false,
            work_sidebar_more_open: false,
            work_sidebar_project_nav_hovered: false,
            work_sidebar_filter_scope: state::WorkSidebarFilterScope::default(),
            work_sidebar_filter_status: state::WorkSidebarFilterStatus::default(),
            work_sidebar_filter_time: state::WorkSidebarFilterTime::default(),
            work_sidebar_tab: state::WorkSidebarTab::default(),
            work_sidebar_visible_tasks: state::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
            work_sidebar_visible_spaces: state::WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS,
            work_space_picker_open: false,
            dragging_work_sidebar: false,
            work_sidebar_drag_anchor_x: None,
            work_sidebar_drag_start_width: 292.0,
            dragging_work_split: false,
            work_split_drag_anchor_x: None,
            work_split_drag_start_ratio: 0.6,
            work_resource_fullscreen: false,
            work_resource_half_fullscreen: false,
            work_chat_fullscreen: false,
            work_chat_half_fullscreen: false,
            work_browser: state::WorkBrowserState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            work_browser_controller: crate::app::webview_runtime::WorkBrowserController::default(),
            work_expanded_spaces: HashSet::new(),
            work_sidebar_space_task_visible: HashMap::new(),
            project_path: None,
            project_is_git_repo: false,
            project_id: None,
            project_path_input: String::new(),

            // ========== 输入和模型配置 ==========
            input_text: String::new(),
            model: cfg_model.clone(),
            auto_model: cfg_auto_model,
            reasoning_effort: None,
            acp_agent: cfg_acp_agent.clone(),
            acp_history_mode: cfg_acp_history_mode,
            acp_recent_count: cfg_acp_recent_count,
            acp_agents,
            acp_settings: super::state::AcpSettingsState::default(),
            mcp_settings: super::state::McpSettingsState::default(),
            file_url_input: String::new(),
            files: vec![],

            // ========== 聊天相关状态 ==========
            chat,
            chat_message_ids: Vec::new(),
            chat_render_cache: HashMap::new(),
            chat_visible_text_cache: Vec::new(),
            chat_copy_hash_cache: Vec::new(),
            chat_message_expanded: std::collections::HashSet::new(),
            chat_message_estimated_heights,
            chat_virtualizer,
            chat_height_index,
            chat_message_measured_heights: HashMap::new(),
            chat_message_editors,
            chat_special_text_editors: HashMap::new(),
            chat_markdown_blocks: HashMap::new(),
            chat_markdown_block_editors: HashMap::new(),
            chat_tool_text_editors: HashMap::new(),
            chat_think_editors: HashMap::new(),
            chat_think_expanded: std::collections::HashSet::new(),
            chat_think_collapsed: std::collections::HashSet::new(),
            chat_think_hovered_idx: None,
            chat_tool_file_expanded: std::collections::HashSet::new(),
            chat_tool_file_hovered: None,
            chat_tool_expanded: std::collections::HashSet::new(),
            chat_tool_hovered_idx: None,
            chat_explore_expanded: std::collections::HashSet::new(),
            chat_explore_summary_animations: HashMap::new(),
            tool_detail_dialog: None,
            chat_think_scroll_ids: HashMap::new(),
            chat_stream_think_msg_idx: None,
            chat_stream_think_count: 0,
            chat_stream_think_open_idx: None,
            chat_context_menu_target: None,
            chat_context_menu_pos: None,
            chat_context_menu_text: String::new(),
            input_context_menu_open: false,
            input_context_menu_pos: None,
            chat_reset_menu_idx: None,
            chat_fork_dialog_idx: None,
            chat_todo_expanded: true,
            chat_todo_anim: 1.0,
            chat_todo_placement: TodoPanelPlacement::ChatTopRight,
            chat_todo_session_id: None,
            chat_todo_items: vec![],

            // ========== Token使用统计 ==========
            usage,
            active_session_view_state: crate::app::state::ActiveSessionViewState::default(),
            chat_panel_mode: crate::app::state::ChatPanelMode::default(),
            trajectory_search_text: String::new(),
            trajectory_details:
                crate::app::components::chat_panel::trajectory::TrajectoryDetailsState::default(),
            trajectory_controls:
                crate::app::components::chat_panel::trajectory::TrajectoryControlsState::default(),
            trajectory_raw_loading: std::collections::HashSet::new(),
            trajectory_raw_attempts: std::collections::HashMap::new(),
            trajectory_raw_errors: std::collections::HashMap::new(),
            usage_model_info: None,
            usage_step_expanded: std::collections::HashSet::new(),
            usage_step_raw_payloads: std::collections::HashMap::new(),
            usage_step_raw_loading: std::collections::HashSet::new(),
            usage_step_raw_modal: None,
            merge_view: true,

            // ========== 文件和分支管理 ==========
            expanded_files: vec![],
            expanded_files_set: HashSet::new(),
            context_expansions: HashMap::new(),
            branches: vec![],
            selected_branch: None,
            git_worktree_options: vec![],
            selected_git_worktree_directory: None,
            git_worktree_options_loading: false,
            git_worktree_options_project_path: None,
            git_worktree_menu_open: false,
            project_updated_at_ms: None,
            recent_projects: load_recent_projects(),

            // ========== 请求和队列状态 ==========
            is_requesting: false,
            submit_anim: 0,
            active_agent_request: None,
            agent_stream_id: 0,
            queue: vec![],

            // ========== 窗口和布局 ==========
            split_ratio: 0.6,
            dragging_split: false,
            split_drag_anchor_x: None,
            split_drag_start_ratio: 0.0,
            window_size: (1200.0, 800.0),
            fullscreen_layout_settling: false,
            startup_resize_checked: false,

            // ========== 设置面板 ==========
            show_settings: cfg_show_settings,
            settings_sidebar_collapsed: false,
            show_system_settings: false,
            show_about_modal: false,
            show_cli_install_modal: false,
            cli_install_modal_title: String::new(),
            cli_install_modal_message: String::new(),
            cli_install_modal_current_version: String::new(),
            cli_install_modal_server_version: String::new(),
            cli_install_modal_show_update_action: false,
            cli_install_modal_show_install_action: false,
            cli_install_modal_use_app_update_action: false,
            cli_install_modal_is_checking_update: false,
            app_update_progress: None,
            app_update_progress_rx: None,
            question_modal_request_id: None,
            question_modal_request: None,
            question_modal_answers: vec![],
            question_modal_custom: vec![],
            permission_modal_request_id: None,
            permission_modal_request: None,
            permission_modal_requests: vec![],
            active_menu: None,
            open_external_app,
            open_external_platform,
            open_external_exists,

            // ========== 面板拖拽和尺寸 ==========
            settings_panel_width: 476.0,
            dragging_settings: false,
            settings_drag_anchor_x: None,
            settings_drag_start_width: 0.0,
            file_manager_width: cfg_file_manager_width,
            dragging_file_manager: false,
            file_manager_drag_anchor_x: None,
            file_manager_start_width: cfg_file_manager_width,
            file_manager_show_changes: cfg_file_manager_show_changes,
            file_manager_refresh_frame: 0,
            status_animation_frame: 0,
            file_manager_changes_refreshing: false,
            file_manager_file_tree_refreshing: false,
            show_file_manager: cfg_show_file_manager,

            // ========== 弹出菜单和Popover ==========
            show_model_popover: false,
            show_mode_popover: false,
            show_workspace_mode_popover: false,
            show_send_mode_popover: false,
            show_file_popover: false,
            show_acp_popover: false,
            show_reasoning_effort_popover: false,
            show_usage_popover: false,
            show_session_tool_selector_popover: false,
            show_agent_sessions_popover: false,
            show_session_actions_popover: false,
            show_executor_popover: false,
            session_title_last_click: None,
            model_popover_hover: None,
            auto_max_mode: cfg_auto_max_mode,
            last_call_log_path: None,
            last_session_snapshot_path: None,

            // ========== 搜索和缓存 ==========
            search_text: String::new(),
            show_search_overlay: false,
            file_index_cache: HashMap::new(),
            file_index_revision: 0,
            file_tree_model_cache: HashMap::new(),
            search_panel_file_cache_query: String::new(),
            search_panel_file_cache_project_path: None,
            search_panel_file_cache_revision: 0,
            search_panel_file_cache_results: Vec::new(),
            file_search_cache_query: String::new(),
            file_search_cache_project_path: None,
            file_search_cache_revision: 0,
            file_search_cache_entries: Vec::new(),
            input_editor: iced::widget::text_editor::Content::new(),
            last_copied_code_hash: None,
            last_copy_time: None,
            chat_auto_scroll: true,

            // ========== UI组件ID ==========
            chat_scroll_id: iced::widget::Id::new("chat"),
            chat_scroll_offset_y: 1.0,
            chat_scroll_viewport_h: 0.0,
            task_pet_scroll_id: iced::widget::Id::new("task_pet"),
            task_pet_position: iced::Point::new(620.0, 96.0),
            task_pet_collapsed: true,
            task_pet_expand_progress: 0.0,
            task_pet_expand_target: None,
            task_pet_dragging: false,
            task_pet_drag_anchor: None,
            task_pet_drag_start: iced::Point::ORIGIN,
            task_pet_drag_direction: 1,
            task_pet_walk_until_ms: 0,
            task_pet_avatar_kind: state::TaskPetAvatarKind::Robot,
            task_pet_robot_hovered: false,
            task_pet_context_menu_pos: None,
            task_pet_items: Vec::new(),
            task_pet_hovered_request_id: None,
            task_pet_reply_request_id: None,
            task_pet_reply_input: String::new(),
            task_pet_dismissed_request_ids: HashSet::new(),
            chat_stream_autoscroll_last_ms: 0,
            chat_autoscroll_hold_until_ms: 0,
            chat_panel_fullscreen: false,
            chat_panel_half_fullscreen: false,
            show_chat_fullscreen_overlay: false,
            input_editor_id: iced::widget::Id::new("input_editor"),
            file_search_scroll_id: iced::widget::Id::new("file_search"),
            slash_palette_scroll_id: iced::widget::Id::new("slash_palette"),
            preview_scroll_id: iced::widget::Id::new("preview"),
            preview_tabs_scroll_id: iced::widget::Id::new("preview_tabs"),
            git_diff_scroll_id: iced::widget::Id::new("git_diff"),
            git_diff_fullscreen: false,
            git_diff_half_fullscreen: false,
            home_apps_bar_scroll_x: 0.0,
            home_apps_bar_scroll_id: iced::widget::Id::new("home_apps_bar"),

            // ========== 光标和窗口位置 ==========
            cursor_position: iced::Point::ORIGIN,
            window_position: (0.0, 0.0),
            main_window_id: None,
            task_pet_window_id: None,

            // ========== 最近项目编辑 ==========
            recent_projects_edits: {
                let recent = load_recent_projects();
                recent.iter().map(|p| display_name_for_path(&recent_meta, p)).collect()
            },
            recent_project_delete_confirm_idx: None,
            recent_project_remove_confirm_path: None,

            // ========== 对话流配置 ==========
            dialogue_flow_permission_editor: iced::widget::text_editor::Content::with_text("{}"),
            dialogue_flow_show_reasoning_summary: system_settings_cfg
                .dialogue_flow_show_reasoning_summary,
            dialogue_flow_expand_shell_tool_section: system_settings_cfg
                .dialogue_flow_expand_shell_tool_section,
            dialogue_flow_expand_edit_tool_section: system_settings_cfg
                .dialogue_flow_expand_edit_tool_section,
            quick_messages: system_settings_cfg.quick_messages.clone(),
            quick_message_open_group: None,
            show_shiju_skill_menu: false,
            quick_message_handoff: None,
            chat_send_behavior: cfg_chat_send_behavior,
            dialogue_flow_settings_save_message: None,
            recent_projects_meta: recent_meta,
            spinner_frame: 0,
            file_to_discard: None,

            // ========== 会话管理 ==========
            sessions,
            active_session_id,
            archived_session_ids: cfg_archived_session_ids,
            session_chat_cache: HashMap::new(),
            session_view_cache: crate::app::session::SessionViewCache::new(),
            session_chat_message_id_cache: HashMap::new(),
            session_history_reset_boundaries: HashMap::new(),
            session_previews: HashMap::new(),
            session_runtime_states: std::collections::HashMap::from([(
                "__empty__".to_string(),
                super::state::SessionRuntimeState::with_defaults(cfg_model.clone(), cfg_auto_model),
            )]),
            project_sessions: std::collections::HashMap::new(),
            project_session_load_counts: std::collections::HashMap::new(),
            session_tree_expanded: std::collections::HashSet::new(),
            project_sessions_loading: std::collections::HashSet::new(),
            project_session_has_vertical_scrollbar: std::collections::HashMap::new(),
            session_menu_id: None,
            session_menu_anchor: None,
            project_tools_menu_path: None,

            // ========== 新建会话相关 ==========
            new_session_picker_project: None,
            new_session_creating: false,
            new_session_picker_options: Vec::new(),
            project_worktree_enabled: cfg_project_worktree_enabled,
            new_session_last_directory: cfg_new_session_last_directory,
            new_session_worktree_name: String::new(),
            new_session_confirm_delete_directory: None,
            new_session_force_delete_directory: None,
            new_session_delete_error: None,
            new_session_confirm_reset_directory: None,
            new_session_reset_error: None,
            session_rename_id: None,
            session_rename_value: String::new(),

            // ========== 项目编辑 ==========
            project_edit_path: None,
            project_edit_tab: crate::app::state::ProjectEditTab::General,
            project_edit_name: String::new(),
            project_edit_icon: String::new(),
            project_edit_icon_hovered: false,
            project_edit_icon_color: String::new(),
            project_edit_icon_color_picker_open: false,
            project_edit_icon_color_format: crate::app::components::color_picker::ColorFormat::Hex,
            project_edit_start_script: String::new(),
            project_edit_start_script_editor: iced::widget::text_editor::Content::new(),
            project_edit_worktree_enabled: false,
            project_edit_task_board_settings: crate::app::task::TaskBoardSettings::new(),
            project_edit_max_concurrent_input: String::new(),
            project_edit_task_board_auto_refresh: true,
            project_edit_session_auto_refresh:
                crate::app::state::default_recent_project_session_auto_refresh(),
            project_edit_session_refresh_interval_seconds_input: String::new(),
            project_edit_task_board_refresh_interval_seconds_input: String::new(),
            project_edit_task_board_scheduler_tick_interval_seconds_input: String::new(),
            project_edit_task_board_auto_promote_tick_interval_seconds_input: String::new(),
            project_edit_failed_retry_minutes_input: String::new(),
            project_edit_failed_retry_max_attempts_input: String::new(),
            project_edit_running_timeout_minutes_input: String::new(),
            project_edit_pr_submitted_stall_timeout_seconds_input: String::new(),

            // ========== Diff和终端 ==========
            show_diff: true,
            show_git_diff_summary: cfg_show_git_diff_summary,
            show_git_diff_highlight: cfg_show_git_diff_highlight,
            terminal: TerminalState::new(
                cfg_show_terminal,
                cfg_shell,
                cfg_term_theme,
                cfg_term_font_family,
                cfg_term_font_size,
                None,
            ),
            terminals_by_project: HashMap::new(),
            diff_theme: DiffTheme::GitHub,
            app_theme: effective_app_theme,
            chat_timezone: system_settings_cfg.timezone.clone(),
            active_custom_theme_id: cfg_active_custom_theme_id,
            active_theme_skin: None,
            editor_follow_system_theme: cfg_editor_follow_system_theme,
            editor_theme: cfg_editor_theme,

            // ========== Git状态 ==========
            expanded_hunks: vec![],
            git_commit_message: String::new(),
            git_commit_type: Some(ConventionalCommitType::Feat),
            git_commit_scope: String::new(),
            git_commit_description: String::new(),
            git_commit_in_progress: false,
            show_git_commit_help_modal: false,
            show_git_filter_help_modal: false,
            staged_files_selected: vec![],
            staged_hunks_selected: vec![],
            staged_lines_selected: vec![],
            staged_old_lines_selected: vec![],
            git_diff_selected_lines: vec![],
            git_diff_drag_range: None,
            git_diff_selected_range: None,
            git_diff_dragging: false,
            git_diff_drag_start_text: None,
            git_diff_last_click: None,
            git_diff_hovered_line: None,
            git_diff_comment_draft: None,
            git_diff_context_menu: None,
            git_diff_file_menu: None,
            show_git_copy_modal: false,
            git_copy_modal_editor: iced::widget::text_editor::Content::new(),
            git_copy_modal_use_color: true,
            git_copy_modal_code_editor,
            show_git_custom_diff_modal: false,
            git_custom_diff_hide_inputs: false,
            git_custom_diff_title: String::new(),
            git_custom_diff_before_editor: iced::widget::text_editor::Content::new(),
            git_custom_diff_after_editor: iced::widget::text_editor::Content::new(),
            chat_text_diff: None,
            git_commit_description_editor: iced::widget::text_editor::Content::new(),
            show_git_filter_options: false,
            git_filter_query: String::new(),
            git_filter_included: false,
            git_filter_excluded: false,
            git_filter_new: false,
            git_filter_modified: false,
            git_filter_deleted: false,
            git_focused_file: None,
            git_hovered_file_header: None,
            git_panel_header_hovered: false,
            git_changed_files: vec![],
            git_changed_files_loading: false,
            git_changed_files_repo_path: None,
            git_diff_file_metas: vec![],
            git_diff_file_metas_loading: false,
            git_diff_file_metas_repo_path: None,
            git_diff_contents: HashMap::new(),
            git_diff_contents_loading: HashSet::new(),
            git_diff_scroll_offset_y: 0.0,
            git_diff_scroll_viewport_h: 0.0,

            // ========== 设置标签页 ==========
            settings_tab: SettingsTab::Sessions,
            system_settings_tab: components::system_settings::SystemTab::General,
            system_settings_query: String::new(),
            system_settings_help_tab: None,
            top_bar_gateway_tab: super::state::TopBarGatewayTab::default(),
            provider_settings: super::state::ProviderSettingsState::default(),
            model_settings: super::state::ModelSettingsState::default(),
            embedding_routes_settings: super::state::EmbeddingRoutesSettingsState {
                routes: embedding_routes_cfg
                    .into_iter()
                    .map(|route| super::state::EmbeddingRouteDraft {
                        pattern: route.hint,
                        provider: route.provider,
                        model: route.model,
                        dimensions: route
                            .dimensions
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                        api_key_input: route.api_key.unwrap_or_default(),
                    })
                    .collect(),
                save_error: gateway_cfg_result.err().map(config::server_config_unreachable_error),
                save_success: false,
            },
            model_routes_settings: super::state::ModelRoutesSettingsState {
                routes: model_routes_cfg
                    .iter()
                    .map(|route| super::state::ModelRoute {
                        pattern: route.hint.clone(),
                        provider: route.provider.clone(),
                        model: route.model.clone(),
                        priority_input: query_classification_cfg
                            .rules
                            .iter()
                            .find(|rule| rule.hint == route.hint)
                            .map(|rule| rule.priority.to_string())
                            .unwrap_or_else(|| "0".to_string()),
                    })
                    .collect(),
                save_error: if model_routes_cfg.is_empty()
                    && !query_classification_cfg.rules.is_empty()
                {
                    Some("已检测到 query_classification 规则，但桌面模型路由列表为空".to_string())
                } else {
                    None
                },
            },
            query_classification_settings: super::state::QueryClassificationSettingsState {
                enabled: query_classification_cfg.enabled,
                rules: query_classification_cfg
                    .rules
                    .iter()
                    .map(|rule| super::state::QueryClassificationRuleInput {
                        pattern: rule
                            .patterns
                            .first()
                            .cloned()
                            .or_else(|| rule.keywords.first().cloned())
                            .unwrap_or_else(|| rule.hint.clone()),
                        category: rule.hint.clone(),
                        priority_input: rule.priority.to_string(),
                    })
                    .collect(),
                save_error: None,
            },

            // ========== 心跳设置 ==========
            heartbeat_settings: super::state::HeartbeatSettingsState {
                enabled: heartbeat_cfg.enabled,
                interval_minutes: heartbeat_cfg.interval_minutes.clamp(1, 1440),
                message_input: heartbeat_cfg.message.unwrap_or_default(),
                target_input: heartbeat_cfg.target.unwrap_or_default(),
                to_input: heartbeat_cfg.to.unwrap_or_default(),
                show_help_modal: false,
                save_error: None,
            },

            // ========== 定时任务设置 ==========
            cron_settings: super::state::CronSettingsState {
                enabled: cron_cfg.enabled,
                max_run_history: cron_cfg.max_run_history.clamp(1, 10_000),
                active_tab: super::state::CronSettingsTab::default(),
                jobs_loading: false,
                jobs: Vec::new(),
                selected_job_ids: Vec::new(),
                editing_job_id: None,
                edit_draft: super::state::CronJobDraft::default(),
                add_draft: super::state::CronJobDraft::default(),
                runs_modal_job_id: None,
                runs_modal_loading: false,
                runs_modal_error: None,
                runs_modal: Vec::new(),
                runs_modal_editor: iced::widget::text_editor::Content::new(),
                show_help_modal: false,
                save_error: None,
                action_status: None,
            },

            // ========== 调度器设置 ==========
            scheduler_settings: super::state::SchedulerSettingsState {
                enabled: scheduler_cfg.enabled,
                max_tasks: scheduler_cfg.max_tasks.clamp(1, 10_000) as u32,
                max_concurrent: scheduler_cfg.max_concurrent.clamp(1, 100) as u32,
                show_help_modal: false,
                save_error: None,
            },

            // ========== Hooks 设置 ==========
            hooks_settings: super::state::HooksSettingsState {
                enabled: hooks_cfg.enabled,
                command_logger: hooks_cfg.builtin.command_logger,
                save_error: None,
            },

            // ========== 运行时设置 ==========
            runtime_settings: super::state::RuntimeSettingsState {
                kind: match runtime_cfg.kind.trim() {
                    "native" | "docker" | "wasm" => runtime_cfg.kind.clone(),
                    _ => "native".to_string(),
                },
                docker_image: {
                    let v = runtime_cfg.docker.image.trim().to_string();
                    if v.is_empty() { "alpine:3.20".to_string() } else { v }
                },
                docker_network: {
                    let v = runtime_cfg.docker.network.trim().to_string();
                    if v.is_empty() { "none".to_string() } else { v }
                },
                docker_memory_limit_mb_input: runtime_cfg
                    .docker
                    .memory_limit_mb
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                docker_cpu_limit_input: runtime_cfg
                    .docker
                    .cpu_limit
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                docker_read_only_rootfs: runtime_cfg.docker.read_only_rootfs,
                docker_mount_workspace: runtime_cfg.docker.mount_workspace,
                docker_allowed_workspace_roots_input: runtime_cfg
                    .docker
                    .allowed_workspace_roots
                    .join(", "),
                wasm_tools_dir: {
                    let v = runtime_cfg.wasm.tools_dir.trim().to_string();
                    if v.is_empty() { "tools/wasm".to_string() } else { v }
                },
                wasm_fuel_limit_input: runtime_cfg
                    .wasm
                    .fuel_limit
                    .clamp(1, 100_000_000)
                    .to_string(),
                wasm_memory_limit_mb_input: runtime_cfg
                    .wasm
                    .memory_limit_mb
                    .clamp(1, 4096)
                    .to_string(),
                wasm_max_module_size_mb_input: runtime_cfg
                    .wasm
                    .max_module_size_mb
                    .clamp(1, 4096)
                    .to_string(),
                wasm_allow_workspace_read: runtime_cfg.wasm.allow_workspace_read,
                wasm_allow_workspace_write: runtime_cfg.wasm.allow_workspace_write,
                wasm_allowed_hosts_input: runtime_cfg.wasm.allowed_hosts.join(", "),
                wasm_require_workspace_relative_tools_dir: runtime_cfg
                    .wasm
                    .security
                    .require_workspace_relative_tools_dir,
                wasm_reject_symlink_modules: runtime_cfg.wasm.security.reject_symlink_modules,
                wasm_reject_symlink_tools_dir: runtime_cfg.wasm.security.reject_symlink_tools_dir,
                wasm_strict_host_validation: runtime_cfg.wasm.security.strict_host_validation,
                wasm_capability_escalation_mode: match runtime_cfg
                    .wasm
                    .security
                    .capability_escalation_mode
                {
                    vw_config_types::runtime::WasmCapabilityEscalationMode::Deny => {
                        "deny".to_string()
                    }
                    vw_config_types::runtime::WasmCapabilityEscalationMode::Clamp => {
                        "clamp".to_string()
                    }
                },
                wasm_module_hash_policy: match runtime_cfg.wasm.security.module_hash_policy {
                    vw_config_types::runtime::WasmModuleHashPolicy::Disabled => {
                        "disabled".to_string()
                    }
                    vw_config_types::runtime::WasmModuleHashPolicy::Warn => "warn".to_string(),
                    vw_config_types::runtime::WasmModuleHashPolicy::Enforce => {
                        "enforce".to_string()
                    }
                },
                wasm_module_sha256_input: runtime_cfg
                    .wasm
                    .security
                    .module_sha256
                    .into_iter()
                    .map(|(module, hash)| format!("{module}:{hash}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                reasoning_enabled_input: match runtime_cfg.reasoning_enabled {
                    None => "auto".to_string(),
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                },
                reasoning_level_input: runtime_cfg.reasoning_level.unwrap_or_default(),
                save_error: None,
            },

            // ========== 技能设置 ==========
            skills_settings: super::state::SkillsSettingsState {
                open_skills_enabled: skills_cfg.open_skills_enabled,
                auto_load_project_skills: skills_cfg.auto_load_project_skills,
                directory_provider: skills_cfg.directory_provider,
                open_skills_dir_input: skills_cfg.open_skills_dir.unwrap_or_default(),
                prompt_injection_mode: skills_cfg.prompt_injection_mode,
                active_tab: super::state::SkillsSettingsTab::Skills,
                query: String::new(),
                directory_scope: super::state::SkillsDirectoryScope::Project,
                loading: false,
                catalog: Vec::new(),
                plugin_catalog: Vec::new(),
                plugin_loading: false,
                selected_plugin_id: None,
                plugin_status_message: None,
                plugin_status_is_error: false,
                selected_skill_id: None,
                selected_skill_detail: None,
                detail_loading: false,
                detail_error: None,
                status_message: None,
                status_is_error: false,
                show_help_modal: false,
                save_error: None,
            },

            web_search_settings: super::state::WebSearchSettingsState {
                enabled: web_search_cfg.enabled,
                provider: match web_search_cfg.provider.trim().to_ascii_lowercase().as_str() {
                    "ddg" | "duckduckgo" => "duckduckgo".to_string(),
                    "brave" => "brave".to_string(),
                    "serper" => "serper".to_string(),
                    "google" => "google".to_string(),
                    "bing" => "bing".to_string(),
                    _ => "duckduckgo".to_string(),
                },
                api_key_input: web_search_cfg.api_key.unwrap_or_default(),
                api_url_input: web_search_cfg.api_url.unwrap_or_default(),
                brave_api_key_input: web_search_cfg.brave_api_key.unwrap_or_default(),
                max_results_input: web_search_cfg.max_results.clamp(1, 10).to_string(),
                timeout_secs_input: web_search_cfg.timeout_secs.max(1).to_string(),
                user_agent: if web_search_cfg.user_agent.trim().is_empty() {
                    vw_config_types::tools::DEFAULT_USER_AGENT.to_string()
                } else {
                    web_search_cfg.user_agent
                },
                show_help_modal: false,
                save_error: None,
            },

            browser_settings: super::state::BrowserSettingsState {
                enabled: browser_cfg.enabled,
                allowed_domains_input: browser_cfg.allowed_domains.join("\n"),
                allowed_domains_editor: iced::widget::text_editor::Content::with_text(
                    &browser_cfg.allowed_domains.join("\n"),
                ),
                browser_open: match browser_cfg.browser_open.trim().to_ascii_lowercase().as_str() {
                    "default" | "new_window" | "new_tab" => {
                        browser_cfg.browser_open.trim().to_ascii_lowercase()
                    }
                    _ => "new_tab".to_string(),
                },
                session_name_input: browser_cfg.session_name.unwrap_or_default(),
                backend: match browser_cfg
                    .backend
                    .trim()
                    .to_ascii_lowercase()
                    .replace('-', "_")
                    .as_str()
                {
                    "agent_browser" => "agent_browser".to_string(),
                    "rust_native" | "native" => "native".to_string(),
                    "computer_use" => "computer_use".to_string(),
                    "auto" => "auto".to_string(),
                    _ => "agent_browser".to_string(),
                },
                native_headless: browser_cfg.native_headless,
                native_webdriver_url: browser_cfg.native_webdriver_url,
                native_chrome_path_input: browser_cfg.native_chrome_path.unwrap_or_default(),
                computer_use_endpoint: browser_cfg.computer_use.endpoint,
                computer_use_api_key_input: browser_cfg.computer_use.api_key.unwrap_or_default(),
                computer_use_timeout_ms_input: browser_cfg.computer_use.timeout_ms.to_string(),
                computer_use_allow_remote_endpoint: browser_cfg.computer_use.allow_remote_endpoint,
                computer_use_window_allowlist_input: browser_cfg
                    .computer_use
                    .window_allowlist
                    .join(", "),
                computer_use_max_coordinate_x_input: browser_cfg
                    .computer_use
                    .max_coordinate_x
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                computer_use_max_coordinate_y_input: browser_cfg
                    .computer_use
                    .max_coordinate_y
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                save_error: None,
            },

            http_request_settings: super::state::HttpRequestSettingsState {
                enabled: http_request_cfg.enabled,
                allowed_domains: http_request_cfg.allowed_domains,
                new_allowed_domain_input: String::new(),
                max_response_size: http_request_cfg.max_response_size.min(u32::MAX as usize) as u32,
                timeout_secs: http_request_cfg.timeout_secs.min(u32::MAX as u64) as u32,
                user_agent: http_request_cfg.user_agent,
                save_error: None,
            },

            gateway_settings: super::state::GatewaySettingsState {
                active_tab: super::state::GatewaySettingsTab::Config,
                port: gateway_cfg.port.clamp(1, u16::MAX),
                host_input: {
                    let value = gateway_cfg.host.trim().to_string();
                    if value.is_empty() { "127.0.0.1".to_string() } else { value }
                },
                auth_enabled: gateway_cfg.auth_enabled,
                skeys: gateway_cfg.skeys,
                new_skey_name_input: String::new(),
                new_skey_expires_at_input: String::new(),
                new_skey_calendar_month: String::new(),
                new_skey_calendar_open: false,
                last_created_skey: None,
                last_created_skey_copied: false,
                allow_public_bind: gateway_cfg.allow_public_bind,
                webhook_rate_limit_per_minute: gateway_cfg
                    .webhook_rate_limit_per_minute
                    .clamp(1, 100_000),
                trust_forwarded_headers: gateway_cfg.trust_forwarded_headers,
                rate_limit_max_keys: gateway_cfg.rate_limit_max_keys.min(u32::MAX as usize) as u32,
                idempotency_ttl_secs: gateway_cfg.idempotency_ttl_secs.min(u32::MAX as u64) as u32,
                idempotency_max_keys: gateway_cfg.idempotency_max_keys.min(u32::MAX as usize)
                    as u32,
                node_control_enabled: gateway_cfg.node_control.enabled,
                node_control_auth_token_input: gateway_cfg
                    .node_control
                    .auth_token
                    .unwrap_or_default(),
                node_control_allowed_node_ids_input: gateway_cfg
                    .node_control
                    .allowed_node_ids
                    .join("\n"),
                service_action_running: None,
                service_action_output: None,
                copied_service_command: None,
                pending_service_command_confirmation: None,
                show_help_modal: false,
                save_error: None,
            },

            gateway_client_settings: super::state::GatewayClientSettingsState {
                selected_server_id: gateway_client_cfg.active_server().id.clone(),
                name_input: gateway_client_cfg.active_server().name.clone(),
                servers: gateway_client_cfg
                    .normalized_servers()
                    .iter()
                    .map(super::state::GatewayClientServerDraft::from_config)
                    .collect(),
                health: HashMap::new(),
                host_input: {
                    let value = gateway_client_cfg.active_server().host.trim().to_string();
                    if value.is_empty() { "127.0.0.1".to_string() } else { value }
                },
                port: gateway_client_cfg.active_server().port.clamp(1, u16::MAX),
                platform_id_input: gateway_client_cfg.active_server().platform_id.clone(),
                company_id_input: gateway_client_cfg.active_server().company_id.clone(),
                skey_input: {
                    let active = gateway_client_cfg.active_server();
                    if active.skey.trim().is_empty() { active.bearer_token } else { active.skey }
                },
                pending_remove_server_id: None,
                show_help_modal: false,
                save_error: None,
            },

            // ========== Agent IPC设置 ==========
            agents_ipc_settings: super::state::AgentsIpcSettingsState {
                enabled: agents_ipc_cfg.enabled,
                db_path_input: agents_ipc_cfg.db_path,
                staleness_secs: agents_ipc_cfg.staleness_secs.clamp(1, 86_400),
                show_help_modal: false,
                save_error: None,
            },

            agents_settings: {
                let mut entries = super::state::agent_settings_entries_from_config(&full_agent_cfg);
                if let Some(main) = entries.iter_mut().find(|entry| entry.key == "main") {
                    main.compact_context = full_agent_cfg.agent.compact_context;
                    main.max_tool_iterations =
                        u32::try_from(full_agent_cfg.agent.max_tool_iterations)
                            .unwrap_or(u32::MAX)
                            .max(1);
                    main.max_history_messages =
                        full_agent_cfg.agent.max_history_messages.clamp(1, 1000) as u32;
                    main.parallel_tools = full_agent_cfg.agent.parallel_tools;
                    main.tool_dispatcher = {
                        let value = full_agent_cfg.agent.tool_dispatcher.trim().to_string();
                        if value.is_empty() { "auto".to_string() } else { value }
                    };
                }
                let available_tools = super::state::filter_mcp_from_available_tools(
                    config::load_tools_list_via_gateway(),
                );
                super::state::apply_default_allowed_tools(&mut entries, &available_tools);
                super::state::AgentsSettingsState {
                    loading: true,
                    config_ready: false,
                    project_path: None,
                    providers: Vec::new(),
                    provider_models: Vec::new(),
                    entries,
                    new_agent_key_input: String::new(),
                    selected_agent: super::state::MAIN_AGENT_KEY.to_string(),
                    active_detail_tab: super::state::AGENT_DETAIL_BASIC_TAB.to_string(),
                    active_tools_list_tab: super::state::AGENT_TOOLS_OPTIONAL_TAB.to_string(),
                    active_prompt_tab: super::state::AGENT_PROMPT_SYSTEM_TAB.to_string(),
                    workspace_identity_files: super::state::WORKSPACE_IDENTITY_FILES
                        .iter()
                        .map(|(file_name, label)| super::state::WorkspaceIdentityFileState {
                            file_name: (*file_name).to_string(),
                            label: (*label).to_string(),
                            editor: iced::widget::text_editor::Content::with_text(""),
                            size_bytes: None,
                            modified_at_ms: None,
                        })
                        .collect(),
                    workspace_identity_root_path: None,
                    available_tools,
                    save_error: None,
                    save_in_flight: false,
                    save_pending: false,
                }
            },
            staff_settings: settings_builders::build_staff_settings(&full_agent_cfg),

            // ========== 协调设置 ==========
            coordination_settings: super::state::CoordinationSettingsState {
                enabled: coordination_cfg.enabled,
                lead_agent_input: coordination_cfg.lead_agent,
                max_inbox_messages_per_agent: coordination_cfg
                    .max_inbox_messages_per_agent
                    .clamp(1, 10_000) as u32,
                max_dead_letters: coordination_cfg.max_dead_letters.clamp(1, 10_000) as u32,
                max_context_entries: coordination_cfg.max_context_entries.clamp(1, 20_000) as u32,
                max_seen_message_ids: coordination_cfg.max_seen_message_ids.clamp(1, 100_000)
                    as u32,
                show_help_modal: false,
                save_error: None,
            },

            // ========== 成本控制设置 ==========
            cost_settings: super::state::CostSettingsState::default(),

            // ========== 记忆系统设置 ==========
            memory_settings: super::state::MemorySettingsState {
                active_tab: super::state::MemorySettingsTab::General,
                backend: match memory_cfg.backend.trim().to_ascii_lowercase().as_str() {
                    "sqlite" | "postgres" | "qdrant" | "markdown" | "none" => {
                        memory_cfg.backend.trim().to_ascii_lowercase()
                    }
                    "null" => "none".to_string(),
                    _ => "sqlite".to_string(),
                },
                auto_save: memory_cfg.auto_save,
                hygiene_enabled: memory_cfg.hygiene_enabled,
                archive_after_days: memory_cfg.archive_after_days,
                purge_after_days: memory_cfg.purge_after_days,
                conversation_retention_days: memory_cfg.conversation_retention_days,
                embedding_provider: memory_cfg.embedding_provider,
                embedding_model: memory_cfg.embedding_model,
                embedding_dimensions: memory_cfg.embedding_dimensions.min(u32::MAX as usize) as u32,
                vector_weight: memory_cfg.vector_weight.clamp(0.0, 1.0) as f32,
                keyword_weight: memory_cfg.keyword_weight.clamp(0.0, 1.0) as f32,
                min_relevance_score: memory_cfg.min_relevance_score.clamp(0.0, 1.0) as f32,
                embedding_cache_size: memory_cfg.embedding_cache_size.min(u32::MAX as usize) as u32,
                chunk_max_tokens: memory_cfg.chunk_max_tokens.min(u32::MAX as usize) as u32,
                response_cache_enabled: memory_cfg.response_cache_enabled,
                response_cache_ttl_minutes: memory_cfg.response_cache_ttl_minutes,
                response_cache_max_entries: memory_cfg
                    .response_cache_max_entries
                    .min(u32::MAX as usize) as u32,
                snapshot_enabled: memory_cfg.snapshot_enabled,
                snapshot_on_hygiene: memory_cfg.snapshot_on_hygiene,
                auto_hydrate: memory_cfg.auto_hydrate,
                sqlite_open_timeout_secs: memory_cfg
                    .sqlite_open_timeout_secs
                    .unwrap_or_default()
                    .min(u32::MAX as u64) as u32,
                qdrant_url_input: memory_cfg.qdrant.url.unwrap_or_default(),
                qdrant_collection: {
                    let value = memory_cfg.qdrant.collection.trim().to_string();
                    if value.is_empty() { "vibewindow_memories".to_string() } else { value }
                },
                qdrant_api_key_input: memory_cfg.qdrant.api_key.unwrap_or_default(),
                save_error: None,
            },

            channels_settings,

            // ========== 可靠性设置 ==========
            reliability_settings: super::state::ReliabilitySettingsState {
                provider_retries: reliability_cfg.provider_retries,
                provider_backoff_ms: reliability_cfg.provider_backoff_ms,
                channel_initial_backoff_secs: reliability_cfg.channel_initial_backoff_secs,
                channel_max_backoff_secs: reliability_cfg.channel_max_backoff_secs,
                scheduler_poll_secs: reliability_cfg.scheduler_poll_secs,
                scheduler_retries: reliability_cfg.scheduler_retries,
                show_help_modal: false,
                save_error: None,
            },

            multimodal_settings: build_multimodal_settings(&full_agent_cfg),

            // ========== 安全设置 ==========
            security_settings: super::state::SecuritySettingsState {
                active_tab: super::state::SecuritySettingsTab::Bash,
                // Bash 命令执行前安全检查
                bash: security_cfg.bash.clone(),
                // 沙箱配置
                sandbox_enabled_input: match security_cfg.sandbox.enabled {
                    None => "auto".to_string(),
                    Some(true) => "true".to_string(),
                    Some(false) => "false".to_string(),
                },
                sandbox_backend_input: match security_cfg.sandbox.backend {
                    vw_config_types::security::SandboxBackend::Auto => "auto".to_string(),
                    vw_config_types::security::SandboxBackend::Landlock => "landlock".to_string(),
                    vw_config_types::security::SandboxBackend::Firejail => "firejail".to_string(),
                    vw_config_types::security::SandboxBackend::Bubblewrap => {
                        "bubblewrap".to_string()
                    }
                    vw_config_types::security::SandboxBackend::Docker => "docker".to_string(),
                    vw_config_types::security::SandboxBackend::None => "none".to_string(),
                },
                sandbox_firejail_args: security_cfg.sandbox.firejail_args.clone(),
                // 资源限制配置
                resources_max_memory_mb: security_cfg.resources.max_memory_mb.clamp(32, 65_536),
                resources_max_cpu_time_seconds: security_cfg
                    .resources
                    .max_cpu_time_seconds
                    .clamp(1, 86_400),
                resources_max_subprocesses: security_cfg
                    .resources
                    .max_subprocesses
                    .clamp(1, 10_000),
                resources_memory_monitoring: security_cfg.resources.memory_monitoring,
                // 审计配置
                audit_enabled: security_cfg.audit.enabled,
                audit_log_path: {
                    let v = security_cfg.audit.log_path.trim().to_string();
                    if v.is_empty() { "audit.log".to_string() } else { v }
                },
                audit_max_size_mb: security_cfg.audit.max_size_mb.clamp(1, 10_000),
                audit_sign_events: security_cfg.audit.sign_events,
                // OTP配置
                otp_enabled: security_cfg.otp.enabled,
                otp_method_input: match security_cfg.otp.method {
                    vw_config_types::security::OtpMethod::Totp => "totp".to_string(),
                    vw_config_types::security::OtpMethod::Pairing => "pairing".to_string(),
                    vw_config_types::security::OtpMethod::CliPrompt => "cli-prompt".to_string(),
                },
                otp_token_ttl_secs: security_cfg.otp.token_ttl_secs.clamp(1, 600),
                otp_cache_valid_secs: security_cfg.otp.cache_valid_secs.clamp(1, 86_400),
                otp_gated_actions: security_cfg.otp.gated_actions.clone(),
                otp_gated_domains: security_cfg.otp.gated_domains.clone(),
                otp_gated_domain_categories: security_cfg.otp.gated_domain_categories.clone(),
                // 紧急停止配置
                estop_enabled: security_cfg.estop.enabled,
                estop_state_file: {
                    let v = security_cfg.estop.state_file.trim().to_string();
                    if v.is_empty() { vw_config_types::paths::estop_state_file_path() } else { v }
                },
                estop_require_otp_to_resume: security_cfg.estop.require_otp_to_resume,
                // 系统调用异常检测配置
                syscall_anomaly_enabled: security_cfg.syscall_anomaly.enabled,
                syscall_anomaly_strict_mode: security_cfg.syscall_anomaly.strict_mode,
                syscall_anomaly_alert_on_unknown_syscall: security_cfg
                    .syscall_anomaly
                    .alert_on_unknown_syscall,
                syscall_anomaly_max_denied_events_per_minute: security_cfg
                    .syscall_anomaly
                    .max_denied_events_per_minute
                    .clamp(1, 10_000),
                syscall_anomaly_max_total_events_per_minute: security_cfg
                    .syscall_anomaly
                    .max_total_events_per_minute
                    .clamp(1, 100_000),
                syscall_anomaly_max_alerts_per_minute: security_cfg
                    .syscall_anomaly
                    .max_alerts_per_minute
                    .clamp(1, 10_000),
                syscall_anomaly_alert_cooldown_secs: security_cfg
                    .syscall_anomaly
                    .alert_cooldown_secs
                    .clamp(1, 3600),
                syscall_anomaly_log_path: {
                    let v = security_cfg.syscall_anomaly.log_path.trim().to_string();
                    if v.is_empty() { "syscall-anomalies.log".to_string() } else { v }
                },
                syscall_anomaly_baseline_syscalls: security_cfg
                    .syscall_anomaly
                    .baseline_syscalls
                    .clone(),
                list_editor: super::state::SecurityListEditorState::default(),
                // Canary令牌和语义保护
                canary_tokens: security_cfg.canary_tokens,
                semantic_guard: security_cfg.semantic_guard,
                semantic_guard_collection: {
                    let v = security_cfg.semantic_guard_collection.trim().to_string();
                    if v.is_empty() { "semantic_guard".to_string() } else { v }
                },
                semantic_guard_threshold: security_cfg.semantic_guard_threshold.clamp(0.0, 1.0),
                show_help_modal: false,
                save_error: None,
            },

            // ========== 自主性设置 ==========
            autonomy_settings: super::state::AutonomySettingsState::from_config(&autonomy_cfg),

            // ========== 可观测性设置 ==========
            observability_settings: super::state::ObservabilitySettingsState {
                backend: match observability_cfg.backend.trim() {
                    "none" | "log" | "prometheus" | "otel" => observability_cfg.backend.clone(),
                    _ => "none".to_string(),
                },
                otel_endpoint_input: observability_cfg.otel_endpoint.unwrap_or_default(),
                otel_service_name_input: observability_cfg.otel_service_name.unwrap_or_default(),
                runtime_trace_mode: match observability_cfg.runtime_trace_mode.trim() {
                    "none" | "rolling" | "full" => observability_cfg.runtime_trace_mode.clone(),
                    _ => "none".to_string(),
                },
                runtime_trace_path_input: {
                    let v = observability_cfg.runtime_trace_path.trim().to_string();
                    if v.is_empty() { "state/runtime-trace.jsonl".to_string() } else { v }
                },
                runtime_trace_max_entries: observability_cfg
                    .runtime_trace_max_entries
                    .clamp(1, 100_000) as u32,
                show_help_modal: false,
                save_error: None,
            },

            // ========== 存储设置 ==========
            storage_settings: super::state::StorageSettingsState {
                provider: storage_cfg.provider.config.provider,
                db_url_input: storage_cfg.provider.config.db_url.unwrap_or_default(),
                schema: {
                    let v = storage_cfg.provider.config.schema.trim().to_string();
                    if v.is_empty() { "public".to_string() } else { v }
                },
                table: {
                    let v = storage_cfg.provider.config.table.trim().to_string();
                    if v.is_empty() { "memories".to_string() } else { v }
                },
                connect_timeout_secs_input: storage_cfg
                    .provider
                    .config
                    .connect_timeout_secs
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                tls: storage_cfg.provider.config.tls,
                save_error: None,
            },

            // ========== 代理设置 ==========
            proxy_settings: super::state::ProxySettingsState {
                enabled: proxy_cfg.enabled,
                http_proxy: proxy_cfg.http_proxy.unwrap_or_default(),
                https_proxy: proxy_cfg.https_proxy.unwrap_or_default(),
                all_proxy: proxy_cfg.all_proxy.unwrap_or_default(),
                no_proxy_input: proxy_cfg.no_proxy.join(", "),
                scope: proxy_cfg.scope,
                services_input: proxy_cfg.services.join(", "),
                show_help_modal: false,
                save_error: None,
            },

            // ========== 隧道设置 ==========
            tunnel_settings: super::state::TunnelSettingsState {
                provider: match tunnel_cfg.provider.trim().to_ascii_lowercase().as_str() {
                    "cloudflare" => "cloudflare".to_string(),
                    "tailscale" => "tailscale".to_string(),
                    "ngrok" => "ngrok".to_string(),
                    "custom" => "custom".to_string(),
                    _ => "none".to_string(),
                },
                cloudflare_token: tunnel_cfg
                    .cloudflare
                    .as_ref()
                    .map(|config| config.token.clone())
                    .unwrap_or_default(),
                tailscale_funnel: tunnel_cfg
                    .tailscale
                    .as_ref()
                    .map(|config| config.funnel)
                    .unwrap_or(false),
                tailscale_hostname: tunnel_cfg
                    .tailscale
                    .as_ref()
                    .and_then(|config| config.hostname.clone())
                    .unwrap_or_default(),
                ngrok_auth_token: tunnel_cfg
                    .ngrok
                    .as_ref()
                    .map(|config| config.auth_token.clone())
                    .unwrap_or_default(),
                ngrok_domain: tunnel_cfg
                    .ngrok
                    .as_ref()
                    .and_then(|config| config.domain.clone())
                    .unwrap_or_default(),
                custom_start_command: tunnel_cfg
                    .custom
                    .as_ref()
                    .map(|config| config.start_command.clone())
                    .unwrap_or_default(),
                custom_health_url: tunnel_cfg
                    .custom
                    .as_ref()
                    .and_then(|config| config.health_url.clone())
                    .unwrap_or_default(),
                custom_url_pattern: tunnel_cfg
                    .custom
                    .as_ref()
                    .and_then(|config| config.url_pattern.clone())
                    .unwrap_or_default(),
                save_error: None,
            },

            // ========== 主题设置 ==========
            themes_settings: super::state::ThemesSettingsState::default(),

            // ========== 文件树状态 ==========
            file_tree_expanded: cfg_file_tree_expanded.clone(),
            file_tree_expanded_set: cfg_file_tree_expanded.into_iter().collect(),
            file_tree_menu_path,
            file_tree_menu_anchor,
            file_tree_menu_source,
            file_tree_clipboard: None,
            dragging_file_paths: Vec::new(),
            dragging_file_position: None,
            pending_drop_file_paths: Vec::new(),
            pending_drop_file_position: None,
            input_drop_hovered: false,
            file_tree_rename_path: None,
            file_tree_rename_value: String::new(),

            // ========== 文件搜索 ==========
            file_search_query: String::new(),
            show_file_search: false,
            file_search_anchor: None,
            file_search_selected_index: 0,

            // ========== Slash 快捷选择 ==========
            slash_palette_query: String::new(),
            show_slash_palette: false,
            slash_palette_selected_index: 0,
            slash_palette_expanded_kinds: std::collections::BTreeSet::new(),

            // ========== 查找结果标签页 ==========
            find_results_tabs: Vec::new(),
            active_find_results_tab_id: None,
            find_results_tab_menu_id: None,
            find_results_tab_menu_pos: None,
            tool_files_filter: String::new(),
            file_ref_hovered_index: None,

            // ========== 预览标签页 ==========
            preview_tabs: Vec::new(),
            active_preview_path: None,
            project_preview_tabs: HashMap::new(),
            project_preview_active_path: HashMap::new(),
            preview_tab_menu_path: None,
            preview_tab_menu_pos: None,

            // ========== LSP状态（仅非WASM平台） ==========
            #[cfg(not(target_arch = "wasm32"))]
            lsp_events,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_event_sender,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_manager,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_overlay: iced_code_editor::LspOverlayState::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_disabled: false,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_applying_completion: false,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_anchor: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_overlay_path: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_hover_hide_deadline: None,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_progress: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            lsp_status: None,

            // ========== 预览导航 ==========
            pending_preview_goto: None,
            preview_trace_back: Vec::new(),
            preview_trace_forward: Vec::new(),
            preview_trace_navigating: false,
            previous_preview_path: None,
            focus_area: FocusArea::None,
            show_preview_context_menu: false,
            preview_context_target: None,
            preview_context_menu_pos: None,
            preview_nav_popup: None,

            error_message: None,
            show_preview_settings: false,
            show_preview_fullscreen_overlay: false,

            // ========== 编辑器设置 ==========
            current_font_size: system_settings_cfg.editor_font_size.max(1.0),
            current_line_height: system_settings_cfg.editor_line_height.max(1.0),
            auto_adjust_line_height: system_settings_cfg.editor_auto_line_height,
            preview_auto_save_mode: system_settings_cfg.preview_auto_save,
            current_language: editor_language,
            knowledge: super::state::KnowledgeUiState::default(),
            workflow_state: crate::apps::workflow::state::WorkflowState::default(),
            // ========== 标签页管理 ==========
            active_tab_id: Some("home".to_string()),
            hovered_tab_id: None,
            hovered_recent_project: None,
            open_tabs: vec![AppTab {
                id: "home".to_string(),
                title: "首页".to_string(),
                screen: Screen::Home,
                project_path: None,
            }],
            apps_search_query: String::new(),
            web_bookmarks: cfg_web_bookmarks,

            // ========== Web书签编辑 ==========
            show_web_links_menu: false,
            web_bookmark_title_input: String::new(),
            web_bookmark_url_input: String::new(),
            web_bookmark_width_input: String::new(),
            web_bookmark_height_input: String::new(),
            editing_web_bookmark: None,
            edit_web_bookmark_title_input: String::new(),
            edit_web_bookmark_url_input: String::new(),
            edit_web_bookmark_width_input: String::new(),
            edit_web_bookmark_height_input: String::new(),
            edit_web_bookmark_cookie_configs_editor: iced::widget::text_editor::Content::new(),

            // ========== 通知系统 ==========
            notifications: Vec::new(),
            notifications_expanded: false,
            next_notification_id: 0,
            notifications_scroll_id: iced::widget::Id::new("notifications_scroll"),
            notification_editors: std::collections::HashMap::new(),
            copied_notification_id: None,
            active_toast: None,
            next_toast_id: 0,
            #[cfg(not(target_arch = "wasm32"))]
            account: crate::app::state::AccountState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            expert_hub: {
                let local_version = crate::app::skill_hub::shiju_installation()
                    .ok()
                    .flatten()
                    .and_then(|installation| installation.version);
                let cached =
                    crate::app::skill_hub::package_version_cache_path().ok().and_then(|path| {
                        crate::app::skill_hub::load_package_version_cache(&path).ok().flatten()
                    });
                let mut state = crate::app::skill_hub::ExpertHubState::default();
                state.package_status =
                    crate::app::skill_hub::ShijuPackageStatus::from_sources(local_version, cached);
                state
            },
            workflow_manager: crate::app::views::workflow_manager::WorkflowManagerState::default(),

            // ========== 任务板基础状态 ==========
            show_task_board: false,
            task_board_loading: false,
            task_board_view_mode: crate::app::state::TaskBoardViewMode::default(),
            task_board_tasks: Vec::new(),
            task_board_staff_config: crate::app::task::TaskBoardStaffConfig::default(),
            task_board_staff_config_workflow_id: None,
            task_board_staff_projection: crate::app::task::staff::TaskBoardStaffProjection::default(
            ),
            task_board_staff_config_load_error: None,
            task_board_staff_capability_catalog: Vec::new(),
            task_board_staff_capability_catalog_workflow_id: None,
            task_board_staff_capability_draft: Vec::new(),
            task_board_staff_capability_catalog_loading: false,
            task_board_staff_capability_saving: false,
            task_board_staff_capability_catalog_error: None,
            task_board_staff_command_queue:
                crate::app::task::staff::TaskBoardStaffCommandQueue::default(),
            task_board_staff_events: Vec::new(),
            task_board_staff_feed_tab: crate::app::state::TaskBoardStaffFeedTab::default(),
            task_board_staff_task_status_filter: Vec::new(),
            task_board_staff_command_input: String::new(),
            task_board_staff_filter_member_id: None,
            task_board_staff_assignment_open: false,
            task_board_staff_assignment_draft: None,
            task_board_staff_workflow_binding_draft: None,
            task_board_staff_stage_action_draft: None,
            task_board_staff_stage_action_reset_confirm: false,
            task_board_staff_workflow_options: Vec::new(),
            task_board_staff_workflow_options_error: None,
            task_board_staff_assignment_tabs: HashMap::new(),
            task_board_staff_orchestration_tab:
                crate::app::state::TaskBoardStaffOrchestrationTab::default(),
            task_board_create_modal_open: false,
            task_board_settings_modal_open: false,
            task_board_settings_modal_tab: crate::app::state::TaskBoardSettingsModalTab::default(),

            // ========== 任务导入 ==========
            task_board_is_import_mode: false,
            task_board_import_editor: iced::widget::text_editor::Content::new(),
            task_board_import_prompt_format: crate::app::task::TaskImportPromptFormat::Json,
            task_board_import_prompt_collapsed: true,
            task_board_column_has_vertical_scrollbar: std::collections::HashMap::new(),

            // ========== 任务草稿和执行 ==========
            task_board_draft: crate::app::task::TaskDraft::default(),
            task_board_last_model: "auto".to_string(),
            task_board_last_acp_agent: cfg_acp_agent.clone(),
            task_board_selected_tasks: std::collections::HashSet::new(),
            task_board_bulk_active_status: None,
            task_board_bulk_priority_input: "999".to_string(),
            task_board_bulk_model_input: "auto".to_string(),
            task_board_bulk_agent: crate::app::task::TASK_AGENT_MAIN.to_string(),
            task_board_bulk_acp_agent: cfg_acp_agent.clone(),
            task_board_bulk_reasoning_effort: None,
            task_board_bulk_task_split_mode: crate::app::task::TaskSplitMode::FollowSystem,
            task_board_manual_merge_queue: Vec::new(),
            task_board_selected_task: None,
            task_board_viewing_logs: None,
            task_board_detail_tab: crate::app::state::TaskBoardDetailTab::default(),
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
            task_board_settings: crate::app::task::TaskBoardSettings::new(),

            // ========== 任务执行器 ==========
            task_board_executor: crate::app::task::TaskExecutorState::new(),
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
            task_board_matrix_event_cursor: 0,
            task_board_processed_matrix_event_ids: std::collections::VecDeque::new(),
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

            // ========== 独立WebView子窗口（仅非WASM平台） ==========
            #[cfg(not(target_arch = "wasm32"))]
            independent_webview_children: Vec::new(),
        };

        if let Some(runtime) = app.session_runtime_states.get_mut("__empty__") {
            runtime.acp_agent = app.acp_agent.clone();
        }

        crate::app::theme_catalog::sync_active_theme_skin(&mut app);

        // 对终端应用主题（仅非WASM平台）
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.terminal.apply_app_theme(&app.app_theme);
        }

        let startup_task = Task::batch(vec![
            startup::build_startup_task(&mut app),
            crate::app::views::workflow_manager::load_lifecycle_workflows_task(false),
        ]);

        (app, startup_task)
    }
}

#[cfg(test)]
#[path = "new_tests.rs"]
mod new_tests;
