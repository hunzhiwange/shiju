//! Goose's reviewed native scanner reads only the request-owned skill view.
use crate::config::schema::AcpAgentConfig;

const ROOT_ENV: &str = "SHIJU_GOOSE_SKILL_ROOT";

pub(super) fn configure(
    config: &mut AcpAgentConfig,
    view: Option<&super::MountedSkills>,
) -> Result<(), super::Error> {
    if config.env.contains_key(ROOT_ENV) {
        return Err(super::super::acp_option_error(
            "conflicting Goose skill directory environment",
        ));
    }
    let root = if let Some(view) = view {
        view.0.validate().map_err(super::super::to_api_error)?;
        vw_acp::verify_reviewed_agent_executable("goose", &config.command)
            .map_err(super::super::to_api_error)?;
        // Older model-source-only Goose builds do not implement scanner isolation.
        let receipt = std::path::Path::new(&config.command)
            .canonicalize()
            .map_err(super::super::to_api_error)?
            .parent()
            .unwrap()
            .join("build-receipt.json");
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipt).map_err(super::super::to_api_error)?)
                .map_err(super::super::to_api_error)?;
        if receipt["patch_sha256"]
            != "3a47bbbdcf861d680ff051879d81d23ef872d035a01a0549f9cedecd65b54e06"
        {
            return Err(super::super::acp_option_error(
                "Goose skill scanner requires the reviewed directory patch",
            ));
        }
        view.0.root().to_string_lossy().into_owned()
    } else {
        // An empty selection disables all native skill levels in the reviewed scanner.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "goose_tests.rs"]
mod tests;
