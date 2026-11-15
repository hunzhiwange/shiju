use super::*;

#[test]
fn installer_exit_codes_require_completed_installation() {
    assert!(installation_result("windows", Some(0)).is_ok());
    assert!(installation_result("windows", Some(3010)).is_err());
    assert!(installation_result("windows", Some(1641)).is_err());
    assert!(installation_result("windows", Some(1603)).is_err());
    assert!(installation_result("linux", Some(0)).is_ok());
    assert!(installation_result("linux", Some(1)).is_err());
    assert!(installation_result("linux", None).is_err());
}

#[cfg(windows)]
#[test]
fn windows_installer_command_opens_canonical_package_in_unicode_cache() {
    use std::ffi::OsStr;

    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("中文 cache");
    std::fs::create_dir(&cache).unwrap();
    let package = cache.join("ShiJu update.msi");
    std::fs::write(&package, b"verified").unwrap();
    let verified = package.canonicalize().unwrap();
    assert!(matches!(
        verified.components().next(),
        Some(std::path::Component::Prefix(prefix)) if prefix.kind().is_verbatim()
    ));

    let installer = windows_installer_path(&verified).unwrap();
    let log = installer.with_file_name("installer.log");
    let command = windows_installer_command(&msiexec().unwrap(), installer, &log, false);
    assert_eq!(command.get_program(), msiexec().unwrap().as_os_str());
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args[0], OsStr::new("/i"));
    let installer = Path::new(args[1]);
    assert!(!matches!(
        installer.components().next(),
        Some(std::path::Component::Prefix(prefix)) if prefix.kind().is_verbatim()
    ));
    assert_eq!(installer.canonicalize().unwrap(), verified);
    assert_eq!(std::fs::read(installer).unwrap(), b"verified");
    assert_eq!(windows_installer_path(installer).unwrap(), installer);
    assert_eq!(Path::new(args.last().unwrap()), log);
}

#[test]
fn windows_installer_command_enables_restart_manager_and_retains_diagnostics() {
    use std::ffi::OsStr;

    let directory = tempfile::tempdir().unwrap();
    let installer = directory.path().join("中文 cache").join("ShiJu update.msi");
    let log = installer.with_file_name("installer.log");
    let command = windows_installer_command(Path::new("msiexec.exe"), &installer, &log, false);
    let args: Vec<_> = command.get_args().collect();

    assert_eq!(args[0], OsStr::new("/i"));
    assert_eq!(Path::new(args[1]), installer);
    assert!(!args.iter().any(|arg| arg.to_string_lossy().starts_with("MSIRESTARTMANAGERCONTROL=")));
    assert!(args.contains(&OsStr::new("MSIDISABLERMRESTART=1")));
    assert!(args.contains(&OsStr::new("/norestart")));
    assert!(args.contains(&OsStr::new("REBOOT=ReallySuppress")));
    assert_eq!(args[args.len() - 2], OsStr::new("/L*V"));
    assert_eq!(Path::new(args.last().unwrap()), log);
}

#[test]
fn windows_installer_command_refreshes_existing_product_without_skipping_first_install() {
    use std::ffi::OsStr;

    let executable = Path::new("msiexec.exe");
    let installer = Path::new("ShiJu update.msi");
    let log = Path::new("installer.log");
    for installed in [false, true] {
        let command = windows_installer_command(executable, installer, log, installed);
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.contains(&OsStr::new("REINSTALL=ALL")), installed);
        assert_eq!(args.contains(&OsStr::new("REINSTALLMODE=vamus")), installed);
    }
}

#[test]
fn windows_installation_failure_reports_package_log_and_actual_exit_code() {
    let installer = Path::new("ShiJu update.msi");
    let log = Path::new("installer.log");
    let error = windows_installation_result(Some(1603), installer, log).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("1603"));
    assert!(message.contains("安装包：ShiJu update.msi"));
    assert!(message.contains("安装日志：installer.log"));
    assert!(message.contains("具体失败原因"));
    assert!(windows_installation_result(Some(0), installer, log).is_ok());
    assert!(windows_installation_result(Some(3010), installer, log).is_err());
}

#[cfg(windows)]
#[test]
fn windows_installer_path_rejects_paths_that_cannot_be_safely_simplified() {
    let long = format!(r"\\?\C:\{}\update.msi", "folder\\".repeat(50));
    for path in [
        long.as_str(),
        r"\\?\C:\cache.\update.msi",
        r"\\?\C:\cache \update.msi",
        r"\\?\C:\NUL\update.msi",
        r"update.msi",
    ] {
        let error = windows_installer_path(Path::new(path)).unwrap_err();
        assert!(error.to_string().contains("Windows Installer"), "{path}: {error}");
    }
}

#[cfg(unix)]
#[test]
fn updater_wait_is_bound_to_actual_parent_process() {
    assert!(ParentProcess::open(unsafe { libc::getppid() } as u32).is_ok());
    assert!(ParentProcess::open(std::process::id()).is_err());
    assert!(ParentProcess::open(0).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_swap_replaces_entire_bundle_and_can_restore_old_bundle() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("中文 ShiJu.app");
    let staged = root.path().join("new.app");
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(target.join("old-only"), b"old").unwrap();
    std::fs::write(staged.join("new-only"), b"new").unwrap();
    macos::swap_bundles(&staged, &target).unwrap();
    assert_eq!(std::fs::read(target.join("new-only")).unwrap(), b"new");
    assert!(!target.join("old-only").exists());
    assert_eq!(std::fs::read(staged.join("old-only")).unwrap(), b"old");
    macos::swap_bundles(&staged, &target).unwrap();
    assert_eq!(std::fs::read(target.join("old-only")).unwrap(), b"old");
}

#[cfg(target_os = "macos")]
#[test]
fn macos_swap_failure_keeps_installed_bundle() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("ShiJu.app");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("old"), b"old").unwrap();
    assert!(macos::swap_bundles(&root.path().join("missing.app"), &target).is_err());
    assert_eq!(std::fs::read(target.join("old")).unwrap(), b"old");
}

#[cfg(target_os = "macos")]
#[test]
fn macos_dmg_stages_verified_complete_bundle_and_rejects_wrong_version() {
    use std::fs;
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let installed = root_path.join("已安装 ShiJu.app");
    let image_source = root_path.join("image-source");
    fs::create_dir(&image_source).unwrap();
    let packaged = image_source.join("ShiJu.app");
    for app in [&installed, &packaged] {
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        fs::copy("/usr/bin/true", app.join("Contents/MacOS/shi-ju")).unwrap();
        fs::write(
            app.join("Contents/Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>com.rsung.shiju</string>
<key>CFBundleExecutable</key><string>shi-ju</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>1.2.3</string>
</dict></plist>"#,
        )
        .unwrap();
        fs::write(app.join("Contents/Resources/asset"), b"complete bundle resource").unwrap();
        command_output(Command::new("/usr/bin/codesign").args(["--force", "--sign", "-"]).arg(app))
            .unwrap();
    }
    let dmg = root_path.join("verified.dmg");
    command_output(
        Command::new("/usr/bin/hdiutil")
            .args(["create", "-quiet", "-format", "UDZO", "-srcfolder"])
            .arg(&image_source)
            .arg(&dmg),
    )
    .unwrap();
    let executable = installed.join("Contents/MacOS/shi-ju");
    assert!(macos::prepare(&dmg, &executable, "9.9.9").is_err());
    assert!(installed.join("Contents/Resources/asset").is_file());
    let stage = macos::prepare(&dmg, &executable, "1.2.3").unwrap();
    let staged = stage.path().join("ShiJu.app");
    assert_eq!(
        fs::read(staged.join("Contents/Resources/asset")).unwrap(),
        b"complete bundle resource"
    );
    assert_eq!(stage.path().parent(), installed.parent());
    macos::swap_bundles(&staged, &installed).unwrap();
    command_output(
        Command::new("/usr/bin/codesign").args(["--verify", "--deep", "--strict"]).arg(&installed),
    )
    .unwrap();
}
