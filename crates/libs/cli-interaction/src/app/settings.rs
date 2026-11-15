//! 设置面板与校验。

use super::{
    GatewayV3Bridge,
    PermissionMode,
    ReasoningEffort,
    Result,
    SETTINGS_ACCESS_SURFACE,
    SETTINGS_ACP_SURFACE,
    SETTINGS_DELEGATE_SURFACE,
    SETTINGS_EFFORT_SURFACE,
    SETTINGS_MODEL_SURFACE,
    SETTINGS_PERMISSION_SURFACE,
    SETTINGS_PLAN_SURFACE,
    SETTINGS_SUBAGENT_SURFACE,
    SETTINGS_SURFACE,
    SettingKey,
    SettingsPanel,
    SettingsTarget,
    SettingsValues,
    UiConfirmIntent,
    UiConfirmOverlay,
    V3DisplayEvent,
    V3GatewayApp,
    V3SelectionItem,
    effort_id,
    permission_mode_id,
    push_runner_error,
    setting_item,
};

impl V3GatewayApp {
    fn settings_values(&self) -> SettingsValues {
        SettingsValues {
            model: self.state.session_options.model.clone(),
            delegate_agent: self.state.session_options.delegate_agent.clone(),
            acp_backend: self.state.session_options.acp_backend.clone(),
            full_access_enabled: self.state.session_options.explicit_full_access_enabled(),
            subagent_mode_enabled: self.state.session_options.subagent_mode_enabled(),
            workflow_mode: self.state.session_options.workflow_mode,
            goal_mode: self.state.session_options.goal_mode,
            effort: self.state.session_options.effort,
            fast: self.state.session_options.fast,
            theme: self.state.theme_name.clone(),
            permission_mode: self.state.session_options.permission_mode,
            vim_mode: cli_interaction_model::vim::VimMode::Disabled,
        }
    }

    pub(super) fn open_settings(&mut self) {
        let panel = SettingsPanel::for_capabilities(
            self.settings_values(),
            &self.state.session_capabilities,
        );
        let items = panel
            .rows
            .iter()
            .filter_map(|row| {
                let id = match row.key {
                    SettingKey::Model => "model",
                    SettingKey::DelegateAgent => "delegate",
                    SettingKey::AcpBackend => "acp",
                    SettingKey::FullAccess => "full_access",
                    SettingKey::SubagentMode => "subagents",
                    SettingKey::Effort => "effort",
                    SettingKey::Permissions => "permissions",
                    _ => return None,
                };
                let current = panel.value_label(row.key);
                Some(V3SelectionItem {
                    id: id.to_string(),
                    label: row.label.to_string(),
                    description: Some(
                        row.unavailable_reason
                            .clone()
                            .unwrap_or_else(|| format!("当前：{current}")),
                    ),
                    disabled: row.unavailable_reason.is_some(),
                    disabled_reason: row.unavailable_reason.clone(),
                    ..V3SelectionItem::default()
                })
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_SURFACE, "会话设置", items);
    }

    pub(super) fn open_settings_target(&mut self, target: SettingsTarget) {
        match target {
            SettingsTarget::Model => self.open_model_picker(),
            SettingsTarget::Effort => self.open_effort_picker(),
            SettingsTarget::Permissions => self.open_permission_picker(false),
            SettingsTarget::AcpBackend => self.open_acp_picker(),
            SettingsTarget::DelegateAgent => self.open_delegate_picker(),
            SettingsTarget::FullAccess => self.open_access_picker(),
            SettingsTarget::SubagentMode => self.open_subagent_picker(),
        }
    }

    pub(super) fn open_model_picker(&mut self) {
        if self.state.session_capabilities.models.is_empty() {
            push_runner_error(self, "当前 Gateway 没有声明可选模型");
            return;
        }
        let current = self.state.session_options.model.as_deref();
        let items = self
            .state
            .session_capabilities
            .models
            .iter()
            .map(|model| setting_item(model, model, current == Some(model.as_str())))
            .collect();
        self.widget.show_selection_overlay(SETTINGS_MODEL_SURFACE, "选择模型", items);
    }

    pub(super) fn open_effort_picker(&mut self) {
        if self
            .state
            .session_capabilities
            .efforts_for_model(self.state.session_options.model.as_deref())
            .is_empty()
        {
            push_runner_error(self, "当前 provider 没有声明推理强度能力");
            return;
        }
        let current = self.state.session_options.effort;
        let items = self
            .state
            .session_capabilities
            .efforts_for_model(self.state.session_options.model.as_deref())
            .iter()
            .copied()
            .map(|effort| {
                let id = effort_id(effort);
                setting_item(id, id, effort == current)
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_EFFORT_SURFACE, "选择推理强度", items);
    }

    pub(super) fn open_permission_picker(&mut self, plan_only: bool) {
        let current = self.state.session_options.permission_mode;
        let items = self
            .state
            .session_capabilities
            .permission_modes
            .iter()
            .copied()
            .filter(|mode| {
                !plan_only || matches!(mode, PermissionMode::Default | PermissionMode::Plan)
            })
            .map(|mode| {
                let id = permission_mode_id(mode);
                setting_item(id, id, mode == current)
            })
            .collect();
        let surface = if plan_only { SETTINGS_PLAN_SURFACE } else { SETTINGS_PERMISSION_SURFACE };
        self.widget.show_selection_overlay(surface, "选择权限模式", items);
    }

    fn open_acp_picker(&mut self) {
        let current = self.state.session_options.acp_backend.as_deref();
        let items = self
            .state
            .session_capabilities
            .acp_backends
            .iter()
            .map(|backend| {
                let id = backend.key.as_deref().unwrap_or("default");
                setting_item(id, backend.label.as_str(), current == backend.key.as_deref())
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_ACP_SURFACE, "选择 ACP 后端", items);
    }

    fn open_delegate_picker(&mut self) {
        let current = self.state.session_options.delegate_agent.as_deref();
        let items = self
            .state
            .session_capabilities
            .delegate_agents
            .iter()
            .map(|agent| {
                let id = agent.key.as_deref().unwrap_or("main");
                setting_item(id, agent.label.as_str(), current == agent.key.as_deref())
            })
            .collect();
        self.widget.show_selection_overlay(SETTINGS_DELEGATE_SURFACE, "选择委托 Agent", items);
    }

    fn open_access_picker(&mut self) {
        let full = self.state.session_options.full_access_enabled();
        self.widget.show_selection_overlay(
            SETTINGS_ACCESS_SURFACE,
            "选择访问范围",
            vec![
                setting_item("standard", "工作区写入", !full),
                setting_item("full", "完整访问（危险）", full),
            ],
        );
    }

    fn open_subagent_picker(&mut self) {
        let enabled = self.state.session_options.subagent_mode_enabled();
        self.widget.show_selection_overlay(
            SETTINGS_SUBAGENT_SURFACE,
            "选择子智能体模式",
            vec![
                setting_item("disabled", "关闭", !enabled),
                setting_item("enabled", "开启", enabled),
            ],
        );
    }

    pub(super) fn apply_model(&mut self, model: String) {
        let mut values = self.settings_values();
        values.model = Some(model);
        if !self
            .state
            .session_capabilities
            .efforts_for_model(values.model.as_deref())
            .contains(&values.effort)
        {
            values.effort = ReasoningEffort::Auto;
        }
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_effort(&mut self, effort: ReasoningEffort) {
        let mut values = self.settings_values();
        values.effort = effort;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_permission_mode(&mut self, mode: PermissionMode) {
        let mut values = self.settings_values();
        values.permission_mode = mode;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_acp_backend(&mut self, backend: Option<String>) {
        let mut values = self.settings_values();
        values.acp_backend = backend;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_delegate_agent(&mut self, agent: Option<String>) {
        let mut values = self.settings_values();
        values.delegate_agent = agent;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_full_access(&mut self, enabled: bool) {
        let mut values = self.settings_values();
        values.full_access_enabled = enabled;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_subagent_mode(&mut self, enabled: bool) {
        let mut values = self.settings_values();
        values.subagent_mode_enabled = enabled;
        self.apply_settings_or_report(values);
    }

    fn apply_settings_or_report(&mut self, values: SettingsValues) {
        if let Err(error) = self.apply_settings_values(values, false) {
            push_runner_error(self, error);
        }
    }

    pub(super) fn apply_settings_values(
        &mut self,
        values: SettingsValues,
        dangerous_confirmed: bool,
    ) -> Result<(), String> {
        self.validate_settings_values(&values)?;
        let enables_full_access =
            values.full_access_enabled && !self.state.session_options.full_access_enabled();
        let enables_bypass = values.permission_mode == PermissionMode::Bypass
            && !(self.state.session_options.permission_mode == PermissionMode::Bypass
                && self.state.session_options.bypass_confirmed);
        if !dangerous_confirmed && (enables_full_access || enables_bypass) {
            let mut risks = Vec::new();
            if enables_full_access {
                risks.push("完整文件系统与命令访问");
            }
            if enables_bypass {
                risks.push("绕过常规权限模式");
            }
            self.push_session_confirmation(UiConfirmOverlay {
                request_id: "settings-confirm:apply".to_string(),
                title: "确认危险会话设置".to_string(),
                body: format!("将为后续请求启用：{}。当前请求不会被修改。", risks.join("、")),
                confirm_label: "确认启用".to_string(),
                cancel_label: "保持安全设置".to_string(),
                destructive: true,
                intent: UiConfirmIntent::ApplySessionSettings { values: Box::new(values) },
            });
            return Ok(());
        }

        self.state.session_options.model.clone_from(&values.model);
        self.state.status.model_name.clone_from(&values.model);
        self.state.session_options.effort = values.effort;
        self.state.session_options.delegate_agent.clone_from(&values.delegate_agent);
        self.state.session_options.select_acp_backend(values.acp_backend.clone());
        self.state.session_options.set_full_access(values.full_access_enabled);
        self.state.session_options.set_subagent_mode(values.subagent_mode_enabled);
        self.state.session_options.set_permission_mode(values.permission_mode, dangerous_confirmed);
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        self.widget.set_session_header(display);
        self.widget.push_history(V3DisplayEvent::System {
            text: "会话设置已更新，将从下一条请求开始生效。".to_string(),
        });
        Ok(())
    }

    fn validate_settings_values(&self, values: &SettingsValues) -> Result<(), String> {
        let capabilities = &self.state.session_capabilities;
        if values.model != self.state.session_options.model
            && let Some(model) = values.model.as_ref()
            && !capabilities.models.iter().any(|candidate| candidate == model)
        {
            return Err(format!("Gateway 未声明模型能力：{model}"));
        }
        if values.effort != self.state.session_options.effort
            && values.effort != ReasoningEffort::Auto
            && !capabilities.efforts_for_model(values.model.as_deref()).contains(&values.effort)
        {
            return Err(format!("Gateway 未声明推理强度：{}", effort_id(values.effort)));
        }
        if values.permission_mode != self.state.session_options.permission_mode
            && !capabilities.permission_modes.contains(&values.permission_mode)
        {
            return Err(format!(
                "Gateway 未声明权限模式：{}",
                permission_mode_id(values.permission_mode)
            ));
        }
        if values.full_access_enabled
            && !self.state.session_options.explicit_full_access_enabled()
            && !capabilities.permission_modes.contains(&PermissionMode::Bypass)
        {
            return Err("Gateway 安全配置未声明完整访问能力".to_string());
        }
        if values.acp_backend != self.state.session_options.acp_backend
            && !capabilities
                .acp_backends
                .iter()
                .any(|candidate| candidate.key == values.acp_backend)
        {
            return Err("Gateway 未声明所选 ACP 后端".to_string());
        }
        if values.delegate_agent != self.state.session_options.delegate_agent
            && !capabilities
                .delegate_agents
                .iter()
                .any(|candidate| candidate.key == values.delegate_agent)
        {
            return Err("Gateway 配置未声明所选委托 Agent".to_string());
        }
        Ok(())
    }
}
