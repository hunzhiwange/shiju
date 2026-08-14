use super::super::model::{PermissionMode, SessionCapabilityMatrix, SessionOptions};

pub(crate) trait SessionOptionsRuntime {
    fn save_session_options(&mut self, options: &SessionOptions) -> Result<(), String>;
}

pub(crate) fn commit_session_options(
    current: &mut SessionOptions,
    requested: SessionOptions,
    capabilities: &SessionCapabilityMatrix,
    confirm_bypass: bool,
    runtime: &mut dyn SessionOptionsRuntime,
) -> Result<(), String> {
    validate_options(&requested, capabilities)?;
    if requested.permission_mode == PermissionMode::Bypass
        && !current.bypass_confirmed
        && !confirm_bypass
    {
        return Err(
            "bypass mode requires explicit confirmation; filesystem and command safety checks may be skipped"
                .to_string(),
        );
    }
    runtime.save_session_options(&requested)?;
    *current = requested;
    if current.permission_mode == PermissionMode::Bypass {
        current.bypass_confirmed = true;
    }
    Ok(())
}

fn validate_options(
    options: &SessionOptions,
    capabilities: &SessionCapabilityMatrix,
) -> Result<(), String> {
    if options.model.as_ref().is_some_and(|model| !capabilities.models.contains(model)) {
        return Err("selected model is not supported by the active provider".to_string());
    }
    if !capabilities.efforts.contains(&options.effort) {
        return Err("selected reasoning effort is not supported".to_string());
    }
    if options.thinking && !capabilities.supports_thinking {
        return Err("thinking mode is not supported by the active VibeWindow backend".to_string());
    }
    if options.fast && !capabilities.supports_fast {
        return Err("fast mode is not supported by the active VibeWindow backend".to_string());
    }
    if !capabilities.permission_modes.contains(&options.permission_mode) {
        return Err("selected permission mode is not supported".to_string());
    }
    Ok(())
}
