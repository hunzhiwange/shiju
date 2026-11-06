//! Execution-host terminal adapter. No client command strings or remote dispatch.
use super::request::LaunchMaterial;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosixPlatform {
    Macos,
    Linux,
}

#[derive(Debug)]
pub(super) struct TerminalCommand {
    pub(super) launcher: Option<tempfile::TempPath>,
    pub(super) program: String,
    pub(super) args: Vec<String>,
}

pub(super) fn terminal_commands(
    platform: PosixPlatform,
    preferred: Option<&str>,
    shell: &Path,
    material: &LaunchMaterial,
) -> Result<Vec<TerminalCommand>, String> {
    if material.skip_permissions {
        return Err("permission options require the provider launch materializer".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in [shell, material.program.as_path()] {
            if path
                .metadata()
                .map_err(|_| "execution-host executable is missing")?
                .permissions()
                .mode()
                & 0o111
                == 0
            {
                return Err("execution-host file is not executable".into());
            }
        }
    }
    let shell_name = shell.file_name().and_then(|s| s.to_str()).ok_or("invalid shell")?;
    if !shell.is_absolute()
        || !matches!(shell_name, "bash" | "zsh" | "fish" | "sh" | "dash")
        || !shell.is_file()
    {
        return Err(
            "supported execution-host shell is missing (bash, zsh, fish, sh or dash)".into()
        );
    }
    let text = |path: &Path| -> Result<String, String> {
        let value = path.to_str().ok_or("non-UTF8 launch path")?;
        if !path.is_absolute() || value.chars().any(char::is_control) {
            return Err("invalid execution-host launch path".into());
        }
        Ok(value.to_owned())
    };
    if !material.cwd.is_dir() || !material.program.is_file() {
        return Err("execution-host launch program or directory is missing".into());
    }
    let quote = |value: &str| {
        if shell_name == "fish" {
            format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
        } else {
            format!("'{}'", value.replace('\'', "'\\''"))
        }
    };
    let mut command = format!(
        "cd {} || exit 1; exec {}",
        quote(&text(&material.cwd)?),
        quote(&text(&material.program)?)
    );
    for arg in &material.args {
        if arg.chars().any(char::is_control) {
            return Err("invalid launch argument".into());
        }
        command.push(' ');
        command.push_str(&quote(arg));
    }
    let shell_args = vec![
        text(shell)?,
        match shell_name {
            "zsh" => "-lic",
            "sh" | "dash" => "-c",
            _ => "-ic",
        }
        .into(),
        command,
    ];
    let shell_line = shell_args
        .iter()
        .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let apple = |app: &str, iterm: bool| {
        let escaped = shell_line.replace('\\', "\\\\").replace('"', "\\\"");
        let script = if iterm {
            format!(
                "tell application \"iTerm\"\nactivate\ncreate window with default profile command \"{escaped}\"\nend tell"
            )
        } else {
            format!("tell application \"{app}\"\nactivate\ndo script \"{escaped}\"\nend tell")
        };
        TerminalCommand {
            launcher: None,
            program: "/usr/bin/osascript".into(),
            args: vec!["-e".into(), script],
        }
    };
    match platform {
        PosixPlatform::Macos => {
            let terminal = preferred.unwrap_or("terminal");
            let first = match terminal {
                "terminal" => apple("Terminal", false),
                "iterm2" => apple("iTerm", true),
                "otty" => {
                    let roots = [
                        Some(std::path::PathBuf::from("/Applications")),
                        std::env::var_os("HOME")
                            .map(|home| std::path::PathBuf::from(home).join("Applications")),
                    ];
                    let cli = roots
                        .into_iter()
                        .flatten()
                        .map(|root| root.join("Otty.app/Contents/MacOS/otty-cli"))
                        .find(|path| path.is_file())
                        .ok_or("Otty CLI is missing from Applications")?;
                    TerminalCommand {
                        launcher: None,
                        program: text(&cli)?,
                        args: vec!["open".into(), "--command".into(), shell_line.clone()],
                    }
                }
                "alacritty" | "kitty" | "ghostty" | "wezterm" | "kaku" => {
                    let app = match terminal {
                        "alacritty" => "Alacritty",
                        "ghostty" => "Ghostty",
                        "wezterm" => "WezTerm",
                        "kaku" => "Kaku",
                        _ => "kitty",
                    };
                    let mut args = vec!["-na".into(), app.into(), "--args".into()];
                    if terminal == "wezterm" || terminal == "kaku" {
                        args.push("start".into());
                        args.push("--".into());
                    } else if terminal != "kitty" {
                        args.push("-e".into());
                    }
                    args.extend(shell_args);
                    TerminalCommand { launcher: None, program: "/usr/bin/open".into(), args }
                }
                "warp" => {
                    use std::io::Write;
                    let mut file = tempfile::NamedTempFile::new()
                        .map_err(|_| "cannot create private Warp launcher")?;
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        file.as_file()
                            .set_permissions(std::fs::Permissions::from_mode(0o700))
                            .map_err(|_| "cannot secure Warp launcher")?;
                    }
                    writeln!(file, "#!/bin/sh\nrm -f -- \"$0\"\nexec {shell_line}")
                        .map_err(|_| "cannot write Warp launcher")?;
                    let encoded = text(file.path())?
                        .bytes()
                        .map(|byte| format!("%{byte:02X}"))
                        .collect::<String>();
                    TerminalCommand {
                        launcher: Some(file.into_temp_path()),
                        program: "/usr/bin/open".into(),
                        args: vec![
                            "-a".into(),
                            "Warp".into(),
                            format!("warp://action/new_tab?path={encoded}"),
                        ],
                    }
                }
                _ => return Err("unsupported macOS terminal".into()),
            };
            let mut commands = vec![first];
            if terminal != "terminal" {
                commands.push(apple("Terminal", false));
            }
            Ok(commands)
        }
        PosixPlatform::Linux => {
            let defaults = [
                "gnome-terminal",
                "konsole",
                "xfce4-terminal",
                "mate-terminal",
                "lxterminal",
                "alacritty",
                "kitty",
                "ghostty",
            ];
            if preferred.is_some_and(|p| !defaults.contains(&p)) {
                return Err("unsupported Linux terminal".into());
            }
            let mut names = Vec::new();
            if let Some(p) = preferred {
                names.push(p);
            }
            names.extend(defaults.into_iter().filter(|p| Some(*p) != preferred));
            Ok(names
                .into_iter()
                .map(|name| {
                    let mut args = vec![if matches!(name, "gnome-terminal" | "mate-terminal") {
                        "--".into()
                    } else {
                        "-e".into()
                    }];
                    if matches!(name, "xfce4-terminal" | "lxterminal") {
                        args.push(shell_line.clone());
                    } else {
                        args.extend(shell_args.clone());
                    }
                    TerminalCommand { launcher: None, program: name.into(), args }
                })
                .collect())
        }
    }
}

/// Call only on the authorized execution host, with freshly validated material.
/// Provider/permission options must be materialized by their owning adapter.
pub fn launch_posix_terminal(
    preferred: Option<&str>,
    shell: &Path,
    material: &LaunchMaterial,
) -> Result<(), String> {
    let platform = if cfg!(target_os = "macos") {
        PosixPlatform::Macos
    } else if cfg!(target_os = "linux") {
        PosixPlatform::Linux
    } else {
        return Err("POSIX terminal requires macOS or Linux execution host".into());
    };
    execute_commands(
        terminal_commands(platform, preferred, shell, material)?,
        &material.cwd,
        platform,
    )
}

pub(super) fn execute_commands(
    commands: Vec<TerminalCommand>,
    cwd: &Path,
    platform: PosixPlatform,
) -> Result<(), String> {
    for command in commands {
        let mut process = Command::new(&command.program);
        process
            .args(&command.args)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let accepted = if platform == PosixPlatform::Linux {
            match process.spawn() {
                Ok(mut child) => {
                    std::thread::spawn(move || {
                        let _ = child.wait();
                    });
                    true
                }
                Err(_) => false,
            }
        } else {
            process.status().is_ok_and(|status| status.success())
        };
        if accepted {
            if let Some(file) = command.launcher {
                file.keep().map_err(|_| "cannot retain Warp launcher")?;
            }
            return Ok(());
        }
    }
    Err("no available terminal accepted launch; install or configure a supported terminal".into())
}
