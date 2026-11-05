//! Fixed native recipes consumed by the authorized host executor.
use super::native_primary::{InstallCommand, InstallEnvironment};
use vw_api_types::agent_apps::AppId;

/// Official installers carry their own runtime where needed (MiniMax bundles
/// Node). Do not gate the native attempt on the system Node version. Only a
/// failed native attempt enters the reviewed npm fallback.
pub fn install_command(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<InstallCommand, &'static str> {
    recipe(app, environment, None)
}

/// Reinstall the discovered MiniMax script installation in place. Never use
/// `mcode update`: with non-TTY streams it can exit successfully without updating.
/// No global npm fallback is allowed for an anchored installation.
pub fn mcode_update_command(
    environment: InstallEnvironment,
    install_root: &str,
) -> Result<InstallCommand, &'static str> {
    if install_root.is_empty() || install_root.contains(['\0', '\n', '\r']) {
        return Err("invalid MiniMax installation root");
    }
    recipe(AppId::Mcode, environment, Some(install_root))
}

fn recipe(
    app: AppId,
    environment: InstallEnvironment,
    root: Option<&str>,
) -> Result<InstallCommand, &'static str> {
    let base = match app {
        AppId::GrokBuild => "https://x.ai/cli/install",
        AppId::Mcode => "https://filecdn.minimax.chat/public/install",
        _ => return Err("application has no reviewed Grok/MiniMax native recipe"),
    };
    let npm = super::npm::install_args(app)?
        .iter()
        .map(|arg| format!("'{arg}'"))
        .collect::<Vec<_>>()
        .join(" ");
    match environment {
        InstallEnvironment::Posix | InstallEnvironment::Wsl => {
            let prefix = root
                .map(|root| format!("export MCODE_INSTALL_DIR='{}'; ", root.replace('\'', "'\\''")))
                .unwrap_or_default();
            let native = format!(
                "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; {prefix}curl -fsSL {base}.sh -o \"$tmp\" && bash \"$tmp\")"
            );
            let script = if root.is_some() { native } else { format!("{native} || npm {npm}") };
            Ok(InstallCommand { program: "bash", args: vec!["-c".into(), script] })
        }
        InstallEnvironment::Windows => {
            let prefix = root
                .map(|root| format!("$env:MCODE_INSTALL_DIR = '{}'; ", root.replace('\'', "''")))
                .unwrap_or_default();
            // Structured -Command argv bypasses cmd parsing. Terminating download
            // errors and installer native exit codes both reach the fallback.
            // Isolate installer `exit` in a child so it cannot skip our fallback.
            let failure = if root.is_some() {
                "exit 1".to_owned()
            } else {
                format!("& npm.cmd {npm}; exit $LASTEXITCODE")
            };
            let script = format!(
                "$ErrorActionPreference = 'Stop'; {prefix}try {{ $global:LASTEXITCODE = 0; & powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command '$ErrorActionPreference = \"Stop\"; Invoke-RestMethod \"{base}.ps1\" | Invoke-Expression'; if ($LASTEXITCODE -ne 0) {{ throw 'installer failed' }} }} catch {{ {failure} }}"
            );
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
