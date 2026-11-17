//! DimCode 0.5.28 imports a request-private provider into its private SQLite store.
use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"]
            || profile.config().args == ["-y", "dimcode@0.5.28", "acp"],
        "DimCode custom arguments are not verified"
    );
    anyhow::ensure!(
        profile.config().env.keys().all(|key| key == super::super::profile::REFERENCE_ENV),
        "DimCode custom environment is not verified"
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
        "DimCode requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "DimCode requires a selected tool-capable model"
    );
    match std::path::Path::new(directory).join(".dimcode/providers.json").symlink_metadata() {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => anyhow::bail!("DimCode project provider overrides are not verified"),
    }
    // Preserve HOME. Only the supplier's application state is request-owned.
    let home = profile.directory().join("v2");
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new().mode(0o700).create(&home)?;
    }
    #[cfg(not(unix))]
    anyhow::bail!("DimCode private state ACL is not verified on this platform");
    let selected = json!({"providerId":"session-selected", "modelId":profile.model_alias});
    let config = json!({"defaultProvider":"session-selected", "defaultModel":profile.model_alias,
        "subagentModel":selected, "assistantModel":selected,
        "metadata":{"subagentDefaultMigratedV1":true}});
    let providers = json!({"providers":[{"providerId":"session-selected", "enabled":true,
        "apiKey":profile.binding.bearer_token(),
        "baseUrl":format!("{}/v1", endpoint.trim_end_matches('/')),
        "metadata":{"adapter":"openai-compatible", "activeModelId":profile.model_alias,
            "models":[{"id":profile.model_alias}]}}]});
    for (name, value) in [("config.json", config), ("providers.json", providers)] {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new_in(&home)?;
        file.write_all(&serde_json::to_vec(&value)?)?;
        file.as_file().sync_all()?;
        file.persist_noclobber(home.join(name))?;
    }
    profile.overlay_environment(HashMap::from([(
        "DIMCODE_HOME".into(),
        home.to_string_lossy().into_owned(),
    )]))?;
    // Apply the exact native model before prompt; never rely on workspace defaults.
    profile.session_model_id = Some(format!("session-selected/{}", profile.model_alias));
    Ok(())
}
