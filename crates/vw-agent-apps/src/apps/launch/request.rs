//! Validate on the execution host, using freshly discovered, host-scoped observations.
use std::path::PathBuf;
use vw_api_types::agent_apps::{AppInstallation, AppInstallationRuntimeStatus, LaunchRequest};

/// Internal material only; never deserialize this from a client or execute it as a shell string.
#[derive(Debug, PartialEq, Eq)]
pub struct LaunchMaterial {
    pub program: PathBuf,
    pub cwd: PathBuf,
    pub args: Vec<String>,
    pub skip_permissions: bool,
}

/// `installations`, `authorized_roots` and permission decision must come from the
/// authenticated execution-host caller, never from request fields. Revalidate
/// immediately before execution; this validation does not grant authorization.
pub fn validate_launch_request(
    request: &LaunchRequest,
    target_host_id: &str,
    installations: &[AppInstallation],
    authorized_roots: &[PathBuf],
    skip_permissions_authorized: bool,
) -> Result<LaunchMaterial, String> {
    let mut matches = installations.iter().filter(|installation| {
        installation.id == request.installation_id
            && installation.app_id == request.app_id
            && installation.target_host_id == target_host_id
    });
    let installation = matches.next().ok_or("launch instance not found on authorized host")?;
    if matches.next().is_some() {
        return Err("ambiguous launch instance".into());
    }
    if request.app_id == vw_api_types::agent_apps::AppId::ClaudeDesktop {
        return Err("desktop application is not a terminal launch target".into());
    }
    // Filesystem checks must execute in the installation's own environment.
    // WSL materialization belongs to its host adapter, never the Windows filesystem.
    let local_environment = match installation.environment {
        vw_api_types::agent_apps::AppInstallationEnvironment::Macos => cfg!(target_os = "macos"),
        vw_api_types::agent_apps::AppInstallationEnvironment::Windows => cfg!(windows),
        vw_api_types::agent_apps::AppInstallationEnvironment::Linux => cfg!(target_os = "linux"),
        vw_api_types::agent_apps::AppInstallationEnvironment::Wsl { .. } => false,
    };
    if !local_environment {
        return Err("launch validation requires the installation execution environment".into());
    }
    if installation.runtime_status != AppInstallationRuntimeStatus::Runnable {
        return Err("launch instance is not runnable".into());
    }
    if request.skip_permissions && !skip_permissions_authorized {
        return Err("permission skipping is not authorized".into());
    }
    if request.cwd.chars().any(char::is_control) {
        return Err("invalid launch directory".into());
    }
    let cwd = PathBuf::from(&request.cwd);
    if !cwd.is_absolute() {
        return Err("launch directory must be absolute".into());
    }
    let cwd = cwd.canonicalize().map_err(|_| "launch directory cannot be resolved")?;
    if !cwd.is_dir()
        || !authorized_roots.iter().any(|root| {
            root.is_absolute()
                && root.canonicalize().is_ok_and(|root| root.is_dir() && cwd.starts_with(root))
        })
    {
        return Err("launch directory is outside authorized directories".into());
    }
    if request.args.len() > 32
        || request.args.iter().any(|arg| {
            // No generic option forwarding: CLI flags can override sandbox, config,
            // cwd or permission policy through aliases. Reviewed application-specific
            // options belong in the materializer, not in this untrusted argument list.
            arg.starts_with('-')
                || arg.len() > 8192
                || arg.chars().any(|c| {
                    c.is_control()
                        || matches!(c, ';' | '&' | '|' | '<' | '>' | '$' | '`' | '%' | '!')
                })
        })
    {
        return Err("unsafe or unsupported launch argument".into());
    }
    let program = PathBuf::from(&installation.path);
    if !program.is_absolute() || installation.path.chars().any(char::is_control) {
        return Err("invalid observed launch program".into());
    }
    let program = program.canonicalize().map_err(|_| "observed launch program is missing")?;
    if !program.is_file() {
        return Err("observed launch program is not a file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = program.metadata().map_err(|_| "observed launch program is unavailable")?;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("observed launch program is not executable".into());
        }
    }
    if let Some(expected) = &installation.real_path {
        let expected = PathBuf::from(expected);
        if !expected.is_absolute() || expected != program {
            return Err("observed launch program changed; rediscovery required".into());
        }
    }
    Ok(LaunchMaterial {
        program,
        cwd,
        args: request.args.clone(),
        skip_permissions: request.skip_permissions,
    })
}
