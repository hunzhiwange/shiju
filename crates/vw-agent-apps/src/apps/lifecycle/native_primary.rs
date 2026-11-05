use vw_api_types::agent_apps::AppId;

#[derive(Debug, Clone, Copy)]
/// Target execution environment; WSL recipes run inside the authorized distro.
pub enum InstallEnvironment {
    Posix,
    Wsl,
    Windows,
}

#[derive(Debug)]
/// Fixed recipe for the authorized host, not an execution or permission boundary.
/// Resolve programs on that host (including npm.cmd on Windows); batch adapters
/// must preserve `call` semantics. No caller text may be appended to shell source.
pub struct InstallCommand {
    pub program: &'static str,
    pub args: Vec<String>,
}

/// Official POSIX installer followed only on failure by the reviewed npm recipe.
/// Windows uses npm directly, matching the preserved upstream Windows entry.
pub fn install_command(
    app: AppId,
    environment: InstallEnvironment,
) -> Result<InstallCommand, &'static str> {
    let url = match app {
        AppId::Claude => "https://claude.ai/install.sh",
        AppId::OpenCode => "https://opencode.ai/install",
        _ => return Err("application has no reviewed primary native recipe"),
    };
    let npm_args = super::npm::install_args(app)?;
    match environment {
        InstallEnvironment::Windows => Ok(InstallCommand {
            program: "npm",
            args: npm_args.iter().map(|arg| (*arg).to_owned()).collect(),
        }),
        InstallEnvironment::Posix | InstallEnvironment::Wsl => {
            // Only reviewed static tokens enter the script. Quote paths because
            // TMPDIR may contain spaces. Keep download and execution joined by &&:
            // a partial download must never reach bash, even if it looks valid.
            let npm = npm_args.iter().map(|arg| format!("'{arg}'")).collect::<Vec<_>>().join(" ");
            let script = format!(
                "(tmp=$(mktemp) || exit $?; trap 'rm -f \"$tmp\"' EXIT; curl -fsSL {url} -o \"$tmp\" && bash \"$tmp\") || npm {npm}"
            );
            Ok(InstallCommand { program: "bash", args: vec!["-c".into(), script] })
        }
    }
}
