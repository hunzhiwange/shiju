use super::AcpLaunchProfile;
#[cfg(unix)]
use std::collections::HashMap;

/// siGit Code's explicit environment overrides are read without changing CLI defaults.
#[cfg(unix)]
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["--acp"],
        "siGit Code custom launch arguments are not verified"
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
        "siGit Code requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "siGit Code requires a selected tool-capable model"
    );
    let home = {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let root = profile.state_root.as_deref().unwrap_or(profile.directory());
        anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid siGit Code state root");
        let reference = super::super::profile::launch_reference(&profile.binding.identity)?;
        let reference = reference.strip_prefix("vw-session-model:").unwrap();
        let mut home = root.to_path_buf();
        for component in ["acp-sigit-state", reference] {
            home.push(component);
            match std::fs::DirBuilder::new().mode(0o700).create(&home) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
            let metadata = home.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && metadata.permissions().mode() & 0o777 == 0o700
                    && metadata.uid() == unsafe { libc::geteuid() },
                "insecure siGit Code state directory"
            );
        }
        home
    };
    let mut env = HashMap::from([
        ("SIGIT_MODEL".into(), profile.model_alias.clone()),
        ("OPENAI_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("OPENAI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("SIGIT_CONFIG_DIR".into(), home.to_string_lossy().into_owned()),
        ("SIGIT_MCP".into(), "off".into()),
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

#[cfg(not(unix))]
pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("siGit Code state ACL is not verified on this platform")
}

#[cfg(all(test, unix))]
#[path = "sigit_tests.rs"]
mod tests;
