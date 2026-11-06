//! Windows execution-host adapter. Call only with freshly authorized host material.
use super::request::LaunchMaterial;
use crate::apps::discovery::wsl::{WslShell, WslTarget, parse_target};
use base64::Engine;

#[derive(Debug)]
pub(super) struct WindowsCommand {
    pub program: String,
    pub args: Vec<String>,
}

fn ps(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn linux_path(value: &str, distro: &str) -> Result<String, String> {
    let path = if value.starts_with("\\\\") {
        let normalized = value.replace('\\', "/");
        let parts: Vec<_> = normalized.splitn(5, '/').collect();
        if parts.len() != 5
            || !(parts[2].eq_ignore_ascii_case("wsl$")
                || parts[2].eq_ignore_ascii_case("wsl.localhost"))
            || parts[3] != distro
        {
            return Err("WSL launch path does not belong to selected distribution".into());
        }
        format!("/{}", parts[4])
    } else {
        value.to_owned()
    };
    if !path.starts_with('/') || path.contains('\\') || path.split('/').any(|p| p == "..") {
        return Err("invalid WSL launch path".into());
    }
    Ok(path)
}

pub(super) fn terminal_commands(
    preferred: Option<&str>,
    target: Option<&WslTarget>,
    material: &LaunchMaterial,
) -> Result<Vec<WindowsCommand>, String> {
    if material.skip_permissions {
        return Err("permission options require the provider launch materializer".into());
    }
    let terminal = preferred.unwrap_or("cmd");
    if !matches!(terminal, "cmd" | "powershell" | "wt") {
        return Err("unsupported Windows terminal".into());
    }
    let program = material.program.to_str().ok_or("non-UTF8 program path")?;
    let cwd = material.cwd.to_str().ok_or("non-UTF8 working directory")?;
    if [program, cwd]
        .into_iter()
        .chain(material.args.iter().map(String::as_str))
        .any(|s| s.chars().any(char::is_control))
    {
        return Err("invalid launch path or argument".into());
    }
    let script = if let Some(target) = target {
        let (shell, flag) = match &target.shell {
            WslShell::Explicit { shell, flag } => (Some(shell.as_str()), Some(flag.as_str())),
            WslShell::Auto { flag } => (None, flag.as_deref()),
        };
        let checked = parse_target(&format!(r"\\wsl$\{}", target.distro), shell, flag)
            .map_err(|_| "invalid WSL target")?;
        let program = linux_path(program, &checked.distro)?;
        let cwd = linux_path(cwd, &checked.distro)?;
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        // Use POSIX sh for the inner command even when the login shell is fish.
        let mut inner = format!(
            "cd {} && test -f {} && test -x {} && exec {}",
            quote(&cwd),
            quote(&program),
            quote(&program),
            quote(&program)
        );
        for arg in &material.args {
            inner.push(' ');
            inner.push_str(&quote(arg));
        }

        let (shell, flag) = match checked.shell {
            WslShell::Explicit { shell, flag } => (shell, flag),
            // Auto resolves deterministically to sh inside this distro, never a host shell.
            WslShell::Auto { flag } => ("sh".into(), flag.unwrap_or("-c".into())),
        };
        // fish uses different single-quote escaping; pass the inner script as argv via sh.
        let inner_arg = if shell == "fish" {
            format!("'{}'", inner.replace('\\', "\\\\").replace('\'', "\\'"))
        } else {
            quote(&inner)
        };
        let payload = format!("exec sh -c {inner_arg}");
        let argv = ["--distribution", &checked.distro, "--exec", &shell, &flag, &payload];
        format!(
            "& wsl.exe {}; if ($LASTEXITCODE -ne 0) {{ throw 'WSL launch failed' }}",
            argv.into_iter().map(ps).collect::<Vec<_>>().join(" ")
        )
    } else {
        let absolute = |s: &str| {
            s.as_bytes().get(1) == Some(&b':')
                && s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
                && matches!(s.as_bytes().get(2), Some(b'\\' | b'/'))
        };
        if !absolute(program) || !absolute(cwd) {
            return Err("Windows launch requires absolute local paths".into());
        }
        // PowerShell literals preserve Unicode; cmd metacharacters in batch arguments
        // cannot be safely forwarded through its implicit batch parser.
        if (program.to_ascii_lowercase().ends_with(".cmd")
            || program.to_ascii_lowercase().ends_with(".bat"))
            && std::iter::once(program)
                .chain(material.args.iter().map(String::as_str))
                .any(|s| s.contains(['%', '!', '"', '^', '&', '|', '<', '>', '(', ')']))
        {
            return Err("unsafe Windows batch launch argument".into());
        }
        format!(
            "$ErrorActionPreference='Stop'; if (!(Test-Path -LiteralPath {} -PathType Leaf)) {{ throw 'Launch program missing' }}; Set-Location -LiteralPath {}; & {} {}; if ($LASTEXITCODE -ne 0) {{ throw 'Application launch failed' }}",
            ps(program),
            ps(cwd),
            ps(program),
            material.args.iter().map(|s| ps(s)).collect::<Vec<_>>().join(" ")
        )
    };
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(script.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>());
    let powershell = vec![
        "-NoLogo".into(),
        "-NoProfile".into(),
        "-NoExit".into(),
        "-EncodedCommand".into(),
        encoded,
    ];
    let cmd = || WindowsCommand {
        program: "cmd.exe".into(),
        args: vec![
            "/D".into(),
            "/V:OFF".into(),
            "/K".into(),
            format!("powershell.exe {}", powershell.join(" ")),
        ],
    };
    let first = match terminal {
        "cmd" => cmd(),
        "powershell" => {
            WindowsCommand { program: "powershell.exe".into(), args: powershell.clone() }
        }
        _ => WindowsCommand {
            program: "wt.exe".into(),
            args: [vec!["new-tab".into(), "powershell.exe".into()], powershell.clone()].concat(),
        },
    };
    let mut commands = vec![first];
    if terminal != "cmd" {
        commands.push(cmd());
    }
    Ok(commands)
}

/// Fresh authorization and WSL filesystem observation belong to the host caller.
/// Success means terminal handoff, not successful application execution.
pub fn launch_windows_terminal(
    preferred: Option<&str>,
    target: Option<&WslTarget>,
    material: &LaunchMaterial,
) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Windows terminal requires a Windows execution host".into());
    }
    execute_commands(terminal_commands(preferred, target, material)?)
}

pub(super) fn execute_commands(commands: Vec<WindowsCommand>) -> Result<(), String> {
    for command in commands {
        let mut process = std::process::Command::new(command.program);
        process
            .args(command.args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x00000010); // CREATE_NEW_CONSOLE
        }
        if let Ok(mut child) = process.spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
    }
    Err("no available Windows terminal accepted launch".into())
}
