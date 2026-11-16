use super::*;
use iced::Task;
use std::collections::HashMap;

mod settings;

use desktop_settings_runtime::app_init::settings::build_multimodal_settings;

#[cfg(test)]
#[path = "new/settings_tests.rs"]
mod settings_tests;

impl App {
    pub(crate) fn apply_bootstrap_agent_config(
        &mut self,
        config: &config_aggregate::Config,
    ) {
        settings::apply_bootstrap_agent_config(self, config, &config.gateway);
        self.session_state.acp_agents = sort_acp_agents(&config.acp);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn apply_startup_agent_config(
        &mut self,
        config: &config_aggregate::Config,
        gateway: &config_gateway::GatewayConfig,
    ) {
        settings::apply_bootstrap_agent_config(self, config, gateway);
    }

    /// 使用本机连接配置和默认值构造首屏，由启动任务在服务就绪后回填配置。
    pub fn new() -> (Self, Task<Message>) {
        let cfg = serde_json::json!({});
        let system_settings_cfg = config_ui::AppSystemSettingsConfig::default();
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
        let cfg_acp_agent = cfg
            .get("acp_agent")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(ToString::to_string);

        // Git差异显示配置
        let cfg_show_git_diff_summary = cfg
            .get("show_git_diff_summary")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);
        let cfg_show_git_diff_highlight = cfg
            .get("show_git_diff_highlight")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);
        // 项目worktree功能开关
        let cfg_project_worktree_enabled = system_settings_cfg.project_worktree_enabled.clone();
        let gateway_client_cfg = config::load_gateway_client_config();

        #[cfg(not(target_arch = "wasm32"))]
        let full_agent_cfg = config::load_local_agent_config().unwrap_or_default();
        #[cfg(target_arch = "wasm32")]
        let full_agent_cfg = config_aggregate::Config::default();
        let global_acp_cfg = &full_agent_cfg.acp;
        let acp_agents = sort_acp_agents(global_acp_cfg);
        let cfg_acp_agent =
            cfg_acp_agent.filter(|agent| acp_agents.iter().any(|candidate| candidate == agent));
        let autonomy_cfg = full_agent_cfg.autonomy.clone();
        let gateway_cfg_result: Result<_, String> = Ok(full_agent_cfg.gateway.clone());
        let channels_cfg = full_agent_cfg.channels_config.clone();

        // 新建会话时的默认目录

        let (open_external_platform, open_external_exists, open_external_app) =
            external_apps::resolve_external_apps(&cfg);

        // 初始化会话相关状态

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
        // 创建应用程序实例并初始化所有状态字段
        let mut app = Self {
            git: desktop_git_ui::state::State::new(cfg_show_git_diff_summary, cfg_show_git_diff_highlight),
            // ========== 基础状态 ==========
            #[cfg(not(target_arch = "wasm32"))]
            startup_state: state::NativeStartupState::Starting,
            #[cfg(not(target_arch = "wasm32"))]
            startup_started_at: std::time::Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            startup_configuration: Default::default(),
            navigation_workspace: desktop_navigation::app_init::work_workspace(),
            navigation_mode: desktop_navigation::app_init::workspace_mode(&cfg),
            layout_drag: desktop_navigation::app_init::layout_drag(&cfg),
            popover_visibility: Default::default(),
            navigation_panels: desktop_navigation::app_init::panels(&cfg),
            usage_panel: Default::default(),
            update_panel: Default::default(),
            navigation_tabs: desktop_navigation::app_init::tabs(),
            companion_navigation: Default::default(),
            web_navigation: desktop_apps_ui::web::State { web_bookmarks: cfg_web_bookmarks, ..Default::default() },
            external_navigation: desktop_navigation::panel::external::State {
                open_external_app,
                open_external_exists,
            },
            navigation_sidebar: desktop_navigation::app_init::sidebar(&cfg),
            session_ui_projection: Default::default(),
            work_browser: state::WorkBrowserState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            work_browser_controller: crate::app::webview_runtime::WorkBrowserController::default(),
            workspace_state: desktop_workspace::project_state::State::new(&cfg, load_recent_projects(), recent_meta, cfg_project_worktree_enabled),

            project_input: Default::default(),
            project_find: Default::default(),

            // ========== 输入和模型配置 ==========
            #[cfg(not(target_arch = "wasm32"))]
            voice: Default::default(),
            session_state: desktop_session::session::state::State::new(&cfg, cfg_acp_agent.clone(), acp_agents),

            acp_settings: super::state::AcpSettingsState::default(),
            mcp_settings: super::state::McpSettingsState::default(),

            // ========== 聊天相关状态 ==========
            chat_view: desktop_chat_view::view_snapshot::SessionViewSnapshot::initial(),

            chat_interaction: desktop_chat_view::interaction::State::new(),

            todo_state: desktop_chat_view::todo::State::new(),

            usage_center: Default::default(),

            // ========== 窗口和布局 ==========
            window_size: (1200.0, 800.0),
            mobile_layout: crate::app::state::MobileLayoutState::new(1200.0, 800.0),
            fullscreen_layout_settling: false,
            startup_resize_checked: false,

            // ========== 设置面板 ==========
            general_settings: desktop_settings_appearance::general_state::State::new(&system_settings_cfg),
            #[cfg(not(target_arch = "wasm32"))]
            web_install: Default::default(),
            desktop_update: crate::app::desktop_update::DesktopUpdate::from_config(&cfg),

            open_external_platform,

            // ========== 面板拖拽和尺寸 ==========
            file_manager: desktop_workspace::file_manager::FileManagerState::from_config(&cfg),
            status_animation_frame: 0,

            model_popover_hover: None,

            // ========== 搜索和缓存 ==========
            search: desktop_search::State::default(),

            clipboard_state: desktop_navigation::clipboard::Feedback::new(),

            pet_state: desktop_task_board::pet_state::State::new(),

            task_pet_size_pending: None,

            file_search_scroll_id: iced::widget::Id::new("file_search"),
            slash_palette_scroll_id: iced::widget::Id::new("slash_palette"),
            preview_state: desktop_editor::preview::state::State::new(&system_settings_cfg),

            // ========== 光标和窗口位置 ==========
            cursor_position: iced::Point::ORIGIN,
            window_position: (0.0, 0.0),
            main_window_id: None,
            released_main_windows: Vec::new(),
            task_pet_window_id: None,
            shutdown_requested: false,
            desktop_preferences_persistence: Default::default(),

            spinner_frame: 0,

            // ========== 项目编辑 ==========
            project_editor: Default::default(),

            // ========== Diff和终端 ==========
            terminal_config_persistence: Default::default(),
            terminal: TerminalState::with_system_settings(cfg_show_terminal, &system_settings_cfg),
            terminals_by_project: HashMap::new(),
            appearance: desktop_settings_appearance::theme_state::State::new(&system_settings_cfg),
            input_feedback: Default::default(),

            launch_at_login: crate::app::desktop_lifecycle::launch_at_login_enabled(),
            launch_at_login_busy: false,
            close_to_tray: crate::app::desktop_lifecycle::close_to_tray_enabled(),

            archived_chats: super::state::ArchivedChatsState::default(),
            worktree_settings: Default::default(),

            provider_settings: super::state::ProviderSettingsState::default(),
            model_settings: super::state::ModelSettingsState::default(),
            embedding_routes_settings: settings::build_embedding_routes_settings(
                &full_agent_cfg,
                &gateway_cfg_result,
            ),
            model_routes_settings: settings::build_model_routes_settings(&full_agent_cfg),
            query_classification_settings: settings::build_query_classification_settings(
                &full_agent_cfg,
            ),

            // ========== 心跳设置 ==========
            heartbeat_settings: settings::build_heartbeat_settings(&full_agent_cfg),

            // ========== 定时任务设置 ==========
            cron_settings: settings::build_cron_settings(&full_agent_cfg),

            // ========== 调度器设置 ==========
            scheduler_settings: settings::build_scheduler_settings(&full_agent_cfg),

            // ========== Hooks 设置 ==========
            hooks_settings: settings::build_hooks_settings(&full_agent_cfg),

            // ========== 运行时设置 ==========
            runtime_settings: settings::build_runtime_settings(&full_agent_cfg),

            // ========== 技能设置 ==========
            skills_settings: settings::build_skills_settings(&full_agent_cfg.skills),

            web_search_settings: settings::build_web_search_settings(&full_agent_cfg),

            #[cfg(not(target_arch = "wasm32"))]
            computer_control_settings: Default::default(),
            browser_settings: settings::build_browser_settings(&full_agent_cfg),

            http_request_settings: settings::build_http_request_settings(&full_agent_cfg),

            gateway_settings: settings::build_gateway_settings(&gateway_cfg_result),

            apps_catalog: super::state::AppsCatalogState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            agent_apps_auth: crate::app::state::agent_apps_auth::State::with_browser_preference(
                crate::app::message::agent_apps_auth::browser::load_preference(),
            ),
            app_providers: Default::default(),
            gateway_client_settings:
                desktop_settings_network::app_init::settings::build_gateway_client_settings(
                    &gateway_client_cfg,
                ),

            agents_settings: super::state::AgentsSettingsState {
                entries: super::state::agent_settings_entries_from_config(&full_agent_cfg),
                ..Default::default()
            },

            // ========== 成本控制设置 ==========
            cost_settings: super::state::CostSettingsState::default(),

            // ========== 记忆系统设置 ==========
            memory_settings: settings::build_memory_settings(&full_agent_cfg),

            channels_settings,

            // ========== 可靠性设置 ==========
            reliability_settings: settings::build_reliability_settings(&full_agent_cfg),

            multimodal_settings: build_multimodal_settings(&full_agent_cfg),

            // ========== 安全设置 ==========
            security_settings: settings::build_security_settings(&full_agent_cfg),

            // ========== 自主性设置 ==========
            autonomy_settings: super::state::AutonomySettingsState::from_config(&autonomy_cfg),

            // ========== 可观测性设置 ==========
            observability_settings: settings::build_observability_settings(&full_agent_cfg),

            // ========== 存储设置 ==========
            storage_settings: settings::build_storage_settings(&full_agent_cfg),

            // ========== 代理设置 ==========
            proxy_settings: settings::build_proxy_settings(&full_agent_cfg),

            // ========== 隧道设置 ==========
            tunnel_settings: settings::build_tunnel_settings(&full_agent_cfg),

            // ========== 主题设置 ==========
            themes_settings: super::state::ThemesSettingsState::default(),

            file_tree_interaction: Default::default(),
            input_suggestions: Default::default(),

            // ========== 查找结果标签页 ==========

            // ========== LSP状态（仅非WASM平台） ==========
            #[cfg(not(target_arch = "wasm32"))]
            lsp_events,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_event_sender,
            #[cfg(not(target_arch = "wasm32"))]
            lsp_manager,

            focus_area: FocusArea::None,

            knowledge: super::state::KnowledgeUiState::default(),
            workflow_state: crate::apps::workflow::state::WorkflowState::default(),

            // ========== 通知系统 ==========
            notification_state: desktop_navigation::message::notification::OwnedState::new(),

            toast_state: desktop_ui_controls::toast::State::new(),

            account: crate::app::state::AccountState::default(),
            #[cfg(not(target_arch = "wasm32"))]
            shiju_apps: crate::app::shiju_apps::ShijuAppsState::default(),
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

            task_board: desktop_task_board::state::State::new(&cfg, cfg_acp_agent.clone(), init_ms as u64),

        };

        if let Some(runtime) = app.session_state.session_runtime_states.get_mut("__empty__") {
            runtime.acp_agent = app.session_state.acp_agent.clone();
        }

        crate::app::theme_catalog::sync_active_theme_skin(&mut app);

        // 对终端应用主题（仅非WASM平台）
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.terminal.apply_app_theme(&app.appearance.app_theme);
        }

        let startup_task = Task::batch(vec![
            startup::build_startup_task(&mut app),
            crate::app::views::workflow_manager::load_lifecycle_workflows_task(false),
        ]);

        (app, startup_task)
    }
}
