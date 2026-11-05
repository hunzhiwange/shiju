use super::report::*;
use vw_api_types::agent_apps::*;

fn installation(path: &str, real: &str) -> AppInstallation {
    AppInstallation {
        id: AppInstallationId(path.into()),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: path.into(),
        real_path: Some(real.into()),
        source: AppInstallationSource::Unknown,
        is_path_default: false,
        version: Some("1.0".into()),
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}

#[test]
fn cc_apps_installation_conflicts_same_name_distinct_instances_and_thresholds() {
    let a = installation("/a/claude", "/a/claude");
    let b = installation("/b/claude", "/b/claude");
    let report = build(vec![a.clone(), b.clone()]).unwrap();
    assert_eq!(report.installations.len(), 2);
    assert!(requires_upgrade_confirmation(&report));
    assert!(!is_conflicting(&report));
    let mut different = b.clone();
    different.version = Some("2.0".into());
    assert!(is_conflicting(&build(vec![a.clone(), different]).unwrap()));
    let mut broken = b;
    broken.version = None;
    broken.runtime_status = AppInstallationRuntimeStatus::ExitedWithError { exit_code: 1 };
    assert!(is_conflicting(&build(vec![a, broken]).unwrap()));
}

#[test]
fn cc_apps_installation_conflicts_alias_preserves_default_and_source() {
    let a = installation("/usr/local/bin/claude", "/usr/local/Cellar/claude/1/bin/claude");
    let mut b = a.clone();
    b.path = "/custom/claude".into();
    b.id = AppInstallationId("default".into());
    b.is_path_default = true;
    let report = build(vec![a, b]).unwrap();
    assert_eq!(report.installations.len(), 1);
    assert_eq!(report.installations[0].path, "/custom/claude");
    assert_eq!(report.installations[0].id.0, "default");
    assert_eq!(report.installations[0].source, AppInstallationSource::Homebrew);
    assert!(!requires_upgrade_confirmation(&report));
    assert!(!is_conflicting(&report));
}

#[test]
fn cc_apps_installation_conflicts_rejects_mixed_scope_and_relative_paths() {
    let a = installation("/a/claude", "/a/claude");
    let mut b = a.clone();
    b.target_host_id = "other".into();
    assert!(build(vec![a.clone(), b]).is_err());
    let mut b = a.clone();
    b.path = "claude".into();
    assert!(build(vec![b]).is_err());
    let mut b = a.clone();
    b.real_path = Some("relative".into());
    assert!(build(vec![b]).is_err());
    let mut b = a.clone();
    b.environment = AppInstallationEnvironment::Linux;
    assert!(build(vec![a, b]).is_err());
    assert!(!is_conflicting(&build(vec![]).unwrap()));
}

#[test]
fn cc_apps_installation_conflicts_windows_normalization_and_unknown_versions() {
    let mut a = installation(r"C:\Volta\bin\claude.cmd", r"C:\Volta\bin\claude.cmd");
    a.environment = AppInstallationEnvironment::Windows;
    a.version = None;
    let mut b = a.clone();
    b.path = "c:/volta/bin/CLAUDE.cmd".into();
    b.real_path = Some(b.path.clone());
    b.is_path_default = true;
    let report = build(vec![a.clone(), b]).unwrap();
    assert_eq!(report.installations.len(), 1);
    assert!(report.installations[0].is_path_default);
    assert_eq!(report.installations[0].source, AppInstallationSource::Volta);
    let mut b = a.clone();
    b.path = r"C:\other\claude.exe".into();
    b.real_path = None;
    assert!(!is_conflicting(&build(vec![a.clone(), b.clone()]).unwrap()));
    b.version = Some("1".into());
    assert!(is_conflicting(&build(vec![a, b]).unwrap()));
}

#[test]
fn cc_apps_installation_conflicts_runtime_only_and_failed_probe_thresholds() {
    let a = installation("/a/claude", "/a/claude");
    let mut b = installation("/b/claude", "/b/claude");
    b.runtime_status =
        AppInstallationRuntimeStatus::ProbeFailed { reason: AppInstallationProbeFailure::TimedOut };
    assert!(is_conflicting(&build(vec![a.clone(), b.clone()]).unwrap()));
    let mut a = a;
    a.runtime_status = AppInstallationRuntimeStatus::ExitedWithError { exit_code: 1 };
    assert!(!is_conflicting(&build(vec![a, b]).unwrap()));
}

#[cfg(unix)]
#[test]
fn cc_apps_installation_conflicts_real_symlinks_are_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("claude");
    let alias = temp.path().join("alias");
    std::fs::write(&real, b"installation fixture").unwrap();
    std::os::unix::fs::symlink(&real, &alias).unwrap();
    let canonical = std::fs::canonicalize(&real).unwrap();
    let a = installation(real.to_str().unwrap(), canonical.to_str().unwrap());
    let mut b = installation(
        alias.to_str().unwrap(),
        std::fs::canonicalize(&alias).unwrap().to_str().unwrap(),
    );
    b.is_path_default = true;
    let report = build(vec![a, b]).unwrap();
    assert_eq!(report.installations.len(), 1);
    assert_eq!(report.installations[0].path, alias.to_str().unwrap());
    assert_eq!(std::fs::read(&real).unwrap(), b"installation fixture");
    assert_eq!(std::fs::read_link(&alias).unwrap(), real);
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 2);
}
