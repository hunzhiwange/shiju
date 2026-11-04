use super::AcpLaunchProfile;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.as_slice() == ["--experimental-acp"],
        "iFlow managed model requires the pinned ACP argument list"
    );
    anyhow::ensure!(profile.model_supports_temperature, "iFlow requires temperature support");
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid iFlow model relay endpoint"))?;
    let loopback = match url.host() {
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    };
    anyhow::ensure!(
        url.scheme() == "http"
            && loopback
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "iFlow requires an owned loopback model relay"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "iFlow requires a selected model");
    // Alias variants and alternate credentials must not override the canonical settings.
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let upper = key.to_ascii_uppercase();
            upper.starts_with("IFLOW_")
                || upper.starts_with("SHIJU_IFLOW_")
                || upper.starts_with("OPENAI_")
        }),
        "conflicting iFlow model environment"
    );
    let directory = profile
        .directory()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("iFlow config root is not UTF-8"))?
        .to_owned();
    // ACP history survives bearer rotation; route settings stay in the disposable root.
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&serde_json::json!({
        "tenant": identity.tenant, "project": identity.project, "session": identity.session,
        "source": "session_model", "driver": "iflow",
    }))?));
    let history_root = profile
        .directory()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("iFlow runtime has no parent"))?
        .join("shiju-acp-iflow-history");
    let history = history_root.join(reference);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let owner = profile.directory().symlink_metadata()?.uid();
        for path in [&history_root, &history] {
            match std::fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private iFlow history"),
            }
            let metadata = path.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == owner
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure iFlow history"
            );
        }
    }
    #[cfg(not(unix))]
    anyhow::bail!("iFlow history ACL verification unavailable on this platform");
    let history = history.to_str().ok_or_else(|| anyhow::anyhow!("iFlow history is not UTF-8"))?;
    profile.overlay_environment(std::collections::HashMap::from([
        ("IFLOW_HOME".into(), directory.clone()),
        ("SHIJU_IFLOW_HISTORY_HOME".into(), history.into()),
        ("IFLOW_CLI_SYSTEM_SETTINGS_PATH".into(), format!("{directory}/system.json")),
        ("IFLOW_apiKey".into(), profile.binding.bearer_token().into()),
        ("IFLOW_baseUrl".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("IFLOW_modelName".into(), profile.model_alias.clone()),
        ("IFLOW_selectedAuthType".into(), "openai-compatible".into()),
        ("IFLOW_multimodalModelName".into(), profile.model_alias.clone()),
        ("IFLOW_outputTokensLimit".into(), profile.model_output_limit.to_string()),
        ("IFLOW_tokensLimit".into(), profile.model_context_limit.to_string()),
        ("IFLOW_disableAutoUpdate".into(), "true".into()),
        ("IFLOW_usageStatisticsEnabled".into(), "false".into()),
        ("SHIJU_IFLOW_MODEL_LOCK".into(), profile.model_alias.clone()),
    ]))?;
    profile.append_arguments(&["--shiju-model-source".into()])?;
    Ok(())
}

#[cfg(test)]
#[path = "iflow_tests.rs"]
mod tests;
