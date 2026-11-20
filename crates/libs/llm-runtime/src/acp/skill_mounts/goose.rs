//! Goose's reviewed native scanner appends the request-owned skill view.
use agent_config::schema::AcpAgentConfig;

const COEXISTENCE_PATCH: &str = "3b75b96dc038b60f294de02354de34150a204e5e08f51a5bd37ff9095898ebcb";
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
        acp_agent_registry::verify_reviewed_agent_executable("goose", &config.command)
            .map_err(super::super::to_api_error)?;
        // The registry also recognizes a legacy model-source artifact. Require
        // the coexistence receipt before passing any session data.
        let receipt = std::path::Path::new(&config.command)
            .canonicalize()
            .map_err(super::super::to_api_error)?
            .parent()
            .unwrap()
            .join("build-receipt.json");
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(receipt).map_err(super::super::to_api_error)?)
                .map_err(super::super::to_api_error)?;
        if receipt["patch_sha256"] != COEXISTENCE_PATCH {
            return Err(super::super::acp_option_error(
                "Goose skill scanner requires the reviewed coexistence patch",
            ));
        }
        view.0.root().to_string_lossy().into_owned()
    } else {
        // Mask inherited session roots while leaving native discovery on.
        String::new()
    };
    config.env.insert(ROOT_ENV.into(), root);
    Ok(())
}

#[cfg(test)]
#[path = "goose_tests.rs"]
mod tests;
