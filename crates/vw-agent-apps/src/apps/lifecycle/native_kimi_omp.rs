//! Official Kimi and bun-based Oh My Pi installer recipes for the authorized host.
use super::native_primary::{InstallCommand, InstallEnvironment};
use vw_api_types::agent_apps::AppId;

/// Oh My Pi's official installer provisions bun and its own package, never Pi's
/// npm package. Runtime provisioning stays with the official installer.
pub fn install_command(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<InstallCommand, &'static str> {
    let base = match app {
        AppId::Kimi => "https://code.kimi.com/kimi-code/install",
        AppId::Omp => "https://omp.sh/install",
        _ => return Err("application has no reviewed Kimi/Oh My Pi native recipe"),
    };
    match environment {
        InstallEnvironment::Posix | InstallEnvironment::Wsl => {
            let (url, shell) = if app == AppId::Kimi {
                (format!("{base}.sh"), "bash")
            } else {
                (base.to_owned(), "sh")
            };
            Ok(InstallCommand {
                program: "bash",
                args: vec![
                    "-c".into(),
                    format!(
                        "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL {url} -o \"$tmp\" && {shell} \"$tmp\")"
                    ),
                ],
            })
        }
        InstallEnvironment::Windows => Ok(InstallCommand {
            program: "powershell",
            args: vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-Command".into(),
                format!(
                    "& powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command '$ErrorActionPreference = \"Stop\"; Invoke-RestMethod \"{base}.ps1\" | Invoke-Expression'; exit $LASTEXITCODE"
                ),
            ],
        }),
    }
}

/// The executor must provide the discovered real target (resolve symlinks first)
/// and anchor the returned program to that installation before executing it.
/// Nix's updater only prints advice and exits zero, so reject that channel.
pub fn update_command(
    app: AppId,
    _environment: InstallEnvironment,
    real_target: &str,
) -> Result<InstallCommand, &'static str> {
    if real_target.trim().is_empty() {
        return Err("resolved installation target required for update");
    }
    match app {
        AppId::Kimi => {
            Ok(InstallCommand { program: "kimi", args: vec!["upgrade".into(), "--yes".into()] })
        }
        AppId::Omp if real_target.starts_with("/nix/store/") => {
            Err("Nix-managed Oh My Pi must be updated through its Nix configuration")
        }
        AppId::Omp => Ok(InstallCommand { program: "omp", args: vec!["update".into()] }),
        _ => Err("application has no reviewed Kimi/Oh My Pi update recipe"),
    }
}

/// For legacy native Kimi versions that ignore --yes, rerun the installer in
/// the detected root. Package-manager installations must use their own updater.
pub fn kimi_installer_update_command(
    environment: InstallEnvironment,
    root: &str,
) -> Result<InstallCommand, &'static str> {
    if root.trim().is_empty() || root.contains('\0') {
        return Err("detected Kimi installation root required");
    }
    let mut command = install_command(AppId::Kimi, environment)?;
    let script = command.args.last_mut().expect("installer has script");
    match environment {
        InstallEnvironment::Posix | InstallEnvironment::Wsl => {
            *script =
                format!("export KIMI_INSTALL_DIR='{}'; {script}", root.replace('\'', "'\\''"));
        }
        InstallEnvironment::Windows => {
            // Set the environment in the parent so the isolated child inherits it.
            *script = format!("$env:KIMI_INSTALL_DIR = '{}'; {script}", root.replace('\'', "''"));
        }
    }
    Ok(command)
}
