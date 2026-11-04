use super::AcpLaunchProfile;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    // Only the reviewed v1.57.0 patch accepts this launch flag. An unpatched
    // binary exits before ACP instead of silently ignoring the model lock.
    anyhow::ensure!(
        profile.config().args.as_slice() == ["acp"],
        "Goose managed model requires the pinned acp argument list"
    );
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Goose model relay endpoint"))?;
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
        "Goose requires an owned loopback model relay"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "Goose requires a selected model");
    let directory = profile
        .directory()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Goose config root is not UTF-8"))?
        .to_owned();
    profile.overlay_environment(std::collections::HashMap::from([
        ("GOOSE_PATH_ROOT".into(), directory),
        ("GOOSE_PROVIDER".into(), "openai".into()),
        ("GOOSE_MODEL".into(), profile.model_alias.clone()),
        ("SHIJU_GOOSE_MODEL_LOCK".into(), profile.model_alias.clone()),
        ("GOOSE_DISABLE_KEYRING".into(), "true".into()),
        ("GOOSE_ADDITIONAL_CONFIG_FILES".into(), String::new()),
        ("OPENAI_HOST".into(), endpoint.trim_end_matches('/').into()),
        ("OPENAI_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("OPENAI_BASE_PATH".into(), "v1/chat/completions".into()),
        ("OPENAI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("OPENAI_CUSTOM_HEADERS".into(), String::new()),
        ("OPENAI_ORGANIZATION".into(), String::new()),
        ("OPENAI_PROJECT".into(), String::new()),
    ]))?;
    profile.append_arguments(&["--shiju-model-source".into()])?;
    Ok(())
}

#[cfg(test)]
#[path = "goose_tests.rs"]
mod tests;
