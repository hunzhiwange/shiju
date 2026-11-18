//! Configure Corust's native OneRouter client with the request-owned Chat relay.
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
        config.args.is_empty() && config.env.keys().all(|key| key == REFERENCE_ENV),
        "Corust requires reviewed ACP arguments and environment"
    );
    acp_agent_registry::verify_reviewed_agent_executable("corust-agent", &config.command)
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
        "Corust requires a private loopback relay"
    );
    let workspace = Path::new(directory).canonicalize()?;
    anyhow::ensure!(workspace.is_dir() && workspace.parent().is_some(), "Invalid Corust workspace");
    let model = profile.model_alias.clone();
    anyhow::ensure!(
        profile.model_supports_tool_call && !model.is_empty(),
        "Corust requires a selected tool-capable model"
    );

    let runtime = profile.directory();
    let config_dir = runtime.join("corust/config");
    std::fs::create_dir_all(&config_dir)?;
    // The native inference client appends an absolute path without joining it.
    let gateway_url = endpoint.as_str().trim_end_matches('/');
    let base = json!({
        "version": "0.6.0",
        "llm": {"model": model, "max_tokens": profile.model_output_limit},
        "gateway": {"url": gateway_url, "timeout_secs": 300},
        "tools": {"max_file_size": 1048576},
        "log": {"log_dir": runtime.join("logs"), "log_level": "off", "display_in_frontend": false},
        "rag_config": {"url": endpoint.as_str()},
        "evaluation": {},
        "scenario": "general",
        "prompts": {},
        "code_quality_mode": "restrict",
    });
    let models = json!({"workspace_models": {workspace.to_str().ok_or_else(|| anyhow::anyhow!("Invalid Corust workspace path"))?: model}});
    for (name, config) in [("base.yaml", base), ("workspace-settings.yaml", models)] {
        let path = profile.write_config(name, &serde_json::to_vec(&config)?)?;
        std::fs::rename(path, config_dir.join(name))?;
    }
    let launch = profile.write_config("corust-launch.json", &serde_json::to_vec(&json!({
        "workspace": workspace, "endpoint": endpoint.as_str(), "token": profile.binding.bearer_token(),
    }))?)?;
    profile.overlay_environment(HashMap::from([(
        "SHIJU_CORUST_LAUNCH".into(),
        launch.to_string_lossy().into_owned(),
    )]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
