use super::process;
use super::process::*;
use std::process::Command;
use vw_api_types::agent_apps::*;

fn report() -> AppInstallationReport {
    AppInstallationReport {
        installations: vec![AppInstallation {
            id: AppInstallationId("fixture".into()),
            app_id: AppId::Gemini,
            target_host_id: "fixture-host".into(),
            environment: AppInstallationEnvironment::Macos,
            path: "/fixture/gemini".into(),
            real_path: Some("/fixture/gemini".into()),
            source: AppInstallationSource::Native,
            is_path_default: true,
            version: Some("1.0.0".into()),
            runtime_status: AppInstallationRuntimeStatus::Runnable,
        }],
    }
}
#[test]
fn cc_apps_lifecycle_process_result_waits_and_checks_exit() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("finished");
    let mut cmd = Command::new("/bin/bash");
    cmd.args([
        "-c",
        "sleep 0.03; touch \"$1\"; printf 'token=secret'; printf 'error text' >&2",
        "fixture",
    ])
    .arg(&marker);
    let result = process::execute(cmd, AppId::Gemini, || {
        assert!(marker.exists());
        Ok(report())
    })
    .unwrap();
    assert_eq!(result.exit_code, Some(0));
    assert!(!result.output.contains("secret"));
    let mut cmd = Command::new("/bin/bash");
    cmd.args(["-c", "echo success; exit 17"]);
    assert!(
        process::execute(cmd, AppId::Gemini, || panic!("failure must not report success"))
            .unwrap_err()
            .contains("17")
    );
    let mut cmd = Command::new("/bin/bash");
    cmd.args(["-c", "printf '%100000s' x"]);
    assert!(process::execute(cmd, AppId::Gemini, || Ok(report())).unwrap().output.contains("8192"));
    let mut cmd = Command::new("/bin/true");
    cmd.arg("");
    assert!(
        process::execute(cmd, AppId::Gemini, || Ok(AppInstallationReport {
            installations: vec![]
        }))
        .is_err()
    );
}
#[tokio::test]
async fn cc_apps_lifecycle_process_result_rejects_node_before_write() {
    let result = install(
        AppId::Dsh,
        super::native_primary::InstallEnvironment::Posix,
        || {
            Ok(crate::apps::versions::node::NodeRequirement::Gap {
                required: ">=24".into(),
                current: "18.0.0".into(),
            })
        },
        || panic!("must not probe a rejected write"),
    )
    .await;
    assert_eq!(result.unwrap_err(), "Node upgrade required");
}
#[test]
fn cc_apps_lifecycle_process_result_windows_and_pipefail() {
    let batch =
        process::windows_batch("C:\\Program Files\\node\\npm.cmd", &["i".into(), "-g".into()])
            .unwrap();
    assert!(batch.contains("chcp 65001"));
    assert!(batch.contains("call \"C:\\Program Files"));
    assert!(batch.ends_with("exit /b %errorlevel%\r\n"));
    assert!(process::windows_batch("npm.cmd", &["%TOKEN%".into()]).is_err());
    let cmd = process::command("bash", &["-c".into(), "false | true".into()], &[], None).unwrap();
    assert!(process::execute(cmd, AppId::Gemini, || panic!("pipe failed")).is_err());
}
#[tokio::test]
async fn cc_apps_lifecycle_process_result_stale_plan_has_no_side_effect() {
    let mut initial = report();
    initial.installations[0].source = AppInstallationSource::Nvm;
    initial.installations[0].path = "/fixture/bin/gemini".into();
    initial.installations[0].real_path = Some("/fixture/lib/node_modules/gemini/cli.js".into());
    let companions = vec!["/fixture/bin/npm".into()];
    let plan = super::update::plan(&initial, &initial.installations[0].id, &companions).unwrap();
    initial.installations[0].version = Some("2.0.0".into());
    let result = upgrade(
        AppId::Gemini,
        plan,
        true,
        move || Ok((initial, companions)),
        || panic!("stale plan must fail before Node or process"),
        || panic!("no postprobe"),
    )
    .await;
    assert_eq!(result.unwrap_err(), "installation changed since preview");
}
#[cfg(unix)]
#[tokio::test]
async fn cc_apps_lifecycle_process_result_upgrade_runs_reviewed_argv() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let npm = dir.path().join("npm");
    let marker = dir.path().join("argv");
    std::fs::write(&npm, format!("#!/bin/bash\nprintf '%s\\n' \"$@\" > '{}'\n", marker.display()))
        .unwrap();
    std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut initial = report();
    initial.installations[0].app_id = AppId::Codex;
    initial.installations[0].source = AppInstallationSource::Nvm;
    initial.installations[0].path = dir.path().join("codex").to_string_lossy().into();
    initial.installations[0].real_path =
        Some(dir.path().join("lib/node_modules/codex/cli.js").to_string_lossy().into());
    let companions = vec![npm.to_string_lossy().into_owned()];
    let plan = super::update::plan(&initial, &initial.installations[0].id, &companions).unwrap();
    let after = initial.clone();
    let result = upgrade(
        AppId::Codex,
        plan,
        true,
        move || Ok((initial, companions)),
        || Ok(crate::apps::versions::node::NodeRequirement::NotApplicable),
        move || {
            assert!(marker.exists());
            assert_eq!(std::fs::read_to_string(marker).unwrap(), "i\n-g\n@openai/codex@latest\n");
            Ok(after)
        },
    )
    .await
    .unwrap();
    assert_eq!(result.exit_code, Some(0));
}
