//! Bind the reviewed local Cortex composition to the request-owned Chat relay.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
use serde_json::json;
use std::{collections::HashMap, path::Path};

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    let config = profile.config();
    anyhow::ensure!(
        config.args == ["acp", "serve"] && config.env.keys().all(|key| key == REFERENCE_ENV),
        "Cortex Code requires reviewed ACP arguments and environment"
    );
    acp_agent_registry::verify_reviewed_agent_executable("cortex-code", &config.command)
        .map_err(anyhow::Error::msg)?;
    let endpoint = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        endpoint.scheme() == "http"
            && endpoint.host_str() == Some("127.0.0.1")
            && endpoint.port().is_some()
            && endpoint.path() == "/"
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none(),
        "Cortex Code requires a private loopback relay"
    );
    let workspace = Path::new(directory).canonicalize()?;
    let write_root = match std::env::var_os("SHIJU_CORTEX_WRITE_ROOT") {
        Some(path) => std::fs::canonicalize(path)?,
        None => workspace.clone(),
    };
    anyhow::ensure!(
        workspace.parent().is_some() && write_root.is_dir() && write_root.starts_with(&workspace),
        "Cortex Code write root must be within the workspace"
    );
    let model = profile.model_alias.clone();
    anyhow::ensure!(
        profile.model_supports_tool_call && !model.is_empty() && model != "auto",
        "Cortex Code requires a selected tool-capable model"
    );
    let launch = profile.write_config(
        "cortex-launch.json",
        &serde_json::to_vec(&json!({
            "workspace": workspace,
            "write_root": write_root,
            "endpoint": endpoint.join("v1")?.as_str(),
            "token": profile.binding.bearer_token(),
            "model": model,
        }))?,
    )?;
    profile.overlay_environment(HashMap::from([(
        "SHIJU_CORTEX_LAUNCH".into(),
        launch.to_string_lossy().into_owned(),
    )]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
