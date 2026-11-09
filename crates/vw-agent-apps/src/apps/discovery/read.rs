//! Local reads restricted to search roots selected by the execution-host administrator.
use super::{
    claude_desktop::{self, DesktopPlatform},
    report, version,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{io, path::PathBuf, time::Duration};
use vw_api_types::agent_apps::{
    AppDesktopObservation, AppId, AppInstallation, AppInstallationEnvironment, AppInstallationId,
    AppInstallationSource, AppReadResponse,
};

pub struct LocalReadEnvironment {
    pub host_id: String,
    pub home: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub applications: PathBuf,
}

fn validate_roots(env: &LocalReadEnvironment) -> io::Result<()> {
    if !env.home.is_absolute()
        || !env.applications.is_absolute()
        || env.search_paths.len() > 64
        || env.search_paths.iter().any(|p| !p.is_absolute())
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid host roots"));
    }
    if !env.home.is_dir() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "execution host home unavailable"));
    }
    Ok(())
}

fn cli_command(app: AppId) -> io::Result<String> {
    crate::apps::catalog::catalog()
        .into_iter()
        .find(|c| c.metadata.id == app)
        .and_then(|c| c.metadata.cli_command)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "no CLI for application"))
}

fn cli_entries<'a>(
    command: &'a str,
    env: &'a LocalReadEnvironment,
) -> impl Iterator<Item = io::Result<PathBuf>> + 'a {
    env.search_paths.iter().filter_map(move |directory| {
        let path = directory.join(command);
        match path.symlink_metadata() {
            Ok(_) => Some(Ok(path)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => Some(Err(error)),
        }
    })
}

/// Resolve the path-default CLI without a version subprocess or ambient PATH.
#[cfg(target_os = "macos")]
pub fn locate_cli(app: AppId, env: &LocalReadEnvironment) -> io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    validate_roots(env)?;
    let command = cli_command(app)?;
    let path = cli_entries(&command, env)
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "CLI not installed"))??
        .canonicalize()?;
    let metadata = path.metadata()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "CLI is not executable"));
    }
    Ok(path)
}

/// Blocking host IO. Caller must authenticate and authorize this exact environment first.
pub fn read(app: AppId, env: &LocalReadEnvironment) -> io::Result<AppReadResponse> {
    validate_roots(env)?;
    if app == AppId::ClaudeDesktop {
        let platform =
            if cfg!(target_os = "macos") { DesktopPlatform::Macos } else { DesktopPlatform::Linux };
        let base = if platform == DesktopPlatform::Linux {
            env.home.join(".config")
        } else {
            env.home.clone()
        };
        let presence = claude_desktop::probe(platform, &base, &env.applications)?;
        let display = |paths: Vec<PathBuf>| {
            paths.into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
        };
        return Ok(AppReadResponse {
            installations: vec![],
            desktop: Some(AppDesktopObservation {
                supported: presence.supported,
                app_bundles: presence.app_bundles.map(display),
                config_directories: display(presence.config_directories),
            }),
        });
    }
    let command = cli_command(app)?;
    let mut installations = Vec::new();
    for path in cli_entries(&command, env) {
        let path = path?;
        let probe = version::probe_local_version(&path, Duration::from_secs(2))?;
        let entry = path.to_string_lossy().into_owned();
        // Length-delimited components avoid collisions across host/path boundaries.
        let id = format!("{}:{}:{}:{}", env.host_id.len(), env.host_id, command.len(), command);
        let is_path_default = installations.is_empty();
        installations.push(AppInstallation {
            id: AppInstallationId(URL_SAFE_NO_PAD.encode(format!("{id}:{}:{entry}", entry.len()))),
            app_id: app,
            target_host_id: env.host_id.clone(),
            environment: if cfg!(target_os = "macos") {
                AppInstallationEnvironment::Macos
            } else {
                AppInstallationEnvironment::Linux
            },
            path: entry,
            real_path: path.canonicalize().ok().map(|p| p.to_string_lossy().into_owned()),
            source: if (app == AppId::Agy && crate::apps::lifecycle::local_agy::owns(&path))
                || (app == AppId::Omp && crate::apps::lifecycle::local_omp::owns(&path))
            {
                AppInstallationSource::System
            } else {
                AppInstallationSource::Unknown
            },
            is_path_default,
            version: probe.version,
            runtime_status: probe.runtime_status,
        });
    }
    let report =
        report::build(installations).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(AppReadResponse { installations: report.installations, desktop: None })
}
