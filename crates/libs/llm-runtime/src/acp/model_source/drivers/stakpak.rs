//! Bind the reviewed native Stakpak catalog to the request-owned Chat relay.
use super::{super::profile::REFERENCE_ENV, AcpLaunchProfile};
use serde_json::json;
use std::{collections::HashMap, path::Path};

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"]
            && profile.config().env.keys().all(|key| key == REFERENCE_ENV),
        "Stakpak requires reviewed ACP arguments and environment"
    );
    acp_agent_registry::verify_reviewed_agent_executable("stakpak", &profile.config().command)
        .map_err(anyhow::Error::msg)?;
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Stakpak requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "Stakpak requires a selected tool-capable model"
    );
    for name in [".stakpak/mcp.toml", ".stakpak/mcp.json"] {
        match Path::new(directory).join(name).symlink_metadata() {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => anyhow::bail!("Stakpak project MCP overrides are not verified"),
        }
    }

    let config = format!(
        "[profiles.default]\nprovider = \"local\"\nallowed_tools = [\"stakpak__view\", \"stakpak__create\", \"stakpak__str_replace\"]\n\
         [profiles.default.providers.shiju-session]\ntype = \"custom\"\napi_endpoint = {}\napi_key = {}\n\
         [settings]\ncollect_telemetry = false\nauto_append_gitignore = false\n",
        serde_json::to_string(url.join("v1")?.as_str())?,
        serde_json::to_string(profile.binding.bearer_token())?,
    );
    // The pinned compatible provider reads the OpenAI catalog from a local cache.
    // Its sole entry selects our private transport and the exact relay model ID.
    let model = &profile.model_alias;
    let catalog = json!({
        "fetched_at": 0,
        "providers": {"openai": {"id": "openai", "name": "OpenAI compatible",
            "models": {model: {
                "id": model, "name": model, "provider": "shiju-session",
                "reasoning": profile.model_supports_reasoning,
                "limit": {"context": profile.model_context_limit, "output": profile.model_output_limit},
            }}
        }}
    });
    let root = profile.directory();
    let stakpak = root.join(".stakpak");
    std::fs::create_dir_all(stakpak.join("cache"))?;
    std::fs::rename(
        profile.write_config("config.toml", config.as_bytes())?,
        stakpak.join("config.toml"),
    )?;
    std::fs::rename(
        profile.write_config("models.json", &serde_json::to_vec(&catalog)?)?,
        stakpak.join("cache/models.json"),
    )?;
    profile.overlay_environment(HashMap::from([(
        "SHIJU_STAKPAK_RUNTIME".into(),
        root.to_string_lossy().into_owned(),
    )]))?;
    profile.session_model_id = Some(profile.model_alias.clone());
    Ok(())
}
