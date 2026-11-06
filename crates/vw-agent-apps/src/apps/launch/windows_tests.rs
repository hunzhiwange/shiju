use super::request::LaunchMaterial;
use super::windows::terminal_commands;
use crate::apps::discovery::wsl::parse_target;

fn material() -> LaunchMaterial {
    LaunchMaterial {
        program: r"C:\工具 目录\agent.exe".into(),
        cwd: r"C:\项目 目录".into(),
        args: vec!["中文 ' text".into()],
        skip_permissions: false,
    }
}

#[test]
fn cc_apps_windows_terminal_launch_unicode_and_selection() {
    for terminal in ["cmd", "powershell", "wt"] {
        let commands = terminal_commands(Some(terminal), None, &material()).unwrap();
        assert_eq!(
            commands[0].program,
            if terminal == "wt" {
                "wt.exe"
            } else if terminal == "cmd" {
                "cmd.exe"
            } else {
                "powershell.exe"
            }
        );
        assert!(!commands[0].args.is_empty());
    }
}

#[test]
fn cc_apps_windows_terminal_launch_rejects_unknown_and_cross_distro() {
    assert!(terminal_commands(Some("unknown"), None, &material()).is_err());
    let target = parse_target(r"\\wsl$\Ubuntu", Some("bash"), None).unwrap();
    let mut input = material();
    input.program = r"\\wsl$\Debian\usr\bin\agent".into();
    input.cwd = "/home/中文".into();
    assert!(terminal_commands(None, Some(&target), &input).is_err());
}

fn decoded(command: &super::windows::WindowsCommand) -> String {
    use base64::Engine;
    let encoded = command.args.last().unwrap().split_whitespace().last().unwrap();
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
    String::from_utf16(
        &bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn cc_apps_windows_terminal_launch_encoded_unicode_and_wsl_binding() {
    let input = material();
    let commands = terminal_commands(Some("powershell"), None, &input).unwrap();
    let script = decoded(&commands[0]);
    assert!(script.contains(r"C:\项目 目录"));
    assert!(script.contains("'中文 '' text'"));
    assert!(script.contains("Set-Location -LiteralPath"));
    for shell in ["bash", "fish", "sh"] {
        let target = parse_target(r"\\wsl$\Ubuntu", Some(shell), None).unwrap();
        let mut input = material();
        input.program = r"\\wsl.localhost\Ubuntu\home\中文 目录\agent".into();
        input.cwd = "/home/中文 目录".into();
        let commands = terminal_commands(Some("wt"), Some(&target), &input).unwrap();
        let script = decoded(&commands[0]);
        assert!(script.contains("'--distribution' 'Ubuntu' '--exec'"));
        assert!(script.contains("/home/中文 目录/agent"));
        assert!(script.contains("test -x"));
        assert!(!script.contains("Debian"));
    }
}

#[test]
fn cc_apps_windows_terminal_launch_fail_closed() {
    let mut input = material();
    input.skip_permissions = true;
    assert!(terminal_commands(None, None, &input).is_err());
    input.skip_permissions = false;
    input.program = "relative.exe".into();
    assert!(terminal_commands(None, None, &input).is_err());
    input.program = r"C:\工具\agent.cmd".into();
    input.args = vec!["%PATH%".into()];
    assert!(terminal_commands(None, None, &input).is_err());
    if !cfg!(windows) {
        assert!(
            super::windows::launch_windows_terminal(None, None, &material())
                .unwrap_err()
                .contains("Windows execution host")
        );
    }
}

#[test]
fn cc_apps_windows_terminal_launch_process_fallback_and_rejection() {
    use super::windows::{WindowsCommand, execute_commands};
    let missing =
        || WindowsCommand { program: "/nonexistent-terminal-task30".into(), args: vec![] };
    assert!(execute_commands(vec![missing()]).is_err());
    #[cfg(unix)]
    {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("handoff");
        execute_commands(vec![
            missing(),
            WindowsCommand {
                program: "/bin/sh".into(),
                args: vec![
                    "-c".into(),
                    "printf '%s' \"$1\" > \"$2\"".into(),
                    "fixture".into(),
                    "中文 目录".into(),
                    output.to_str().unwrap().into(),
                ],
            },
        ])
        .unwrap();
        for _ in 0..100 {
            if std::fs::read_to_string(&output).is_ok_and(|s| s == "中文 目录") {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("terminal boundary fixture did not record argv");
    }
}
