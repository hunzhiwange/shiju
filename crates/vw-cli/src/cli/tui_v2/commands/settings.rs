use super::super::model::settings_panel::{SettingsPanel, SettingsValues};
use super::super::model::{PermissionMode, ReasoningEffort, SessionCapabilityMatrix};
use super::super::runtime::gateway::{GatewayUiRuntime, block_on_gateway};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SettingsCommand {
    Open,
    OpenModelPicker,
    OpenAcpBackendPicker,
    Effort(ReasoningEffort),
    Fast(bool),
    Theme(String),
    PermissionMode(PermissionMode),
    Permissions,
}

pub(crate) trait SettingsRuntime {
    fn save(&mut self, values: &SettingsValues) -> Result<(), String>;
}

impl SettingsRuntime for GatewayUiRuntime {
    fn save(&mut self, values: &SettingsValues) -> Result<(), String> {
        let Some(model) = values.model.as_deref() else {
            return Ok(());
        };
        let directory = self
            .directory()
            .to_str()
            .ok_or_else(|| "project path is not valid UTF-8".to_string())?;
        block_on_gateway(
            self.client().desktop_project_preferences_put(directory, model, false, None),
        )
    }
}

pub(crate) fn commit_settings(
    panel: &mut SettingsPanel,
    runtime: &mut dyn SettingsRuntime,
) -> Result<(), String> {
    if let Err(error) = runtime.save(&panel.pending) {
        panel.cancel();
        return Err(error);
    }
    panel.original = panel.pending.clone();
    Ok(())
}

pub(crate) fn apply_settings_command(
    panel: &mut SettingsPanel,
    command: &SettingsCommand,
    capabilities: &SessionCapabilityMatrix,
    confirm_bypass: bool,
    runtime: &mut dyn SettingsRuntime,
) -> Result<(), String> {
    panel.pending = panel.original.clone();
    match command {
        SettingsCommand::Open
        | SettingsCommand::OpenModelPicker
        | SettingsCommand::OpenAcpBackendPicker
        | SettingsCommand::Permissions => {
            return Err("this settings command opens a dedicated panel".to_string());
        }
        SettingsCommand::Effort(effort) => {
            if !capabilities.efforts.contains(effort) {
                return Err("selected reasoning effort is not supported".to_string());
            }
            panel.pending.effort = *effort;
        }
        SettingsCommand::Fast(fast) => {
            if *fast && !capabilities.supports_fast {
                return Err(
                    "fast mode is not supported by the active VibeWindow backend".to_string()
                );
            }
            panel.pending.fast = *fast;
        }
        SettingsCommand::Theme(theme) => panel.preview_theme(theme)?,
        SettingsCommand::PermissionMode(mode) => {
            if !capabilities.permission_modes.contains(mode) {
                return Err("selected permission mode is not supported".to_string());
            }
            if *mode == PermissionMode::Bypass
                && panel.original.permission_mode != PermissionMode::Bypass
                && !confirm_bypass
            {
                return Err(
                    "bypass mode requires explicit confirmation; filesystem and command safety checks may be skipped"
                        .to_string(),
                );
            }
            panel.pending.permission_mode = *mode;
        }
    }

    validate_settings_values(&panel.pending, &panel.original, capabilities, confirm_bypass)?;
    commit_settings(panel, runtime)
}

pub(crate) fn validate_settings_values(
    requested: &SettingsValues,
    current: &SettingsValues,
    capabilities: &SessionCapabilityMatrix,
    confirm_bypass: bool,
) -> Result<(), String> {
    if requested.model != current.model
        && requested.model.as_ref().is_some_and(|model| !capabilities.models.contains(model))
    {
        return Err("selected model is not supported by the active provider".to_string());
    }
    if requested.delegate_agent != current.delegate_agent
        && !capabilities
            .delegate_agents
            .iter()
            .any(|agent| agent.key == requested.delegate_agent)
    {
        return Err("selected delegate agent is not available in this session".to_string());
    }
    if requested.acp_backend != current.acp_backend
        && !capabilities
            .acp_backends
            .iter()
            .any(|backend| backend.key == requested.acp_backend)
    {
        return Err("selected ACP backend is not available in this session".to_string());
    }
    if requested.effort != current.effort && !capabilities.efforts.contains(&requested.effort) {
        return Err("selected reasoning effort is not supported".to_string());
    }
    if requested.fast != current.fast && requested.fast && !capabilities.supports_fast {
        return Err("fast mode is not supported by the active VibeWindow backend".to_string());
    }
    if requested.permission_mode != current.permission_mode
        && !capabilities.permission_modes.contains(&requested.permission_mode)
    {
        return Err("selected permission mode is not supported".to_string());
    }
    if requested.permission_mode == PermissionMode::Bypass
        && current.permission_mode != PermissionMode::Bypass
        && !confirm_bypass
    {
        return Err(
            "bypass mode requires explicit confirmation; filesystem and command safety checks may be skipped"
                .to_string(),
        );
    }
    Ok(())
}
