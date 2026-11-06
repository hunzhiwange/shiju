//! Hermes and Antigravity native lifecycle recipes for the authorized host.
use super::native_primary::{InstallCommand, InstallEnvironment};
use vw_api_types::agent_apps::AppId;

/// Native installers provide their own runtime setup; never substitute npm or pip.
pub fn install_command(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<InstallCommand, &'static str> {
    recipe(app, environment, false)
}

/// Antigravity's installer is a no-op for existing installs, so failed updates
/// must propagate. Hermes retains its reviewed official-installer fallback.
/// These unanchored recipes require host-side installation/source review before
/// execution; selecting a specific discovered executable belongs to the executor.
pub fn update_command(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<InstallCommand, &'static str> {
    recipe(app, environment, true)
}

fn recipe(
    app: AppId,
    environment: InstallEnvironment,
    update: bool,
) -> Result<InstallCommand, &'static str> {
    let base = match app {
        AppId::Hermes => {
            "https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install"
        }
        AppId::Agy => "https://antigravity.google/cli/install",
        _ => return Err("application has no reviewed Hermes/Antigravity native recipe"),
    };
    if update && app == AppId::Agy {
        return Ok(InstallCommand { program: "agy", args: vec!["update".into()] });
    }
    match environment {
        InstallEnvironment::Posix | InstallEnvironment::Wsl => {
            let installer = format!(
                "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL {base}.sh -o \"$tmp\" && bash \"$tmp\")"
            );
            let script = if update { format!("hermes update || {installer}") } else { installer };
            Ok(InstallCommand { program: "bash", args: vec!["-c".into(), script] })
        }
        InstallEnvironment::Windows => {
            // A child isolates installer `exit`; terminating download errors and
            // native exit codes must reach the caller as failures, not success.
            let installer = format!(
                "& powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command '$ErrorActionPreference = \"Stop\"; Invoke-RestMethod \"{base}.ps1\" | Invoke-Expression'; exit $LASTEXITCODE"
            );
            let script = if update {
                format!(
                    "$ErrorActionPreference = 'Stop'; try {{ & hermes update; if ($LASTEXITCODE -eq 0) {{ exit 0 }} }} catch {{ }}; {installer}"
                )
            } else {
                installer
            };
            Ok(InstallCommand {
                program: "powershell",
                args: vec![
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-ExecutionPolicy".into(),
                    "Bypass".into(),
                    "-Command".into(),
                    script,
                ],
            })
        }
    }
}

/// The official agy installer explicitly supports --dir. Pass the administrator's
/// path as a positional argument; never interpolate it into shell source.
pub(crate) fn agy_managed_install(directory: &std::path::Path) -> InstallCommand {
    InstallCommand { program: "bash", args: vec!["-c".into(),
        "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL https://antigravity.google/cli/install.sh -o \"$tmp\" && bash \"$tmp\" --dir \"$1\")".into(),
        "shiju-agy-install".into(), directory.to_string_lossy().into_owned()] }
}
