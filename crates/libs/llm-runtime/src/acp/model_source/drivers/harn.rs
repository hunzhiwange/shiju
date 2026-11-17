//! Bind the pinned Harn natural-language pipeline to the request-owned relay.
use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.is_empty(),
        "Harn custom launch arguments are not verified"
    );
    anyhow::ensure!(
        profile.config().env.keys().all(|key| key == super::super::profile::REFERENCE_ENV),
        "Harn custom environment is not verified"
    );
    acp_agent_registry::verify_reviewed_agent_executable("harn", &profile.config().command)
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
        "Harn requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "Harn requires a selected tool-capable model"
    );
    for ancestor in std::path::Path::new(directory).ancestors() {
        for name in ["harn.toml", ".env"] {
            match ancestor.join(name).symlink_metadata() {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => anyhow::bail!("Harn project provider/environment overrides are not verified"),
            }
        }
    }
    let provider = profile.binding.identity().provider.clone();
    anyhow::ensure!(
        !provider.is_empty() && !provider.contains(['/', ':']),
        "Invalid Harn provider selector"
    );
    let quote = |value: &str| serde_json::to_string(value);
    let configuration = format!(
        "default_provider = {}\n[providers.{}]\nbase_url = {}\nchat_endpoint = \"/chat/completions\"\nauth_style = \"bearer\"\nauth_env = \"SHIJU_HARN_RELAY_TOKEN\"\n",
        quote(&provider)?,
        quote(&provider)?,
        quote(&format!("{}/v1", endpoint.trim_end_matches('/')))?
    );
    let file = profile.write_config("providers.toml", configuration.as_bytes())?;
    let pipeline = profile.write_config(
        "agent.harn",
        include_bytes!("../../../../../../../patches/acp-model-source/harn/agent.harn"),
    )?;
    let state = profile.directory().join("state");
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&state)?;
    }
    #[cfg(not(unix))]
    anyhow::bail!("Harn private state ACL is not verified on this platform");
    let mut env = HashMap::from([
        ("SHIJU_HARN_PIPELINE".into(), pipeline.to_string_lossy().into_owned()),
        ("HARN_PROVIDERS_CONFIG".into(), file.to_string_lossy().into_owned()),
        ("SHIJU_HARN_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("NO_PROXY".into(), "127.0.0.1,localhost".into()),
        ("no_proxy".into(), "127.0.0.1,localhost".into()),
    ]);
    for key in [
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "HARN_SESSION_STORE_ROOT",
        "HARN_STATE_DIR",
    ] {
        env.insert(key.into(), state.to_string_lossy().into_owned());
    }
    profile.overlay_environment(env)?;
    profile.session_model_id = Some(format!("{provider}/{}", profile.model_alias));
    Ok(())
}
