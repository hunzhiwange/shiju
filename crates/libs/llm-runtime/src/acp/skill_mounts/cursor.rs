//! Cursor's reviewed Agent Store scanner accepts one request-owned skill root.
use agent_config::schema::AcpAgentConfig;

pub(super) const ROOT_ENV: &str = "SHIJU_ACP_CURSOR_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error(
            "conflicting Cursor skill directory environment",
        ));
    }
    if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        let root = view.0.root().to_str().ok_or_else(|| {
            super::super::acp_option_error("Cursor skill directory requires a UTF-8 path")
        })?;
        config.env.insert(ROOT_ENV.into(), root.into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "cursor_tests.rs"]
mod tests;
