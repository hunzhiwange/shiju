use super::AcpLaunchProfile;
#[cfg(any(unix, windows))]
use std::collections::HashMap;

/// Match the managed GLM HTTP catalog and the host approval title.
pub fn projected_mcp_tool_name(server: &str, tool: &str) -> anyhow::Result<String> {
    let valid = |name: &str| {
        name.bytes().any(|byte| byte.is_ascii_alphanumeric())
            && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    };
    anyhow::ensure!(
        server.len().saturating_add(tool.len()) <= 57 && valid(server) && valid(tool),
        "invalid managed GLM MCP tool identity"
    );
    Ok(format!("mcp__{server}__{tool}"))
}

/// GLM Agent's explicit environment overrides are read without changing CLI defaults.
#[cfg(any(unix, windows))]
pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.is_empty() || profile.config().args == ["-y", "glm-acp-agent@1.17.1"],
        "GLM Agent custom launch arguments are not verified"
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
        "GLM Agent requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "GLM Agent requires a selected tool-capable model"
    );
    let home = {
        let root = profile.state_root.as_deref().unwrap_or(profile.directory());
        anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid GLM Agent state root");
        let reference = super::super::profile::launch_reference(profile.binding.identity())?;
        let reference = reference.strip_prefix("vw-session-model:").unwrap();
        let mut home = root.to_path_buf();
        for component in ["acp-glm-state", reference] {
            home.push(component);
            ensure_private_directory(&home)?;
        }
        // GLM 1.17.1 resolves credentials under XDG_CONFIG_HOME/glm-acp-agent.
        #[cfg(windows)]
        ensure_private_directory(&home.join("glm-acp-agent"))?;
        home
    };
    let home = home
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("GLM Agent state path is not valid Unicode"))?;
    let mut env = HashMap::from([
        ("ACP_GLM_AVAILABLE_MODELS".into(), profile.model_alias.clone()),
        ("ACP_GLM_MODEL".into(), profile.model_alias.clone()),
        ("ACP_GLM_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("Z_AI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("XDG_CONFIG_HOME".into(), home.clone()),
        ("ACP_GLM_SESSION_DIR".into(), home),
        ("NODE_OPTIONS".into(), String::new()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(env)?;
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("GLM Agent state ACL is not verified on this platform")
}

#[cfg(unix)]
fn ensure_private_directory(path: &std::path::Path) -> anyhow::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    match std::fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let metadata = path.symlink_metadata()?;
    anyhow::ensure!(
        metadata.is_dir()
            && !metadata.file_type().is_symlink()
            && metadata.permissions().mode() & 0o777 == 0o700
            && metadata.uid() == unsafe { libc::geteuid() },
        "insecure GLM Agent state directory"
    );
    Ok(())
}

#[cfg(windows)]
fn ensure_private_directory(path: &std::path::Path) -> anyhow::Result<()> {
    super::super::windows_private_dir::create_or_verify(path)
}

#[cfg(all(test, any(unix, windows)))]
#[path = "glm_acp_agent_tests.rs"]
mod tests;
