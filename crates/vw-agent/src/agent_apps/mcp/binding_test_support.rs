//! Native installation fixtures used by host runtime integration tests.
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use vw_agent_apps::apps::discovery::read::{self, LocalReadEnvironment};
use vw_api_app_identity::{AppId, AppInstallation};
use vw_api_mcp_management::McpAppTarget;
pub(super) fn private_root() -> std::io::Result<tempfile::TempDir> {
    let root = tempfile::tempdir()?;
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    Ok(root)
}

pub(super) fn installed(
    home: &Path,
    app: AppId,
    executable: &str,
) -> anyhow::Result<(AppInstallation, McpAppTarget)> {
    let bin = home.join("bin");
    fs::create_dir_all(&bin)?;
    let path = bin.join(executable);
    fs::write(&path, "#!/bin/sh\nprintf '1.2.3\\n'\n")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    let env = LocalReadEnvironment {
        host_id: "host".into(),
        home: home.to_owned(),
        search_paths: vec![bin],
        applications: home.join("Applications"),
    };
    let installation = read::read(app, &env)?.installations.remove(0);
    let target = McpAppTarget {
        app_id: app,
        installation_id: installation.id.clone(),
        target_host_id: installation.target_host_id.clone(),
    };
    Ok((installation, target))
}
