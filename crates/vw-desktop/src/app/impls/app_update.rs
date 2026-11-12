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
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use super::message;
use super::{App, Message, Screen};
use crate::app::state::RuntimePlatform;
use crate::app::ui_types::ClipboardShortcutKind;

fn sort_acp_agent_names(mut acp_agents: Vec<String>) -> Vec<String> {
    acp_agents.sort_by(|left, right| {
        let left_key = (left != "codex", left.as_str());
        let right_key = (right != "codex", right.as_str());
        left_key.cmp(&right_key)
    });
    acp_agents
}

#[cfg(not(target_arch = "wasm32"))]
fn startup_retry_interval(elapsed: Duration) -> Duration {
    if elapsed >= Duration::from_secs(24 * 60 * 60) {
        Duration::from_secs(60)
    } else if elapsed >= Duration::from_secs(60 * 60) {
        Duration::from_secs(20)
    } else if elapsed >= Duration::from_secs(5 * 60) {
        Duration::from_secs(10)
    } else {
        Duration::from_secs(2)
    }
}

fn selected_gateway_health_key(
    settings: &crate::app::state::GatewayClientSettingsState,
) -> Option<String> {
    let server = settings.servers.iter().find(|server| server.id == settings.selected_server_id)?;
    crate::app::message::gateway_health::server_health_key(server)
}

#[cfg(any(test, target_arch = "wasm32"))]
fn selected_gateway_became_healthy(
    settings: &crate::app::state::GatewayClientSettingsState,
    results: &[(String, bool)],
) -> bool {
    selected_gateway_health_key(settings).is_some_and(|key| {
        settings.health.get(&key) != Some(&true)
            && results.iter().any(|(candidate, healthy)| *healthy && candidate == &key)
    })
}

fn route_clipboard_shortcut(app: &App, kind: ClipboardShortcutKind) -> Task<Message> {
    iced::widget::operation::is_focused(app.input_editor_id.clone())
        .map(move |input_focused| Message::ClipboardShortcutRouted { kind, input_focused })
}

fn copy_shortcut_without_input(app: &mut App) -> Task<Message> {
    if app.screen == Screen::Project && app.show_git_copy_modal {
        if app.git_copy_modal_use_color {
            message::git::update(
                app,
                message::GitMessage::CopyModalCodeEditorEvent(iced_code_editor::Message::Copy),
            )
        } else {
            Task::done(Message::CopyCode(app.git_copy_modal_editor.text()))
        }
    } else if app.screen == Screen::Preview {
        message::editor::update(app, message::EditorMessage::Copy)
    } else if app.screen == Screen::Project
        && app.show_diff
        && !app.git_diff_selected_lines.is_empty()
    {
        message::git::update(app, message::GitMessage::CopyDiffSelection)
    } else if app.screen == Screen::Project
        && let Some(selection) = &app.chat_markdown_selection
        && app.active_session_id.as_deref() == Some(selection.session_id.as_str())
    {
        Task::done(Message::CopyCode(selection.text.clone()))
    } else {
        message::editor::update(app, message::EditorMessage::Copy)
    }
}

pub(crate) fn copy_chat_message(
    app: &mut App,
    session_id: String,
    message_idx: usize,
) -> Task<Message> {
    if app.active_session_id.as_deref() != Some(session_id.as_str()) {
        return Task::none();
    }
    let Some(text) = crate::app::components::chat_panel::message_view::chat_message_action_text(
        app,
        message_idx,
        false,
    ) else {
        return Task::none();
    };
    copy_code(app, text)
}

fn copy_code(app: &mut App, text: String) -> Task<Message> {
    use std::hash::{Hash, Hasher};
    use std::time::Duration;
    use web_time::SystemTime;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let hash = hasher.finish();
    app.last_copied_code_hash = Some(hash);
    app.last_copy_time = Some(SystemTime::now());

    Task::batch(vec![
        iced::clipboard::write(text).map(|_: ()| Message::CopyDone),
        message::after(Duration::from_secs(2), Message::CopyFeedbackExpired(hash)),
    ])
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
        if self.active_session_id.is_none() {
            self.acp_agent = acp_agent.clone();
        }

        // Existing sessions keep their own model. Updating the draft defaults must also preserve
        // text and task settings entered while the project GET was in flight.
        let runtime = self.get_session_runtime_mut("__empty__");
        if runtime.task_mode_executor.is_none() && runtime.task_mode_model == runtime.model {
            runtime.task_mode_model = model.clone();
        }
        runtime.model = model;
        runtime.auto_model = auto_model;
        if runtime.acp_agent != acp_agent {
            runtime.acp_agent = acp_agent;
            runtime.agent = None;
            runtime.agent_label = None;
            runtime.acp_model_source = Default::default();
            runtime.acp_options = Default::default();
            runtime.acp_options_pending = false;
            runtime.acp_model_input_draft = None;
            runtime.acp_source_pending = None;
            runtime.acp_source_request_id = None;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn load_startup_sessions(&mut self) -> Task<Message> {
        if self.screen != Screen::Home
            || self.project_path.is_some()
            || self.active_session_id.is_some()
            || !self.sessions.is_empty()
        {
            return Task::none();
        }
        let Some(generation) = self.startup_configuration.begin_sessions() else {
            return Task::none();
        };
        self.reload_sessions_for_project(None).map(move |message| match message {
            Message::Project(result) => {
                Message::StartupSessionsLoaded(generation, Box::new(result))
            }
            message => message,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn apply_startup_gateway_snapshot(
        &mut self,
        generation: u64,
        snapshot: crate::app::impls::StartupGatewaySnapshot,
    ) -> Task<Message> {
        if !self.startup_configuration.accept(generation, &snapshot) {
            return Task::none();
        }
        let mut tasks = Vec::new();
        if let Some(Ok(system)) = snapshot.system {
            self.apply_system_settings_snapshot(system, false);
        }
        let scope =
            crate::app::config::preferences_scope(&crate::app::config::gateway_client_endpoint());
        let agents_loaded = matches!(&snapshot.agent, Some(Ok(_)));
        if let Some(result) = snapshot.agent {
            match result {
                Ok(agent) => {
                    self.apply_startup_agent_config(&agent.config, &agent.gateway);
                    if !self.startup_configuration.setting_edited(&scope, "acp_agents") {
                        self.acp_agents = agent.acp_agents;
                    }
                }
                Err(_) => {
                    tracing::debug!(target: "vw_desktop", "agent configuration unavailable during startup")
                }
            }
        }
        if let Some(result) = snapshot.catalog
            && !self.startup_configuration.setting_edited(&scope, "agents_settings")
            && (result.is_ok() || !self.agents_settings.config_ready)
        {
            tasks.push(message::settings::update(
                self,
                message::SettingsMessage::Agents(message::settings::AgentsMessage::ConfigLoaded {
                    project_path: None,
                    result,
                }),
            ));
        }
        if let Some(Ok(tools)) = snapshot.tools {
            tasks.push(message::settings::update(
                self,
                message::SettingsMessage::Agents(message::settings::AgentsMessage::ToolsLoaded(
                    Ok(tools),
                )),
            ));
        }
        let preferences_loaded = matches!(&snapshot.preferences, Some(Ok(_)));
        if let Some(Ok(mut preferences)) = snapshot.preferences {
            if let Some(fields) = preferences.as_object_mut() {
                self.startup_configuration.pending_acp =
                    fields.remove("acp_agent").and_then(|agent| agent.as_str().map(str::to_owned));
                if self.startup_configuration.agent_loaded()
                    && let Some(agent) = self
                        .startup_configuration
                        .pending_acp
                        .take()
                        .filter(|agent| self.acp_agents.contains(agent))
                {
                    fields.insert("acp_agent".into(), agent.into());
                }
            }
            tasks.push(self.update(Message::StartupAppConfigLoaded(Ok(preferences))));
        } else if agents_loaded
            && let Some(agent) = self.startup_configuration.pending_acp.take()
            && self.acp_agents.contains(&agent)
        {
            self.apply_bootstrap_chat_preferences(&serde_json::json!({"acp_agent": agent}));
        }
        if let Some(Ok(apps)) = snapshot.external_apps {
            let platform = apps.platform.as_deref().and_then(RuntimePlatform::from_gateway_str);
            let selection = self.open_external_app;
            self.apply_external_apps_state(platform, apps.apps);
            if self.desktop_preference_edited(&scope, "open_external_app") {
                self.open_external_app = selection;
            }
        }
        if (agents_loaded || preferences_loaded)
            && self.project_path.is_none()
            && self.active_session_id.is_none()
            && !self.task_board_create_modal_open
            && !self.show_task_board
        {
            self.task_board_last_acp_agent.clone_from(&self.acp_agent);
            if !self.task_board_bulk_executor_popover {
                self.task_board_bulk_acp_agent.clone_from(&self.acp_agent);
            }
        }
        Task::batch(tasks)
    }

    fn apply_bootstrap_app_config(&mut self, mut cfg: serde_json::Value) {
        self.retain_unedited_desktop_preferences(&mut cfg);
        self.apply_task_pet_preferences(&cfg);
        if !self.desktop_update.selected_by_user
            && !self.desktop_update.checking
            && self.desktop_update.background_request.is_none()
            && !self.desktop_update.saving_skip
            && !self.desktop_update.busy
            && self.desktop_update.release.is_none()
        {
            let initial_check_started = self.desktop_update.initial_check_started;
            self.desktop_update = crate::app::desktop_update::DesktopUpdate::from_config(&cfg);
            self.desktop_update.initial_check_started = initial_check_started;
        }
        let mut efforts = crate::app::config::session_reasoning_efforts_from_config(&cfg);
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
        self.reasoning_effort = self.current_session_runtime().reasoning_effort;
        self.apply_bootstrap_chat_preferences(&cfg);
        self.auto_max_mode =
            cfg.get("auto_max_mode").and_then(|v| v.as_bool()).unwrap_or(self.auto_max_mode);
        if let Some(workspace_mode) = cfg
            .get("workspace_mode")
            .and_then(|value| value.as_str())
            .and_then(crate::app::state::WorkspaceMode::parse)
        {
            self.workspace_mode = workspace_mode;
        }
        if let Some(collapsed) = cfg.get("work_sidebar_collapsed").and_then(|v| v.as_bool()) {
            self.work_sidebar_collapsed = collapsed;
        }
        if let Some(ids) = cfg.get("input_model_collapsed_provider_ids").and_then(|v| v.as_array())
        {
            self.model_settings.input_collapsed_provider_ids =
                ids.iter().filter_map(|id| id.as_str().map(str::to_owned)).collect();
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
        if let Some(visible) = cfg.get("show_terminal").and_then(|v| v.as_bool()) {
            self.terminal.set_visible(visible);
        }
        if let Some(target) = cfg
            .get("open_external_app")
            .and_then(|v| v.as_str())
            .and_then(Self::parse_external_open_app)
        {
            self.open_external_app = target;
        }
        if !self.dragging_file_manager
            && let Some(width) = cfg
                .get("file_manager_width")
                .and_then(|v| v.as_f64())
                .map(|v| (v as f32).clamp(180.0, 600.0))
        {
            self.file_manager_width = width;
            self.file_manager_start_width = width;
        }
        if let Some(paths) = cfg.get("file_tree_expanded").and_then(serde_json::Value::as_array) {
            self.file_tree_expanded =
                paths.iter().filter_map(|value| value.as_str().map(str::to_owned)).collect();
            self.file_tree_expanded_set = self.file_tree_expanded.iter().cloned().collect();
        }
        if let Some(value) = cfg.get("chat_send_behavior").and_then(serde_json::Value::as_str) {
            self.chat_send_behavior = crate::app::state::ChatSendBehavior::from_str(value);
        }
        let history_mode = cfg
            .get("acp_history_strategy")
            .and_then(serde_json::Value::as_str)
            .map(crate::app::state::AcpHistoryReplayMode::from_str);
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
        if let Some(value) = cfg.get("new_session_last_directory") {
            self.new_session_last_directory = value.as_str().map(str::to_owned);
        }
        if cfg.get("web_bookmarks").is_some() {
            self.web_bookmarks = super::app_init::load_web_bookmarks(&cfg);
        }
        for (key, setting) in [
            ("show_git_diff_summary", &mut self.show_git_diff_summary),
            ("show_git_diff_highlight", &mut self.show_git_diff_highlight),
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

    pub(crate) fn apply_bootstrap_system_settings(
        &mut self,
        system: vw_config_types::ui::AppSystemSettingsConfig,
    ) {
        self.apply_system_settings_snapshot(system, true);
    }

    fn apply_system_settings_snapshot(
        &mut self,
        system: vw_config_types::ui::AppSystemSettingsConfig,
        apply_gateway_client: bool,
    ) {
        use crate::app::config::SystemSettingsSection;

        #[cfg(not(target_arch = "wasm32"))]
        let scope =
            crate::app::config::preferences_scope(&crate::app::config::gateway_client_endpoint());
        let edited = |app: &Self, field: &str| {
            #[cfg(not(target_arch = "wasm32"))]
            return app.startup_configuration.setting_edited(&scope, field);
            #[cfg(target_arch = "wasm32")]
            {
                let _ = (app, field);
                false
            }
        };

        if !self.desktop_locale_selected {
            self.desktop_locale = system.desktop_locale;
            crate::app::i18n::editor::refresh(self);
        }
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

        if !edited(self, "terminal_shell") {
            self.terminal.shell = match system.terminal_shell.as_str() {
                "zsh" => crate::app::Shell::Zsh,
                _ => crate::app::Shell::Bash,
            };
        }
        if !edited(self, "terminal_font_family") {
            self.terminal.set_font_family(system.terminal_font_family);
        }
        if !edited(self, "terminal_font_size") {
            self.terminal.set_font_size(system.terminal_font_size);
        }
        if !edited(self, "editor_metrics") {
            crate::app::message::editor::metrics::apply_snapshot(
                self,
                system.editor_font_size,
                system.editor_line_height,
                system.editor_auto_line_height,
            );
        }
        if !self.system_settings_edited(SystemSettingsSection::Timezone) {
            self.chat_timezone = system.timezone.clone();
        }
        if !self.task_completion_notification_saving
            && !edited(self, "task_completion_notification")
        {
            self.task_completion_notification = system.task_completion_notification;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.merge_startup_worktree_settings(system.project_worktree_enabled);
        #[cfg(target_arch = "wasm32")]
        {
            self.project_worktree_enabled = system.project_worktree_enabled;
        }
        if !self.system_settings_edited(SystemSettingsSection::DialogueFlow) {
            self.dialogue_flow_show_reasoning_summary = system.dialogue_flow_show_reasoning_summary;
            self.dialogue_flow_expand_shell_tool_section =
                system.dialogue_flow_expand_shell_tool_section;
            self.dialogue_flow_expand_edit_tool_section =
                system.dialogue_flow_expand_edit_tool_section;
        }
        if !edited(self, "quick_messages") {
            self.quick_messages = system.quick_messages;
            vw_config_types::ui::upgrade_legacy_quick_message_actions(&mut self.quick_messages);
            vw_config_types::ui::append_missing_builtin_general_quick_messages(
                &mut self.quick_messages,
            );
            vw_config_types::ui::append_missing_builtin_git_quick_messages(
                &mut self.quick_messages,
            );
        }
        if !edited(self, "preview_auto_save") {
            self.preview_auto_save_mode = system.preview_auto_save;
        }
        if !edited(self, "editor_follow_system_theme") {
            self.editor_follow_system_theme = system.editor_follow_system_theme;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let lsp_disabled = !system.lsp_enabled;
            if self.lsp_disabled != lsp_disabled && !edited(self, "lsp_enabled") {
                if system.lsp_enabled {
                    crate::app::message::preview::lsp::enable_lsp(self);
                } else {
                    crate::app::message::preview::lsp::disable_lsp(self);
                }
            }
        }
        let theme_store = crate::app::theme_catalog::theme_store_from_home();
        if !self.system_settings_edited(SystemSettingsSection::Theme) {
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
        }
        if !edited(self, "editor_theme") {
            self.editor_theme = crate::app::theme_catalog::resolve_app_theme_name(
                &system.editor_theme,
                theme_store.as_ref(),
            )
            .unwrap_or_else(|| self.app_theme.clone());
        }
        let editor_theme = self.effective_editor_theme();
        for tab in self.preview_tabs.iter_mut() {
            tab.editor.set_theme(editor_theme.clone());
        }
        self.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&editor_theme));
        if apply_gateway_client
            && !self.system_settings_edited(SystemSettingsSection::GatewayClient)
            && self.gateway_client_settings.save_error.is_none()
        {
            self.apply_bootstrap_gateway_client(system.gateway_client);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.terminal.theme == crate::app::TerminalTheme::System {
            self.terminal.apply_app_theme(&self.app_theme);
        }
    }

    fn apply_bootstrap_gateway_client(
        &mut self,
        config: vw_config_types::ui::GatewayClientSystemSettingsConfig,
    ) {
        let active_gateway_client = config.active_server();
        self.gateway_client_settings.servers = config
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            message::voice::invalidate(self);
            if matches!(
                &message,
                Message::View(message::ViewMessage::CloseRequested(_))
                    | Message::View(message::ViewMessage::WindowClosed(_))
                    | Message::View(message::ViewMessage::GlobalKeyPressed(
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                        _
                    ))
            ) {
                message::voice::cancel(self);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if matches!(
            message,
            Message::Settings(message::SettingsMessage::AgentAppsAuth(
                message::agent_apps_auth::Event::LoginTick
            ))
        ) {
            return Task::none();
        }
        let refresh_pins = self.prepare_session_ui_projection(&message);
        let task = match message {
            Message::SessionDetailsHovered { session_id, project_id, directory } => {
                self.load_session_hover_details(session_id, project_id, directory)
            }
            Message::SessionDetailsLoaded(id, details) => {
                self.session_ui_projection.hover_details_loading.remove(&id);
                self.session_ui_projection.hover_details.insert(id, details);
                Task::none()
            }
            Message::SessionPinsLoaded(generation, result) => {
                self.apply_session_pins(generation, result);
                Task::none()
            }
            #[cfg(not(target_arch = "wasm32"))]
            Message::StartupCliServiceBootstrapped(result) => {
                if self.startup_state == crate::app::state::NativeStartupState::Ready {
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        self.startup_state = crate::app::state::NativeStartupState::Ready;
                        tracing::info!(
                            target: "vw_desktop",
                            "desktop gateway is ready"
                        );
                    }
                    Err(error) => {
                        let retry_after = startup_retry_interval(self.startup_started_at.elapsed());
                        tracing::debug!(
                            target: "vw_desktop",
                            error = %error,
                            retry_after_seconds = retry_after.as_secs(),
                            "desktop CLI service bootstrap will retry"
                        );
                        return Task::perform(
                            async move {
                                tokio::time::sleep(retry_after).await;
                                crate::app::config::bootstrap_cli_service_async().await
                            },
                            Message::StartupCliServiceBootstrapped,
                        );
                    }
                }
                let mut tasks = vec![
                    crate::app::message::account::load_repository(),
                    self.startup_configuration.load(),
                    Task::done(Message::GatewayHealthTick),
                    Task::done(Message::Settings(message::SettingsMessage::ModelsRefresh)),
                ];
                tasks.push(self.load_startup_sessions());
                Task::batch(tasks)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Message::StartupGatewaySnapshotLoaded(generation, snapshot) => {
                self.apply_startup_gateway_snapshot(generation, *snapshot)
            }
            #[cfg(not(target_arch = "wasm32"))]
            Message::StartupSessionsLoaded(generation, message) => {
                let succeeded = matches!(
                    &*message,
                    message::ProjectMessage::SessionBootstrapLoaded { result: Ok(_), .. }
                );
                if self.startup_configuration.accept_sessions(generation, succeeded)
                    && self.active_session_id.is_none()
                {
                    return self.update(Message::Project(*message));
                }
                Task::none()
            }
            Message::StartupAppConfigLoaded(result) | Message::BootstrapAppConfig(result) => {
                if let Ok(cfg) = result {
                    self.apply_bootstrap_app_config(cfg);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if !self.desktop_update.initial_check_started {
                    self.desktop_update.initial_check_started = true;
                    return Task::batch([
                        message::view::sync_task_pet_window(self),
                        Task::done(Message::View(message::ViewMessage::AutoCheckAppUpdate)),
                    ]);
                }
                message::view::sync_task_pet_window(self)
            }
            Message::StartupSystemSettingsLoaded(result)
            | Message::BootstrapSystemSettings(result) => {
                if let Ok(system) = result {
                    self.apply_bootstrap_system_settings(system);
                    let health = Task::done(Message::GatewayHealthTick);
                    // 输入栏依赖模型能力数据，桌面端也必须在启动时加载。
                    return Task::batch(vec![
                        health,
                        Task::done(Message::Settings(
                            crate::app::message::SettingsMessage::ModelsRefresh,
                        )),
                    ]);
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
            Message::ProjectChatPreferencesLoaded(scope, revision, result) => {
                self.project_chat_preferences_loaded(scope, revision, result)
            }
            Message::ProjectChatPreferencesSaved(scope, result) => {
                self.project_chat_preferences_saved(scope, result)
            }
            Message::DesktopPreferencesSaved(scope, result) => {
                self.desktop_preferences_saved(scope, result)
            }
            Message::SystemSettingsSaved(scope, section, result) => {
                self.system_settings_saved(scope, section, result)
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
                #[cfg(not(target_arch = "wasm32"))]
                let selected_gateway_is_healthy =
                    selected_gateway_health_key(&self.gateway_client_settings).is_some_and(|key| {
                        results.iter().any(|(candidate, healthy)| *healthy && candidate == &key)
                    });
                #[cfg(target_arch = "wasm32")]
                let selected_gateway_became_healthy =
                    selected_gateway_became_healthy(&self.gateway_client_settings, &results);
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
                #[cfg(not(target_arch = "wasm32"))]
                if self.startup_state == crate::app::state::NativeStartupState::Ready
                    && selected_gateway_is_healthy
                {
                    return Task::batch([
                        self.startup_configuration.load(),
                        self.load_startup_sessions(),
                    ]);
                }
                #[cfg(target_arch = "wasm32")]
                if selected_gateway_became_healthy {
                    return Task::batch(vec![
                        Task::perform(
                            crate::app::config::load_full_agent_config_async(),
                            |result| Message::StartupAgentConfigLoaded(result.map(Box::new)),
                        ),
                        crate::app::message::settings::reload_agents_config_task(
                            self.agents_settings.project_path.clone(),
                        ),
                    ]);
                }
                Task::none()
            }
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
            #[cfg(not(target_arch = "wasm32"))]
            Message::Voice(event) => message::voice::update(self, event),
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

            // 复制/剪切/粘贴：先判断聊天输入框是否聚焦，再路由到输入区或代码编辑器上下文
            Message::CopyShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Copy),
            Message::CutShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Cut),
            Message::PasteShortcut => route_clipboard_shortcut(self, ClipboardShortcutKind::Paste),
            Message::ClipboardShortcutRouted { kind, input_focused } => match (kind, input_focused)
            {
                (ClipboardShortcutKind::Copy, true) => {
                    self.update(Message::Chat(message::ChatMessage::CopyInputSelection))
                }
                (ClipboardShortcutKind::Cut, true) => {
                    self.update(Message::Chat(message::ChatMessage::CutInputSelection))
                }
                (ClipboardShortcutKind::Paste, true) => {
                    self.update(Message::Chat(message::ChatMessage::PasteIntoInput))
                }
                (ClipboardShortcutKind::Copy, false) => copy_shortcut_without_input(self),
                (ClipboardShortcutKind::Cut, false) => {
                    message::editor::update(self, message::EditorMessage::Cut)
                }
                (ClipboardShortcutKind::Paste, false) => {
                    message::editor::update(self, message::EditorMessage::Paste)
                }
            },

            // 复制代码到剪贴板
            Message::CopyCode(s) => copy_code(self, s),
            Message::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text } => {
                if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                    if let Some(text) = text {
                        self.chat_markdown_selection =
                            Some(crate::app::ui_types::ChatMarkdownSelection {
                                session_id,
                                message_idx,
                                text_idx,
                                text,
                            });
                    } else if self.chat_markdown_selection.as_ref().is_some_and(|selection| {
                        selection.session_id == session_id
                            && selection.message_idx == message_idx
                            && selection.text_idx == text_idx
                    }) {
                        self.chat_markdown_selection = None;
                    }
                }
                Task::none()
            }

            Message::CopyChatMessage { session_id, message_idx } => {
                copy_chat_message(self, session_id, message_idx)
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

        #[cfg(not(target_arch = "wasm32"))]
        if !self.show_system_settings
            || self.system_settings_tab != crate::app::components::system_settings::SystemTab::Auth
        {
            self.agent_apps_auth.login = None;
            self.agent_apps_auth.binding = None;
            if self.agent_apps_auth.account_action.as_ref().is_some_and(|action| {
                !matches!(action.phase, crate::app::state::agent_apps_auth::ActionPhase::Saving)
            }) {
                self.agent_apps_auth.account_action = None;
            }
            self.agent_apps_auth.focused_service = None;
        }
        message::agent_apps_diagnostics::model::invalidate(self);
        #[cfg(not(target_arch = "wasm32"))]
        message::voice::invalidate(self);
        let cleanup = message::settings::mcp_management_probe::invalidate(self);
        let tools_cleanup = message::settings::mcp_management_tools::invalidate(self);
        let cron_history = message::settings::cron_history::reconcile(self);
        let task = Task::batch([task, cleanup, tools_cleanup, cron_history]);
        if refresh_pins { Task::batch([task, self.refresh_session_pins_task()]) } else { task }
    }
}
#[cfg(test)]
#[path = "app_update_tests.rs"]
mod app_update_tests;
