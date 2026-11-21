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

use cli_gateway_transport::gateway::block_on_gateway;
use gateway_session::GatewaySessionClient;
use vw_api_acp_model::acp_model_source::{
    AcpModelSource, AcpModelSourceCapability, AcpModelSourceUpdate, AcpSessionModelSelection,
};
use vw_api_acp_model::acp_session::AcpSessionOptions;

const CLI_LOCAL_MODEL: &str = "acp-cli-local";

fn supports_source(options: &AcpSessionOptions, source: AcpModelSource) -> bool {
    options.model_source_capabilities.iter().any(|capability| {
        matches!(capability, AcpModelSourceCapability::Supported { source: supported } if *supported == source)
    })
}

impl V3GatewayApp {
    fn model_and_agent_locked(&self) -> bool {
        self.state.session.session_id.is_some()
            && self.state.session.fork_execution_started != Some(false)
            && (!self.pending_first_response
                || matches!(
                    self.state.status.turn_terminal,
                    cli_interaction_model::UiTurnTerminal::Done { .. }
                ))
    }

    fn acp_model_capabilities(&self, backend: &str) -> Result<AcpSessionOptions, String> {
        let client = GatewaySessionClient::new(self.runtime.client().transport().clone());
        block_on_gateway(client.acp_model_source_capabilities(backend))
    }

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
            vim_mode: if self.widget.is_vim_enabled() {
                cli_interaction_model::vim::VimMode::Normal
            } else {
                cli_interaction_model::vim::VimMode::Disabled
            },
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
                    SettingKey::Vim => "vim",
                    SettingKey::Effort => "effort",
                    SettingKey::Permissions => "permissions",
                    _ => return None,
                };
                let current = if row.key == SettingKey::Model
                    && self.state.session_options.acp_model_source == AcpModelSource::CliLocal
                {
                    "ACP 智能体模型".to_string()
                } else {
                    panel.value_label(row.key)
                };
                let unavailable_reason = match row.key {
                    SettingKey::Model | SettingKey::DelegateAgent | SettingKey::AcpBackend
                        if self.model_and_agent_locked() =>
                    {
                        Some("会话开始后不能切换".to_string())
                    }
                    SettingKey::Model => None,
                    SettingKey::AcpBackend
                        if self.state.session_options.acp_backend.is_some()
                            || !self.state.session_capabilities.acp_backend_by_agent.is_empty() =>
                    {
                        None
                    }
                    SettingKey::Effort if self.state.session_options.acp_backend.is_some() => {
                        Some("推理强度由 ACP 智能体管理".to_string())
                    }
                    _ => row.unavailable_reason.clone(),
                };
                Some(V3SelectionItem {
                    id: id.to_string(),
                    label: row.label.to_string(),
                    description: Some(
                        unavailable_reason.clone().unwrap_or_else(|| format!("当前：{current}")),
                    ),
                    disabled: unavailable_reason.is_some(),
                    disabled_reason: unavailable_reason,
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
            SettingsTarget::Vim => {
                let enabled = self.widget.is_vim_enabled();
                self.widget.show_selection_overlay(
                    "session_settings_vim",
                    "Vim 编辑（仅当前终端）",
                    vec![setting_item("disabled", "关闭", !enabled),
                         setting_item("enabled", "开启", enabled)],
                );
            }
        }
    }

    pub(super) fn open_model_picker(&mut self) {
        if self.model_and_agent_locked() {
            push_runner_error(self, "会话开始后不能切换模型，请使用 /new");
            return;
        }
        let session = &self.state.session_options;
        let capabilities = match session.acp_backend.as_deref() {
            Some(backend) => match self.acp_model_capabilities(backend) {
                Ok(options) => Some(options),
                Err(error) => {
                    push_runner_error(self, error);
                    return;
                }
            },
            None => None,
        };
        let local = session.acp_model_source == AcpModelSource::CliLocal;
        let mut items = Vec::new();
        if capabilities
            .as_ref()
            .is_some_and(|options| supports_source(options, AcpModelSource::CliLocal))
        {
            items.push(setting_item(CLI_LOCAL_MODEL, "使用 ACP 智能体模型", local));
        }
        if capabilities
            .as_ref()
            .is_none_or(|options| supports_source(options, AcpModelSource::SessionModel))
        {
            let current = session.model.as_deref();
            items.push(setting_item(
                "auto",
                "自动选择模型",
                !local && current.is_none_or(|model| model == "auto"),
            ));
            items.extend(self.state.session_capabilities.models.iter().map(|model| {
                setting_item(model, model, !local && current == Some(model.as_str()))
            }));
        }
        if items.is_empty() {
            push_runner_error(self, "当前智能体没有已确认可用的模型来源");
            return;
        }
        self.widget.show_selection_overlay(SETTINGS_MODEL_SURFACE, "选择模型", items);
    }

    pub(super) fn open_effort_picker(&mut self) {
        if self.state.session_options.acp_backend.is_some() {
            push_runner_error(self, "推理强度由 ACP 智能体管理");
            return;
        }
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
        if self.model_and_agent_locked() {
            push_runner_error(self, "会话开始后不能切换智能体，请使用 /new");
            return;
        }
        let current = self.state.session_options.acp_backend.as_deref();
        let capabilities = &self.state.session_capabilities;
        let delegate = self.state.session_options.delegate_agent.as_deref();
        let mut items: Vec<_> = capabilities
            .acp_backends
            .iter()
            .map(|backend| {
                let id = backend.key.as_deref().unwrap_or("default");
                let selected = current == backend.key.as_deref()
                    && delegate.is_none_or(|key| key.strip_prefix("acp-") == current);
                setting_item(id, backend.label.as_str(), selected)
            })
            .collect();
        items.extend(capabilities.delegate_agents.iter().filter_map(|agent| {
            let key = agent.key.as_deref()?;
            let backend = capabilities.acp_backend_by_agent.get(key)?;
            if key.strip_prefix("acp-") == Some(backend.as_str())
                && capabilities.acp_backends.iter().any(|entry| entry.key.as_ref() == Some(backend))
            {
                return None;
            }
            Some(setting_item(&format!("profile:{key}"), &agent.label, delegate == Some(key)))
        }));
        self.widget.show_selection_overlay(SETTINGS_ACP_SURFACE, "选择 ACP 智能体", items);
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
        if self.model_and_agent_locked() {
            push_runner_error(self, "会话开始后不能切换模型，请使用 /new");
            return;
        }
        let mut values = self.settings_values();
        let source = if model == CLI_LOCAL_MODEL {
            AcpModelSource::CliLocal
        } else {
            values.model = Some(model);
            AcpModelSource::SessionModel
        };
        if let Err(error) = self
            .validate_settings_values(&values)
            .and_then(|()| self.confirm_model_source(&values, source))
        {
            push_runner_error(self, error);
            return;
        }
        if !self
            .state
            .session_capabilities
            .efforts_for_model(values.model.as_deref())
            .contains(&values.effort)
        {
            values.effort = ReasoningEffort::Auto;
        }
        self.state.session_options.model = values.model;
        self.state.status.model_name.clone_from(&self.state.session_options.model);
        self.state.session_options.effort = values.effort;
        self.state.session_options.acp_model_source = source;
        if self.state.session_options.acp_backend.is_some() {
            self.state.session_options.acp_session_mode =
                cli_interaction_model::session_mode::AcpSessionMode::ForceNew;
        }
        self.show_settings_updated();
    }

    fn confirm_model_source(
        &self,
        values: &SettingsValues,
        source: AcpModelSource,
    ) -> Result<(), String> {
        let Some(backend) = values.acp_backend.as_deref() else {
            return if source == AcpModelSource::SessionModel {
                Ok(())
            } else {
                Err("请先选择 ACP 智能体".to_string())
            };
        };
        if self.state.session.fork_execution_started == Some(false)
            && let Some(session_id) = self.state.session.session_id.as_deref()
        {
            let update = AcpModelSourceUpdate {
                agent: backend.to_owned(),
                selection: AcpSessionModelSelection {
                    source,
                    provider_id: None,
                    model: if source == AcpModelSource::SessionModel {
                        values.model.clone()
                    } else {
                        None
                    },
                },
            };
            let client = GatewaySessionClient::new(self.runtime.client().transport().clone());
            let directory = self.runtime.directory_value();
            let options = block_on_gateway(client.acp_session_model_source(
                session_id,
                directory.as_deref(),
                &update,
            ))?;
            if options.acp_model_source != source {
                return Err("Gateway 未确认所选模型来源".to_string());
            }
        } else if !supports_source(&self.acp_model_capabilities(backend)?, source) {
            return Err("此智能体不支持所选模型来源".to_string());
        }
        Ok(())
    }

    pub(super) fn apply_effort(&mut self, effort: ReasoningEffort) {
        if self.state.session_options.acp_backend.is_some() {
            push_runner_error(self, "推理强度由 ACP 智能体管理");
            return;
        }
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
        if values.acp_backend == backend
            && values.delegate_agent.as_deref()
                .is_none_or(|key| key.strip_prefix("acp-") == backend.as_deref())
        {
            return;
        }
        values.acp_backend = backend;
        values.delegate_agent = None;
        self.apply_settings_or_report(values);
    }

    pub(super) fn apply_delegate_agent(&mut self, agent: Option<String>) {
        let mut values = self.settings_values();
        values.acp_backend = agent
            .as_ref()
            .and_then(|key| self.state.session_capabilities.acp_backend_by_agent.get(key).cloned());
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
        let agent_changed = values.acp_backend != self.state.session_options.acp_backend
            || values.delegate_agent != self.state.session_options.delegate_agent;
        if self.model_and_agent_locked()
            && (agent_changed || values.model != self.state.session_options.model)
        {
            return Err("会话开始后不能切换模型或智能体，请使用 /new".to_string());
        }
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

        let source = match (agent_changed, values.acp_backend.as_deref()) {
            (true, Some(backend)) => {
                let options = self.acp_model_capabilities(backend)?;
                if !supports_source(&options, options.acp_model_source) {
                    return Err("此智能体没有已确认可用的模型来源".to_string());
                }
                options.acp_model_source
            }
            (true, None) => AcpModelSource::SessionModel,
            (false, _) => self.state.session_options.acp_model_source,
        };
        self.widget.set_vim_enabled(values.vim_mode != cli_interaction_model::vim::VimMode::Disabled);
        self.state.session_options.model.clone_from(&values.model);
        self.state.status.model_name.clone_from(&values.model);
        self.state.session_options.effort = values.effort;
        self.state.session_options.delegate_agent.clone_from(&values.delegate_agent);
        self.state.session_options.select_acp_backend(values.acp_backend.clone());
        self.state.session_options.acp_model_source = source;
        if agent_changed && values.acp_backend.is_some() {
            self.state.session_options.acp_session_mode =
                cli_interaction_model::session_mode::AcpSessionMode::ForceNew;
            self.state.session_options.effort = ReasoningEffort::Auto;
        }
        self.state.session_options.set_full_access(values.full_access_enabled);
        self.state.session_options.set_subagent_mode(values.subagent_mode_enabled);
        self.state.session_options.set_permission_mode(values.permission_mode, dangerous_confirmed);
        self.show_settings_updated();
        Ok(())
    }

    fn show_settings_updated(&mut self) {
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        self.widget.set_session_header(display);
        self.widget.push_history(V3DisplayEvent::System {
            text: "会话设置已更新，将从下一条请求开始生效。".to_string(),
        });
    }

    fn validate_settings_values(&self, values: &SettingsValues) -> Result<(), String> {
        let capabilities = &self.state.session_capabilities;
        if values.model != self.state.session_options.model
            && let Some(model) = values.model.as_ref()
            && model != "auto"
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
            && !values.delegate_agent.as_ref().is_some_and(|key| {
                capabilities.acp_backend_by_agent.get(key) == values.acp_backend.as_ref()
            })
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
