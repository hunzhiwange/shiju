static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
use super::execute::*;
use vw_api_types::agent_apps::*;

fn fixture(script: &str) -> (tempfile::TempDir, AppInstallation, Vec<String>) {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let package = root.join("lib/node_modules/@google/gemini-cli");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::write(package.join("package.json"), r#"{"name":"@google/gemini-cli"}"#).unwrap();
    let real = package.join("index.js");
    std::fs::write(&real, "selected").unwrap();
    let entry = root.join("bin/gemini");
    symlink(&real, &entry).unwrap();
    let manager = root.join("bin/npm");
    std::fs::write(&manager, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&manager, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(root.join("other"), "retain").unwrap();
    let inst = AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::Gemini,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: entry.to_str().unwrap().into(),
        real_path: Some(real.to_str().unwrap().into()),
        source: AppInstallationSource::Nvm,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    (temp, inst, vec![manager.to_str().unwrap().into()])
}

#[tokio::test]
async fn cc_apps_uninstall_revalidates_plan_success_and_postconditions() {
    let _guard = TEST_LOCK.lock().await;
    for (script, success) in [
        ("printf '%s\\n' \"$@\" > \"$0.argv\"; rm \"${0%/*}/gemini\"", true),
        ("exit 0", false),
        ("rm \"${0%/*}/../lib/node_modules/@google/gemini-cli/index.js\"", false),
        ("exit 17", false),
    ] {
        let (temp, inst, managers) = fixture(script);
        let plan = preview(&inst, &managers, None).unwrap();
        let result = uninstall(plan, true, move || Ok((inst, managers, None))).await;
        assert_eq!(result.is_ok(), success, "{result:?}");
        let root = std::fs::canonicalize(temp.path()).unwrap();
        assert_eq!(std::fs::read_to_string(root.join("other")).unwrap(), "retain");
        if success {
            assert_eq!(
                std::fs::read_to_string(root.join("bin/npm.argv")).unwrap(),
                format!("uninstall\n-g\n--prefix\n{}\n@google/gemini-cli\n", root.display())
            );
        }
    }
}

#[tokio::test]
async fn cc_apps_uninstall_revalidates_plan_rejects_changes_before_delete() {
    let _guard = TEST_LOCK.lock().await;
    for change in 0..7 {
        let (temp, inst, managers) = fixture("touch \"$0.started\"; rm \"${0%/*}/gemini\"");
        let plan = preview(&inst, &managers, None).unwrap();
        let mut fresh = inst.clone();
        match change {
            0 => fresh.source = AppInstallationSource::Unknown,
            1 => fresh.real_path = Some("/different/target".into()),
            2 => fresh.environment = AppInstallationEnvironment::Windows,
            3 => fresh.target_host_id = "other-host".into(),
            4 => fresh.id = AppInstallationId("other".into()),
            5 => fresh.path = "/tampered/entry".into(),
            _ => {}
        }
        assert!(uninstall(plan, change != 6, move || Ok((fresh, managers, None))).await.is_err());
        assert!(std::fs::symlink_metadata(&inst.path).is_ok());
        assert!(!temp.path().join("bin/npm.started").exists());
    }
}

#[tokio::test]
async fn cc_apps_uninstall_revalidates_plan_native_and_stale_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let path = root.join(".local/bin/claude");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let binary = [0xcf, 0xfa, 0xed, 0xfe, 0, 0, 0, 0];
    std::fs::write(&path, binary).unwrap();
    let inst = AppInstallation {
        id: AppInstallationId("native".into()),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: path.to_str().unwrap().into(),
        real_path: Some(path.to_str().unwrap().into()),
        source: AppInstallationSource::System,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    let plan = preview(&inst, &[], None).unwrap();
    std::fs::write(&path, [binary.as_slice(), &[1]].concat()).unwrap();
    let fresh = inst.clone();
    assert!(uninstall(plan, true, move || Ok((fresh, vec![], None))).await.is_err());
    assert!(path.exists());
    let plan = preview(&inst, &[], None).unwrap();
    uninstall(plan, true, move || Ok((inst, vec![], None))).await.unwrap();
    assert!(std::fs::symlink_metadata(path).is_err());
}
