//! Project skills and the host session view share the reviewed Pi RPC catalog.
use super::super::model_source::profile::AcpLaunchProfile;
use acp_types::PermissionMode;
use agent_config::schema::AcpAgentConfig;
use vw_api_acp_model::acp_model_source::AcpModelSource;

pub(in crate::acp) fn restricted_launch(
    agent: &str,
    source: AcpModelSource,
    permission: Option<PermissionMode>,
    tools_disabled: bool,
) -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && agent == "pi"
        && source == AcpModelSource::SessionModel
        && permission == Some(PermissionMode::ApproveReads)
        && !tools_disabled
}

pub(in crate::acp) fn configure_restricted(profile: &mut AcpLaunchProfile) -> anyhow::Result<()> {
    let adapter = std::path::Path::new(&profile.config().command).canonicalize()?;
    let modules = adapter
        .ancestors()
        .nth(3)
        .ok_or_else(|| anyhow::anyhow!("Invalid restricted Pi adapter path"))?;
    let engine = modules.join("@mariozechner/pi-coding-agent/dist/cli.js");
    anyhow::ensure!(
        engine.canonicalize().ok().as_ref() == Some(&engine)
            && profile
                .config()
                .env
                .get("PI_ACP_PI_COMMAND")
                .is_some_and(|path| { std::path::Path::new(path) == engine.as_path() }),
        "Restricted Pi requires the co-installed native engine"
    );
    profile.overlay_environment(std::collections::HashMap::from([(
        "VW_PI_RESTRICTED".into(),
        "1".into(),
    )]))
}

pub(super) const ROOT_ENV: &str = "SHIJU_PI_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) || std::env::var_os(ROOT_ENV).is_some() {
        return Err(super::super::acp_option_error("conflicting Pi skill directory environment"));
    }
    let root = if let Some(view) = view {
        if cfg!(target_os = "linux") {
            return Err(super::super::acp_option_error(
                "Pi skill mounts require a reviewed Linux sandbox",
            ));
        }
        view.0.validate().map_err(super::super::to_api_error)?;
        acp_agent_registry::verify_reviewed_agent_executable("pi", &config.command)
            .map_err(super::super::to_api_error)?;
        view.0.root().to_string_lossy().into_owned()
    } else {
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}
#[cfg(test)]
#[path = "pi_tests.rs"]
mod tests;
