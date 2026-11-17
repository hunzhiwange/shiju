//! Autohand 0.9.11 reads an explicit private OpenAI configuration in ACP mode.
use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["--acp"]
            || profile.config().args == ["-y", "autohand-cli@0.9.11", "--acp"],
        "Autohand custom launch arguments are not verified"
    );
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
        "Autohand requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call
            && profile.model_supports_temperature
            && !profile.model_alias.is_empty(),
        "Autohand Chat requires a selected tool-capable model supporting temperature"
    );
    let directory = profile.directory().to_string_lossy().into_owned();
    let mut env = HashMap::from([
        ("AUTOHAND_HOME".into(), directory.clone()),
        ("AUTOHAND_CONFIG".into(), format!("{directory}/config.json")),
        ("AUTOHAND_PROVIDER".into(), "openai".into()),
        ("AUTOHAND_MODEL".into(), profile.model_alias.clone()),
        ("NODE_OPTIONS".into(), String::new()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(env)?;
    profile.write_config(
        "config.json",
        &serde_json::to_vec(&serde_json::json!({
            "provider": "openai",
            "openai": {
                "authMode": "api-key",
                "apiKey": profile.binding.bearer_token(),
                "baseUrl": format!("{}/v1", endpoint.trim_end_matches('/')),
                "model": profile.model_alias
            },
            "telemetry": {"enabled": false}
        }))?,
    )?;
    Ok(())
}
