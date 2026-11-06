use super::version::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
use vw_api_types::agent_apps::*;
fn fixture(body: &str, executable: bool) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tool");
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }))
        .unwrap();
    (dir, path)
}
#[test]
fn cc_apps_version_probe_broken_install_success_and_nonzero() {
    let (_dir, path) =
        fixture("#!/bin/sh\n[ \"$1\" = --version ] || exit 9\nprintf 'tool 1.2.3\\n'\n", true);
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert_eq!(result.version.as_deref(), Some("1.2.3"));
    assert_eq!(result.runtime_status, AppInstallationRuntimeStatus::Runnable);
    fs::write(&path, "#!/bin/sh\necho 8.9.0 >&2\nexit 7\n").unwrap();
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert!(result.version.is_none());
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ExitedWithError { exit_code: 7 }
    );
}
#[test]
fn cc_apps_version_probe_broken_install_stderr_and_empty_output() {
    for body in ["#!/bin/sh\necho 1.2.3 >&2\n", "#!/bin/sh\necho welcome\n"] {
        let (_dir, path) = fixture(body, true);
        let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
        assert!(result.installed);
        assert!(result.version.is_none());
        assert_eq!(
            result.runtime_status,
            AppInstallationRuntimeStatus::ProbeFailed {
                reason: AppInstallationProbeFailure::InvalidVersionOutput
            }
        );
    }
}
#[test]
fn cc_apps_version_probe_broken_install_deadline_and_output_limit() {
    for body in ["#!/bin/sh\nsleep 10 &\nwait\n", "#!/bin/sh\nwhile :; do echo 1.2.3; done\n"] {
        let (_dir, path) = fixture(body, true);
        let start = Instant::now();
        let result = probe_local_version(&path, Duration::from_millis(80)).unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(result.installed);
        assert!(result.version.is_none());
        assert_ne!(result.runtime_status, AppInstallationRuntimeStatus::Runnable);
    }
}

#[test]
fn cc_apps_version_probe_broken_install_missing_permission_and_damage() {
    let (dir, path) = fixture("#!/bin/sh\necho 1.2.3\n", false);
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::PermissionDenied
        }
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(&path, "#!/nonexistent/shiju-test-interpreter\n").unwrap();
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::DamagedInstallation
        }
    );
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(dir.path().join("absent"), &path).unwrap();
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::DamagedInstallation
        }
    );
    fs::remove_file(&path).unwrap();
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(!result.installed);
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed { reason: AppInstallationProbeFailure::Missing }
    );
    assert!(probe_local_version(std::path::Path::new("tool"), Duration::from_secs(1)).is_err());
    assert!(probe_local_version(&path, Duration::ZERO).is_err());
}

#[test]
fn cc_apps_version_probe_broken_install_bounded_output_and_descendant_cleanup() {
    let (dir, path) = fixture("#!/bin/sh\nwhile :; do echo 1.2.3; done\n", true);
    let result = probe_local_version(&path, Duration::from_secs(2)).unwrap();
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::OutputTooLarge
        }
    );
    let marker = dir.path().join("leaked-child");
    fs::write(&path, format!("#!/bin/sh\n(sleep 0.3; touch '{}') &\nwait\n", marker.display()))
        .unwrap();
    let result = probe_local_version(&path, Duration::from_millis(50)).unwrap();
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed { reason: AppInstallationProbeFailure::TimedOut }
    );
    std::thread::sleep(Duration::from_millis(400));
    assert!(!marker.exists(), "timed out descendant must not continue writing");
    fs::write(&path, "#!/bin/sh\nkill -TERM $$\n").unwrap();
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert!(result.installed);
    assert_eq!(
        result.runtime_status,
        AppInstallationRuntimeStatus::ProbeFailed {
            reason: AppInstallationProbeFailure::DamagedInstallation
        }
    );
}

#[test]
fn cc_apps_omp_managed_version_probe_accepts_official_binary_output() {
    let (_dir, path) = fixture("#!/bin/sh\necho omp/18.17.0\n", true);
    let result = probe_local_version(&path, Duration::from_secs(1)).unwrap();
    assert_eq!(result.version.as_deref(), Some("18.17.0"));
    assert_eq!(result.runtime_status, AppInstallationRuntimeStatus::Runnable);
    for output in ["https://example.test/18.17.0", "other/18.17.0", "omp/invalid"] {
        fs::write(&path, format!("#!/bin/sh\necho '{output}'\n")).unwrap();
        assert!(probe_local_version(&path, Duration::from_secs(1)).unwrap().version.is_none());
    }
}
