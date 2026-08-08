//! 应用程序消息更新模块
//!
//! 本模块实现了应用程序的核心消息处理逻辑，基于 Elm 架构的更新模式。
//! 所有用户交互、系统事件和异步操作结果都通过统一的消息机制路由到对应的处理器。
//!
//! # 主要功能
//!
//! - 消息路由：将不同类型的消息分发到对应的功能模块处理器
//! - 屏幕上下文感知：根据当前屏幕状态执行不同的逻辑分支
//! - 快捷键处理：处理复制、粘贴等全局快捷键操作
//! - 批量消息处理：支持批量执行多个消息以优化性能
//!
//! # 架构设计
//!
//! 采用 Elm 架构的 Update 模式，所有状态变更都通过消息驱动：
//! - 每个功能模块有独立的消息类型和更新函数
//! - 通过模式匹配实现消息分发
//! - 返回 Task 用于执行异步操作（如剪贴板写入）

use iced::Task;
use iced_code_editor::theme;
use std::collections::HashMap;

use super::message;
use super::{App, Message, Screen};
use crate::app::state::RuntimePlatform;

fn sort_acp_agent_names(mut acp_agents: Vec<String>) -> Vec<String> {
    acp_agents.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    acp_agents
}

impl App {
    fn parse_external_open_app(value: &str) -> Option<crate::app::state::ExternalOpenApp> {
        match value {
            "finder" => Some(crate::app::state::ExternalOpenApp::Finder),
            "vscode" => Some(crate::app::state::ExternalOpenApp::VSCode),
            "cursor" => Some(crate::app::state::ExternalOpenApp::Cursor),
            "trae" => Some(crate::app::state::ExternalOpenApp::Trae),
            "windsurf" => Some(crate::app::state::ExternalOpenApp::Windsurf),
            "kiro" => Some(crate::app::state::ExternalOpenApp::Kiro),
            "zed" => Some(crate::app::state::ExternalOpenApp::Zed),
            "textmate" => Some(crate::app::state::ExternalOpenApp::TextMate),
            "antigravity" => Some(crate::app::state::ExternalOpenApp::Antigravity),
            "terminal" => Some(crate::app::state::ExternalOpenApp::Terminal),
            "iterm2" => Some(crate::app::state::ExternalOpenApp::ITerm2),
            "ghostty" => Some(crate::app::state::ExternalOpenApp::Ghostty),
            "xcode" => Some(crate::app::state::ExternalOpenApp::Xcode),
            "android-studio" => Some(crate::app::state::ExternalOpenApp::AndroidStudio),
            "powershell" => Some(crate::app::state::ExternalOpenApp::PowerShell),
            "sublime-text" => Some(crate::app::state::ExternalOpenApp::SublimeText),
            _ => None,
        }
    }

    fn external_open_priority(
        platform: Option<RuntimePlatform>,
    ) -> &'static [crate::app::state::ExternalOpenApp] {
        if matches!(platform, Some(RuntimePlatform::MacOs)) {
            &[
                crate::app::state::ExternalOpenApp::Trae,
                crate::app::state::ExternalOpenApp::Windsurf,
                crate::app::state::ExternalOpenApp::Kiro,
                crate::app::state::ExternalOpenApp::Cursor,
                crate::app::state::ExternalOpenApp::VSCode,
                crate::app::state::ExternalOpenApp::Zed,
                crate::app::state::ExternalOpenApp::Xcode,
                crate::app::state::ExternalOpenApp::AndroidStudio,
                crate::app::state::ExternalOpenApp::SublimeText,
                crate::app::state::ExternalOpenApp::TextMate,
                crate::app::state::ExternalOpenApp::Antigravity,
                crate::app::state::ExternalOpenApp::Ghostty,
                crate::app::state::ExternalOpenApp::ITerm2,
                crate::app::state::ExternalOpenApp::Terminal,
                crate::app::state::ExternalOpenApp::Finder,
            ]
        } else if matches!(platform, Some(RuntimePlatform::Windows)) {
            &[
                crate::app::state::ExternalOpenApp::Trae,
                crate::app::state::ExternalOpenApp::Windsurf,
                crate::app::state::ExternalOpenApp::Kiro,
                crate::app::state::ExternalOpenApp::Cursor,
                crate::app::state::ExternalOpenApp::VSCode,
                crate::app::state::ExternalOpenApp::Zed,
                crate::app::state::ExternalOpenApp::SublimeText,
                crate::app::state::ExternalOpenApp::PowerShell,
                crate::app::state::ExternalOpenApp::Finder,
            ]
        } else {
            &[
                crate::app::state::ExternalOpenApp::Trae,
                crate::app::state::ExternalOpenApp::Windsurf,
                crate::app::state::ExternalOpenApp::Kiro,
                crate::app::state::ExternalOpenApp::Cursor,
                crate::app::state::ExternalOpenApp::VSCode,
                crate::app::state::ExternalOpenApp::Zed,
                crate::app::state::ExternalOpenApp::SublimeText,
                crate::app::state::ExternalOpenApp::Finder,
            ]
        }
    }

    fn apply_external_apps_state(
        &mut self,
        platform: Option<RuntimePlatform>,
        apps: Vec<(String, bool)>,
    ) {
        let mut open_external_exists: HashMap<crate::app::state::ExternalOpenApp, bool> =
            HashMap::new();
        for (id, available) in apps {
            if let Some(target) = Self::parse_external_open_app(&id) {
                open_external_exists.insert(target, available);
            }
        }
        open_external_exists.insert(crate::app::state::ExternalOpenApp::Finder, true);
        self.open_external_platform = platform.or(self.open_external_platform);
        self.open_external_exists = open_external_exists;

        if self.open_external_exists.get(&self.open_external_app).copied().unwrap_or(false) {
            return;
        }
        if let Some(target) = Self::external_open_priority(self.open_external_platform)
            .iter()
            .copied()
            .find(|candidate| self.open_external_exists.get(candidate).copied().unwrap_or(false))
        {
            self.open_external_app = target;
        } else {
            self.open_external_app = crate::app::state::ExternalOpenApp::Finder;
        }
    }

    pub(crate) fn apply_project_chat_preferences(
        &mut self,
        model: String,
        auto_model: bool,
        acp_agent: Option<String>,
    ) {
        self.model = model.clone();
        self.auto_model = auto_model;
        self.acp_agent = acp_agent;
        let mut runtime = crate::app::state::SessionRuntimeState::with_defaults(model, auto_model);
        runtime.acp_agent = self.acp_agent.clone();
        self.session_runtime_states.insert("__empty__".to_string(), runtime);
    }

    fn apply_bootstrap_app_config(&mut self, cfg: serde_json::Value) {
        self.auto_model =
            cfg.get("auto_model").and_then(|v| v.as_bool()).unwrap_or(self.auto_model);
        self.model = cfg.get("model").and_then(|v| v.as_str()).unwrap_or(&self.model).to_string();
        self.acp_agent = cfg
            .get("acp_agent")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .or(self.acp_agent.clone());
        self.auto_max_mode =
            cfg.get("auto_max_mode").and_then(|v| v.as_bool()).unwrap_or(self.auto_max_mode);
        if let Some(workspace_mode) = cfg
            .get("workspace_mode")
            .and_then(|value| value.as_str())
            .and_then(crate::app::state::WorkspaceMode::parse)
        {
            self.workspace_mode = workspace_mode;
        }
        self.show_settings =
            cfg.get("show_settings").and_then(|v| v.as_bool()).unwrap_or(self.show_settings);
        self.show_file_manager = cfg
            .get("show_file_manager")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.show_file_manager);
        self.file_manager_show_changes = cfg
            .get("file_manager_show_changes")
            .and_then(|v| v.as_bool())
            .unwrap_or(self.file_manager_show_changes);
        self.terminal.is_visible =
            cfg.get("show_terminal").and_then(|v| v.as_bool()).unwrap_or(self.terminal.is_visible);
        if let Some(target) = cfg
            .get("open_external_app")
            .and_then(|v| v.as_str())
            .and_then(Self::parse_external_open_app)
        {
            self.open_external_app = target;
        }
        if let Some(width) = cfg
            .get("file_manager_width")
            .and_then(|v| v.as_f64())
            .map(|v| (v as f32).clamp(180.0, 600.0))
        {
            self.file_manager_width = width;
            self.file_manager_start_width = width;
        }
        let file_tree_expanded = cfg
            .get("file_tree_expanded")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.file_tree_expanded_set = file_tree_expanded.iter().cloned().collect();
        self.file_tree_expanded = file_tree_expanded;
    }

    fn apply_bootstrap_system_settings(
        &mut self,
        system: vw_config_types::ui::AppSystemSettingsConfig,
    ) {
        #[cfg(target_arch = "wasm32")]
        vw_provider_resolver::config::set_wasm_gateway_endpoint({
            let active = system.gateway_client.active_server();
            let host = {
                let value = active.host.trim();
                if value.is_empty() { "127.0.0.1".to_string() } else { value.to_string() }
            };
            let skey = if active.skey.trim().is_empty() {
                active.bearer_token.trim().to_string()
            } else {
                active.skey.trim().to_string()
            };
            let auth = vw_gateway_client::GatewayAuth {
                skey: Some(skey).filter(|value| !value.is_empty()),
                ..vw_gateway_client::GatewayAuth::default()
            };
            vw_gateway_client::GatewayEndpoint::new(host, active.port.clamp(1, u16::MAX))
                .with_auth(auth)
        });

        self.terminal.font_family = system.terminal_font_family.clone();
        self.terminal.font_size = system.terminal_font_size;
        self.chat_timezone = system.timezone.clone();
        self.project_worktree_enabled = system.project_worktree_enabled.clone();
        self.dialogue_flow_show_reasoning_summary = system.dialogue_flow_show_reasoning_summary;
        self.dialogue_flow_expand_shell_tool_section =
            system.dialogue_flow_expand_shell_tool_section;
        self.dialogue_flow_expand_edit_tool_section = system.dialogue_flow_expand_edit_tool_section;
        self.quick_messages = system.quick_messages.clone();
        vw_config_types::ui::upgrade_legacy_quick_message_actions(&mut self.quick_messages);
        vw_config_types::ui::append_missing_builtin_general_quick_messages(
            &mut self.quick_messages,
        );
        vw_config_types::ui::append_missing_builtin_git_quick_messages(&mut self.quick_messages);
        self.preview_auto_save_mode = system.preview_auto_save;
        self.editor_follow_system_theme = system.editor_follow_system_theme;
        let theme_store = crate::app::theme_catalog::theme_store_from_home();
        match crate::app::theme_catalog::resolve_app_theme_name(
            &system.app_theme,
            theme_store.as_ref(),
        ) {
            Some(theme) => {
                self.app_theme = theme;
                self.active_custom_theme_id =
                    crate::app::theme_catalog::custom_theme_id_from_config(&system.app_theme)
                        .map(str::to_string);
                crate::app::theme_catalog::sync_active_theme_skin(self);
            }
            None => {
                tracing::warn!(
                    target: "vw_desktop",
                    theme = %system.app_theme,
                    "bootstrap could not resolve app theme; keeping current theme"
                );
            }
        }
        self.editor_theme = crate::app::theme_catalog::resolve_app_theme_name(
            &system.editor_theme,
            theme_store.as_ref(),
        )
        .unwrap_or_else(|| self.app_theme.clone());
        let editor_theme = self.effective_editor_theme();
        for tab in self.preview_tabs.iter_mut() {
            tab.editor.set_theme(editor_theme.clone());
        }
        self.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&editor_theme));
        let active_gateway_client = system.gateway_client.active_server();
        self.gateway_client_settings.servers = system
            .gateway_client
            .normalized_servers()
            .iter()
            .map(crate::app::state::GatewayClientServerDraft::from_config)
            .collect();
        let gateway_health_keys = self
            .gateway_client_settings
            .servers
            .iter()
            .filter_map(crate::app::message::gateway_health::server_health_key)
            .collect::<std::collections::HashSet<_>>();
        self.gateway_client_settings.health.retain(|key, _| gateway_health_keys.contains(key));
        self.gateway_client_settings.selected_server_id = active_gateway_client.id;
        self.gateway_client_settings.name_input = active_gateway_client.name;
        self.gateway_client_settings.host_input = {
            let value = active_gateway_client.host.trim().to_string();
            if value.is_empty() { "127.0.0.1".to_string() } else { value }
        };
        self.gateway_client_settings.port = active_gateway_client.port.clamp(1, u16::MAX);
        self.gateway_client_settings.platform_id_input = active_gateway_client.platform_id;
        self.gateway_client_settings.company_id_input = active_gateway_client.company_id;
        self.gateway_client_settings.skey_input = active_gateway_client.skey;
        #[cfg(not(target_arch = "wasm32"))]
        if self.terminal.theme == crate::app::TerminalTheme::System {
            self.terminal.apply_app_theme(&self.app_theme);
        }
    }

    fn apply_bootstrap_browser_settings(&mut self, browser: vw_config_types::tools::BrowserConfig) {
        self.browser_settings.enabled = browser.enabled;
        self.browser_settings.allowed_domains_input = browser.allowed_domains.join("\n");
        self.browser_settings.allowed_domains_editor =
            iced::widget::text_editor::Content::with_text(&browser.allowed_domains.join("\n"));
        self.browser_settings.browser_open =
            match browser.browser_open.trim().to_ascii_lowercase().as_str() {
                "default" | "new_window" | "new_tab" => {
                    browser.browser_open.trim().to_ascii_lowercase()
                }
                _ => "default".to_string(),
            };
        self.browser_settings.session_name_input = browser.session_name.unwrap_or_default();
        self.browser_settings.backend =
            match browser.backend.trim().to_ascii_lowercase().replace('-', "_").as_str() {
                "agent_browser" => "agent_browser".to_string(),
                "rust_native" | "native" => "native".to_string(),
                "computer_use" => "computer_use".to_string(),
                "auto" => "auto".to_string(),
                _ => "agent_browser".to_string(),
            };
        self.browser_settings.native_headless = browser.native_headless;
        self.browser_settings.native_webdriver_url = browser.native_webdriver_url;
        self.browser_settings.native_chrome_path_input =
            browser.native_chrome_path.unwrap_or_default();
        self.browser_settings.computer_use_endpoint = browser.computer_use.endpoint;
        self.browser_settings.computer_use_api_key_input =
            browser.computer_use.api_key.unwrap_or_default();
        self.browser_settings.computer_use_timeout_ms_input =
            browser.computer_use.timeout_ms.to_string();
        self.browser_settings.computer_use_allow_remote_endpoint =
            browser.computer_use.allow_remote_endpoint;
        self.browser_settings.computer_use_window_allowlist_input =
            browser.computer_use.window_allowlist.join(", ");
        self.browser_settings.computer_use_max_coordinate_x_input = browser
            .computer_use
            .max_coordinate_x
            .map(|value| value.to_string())
            .unwrap_or_default();
        self.browser_settings.computer_use_max_coordinate_y_input = browser
            .computer_use
            .max_coordinate_y
            .map(|value| value.to_string())
            .unwrap_or_default();
    }

    /// 处理应用程序消息并更新状态
    ///
    /// 这是应用程序的核心更新函数，实现了 Elm 架构中的 update 模式。
    /// 所有状态变更都通过此函数进行，确保状态变更的可追踪性和可预测性。
    ///
    /// # 参数
    ///
    /// - `message`: 要处理的消息枚举，包含各种用户交互、系统事件或异步操作结果
    ///
    /// # 返回值
    ///
    /// 返回 `Task<Message>`，表示可能需要执行的异步操作。返回的 Task 可能是：
    /// - `Task::none()`: 无需执行任何操作
    /// - `Task::done()`: 立即产生一个新消息
    /// - `Task::batch()`: 批量执行多个任务
    /// - 其他异步任务（如剪贴板操作）
    ///
    /// # 消息处理流程
    ///
    /// 1. 根据消息类型进行模式匹配
    /// 2. 检查当前屏幕上下文（某些操作仅在特定屏幕下有效）
    /// 3. 委托给对应功能模块的更新函数
    /// 4. 返回可能需要的后续任务
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 处理项目消息
    /// let task = app.update(Message::Project(ProjectMessage::OpenFile(path)));
    ///
    /// // 批量处理多个消息
    /// let task = app.update(Message::Batch(vec![
    ///     Message::CopyCode(content),
    ///     Message::Notification(NotificationMessage::Show("已复制".into())),
    /// ]));
    /// ```
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = match message {
            #[cfg(not(target_arch = "wasm32"))]
            Message::StartupCliServiceBootstrapped(result) => {
                match result {
                    Ok(()) => {
                        if cfg!(debug_assertions) {
                            tracing::debug!(
                                target: "vw_desktop",
                                "desktop CLI service bootstrap skipped for debug build"
                            );
                        } else {
                            tracing::info!(
                                target: "vw_desktop",
                                "desktop CLI service bootstrap completed"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            target: "vw_desktop",
                            error = %error,
                            "desktop CLI service bootstrap failed"
                        );
                    }
                }
                let mut tasks = vec![
                    crate::app::message::account::load_repository(),
                    Task::perform(
                        crate::app::config::load_app_config_async(),
                        Message::StartupAppConfigLoaded,
                    ),
                    Task::perform(
                        crate::app::config::load_system_settings_config_async(),
                        Message::StartupSystemSettingsLoaded,
                    ),
                    Task::perform(
                        crate::app::config::load_full_agent_config_async(),
                        |result| Message::StartupAgentConfigLoaded(result.map(Box::new)),
                    ),
                    Task::perform(
                        crate::app::session_gateway::gateway_external_apps_async(),
                        Message::ExternalAppsLoaded,
                    ),
                    crate::app::message::settings::reload_agents_config_task(
                        self.agents_settings.project_path.clone(),
                    ),
                ];
                tasks.push(self.reload_sessions_for_project(None));
                Task::batch(tasks)
            }
            Message::StartupAppConfigLoaded(result) | Message::BootstrapAppConfig(result) => {
                if let Ok(cfg) = result {
                    self.apply_bootstrap_app_config(cfg);
                }
                Task::none()
            }
            Message::StartupSystemSettingsLoaded(result)
            | Message::BootstrapSystemSettings(result) => {
                if let Ok(system) = result {
                    self.apply_bootstrap_system_settings(system);
                    let health = Task::done(Message::GatewayHealthTick);
                    #[cfg(target_arch = "wasm32")]
                    return Task::batch(vec![
                        health,
                        Task::done(Message::Settings(
                            crate::app::message::SettingsMessage::ModelsRefresh,
                        )),
                    ]);
                    #[cfg(not(target_arch = "wasm32"))]
                    return health;
                }
                Task::none()
            }
            Message::StartupAgentConfigLoaded(result) => {
                match result {
                    Ok(config) => self.apply_bootstrap_agent_config(&config),
                    Err(error) => {
                        tracing::warn!(
                            target: "vw_desktop",
                            error = %error,
                            "failed to reload agent config after gateway bootstrap"
                        );
                    }
                }
                Task::none()
            }
            Message::StartupBrowserConfigLoaded(result)
            | Message::BootstrapBrowserConfig(result) => {
                if let Ok(browser) = result {
                    self.apply_bootstrap_browser_settings(browser);
                }
                Task::none()
            }
            Message::BootstrapAcpAgentsLoaded(result) => {
                if let Ok(acp_agents) = result {
                    self.acp_agents = sort_acp_agent_names(acp_agents);
                }
                Task::none()
            }
            Message::BootstrapArchivedSessions(result) => {
                if let Ok(archived_session_ids) = result {
                    self.archived_session_ids = archived_session_ids;
                }
                Task::none()
            }
            Message::ExternalAppsLoaded(result) => {
                if let Ok(state) = result {
                    let platform =
                        state.platform.as_deref().and_then(RuntimePlatform::from_gateway_str);
                    self.apply_external_apps_state(platform, state.apps);
                }
                Task::none()
            }
            Message::SessionPreviewsLoaded(result) => {
                match result {
                    Ok(previews) => {
                        self.session_previews =
                            crate::app::session::build_session_previews(previews);
                    }
                    Err(error) => {
                        self.session_previews.clear();
                        self.push_notification(format!(
                            "Failed to load session previews: {}",
                            error
                        ));
                    }
                }
                Task::none()
            }
            Message::ProjectChatPreferencesLoaded(project_path, preferences) => {
                if self.project_path.as_deref() == Some(project_path.as_str())
                    && let Some((model, auto_model, acp_agent)) = preferences
                {
                    self.apply_project_chat_preferences(model, auto_model, acp_agent);
                }
                Task::none()
            }
            Message::GatewayHealthTick => {
                let servers = self.gateway_client_settings.servers.clone();
                if servers.is_empty() {
                    Task::none()
                } else {
                    Task::perform(
                        message::gateway_health::check_servers(servers),
                        Message::GatewayHealthChecked,
                    )
                }
            }
            Message::GatewayHealthChecked(results) => {
                let known_keys = self
                    .gateway_client_settings
                    .servers
                    .iter()
                    .filter_map(crate::app::message::gateway_health::server_health_key)
                    .collect::<std::collections::HashSet<_>>();
                self.gateway_client_settings.health.retain(|key, _| known_keys.contains(key));
                for (key, healthy) in results {
                    if known_keys.contains(&key) {
                        self.gateway_client_settings.health.insert(key, healthy);
                    }
                }
                Task::none()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Message::Account(msg) => message::account::update(self, msg),
            #[cfg(not(target_arch = "wasm32"))]
            Message::ShijuApps(msg) => message::shiju_apps::update(self, msg),
            #[cfg(not(target_arch = "wasm32"))]
            Message::SkillHub(msg) => message::skill_hub::update(self, msg),
            // 项目管理相关消息，委托给 project 模块处理
            Message::Project(msg) => message::project::update(self, msg),

            // 视图相关消息，委托给 view 模块处理
            Message::View(msg) => message::view::update(self, msg),

            // Git 版本控制相关消息
            Message::Git(msg) => message::git::update(self, msg),

            // 聊天功能相关消息
            Message::Chat(msg) => message::chat::update(self, msg),

            // 搜索功能相关消息
            Message::Search(msg) => message::search::update(self, msg),

            // 设置相关消息
            Message::Settings(msg) => message::settings::update(self, msg),

            // 终端相关消息
            Message::Terminal(msg) => message::terminal::update(self, msg),

            // 预览相关消息
            Message::Preview(msg) => message::preview::update(self, msg),

            // 编辑器相关消息
            Message::Editor(msg) => message::editor::update(self, msg),

            // 知识库工作台相关消息
            Message::Knowledge(msg) => message::knowledge_tool::update(self, msg),

            // Dify 工作流工具相关消息
            Message::WorkflowTool(msg) => crate::apps::workflow::update(self, msg),

            Message::WorkflowManager(msg) => {
                crate::app::views::workflow_manager::update_app(self, msg)
            }

            // 通知相关消息
            Message::Notification(msg) => message::notification::update(self, msg),

            // 任务看板相关消息
            Message::TaskBoard(msg) => message::task_board::update(self, msg),

            // LSP 心跳消息（仅非 WASM 目标）
            // 用于定期检查语言服务器协议的状态
            #[cfg(not(target_arch = "wasm32"))]
            Message::PreviewLspTick => message::preview::tick_lsp(self),

            // 复制快捷键处理（上下文感知的复制操作）
            Message::CopyShortcut => {
                // 在项目屏幕且显示 Git 复制模态框时
                if self.screen == Screen::Project && self.show_git_copy_modal {
                    if self.git_copy_modal_use_color {
                        // 如果启用了颜色，使用代码编辑器的复制功能
                        message::git::update(
                            self,
                            message::GitMessage::CopyModalCodeEditorEvent(
                                iced_code_editor::Message::Copy,
                            ),
                        )
                    } else {
                        // 否则直接复制模态框编辑器的文本
                        Task::done(Message::CopyCode(self.git_copy_modal_editor.text()))
                    }
                } else if self.screen == Screen::Preview {
                    // 在预览屏幕下，使用编辑器的复制功能
                    message::editor::update(self, message::EditorMessage::Copy)
                } else if self.screen == Screen::Project
                    && self.show_diff
                    && !self.git_diff_selected_lines.is_empty()
                {
                    // 在项目屏幕且显示 diff 且有选中行时，复制 diff 选区
                    message::git::update(self, message::GitMessage::CopyDiffSelection)
                } else {
                    // 其他情况不执行任何操作
                    Task::none()
                }
            }

            // 复制代码到剪贴板
            Message::CopyCode(s) => {
                use std::hash::{Hash, Hasher};
                use std::time::Duration;
                use web_time::SystemTime;

                // 计算复制内容的哈希值，用于后续的去重或追踪
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                s.hash(&mut hasher);
                let h = hasher.finish();

                // 记录最后复制的哈希值和时间
                self.last_copied_code_hash = Some(h);
                self.last_copy_time = Some(SystemTime::now());

                Task::batch(vec![
                    iced::clipboard::write(s).map(|_: ()| Message::CopyDone),
                    message::after(Duration::from_secs(2), Message::CopyFeedbackExpired(h)),
                ])
            }

            // 复制文件内容到剪贴板
            Message::CopyFile(path) => {
                use std::hash::{Hash, Hasher};
                use std::time::Duration;
                use web_time::SystemTime;

                // 读取文件内容，失败时使用空字符串
                let content = std::fs::read_to_string(path).unwrap_or_default();

                // 计算文件内容的哈希值
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                content.hash(&mut hasher);
                let h = hasher.finish();

                // 记录最后复制的哈希值和时间
                self.last_copied_code_hash = Some(h);
                self.last_copy_time = Some(SystemTime::now());

                Task::batch(vec![
                    iced::clipboard::write(content).map(|_: ()| Message::CopyDone),
                    message::after(Duration::from_secs(2), Message::CopyFeedbackExpired(h)),
                ])
            }

            // 复制完成消息，无需执行任何操作
            Message::CopyDone => Task::none(),

            Message::CopyFeedbackExpired(hash) => {
                if self.last_copied_code_hash == Some(hash) {
                    self.last_copied_code_hash = None;
                    self.last_copy_time = None;
                }
                Task::none()
            }

            // 关闭错误提示
            Message::CloseError => {
                self.error_message = None;
                Task::none()
            }

            // 空消息，无需执行任何操作
            Message::None => Task::none(),

            // 批量消息处理
            Message::Batch(messages) => {
                // 对每个消息递归调用 update，收集所有返回的 Task
                let tasks: Vec<Task<Message>> =
                    messages.into_iter().map(|msg| self.update(msg)).collect();
                // 将多个 Task 合并为一个批量 Task
                Task::batch(tasks)
            }

            #[cfg(target_os = "macos")]
            Message::NativeMenu(action) => {
                match crate::app::native_menu::resolve_native_menu_action(self, action) {
                    Some(resolved) => self.update(resolved),
                    None => Task::none(),
                }
            }
        };

        #[cfg(target_os = "macos")]
        {
            crate::app::native_menu::install_or_sync(
                crate::app::native_menu::NativeMenuSnapshot::from_app(self),
            );
        }

        task
    }
}
#[cfg(test)]
#[path = "app_update_tests.rs"]
mod app_update_tests;
