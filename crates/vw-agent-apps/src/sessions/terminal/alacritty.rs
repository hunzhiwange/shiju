//! Alacritty attaches to a host-owned PTY without receiving native CLI credentials.
use crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession};
use crate::sessions::resume::ResumePlan;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use vw_api_types::external_session::ExternalSessionIdentity;

/// Run on the authorized execution host with a freshly rebuilt plan, retaining
/// its authorization guard until the native CLI and PTY resources are reaped.
pub fn launch_resume(
    plan: &ResumePlan,
    resolve_agy: impl FnOnce(&ExternalSessionIdentity, &mut Command) -> Result<(), String>,
) -> Result<(), String> {
    let mut command = plan.command_with_credentials(resolve_agy)?;
    let home = command
        .get_envs()
        .find(|(key, _)| *key == "HOME")
        .and_then(|(_, value)| value)
        .map(PathBuf::from)
        .ok_or("resume execution home is missing")?;
    let app = [
        PathBuf::from("/Applications/Alacritty.app"),
        home.join("Applications/Alacritty.app"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .ok_or("Alacritty is not installed in the supported locations")?;
    let session = TerminalSession::connect_with(Path::new("/tmp"), |_, material| {
        launch_attachment(&app, &home, material)
    })?;
    let status = session.run(&mut command, || {}).map_err(|_| "native resume process failed")?;
    if !status.success() {
        return Err(format!("native resume process exited unsuccessfully ({status})"));
    }
    Ok(())
}

fn launch_attachment(app: &Path, home: &Path, material: &LaunchMaterial) -> Result<(), String> {
    // Only the generated, credential-free attachment reaches Alacritty. Native
    // resume argv, cwd and environment remain in the host-side Command above.
    let status = Command::new("/usr/bin/open")
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .current_dir(&material.cwd)
        .arg("-na")
        .arg(app)
        .arg("--args")
        .arg("--working-directory")
        .arg(&material.cwd)
        .arg("-e")
        .arg(&material.program)
        .args(&material.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "cannot launch Alacritty")?;
    if !status.success() {
        return Err(format!("Alacritty launch failed ({status})"));
    }
    Ok(())
}
