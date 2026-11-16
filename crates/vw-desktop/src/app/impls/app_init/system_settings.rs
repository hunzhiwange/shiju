//! Cross-domain startup assembly; domain states own their local transformations.
use crate::app::App;
use iced_code_editor::theme;

impl App {
    pub(crate) fn apply_bootstrap_system_settings(
        &mut self,
        system: config_ui::AppSystemSettingsConfig,
    ) {
        self.apply_system_settings_snapshot(system, true);
    }

    pub(in crate::app) fn apply_system_settings_snapshot(
        &mut self,
        system: config_ui::AppSystemSettingsConfig,
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

        if !self.general_settings.desktop_locale_selected {
            self.general_settings.desktop_locale = system.desktop_locale;
            crate::app::i18n::editor::refresh(self);
        }
        #[cfg(target_arch = "wasm32")]
        provider_config_reader::set_wasm_gateway_endpoint({
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
            let auth = gateway_endpoint::GatewayAuth {
                skey: Some(skey).filter(|value| !value.is_empty()),
                ..gateway_endpoint::GatewayAuth::default()
            };
            gateway_endpoint::GatewayEndpoint::new(host, active.port.clamp(1, u16::MAX))
                .with_auth(auth)
        });

        if !edited(self, "terminal_shell") {
            self.terminal.set_system_shell(&system.terminal_shell);
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
            self.general_settings.chat_timezone = system.timezone.clone();
        }
        if !self.general_settings.task_completion_notification_saving
            && !edited(self, "task_completion_notification")
        {
            self.general_settings.task_completion_notification =
                system.task_completion_notification;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.merge_startup_worktree_settings(system.project_worktree_enabled);
        #[cfg(target_arch = "wasm32")]
        {
            self.workspace_state.project_worktree_enabled = system.project_worktree_enabled;
        }
        if !self.system_settings_edited(SystemSettingsSection::DialogueFlow) {
            self.general_settings.dialogue_flow_show_reasoning_summary =
                system.dialogue_flow_show_reasoning_summary;
            self.general_settings.dialogue_flow_expand_shell_tool_section =
                system.dialogue_flow_expand_shell_tool_section;
            self.general_settings.dialogue_flow_expand_edit_tool_section =
                system.dialogue_flow_expand_edit_tool_section;
        }
        if !edited(self, "quick_messages") {
            self.general_settings.apply_quick_messages(system.quick_messages);
        }
        if !edited(self, "preview_auto_save") {
            self.preview_state.preview_auto_save_mode = system.preview_auto_save;
        }
        if !edited(self, "editor_follow_system_theme") {
            self.appearance.editor_follow_system_theme = system.editor_follow_system_theme;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let lsp_disabled = !system.lsp_enabled;
            if self.preview_state.lsp_disabled != lsp_disabled && !edited(self, "lsp_enabled") {
                if system.lsp_enabled {
                    crate::app::message::preview::lsp::enable_lsp(self);
                } else {
                    crate::app::message::preview::lsp::disable_lsp(self);
                }
            }
        }
        let theme_store = crate::app::theme_catalog::theme_store_from_home();
        if !self.system_settings_edited(SystemSettingsSection::Theme) {
            if self.appearance.apply_app_theme(&system.app_theme, theme_store.as_ref()) {
                crate::app::theme_catalog::sync_active_theme_skin(self);
            }
        }
        if !edited(self, "editor_theme") {
            self.appearance.apply_editor_theme(&system.editor_theme, theme_store.as_ref());
        }
        let editor_theme = self.effective_editor_theme();
        for tab in self.navigation_tabs.preview_tabs.iter_mut() {
            tab.editor.set_theme(editor_theme.clone());
        }
        self.git.git_copy_modal_code_editor.set_theme(theme::from_iced_theme(&editor_theme));
        if apply_gateway_client
            && !self.system_settings_edited(SystemSettingsSection::GatewayClient)
            && self.gateway_client_settings.save_error.is_none()
        {
            desktop_settings_network::bootstrap::apply_bootstrap_gateway_client(
                &mut self.gateway_client_settings,
                system.gateway_client,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self.terminal.theme == crate::app::TerminalTheme::System {
            self.terminal.apply_app_theme(&self.appearance.app_theme);
        }
    }
}
