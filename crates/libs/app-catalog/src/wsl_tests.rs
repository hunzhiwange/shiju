use super::wsl::{WslShell, parse_target, resolve_target};
use std::io::ErrorKind;

#[test]
fn cc_apps_wsl_target_resolution_distro_and_auto_shell() {
    for root in [
        r"\\wsl$\Ubuntu-22.04\home\user",
        r"\\WSL.LOCALHOST\Ubuntu-22.04\home",
        r"\\?\UNC\wsl$\Ubuntu-22.04\home",
    ] {
        let target = parse_target(root, None, None).unwrap();
        assert_eq!(target.distro, "Ubuntu-22.04");
        assert_eq!(target.shell, WslShell::Auto { flag: None });
    }
    assert_eq!(
        parse_target(r"\\wsl$\Debian", None, Some("-lc")).unwrap().shell,
        WslShell::Auto { flag: Some("-lc".into()) }
    );
}

#[test]
fn cc_apps_wsl_target_resolution_explicit_shell() {
    for (shell, flag) in
        [("sh", "-c"), ("dash", "-c"), ("bash", "-lic"), ("zsh", "-lic"), ("fish", "-lc")]
    {
        assert_eq!(
            parse_target(r"\\wsl$\Debian", Some(&format!("/bin/{shell}")), None).unwrap().shell,
            WslShell::Explicit { shell: shell.into(), flag: flag.into() }
        );
    }
    assert_eq!(
        parse_target(r"\\wsl$\Debian", Some("bash"), Some("-c")).unwrap().shell,
        WslShell::Explicit { shell: "bash".into(), flag: "-c".into() }
    );
}

#[test]
fn cc_apps_wsl_target_resolution_rejects_injection_and_local_roots() {
    for root in [
        r"C:\home",
        "/home",
        r"\\server\Ubuntu",
        r"\\wsl$\",
        r"\\wsl$\--exec",
        r"\\wsl$\..",
        r"\\wsl$\bad;id",
        r"\\wsl$\bad name",
    ] {
        assert_eq!(parse_target(root, None, None).unwrap_err().kind(), ErrorKind::InvalidInput);
    }
    assert!(parse_target(&format!(r"\\wsl$\{}", "a".repeat(65)), None, None).is_err());
    for shell in ["", "bash -c", "bash;id", "../bash", "/tmp/../bin/bash", "powershell", "-bash"] {
        assert!(parse_target(r"\\wsl$\Debian", Some(shell), None).is_err());
    }
    for flag in ["", "--login", "-x", "-c;id"] {
        assert!(parse_target(r"\\wsl$\Debian", None, Some(flag)).is_err());
        assert!(parse_target(r"\\wsl$\Debian", Some("bash"), Some(flag)).is_err());
    }
}

#[test]
fn cc_apps_wsl_target_resolution_non_windows_is_unsupported() {
    if !cfg!(windows) {
        assert_eq!(
            resolve_target(r"\\wsl$\Debian", None, None).unwrap_err().kind(),
            ErrorKind::Unsupported
        );
    }
}
