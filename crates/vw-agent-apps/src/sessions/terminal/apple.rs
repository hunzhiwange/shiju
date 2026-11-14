//! AppleScript receives only a private PTY attachment, never CLI credentials.
use crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession};
use crate::sessions::resume::ResumePlan;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use vw_api_types::external_session::ExternalSessionIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppleTerminal {
    Terminal,
    Iterm2,
}

/// Invoke on the authorized target host with a freshly rebuilt resume plan,
/// retaining the caller's authorization guard until this function returns.
/// The resolver installs AGY credentials in the host process only.
pub fn launch_resume(
    plan: &ResumePlan,
    terminal: AppleTerminal,
    resolve_agy: impl FnOnce(&ExternalSessionIdentity, &mut Command) -> Result<(), String>,
) -> Result<(), String> {
    let mut command = plan.command_with_credentials(resolve_agy)?;
    let home = command
        .get_envs()
        .find(|(key, _)| *key == "HOME")
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
        .ok_or("resume execution home is missing")?;
    let app = application(terminal, &home)?;
    let session = TerminalSession::connect_with(Path::new("/tmp"), |_, material| {
        launch_attachment(terminal, &app, &home, material)
    })?;
    let status = session.run(&mut command, || {}).map_err(|_| "native resume process failed")?;
    if !status.success() {
        return Err(format!("native resume process exited unsuccessfully ({status})"));
    }
    Ok(())
}

fn application(terminal: AppleTerminal, home: &Path) -> Result<PathBuf, String> {
    let paths = match terminal {
        AppleTerminal::Terminal => vec![
            PathBuf::from("/System/Applications/Utilities/Terminal.app"),
            PathBuf::from("/Applications/Utilities/Terminal.app"),
        ],
        AppleTerminal::Iterm2 => {
            vec![PathBuf::from("/Applications/iTerm.app"), home.join("Applications/iTerm.app")]
        }
    };
    paths
        .into_iter()
        .find(|path| path.is_dir())
        .ok_or_else(|| "selected macOS terminal is not installed".into())
}

fn text(path: &Path) -> Result<&str, String> {
    path.to_str()
        .filter(|value| path.is_absolute() && !value.chars().any(char::is_control))
        .ok_or_else(|| "invalid terminal attachment path".into())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn apple_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn launch_attachment(
    terminal: AppleTerminal,
    app: &Path,
    home: &Path,
    material: &LaunchMaterial,
) -> Result<(), String> {
    // TerminalSession owns this generated material and keeps its script alive
    // through attachment. No request-supplied shell or command enters this path.
    let mut attach = format!(
        "cd {} && exec {}",
        shell_quote(text(&material.cwd)?),
        shell_quote(text(&material.program)?)
    );
    for arg in &material.args {
        if arg.chars().any(char::is_control) {
            return Err("invalid terminal attachment argument".into());
        }
        attach.push(' ');
        attach.push_str(&shell_quote(arg));
    }
    let line = apple_quote(&format!("/bin/sh -c {}", shell_quote(&attach)));
    let action = match terminal {
        AppleTerminal::Terminal => format!("do script {line}"),
        AppleTerminal::Iterm2 => format!("create window with default profile command {line}"),
    };
    let script =
        format!("tell application {}\nactivate\n{action}\nend tell", apple_quote(text(app)?));
    let status = Command::new("/usr/bin/osascript")
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .args(["-e", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "cannot start macOS terminal automation")?;
    if !status.success() {
        return Err(format!(
            "macOS terminal automation failed ({status}); check Automation permission"
        ));
    }
    Ok(())
}
