//! Otty attaches to the host-owned PTY; native argv and credentials stay on the host.
use crate::apps::launch::{request::LaunchMaterial, terminal_session::TerminalSession};
use crate::sessions::resume::ResumePlan;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};
use vw_api_types::external_session::ExternalSessionIdentity;

/// Call under the target host authorization guard with a freshly rebuilt plan.
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
    let search_paths = command
        .get_envs()
        .find(|(key, _)| *key == "PATH")
        .and_then(|(_, value)| value)
        .map(|value| std::env::split_paths(value).collect::<Vec<_>>())
        .ok_or("resume execution search paths are missing")?;
    let cli = find_cli(&home, &search_paths)?;
    let session = TerminalSession::connect_with(Path::new("/tmp"), |_, material| {
        launch_attachment(&cli, &home, material)
    })?;
    let status = session.run(&mut command, || {}).map_err(|_| "native resume process failed")?;
    if !status.success() {
        return Err(format!("native resume process exited unsuccessfully ({status})"));
    }
    Ok(())
}

fn find_cli(home: &Path, search_paths: &[PathBuf]) -> Result<PathBuf, String> {
    let mut candidates = vec![
        PathBuf::from("/Applications/Otty.app/Contents/MacOS/otty-cli"),
        home.join("Applications/Otty.app/Contents/MacOS/otty-cli"),
        PathBuf::from("/usr/local/bin/otty"),
        PathBuf::from("/opt/homebrew/bin/otty"),
    ];
    for directory in search_paths {
        if !directory.is_absolute() {
            return Err("invalid Otty search directory".into());
        }
        candidates.push(directory.join("otty"));
        candidates.push(directory.join("otty-cli"));
    }
    for path in candidates {
        match path.metadata() {
            Ok(metadata) if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 => {
                return Ok(path);
            }
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err("cannot inspect Otty CLI candidate".into()),
        }
    }
    Err("Otty CLI is not installed in the supported locations".into())
}

fn quote(value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) {
        return Err("invalid Otty attachment argument".into());
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

fn launch_attachment(cli: &Path, home: &Path, material: &LaunchMaterial) -> Result<(), String> {
    // Only TerminalSession's generated private attachment is shell-encoded here.
    // Session identifiers, native cwd/environment and AGY keys never enter Otty.
    if !material.program.is_absolute() || !material.cwd.is_absolute() {
        return Err("invalid Otty attachment path".into());
    }
    let mut attach = quote(material.program.to_str().ok_or("invalid Otty attachment program")?)?;
    for arg in &material.args {
        attach.push(' ');
        attach.push_str(&quote(arg)?);
    }
    let tab = invoke(cli, home, material, &attach, true)?;
    if tab.success() {
        return Ok(());
    }
    let window = invoke(cli, home, material, &attach, false)
        .map_err(|error| format!("Otty tab launch failed ({tab}); {error}"))?;
    if !window.success() {
        return Err(format!("Otty tab launch failed ({tab}); window launch failed ({window})"));
    }
    Ok(())
}

fn invoke(
    cli: &Path,
    home: &Path,
    material: &LaunchMaterial,
    attach: &str,
    tab: bool,
) -> Result<ExitStatus, String> {
    let mut command = Command::new(cli);
    command
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .current_dir(&material.cwd);
    if tab {
        command
            .args(["tab", "new", "--window", "0", "--cwd"])
            .arg(&material.cwd)
            .arg("--command")
            .arg(attach);
    } else {
        command.args(["open", "--command"]).arg(attach).arg(&material.cwd);
    }
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(
        |_| if tab { "cannot launch Otty tab CLI" } else { "cannot launch Otty window CLI" }.into(),
    )
}
