use super::native::*;
use std::path::Path;
use vw_api_types::agent_apps::*;

fn installation(path: &Path) -> AppInstallation {
    AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: path.to_str().unwrap().into(),
        real_path: Some(path.to_str().unwrap().into()),
        source: AppInstallationSource::System,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}
fn binary(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, [0xcf, 0xfa, 0xed, 0xfe, 0, 0, 0, 0]).unwrap();
}
#[test]
fn cc_apps_native_uninstall_scope_removes_only_binary() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let bin = root.join(".local/bin/claude");
    binary(&bin);
    let data = bin.parent().unwrap().join("settings");
    std::fs::write(&data, "retain").unwrap();
    let plan = preview(&installation(&bin)).unwrap();
    plan.remove().unwrap();
    assert!(!bin.exists());
    assert_eq!(std::fs::read_to_string(data).unwrap(), "retain");
}

#[test]
fn cc_apps_native_uninstall_scope_rejects_unverified_and_scripts() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    for (app, relative) in [
        (AppId::Claude, ".local/bin/claude"),
        (AppId::Codex, "packages/standalone/releases/1/bin/codex"),
        (AppId::OpenCode, ".opencode/bin/opencode"),
        (AppId::Agy, ".local/bin/agy"),
        (AppId::Omp, ".local/bin/omp"),
    ] {
        let path = root.join(relative);
        binary(&path);
        let mut inst = installation(&path);
        inst.app_id = app;
        assert!(preview(&inst).is_ok());
        inst.source = AppInstallationSource::Unknown;
        assert!(preview(&inst).is_err());
        inst.source = AppInstallationSource::System;
        std::fs::write(&path, "#!/bin/sh").unwrap();
        assert!(preview(&inst).is_err());
        assert!(path.exists());
    }
    let path = root.join("custom/claude");
    binary(&path);
    assert!(preview(&installation(&path)).is_err());
}

#[test]
fn cc_apps_native_uninstall_scope_symlinks_and_changed_plan() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let path = root.join(".local/bin/claude");
    binary(&path);
    let launcher = root.join("claude");
    symlink(&path, &launcher).unwrap();
    let mut inst = installation(&path);
    inst.path = launcher.to_str().unwrap().into();
    let plan = preview(&inst).unwrap();
    let other = root.join("other");
    binary(&other);
    std::fs::remove_file(&launcher).unwrap();
    symlink(&other, &launcher).unwrap();
    assert!(plan.remove().is_err());
    assert!(path.exists() && other.exists());
    std::fs::remove_file(&launcher).unwrap();
    symlink(&path, &launcher).unwrap();
    preview(&inst).unwrap().remove().unwrap();
    assert!(!launcher.exists() && !path.exists() && other.exists());
    let external = root.join("external");
    std::fs::create_dir_all(&external).unwrap();
    symlink(&external, root.join(".local/linked")).unwrap();
    let through = root.join(".local/linked/.local/bin/claude");
    binary(&through);
    assert!(preview(&installation(&through)).is_err());
    assert!(through.exists());
}

#[test]
fn cc_apps_native_uninstall_scope_grok_retains_data_and_external_links() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let path = root.join(".grok/bin/grok-1.2.3");
    binary(&path);
    let cached = root.join(".grok/downloads/grok-1.2.3-macos-aarch64");
    binary(&cached);
    let launcher = root.join(".grok/bin/agent");
    symlink(&path, &launcher).unwrap();
    let external = root.join("external");
    binary(&external);
    let external_link = root.join(".grok/bin/grok");
    symlink(&external, &external_link).unwrap();
    let data = root.join(".grok/settings");
    std::fs::write(&data, "retain").unwrap();
    let mut inst = installation(&path);
    inst.app_id = AppId::GrokBuild;
    let plan = preview(&inst).unwrap();
    assert_eq!(plan.paths().len(), 3);
    plan.remove().unwrap();
    assert!(!path.exists() && !cached.exists() && !launcher.exists());
    assert!(external_link.exists() && external.exists() && data.exists());
    inst.real_path = Some("/prefix/node_modules/grok/index.js".into());
    assert_eq!(warnings(&inst, true, true).len(), 1);
    inst.app_id = AppId::Hermes;
    assert_eq!(warnings(&inst, false, false).len(), 1);
    assert!(preview(&inst).is_err());
}

#[test]
fn cc_apps_native_uninstall_scope_windows_wsl_rules_and_directory_rejection() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    for (app, rel) in [
        (AppId::Omp, "omp/omp.exe"),
        (AppId::Agy, "agy/bin/agy.exe"),
        (AppId::Claude, ".local/bin/claude.exe"),
    ] {
        let path = root.join(rel);
        binary(&path);
        let mut inst = installation(&path);
        inst.app_id = app;
        inst.environment = AppInstallationEnvironment::Windows;
        assert!(preview(&inst).is_ok());
    }
    let path = root.join(".opencode/bin/opencode");
    binary(&path);
    let mut inst = installation(&path);
    inst.app_id = AppId::OpenCode;
    inst.environment = AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() };
    let plan = preview(&inst).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(plan.remove().is_err());
    assert!(path.is_dir());
}
