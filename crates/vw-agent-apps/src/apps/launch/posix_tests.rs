use super::posix::*;
use super::request::LaunchMaterial;
use std::path::Path;

#[test]
fn cc_apps_posix_terminal_launch_shell_executes_argv_and_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("result");
    let material = LaunchMaterial {
        program: "/bin/echo".into(),
        cwd: dir.path().into(),
        args: vec!["中文 a'b $literal".into()],
        skip_permissions: false,
    };
    let commands =
        terminal_commands(PosixPlatform::Linux, Some("kitty"), Path::new("/bin/bash"), &material)
            .unwrap();
    assert_eq!(commands[0].program, "kitty");
    let args = &commands[0].args;
    let result = std::process::Command::new(&args[1]).args(&args[2..]).output().unwrap();
    std::fs::write(&output, &result.stdout).unwrap();
    assert!(result.status.success());
    assert_eq!(std::fs::read_to_string(output).unwrap(), "中文 a'b $literal\n");
}

#[test]
fn cc_apps_posix_terminal_launch_rejects_unknown_terminal() {
    let material = LaunchMaterial {
        program: "/bin/echo".into(),
        cwd: "/".into(),
        args: vec![],
        skip_permissions: false,
    };
    assert!(
        terminal_commands(PosixPlatform::Linux, Some("evil;sh"), Path::new("/bin/bash"), &material)
            .is_err()
    );
}

fn material() -> LaunchMaterial {
    LaunchMaterial {
        program: "/bin/pwd".into(),
        cwd: std::env::temp_dir().canonicalize().unwrap(),
        args: vec![],
        skip_permissions: false,
    }
}

#[test]
fn cc_apps_posix_terminal_launch_cwd_survives_shell_startup() {
    for shell in ["/bin/bash", "/bin/zsh"] {
        if !Path::new(shell).exists() {
            continue;
        }
        let material = material();
        let commands =
            terminal_commands(PosixPlatform::Linux, Some("kitty"), Path::new(shell), &material)
                .unwrap();
        let args = &commands[0].args;
        let output = std::process::Command::new(&args[1])
            .args(&args[2..])
            .current_dir("/")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            material.cwd.to_str().unwrap()
        );
    }
}

#[test]
fn cc_apps_posix_terminal_launch_macos_escaping_and_warp_cleanup() {
    let mut material = material();
    material.args = vec!["a\"b\\c'd".into()];
    for name in ["terminal", "iterm2", "alacritty", "kitty", "ghostty", "wezterm", "kaku", "warp"] {
        let commands =
            terminal_commands(PosixPlatform::Macos, Some(name), Path::new("/bin/bash"), &material)
                .unwrap();
        assert!(commands[0].program.starts_with("/usr/bin/"));
        if name == "terminal" {
            let script = &commands[0].args[1];
            assert!(script.contains("do script \""));
            assert!(script.contains("\\\""));
            assert!(script.contains("\\\\"));
        }
        if name == "warp" {
            let file = commands[0].launcher.as_ref().unwrap().to_path_buf();
            let script = std::fs::read_to_string(&file).unwrap();
            assert!(script.starts_with("#!/bin/sh\n"));
            assert!(script.contains("rm -f -- \"$0\""));
            drop(commands);
            assert!(!file.exists());
        }
    }
}

#[test]
fn cc_apps_posix_terminal_launch_missing_shell_and_permissions_fail() {
    let mut material = material();
    assert!(
        terminal_commands(PosixPlatform::Linux, None, Path::new("/missing/bash"), &material)
            .is_err()
    );
    material.skip_permissions = true;
    assert!(
        terminal_commands(PosixPlatform::Macos, None, Path::new("/bin/bash"), &material).is_err()
    );
}

#[test]
fn cc_apps_posix_terminal_launch_process_failure_and_fallback() {
    let cmd =
        |program: &str| TerminalCommand { launcher: None, program: program.into(), args: vec![] };
    assert!(
        execute_commands(vec![cmd("/usr/bin/false")], Path::new("/"), PosixPlatform::Macos)
            .is_err()
    );
    assert!(
        execute_commands(
            vec![cmd("/missing/terminal"), cmd("/usr/bin/true")],
            Path::new("/"),
            PosixPlatform::Macos
        )
        .is_ok()
    );
    assert!(
        execute_commands(vec![cmd("/missing/terminal")], Path::new("/"), PosixPlatform::Linux)
            .is_err()
    );
}

#[test]
fn cc_apps_posix_terminal_launch_fish_argument_rules() {
    let dir = tempfile::tempdir().unwrap();
    let shell = dir.path().join("fish");
    std::fs::write(&shell, "fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut material = material();
    material.args = vec!["a'b\\c".into()];
    let commands =
        terminal_commands(PosixPlatform::Linux, Some("kitty"), &shell, &material).unwrap();
    assert_eq!(commands[0].args[2], "-ic");
    assert!(commands[0].args[3].contains("'a\\'b\\\\c'"));
}
