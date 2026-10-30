use super::*;

#[test]
fn linux_update_uses_system_authorization_and_preserves_literal_package_path() {
    let path = Path::new("/tmp/中文 cache/$(touch injected).deb");
    for (kind, manager) in
        [(InstallerKind::Deb, "/usr/bin/apt-get"), (InstallerKind::Rpm, "/usr/bin/rpm")]
    {
        let command = install_command(kind, path).unwrap();
        assert_eq!(command.get_program(), "/usr/bin/pkexec");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args[0], "--disable-internal-agent");
        assert_eq!(args[1], manager);
        assert_eq!(Path::new(args.last().unwrap()), path);
        assert_eq!(args[args.len() - 2], "--");
        assert!(!args.iter().any(|arg| arg.to_string_lossy().contains("force")));
    }
    assert!(install_command(InstallerKind::Msi, path).is_err());
}

#[test]
fn linux_update_rejects_wrong_owner_version_and_architecture() {
    for kind in [InstallerKind::Deb, InstallerKind::Rpm] {
        let owner = if kind == InstallerKind::Deb { "shiju: /usr/bin/shi-ju" } else { "shiju" };
        let arch = architecture(kind, std::env::consts::ARCH).unwrap();
        let valid = format!("shiju\n1.2.3\n{arch}");
        for (owner, package, success) in [
            (owner, valid.as_str(), true),
            ("other", valid.as_str(), false),
            (owner, "other\n1.2.3\namd64", false),
            (owner, "shiju\n1.2.2\namd64", false),
            (owner, "shiju\n1.2.3\ni386", false),
        ] {
            let mut results = [owner, package].into_iter();
            let result = inspect(
                kind,
                Path::new("/usr/bin/shi-ju"),
                Path::new("/tmp/package"),
                "1.2.3",
                &mut |_| Ok(results.next().unwrap().to_owned()),
            );
            assert_eq!(result.is_ok(), success);
        }
        assert!(
            inspect(
                kind,
                Path::new("/usr/bin/shi-ju"),
                Path::new("/tmp/package"),
                "1.2.3",
                &mut |_| bail!("package database locked")
            )
            .is_err()
        );
    }
}

#[test]
fn linux_update_reports_authorization_cancellation_and_install_failure() {
    assert!(installation_result(Some(0)).is_ok());
    assert!(installation_result(Some(126)).unwrap_err().to_string().contains("已取消"));
    assert!(installation_result(Some(127)).unwrap_err().to_string().contains("授权"));
    for code in [Some(1), Some(100), None] {
        assert!(installation_result(code).is_err());
    }
    assert_eq!(architecture(InstallerKind::Deb, "aarch64").unwrap(), "arm64");
    assert_eq!(architecture(InstallerKind::Deb, "x86_64").unwrap(), "amd64");
    assert_eq!(architecture(InstallerKind::Rpm, "aarch64").unwrap(), "aarch64");
    assert!(architecture(InstallerKind::Deb, "arm").is_err());
}

#[test]
fn linux_update_refuses_unmanaged_executable_before_authorization() {
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("package.deb");
    std::fs::write(&path, b"verified").unwrap();
    #[derive(serde::Deserialize)]
    struct Handoff {
        #[serde(deserialize_with = "crate::app::installer_update::deserialize_handoff")]
        ready: PreparedDesktopUpdate,
    }
    let handoff: Handoff = serde_json::from_value(serde_json::json!({"ready": {
        "path": path, "directory": path.parent().unwrap(), "version": "1.2.3",
        "asset": {"name": "package.deb", "target": "aarch64-unknown-linux-gnu", "kind": "deb",
            "url": "", "size": 8, "sha256": hex::encode(Sha256::digest(b"verified"))}
    }}))
    .unwrap();
    let executable = root.path().join("shi-ju");
    assert!(preflight(&executable, &handoff.ready).unwrap_err().to_string().contains("系统包"));
    assert!(install(&executable, &handoff.ready).is_err());
    assert!(!path.with_file_name("installer.log").exists());
}
