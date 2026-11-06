use super::request::*;
use std::fs;
use vw_api_types::agent_apps::*;

fn fixture(root: &std::path::Path) -> (LaunchRequest, AppInstallation) {
    let program = root.join("codex");
    fs::write(&program, "fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let request = LaunchRequest {
        app_id: AppId::Codex,
        installation_id: AppInstallationId("observed".into()),
        cwd: root.to_str().unwrap().into(),
        args: vec!["hello world 中文".into()],
        skip_permissions: false,
    };
    let installation = AppInstallation {
        id: request.installation_id.clone(),
        app_id: AppId::Codex,
        target_host_id: "host".into(),
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else if cfg!(windows) {
            AppInstallationEnvironment::Windows
        } else {
            AppInstallationEnvironment::Linux
        },
        path: program.to_str().unwrap().into(),
        real_path: None,
        source: AppInstallationSource::Native,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    (request, installation)
}

#[test]
fn cc_apps_launch_request_validation_accepts_observed_instance_and_argv() {
    let root = tempfile::tempdir().unwrap();
    let (request, installation) = fixture(root.path());
    let material =
        validate_launch_request(&request, "host", &[installation], &[root.path().into()], false)
            .unwrap();
    assert_eq!(material.cwd, root.path().canonicalize().unwrap());
    assert_eq!(material.program, root.path().join("codex").canonicalize().unwrap());
    assert_eq!(material.args, request.args);
    assert!(!material.skip_permissions);
}

#[test]
fn cc_apps_launch_request_validation_rejects_identity_cwd_and_permission_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let (request, installation) = fixture(root.path());
    let check = |r: &LaunchRequest, host: &str, roots: &[std::path::PathBuf]| {
        validate_launch_request(r, host, std::slice::from_ref(&installation), roots, false)
    };
    assert!(check(&request, "other-host", &[root.path().into()]).is_err());
    assert!(check(&request, "host", &[]).is_err());
    for cwd in [
        "relative".to_owned(),
        root.path().join("missing").display().to_string(),
        installation.path.clone(),
        format!("{}\n", request.cwd),
    ] {
        let mut invalid = request.clone();
        invalid.cwd = cwd;
        assert!(check(&invalid, "host", &[root.path().into()]).is_err());
    }
    let mut invalid = request.clone();
    invalid.installation_id.0 = "/bin/sh".into();
    assert!(check(&invalid, "host", &[root.path().into()]).is_err());
    invalid = request.clone();
    invalid.app_id = AppId::Claude;
    assert!(check(&invalid, "host", &[root.path().into()]).is_err());
    invalid = request.clone();
    invalid.skip_permissions = true;
    assert!(check(&invalid, "host", &[root.path().into()]).is_err());
    assert!(
        validate_launch_request(&invalid, "host", &[installation], &[root.path().into()], true)
            .unwrap()
            .skip_permissions
    );
}

#[test]
fn cc_apps_launch_request_validation_rejects_argument_injection() {
    let root = tempfile::tempdir().unwrap();
    let (mut request, installation) = fixture(root.path());
    for arg in [
        "a\nb",
        "a\rb",
        "a\0b",
        "$(touch pwn)",
        "x;sh",
        "a|sh",
        "%COMSPEC%",
        "`sh`",
        "--dangerously-skip-permissions",
        "--yolo",
        "-c",
    ] {
        request.args = vec![arg.into()];
        assert!(
            validate_launch_request(
                &request,
                "host",
                std::slice::from_ref(&installation),
                &[root.path().into()],
                true
            )
            .is_err(),
            "{arg:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn cc_apps_launch_request_validation_rejects_symlink_escape() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let (mut request, installation) = fixture(root.path());
    std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
    request.cwd = root.path().join("escape").display().to_string();
    assert!(
        validate_launch_request(&request, "host", &[installation], &[root.path().into()], false)
            .is_err()
    );
}

#[test]
fn cc_apps_launch_request_validation_rejects_stale_ambiguous_and_foreign_instances() {
    let root = tempfile::tempdir().unwrap();
    let (request, installation) = fixture(root.path());
    let check = |items: &[AppInstallation]| {
        validate_launch_request(&request, "host", items, &[root.path().into()], false)
    };
    assert!(check(&[installation.clone(), installation.clone()]).is_err());
    let mut changed = installation.clone();
    changed.runtime_status = AppInstallationRuntimeStatus::ExitedWithError { exit_code: 1 };
    assert!(check(&[changed]).is_err());
    for environment in [
        AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() },
        if cfg!(windows) {
            AppInstallationEnvironment::Linux
        } else {
            AppInstallationEnvironment::Windows
        },
    ] {
        let mut changed = installation.clone();
        changed.environment = environment;
        assert!(check(&[changed]).is_err());
    }
    let mut changed = installation.clone();
    changed.real_path = Some(root.path().join("old").display().to_string());
    assert!(check(&[changed]).is_err());
    fs::remove_file(&installation.path).unwrap();
    assert!(check(&[installation]).is_err());
}

#[test]
fn cc_apps_launch_request_validation_bounds_arguments_and_rejects_desktop() {
    let root = tempfile::tempdir().unwrap();
    let (mut request, mut installation) = fixture(root.path());
    for args in [vec!["x".into(); 33], vec!["x".repeat(8193)]] {
        request.args = args;
        assert!(
            validate_launch_request(
                &request,
                "host",
                std::slice::from_ref(&installation),
                &[root.path().into()],
                false
            )
            .is_err()
        );
    }
    request.args.clear();
    request.app_id = AppId::ClaudeDesktop;
    installation.app_id = AppId::ClaudeDesktop;
    assert!(
        validate_launch_request(&request, "host", &[installation], &[root.path().into()], false)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn cc_apps_launch_request_validation_rejects_nonexecutable_program() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let (request, installation) = fixture(root.path());
    fs::set_permissions(&installation.path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        validate_launch_request(&request, "host", &[installation], &[root.path().into()], false)
            .is_err()
    );
}
