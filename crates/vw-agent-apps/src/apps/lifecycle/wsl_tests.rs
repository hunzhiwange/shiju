use super::wsl;

#[cfg(unix)]
static EXECUTION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
use crate::apps::discovery::wsl::parse_target;
use vw_api_types::agent_apps::*;

fn installation() -> AppInstallation {
    AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::Codex,
        target_host_id: "authorized-host".into(),
        environment: AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() },
        path: "/opt/node/bin/codex".into(),
        real_path: Some("/opt/node/lib/node_modules/@openai/codex/bin/codex.js".into()),
        source: AppInstallationSource::System,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}
fn target(distro: &str) -> crate::apps::discovery::wsl::WslTarget {
    parse_target(&format!(r"\\wsl$\{distro}\home"), Some("fish"), None).unwrap()
}
#[test]
fn cc_apps_wsl_lifecycle_target_uninstall_bound_distro_and_paths() {
    assert!(wsl::uninstall(target("Ubuntu"), &installation()).is_ok());
    assert!(wsl::uninstall(target("Debian"), &installation()).is_err());
    let mut bad = installation();
    bad.path = r"C:\node\codex.cmd".into();
    assert!(wsl::uninstall(target("Ubuntu"), &bad).is_err());
}

#[test]
fn cc_apps_wsl_lifecycle_target_recipes_and_shell_arguments() {
    for app in [
        AppId::Claude,
        AppId::Codex,
        AppId::Gemini,
        AppId::GrokBuild,
        AppId::OpenCode,
        AppId::OpenClaw,
        AppId::Hermes,
        AppId::Pi,
        AppId::Mcode,
        AppId::Kimi,
        AppId::Dsh,
        AppId::Agy,
        AppId::Omp,
    ] {
        let plan = wsl::install(target("Ubuntu"), app).unwrap();
        let command = plan.command(std::path::Path::new("wsl.exe")).unwrap();
        let args: Vec<_> = command.get_args().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(&args[..6], &["-d", "Ubuntu", "--", "fish", "-lc", args[5]]);
        assert!(args[5].starts_with("sh -c "));
        assert!(!args[5].contains("powershell"));
    }
    let upgrade =
        wsl::upgrade(target("Ubuntu"), &installation(), &["/opt/node/bin/npm".into()]).unwrap();
    assert!(
        upgrade
            .command(std::path::Path::new("wsl.exe"))
            .unwrap()
            .get_args()
            .last()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("/opt/node/bin/npm")
    );
    assert!(wsl::upgrade(target("Debian"), &installation(), &[]).is_err());
    let auto = parse_target(r"\\wsl$\Ubuntu", None, None).unwrap();
    assert!(
        wsl::install(auto, AppId::Codex).unwrap().command(std::path::Path::new("wsl.exe")).is_err()
    );
}

#[cfg(unix)]
fn executable(path: &std::path::Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, script).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn cc_apps_wsl_lifecycle_target_process_side_effects_and_revalidation() {
    let _execution = EXECUTION.lock().await;
    use crate::apps::versions::node::NodeRequirement;
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap().join("node '中文");
    let bin = root.join("bin");
    let package = root.join("lib/node_modules/@openai/codex/bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&package).unwrap();
    let real = package.join("codex.js");
    std::fs::write(&real, "fixture").unwrap();
    let entry = bin.join("codex");
    std::os::unix::fs::symlink(&real, &entry).unwrap();
    let survivor = bin.join("other-distro");
    std::fs::write(&survivor, "keep").unwrap();
    executable(
        &bin.join("npm"),
        "#!/bin/sh\n[ \"$1\" = uninstall ] && [ \"$2\" = -g ] && [ \"$3\" = --prefix ] && [ \"$5\" = @openai/codex ] || exit 19\nrm -- \"$4/bin/codex\"\n",
    );
    let wrapper = temp.path().join("wsl.exe");
    executable(
        &wrapper,
        "#!/bin/sh\n[ \"$1\" = -d ] && [ \"$2\" = Ubuntu ] && [ \"$3\" = -- ] && [ \"$4\" = fish ] && [ \"$5\" = -lc ] || exit 23\nexec /bin/sh -c \"$6\"\n",
    );
    let mut inst = installation();
    inst.path = entry.to_str().unwrap().into();
    inst.real_path = Some(real.to_str().unwrap().into());
    let preview = wsl::uninstall(target("Ubuntu"), &inst).unwrap();
    let fresh = wsl::uninstall(
        target("Debian"),
        &AppInstallation {
            environment: AppInstallationEnvironment::Wsl { distribution: "Debian".into() },
            ..inst.clone()
        },
    )
    .unwrap();
    assert!(
        wsl::execute_on(
            preview,
            true,
            move || Ok(fresh),
            || Ok(NodeRequirement::NotApplicable),
            || Ok(()),
            wrapper.clone()
        )
        .await
        .is_err()
    );
    assert!(entry.exists());
    let preview = wsl::uninstall(target("Ubuntu"), &inst).unwrap();
    let fresh = wsl::uninstall(target("Ubuntu"), &inst).unwrap();
    wsl::execute_on(
        preview,
        true,
        move || Ok(fresh),
        || Ok(NodeRequirement::NotApplicable),
        || Ok(()),
        wrapper.clone(),
    )
    .await
    .unwrap();
    assert!(std::fs::symlink_metadata(&entry).is_err());
    assert!(survivor.exists());
    // A nonzero OS boundary never falls back to a host installer or succeeds.
    executable(&wrapper, "#!/bin/sh\nexit 17\n");
    let preview = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
    let fresh = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
    let error = wsl::execute_on(
        preview,
        true,
        move || Ok(fresh),
        || Ok(NodeRequirement::NotApplicable),
        || panic!("postcondition after failed process"),
        wrapper,
    )
    .await
    .unwrap_err();
    assert!(error.contains("17"));
}

#[cfg(unix)]
#[tokio::test]
async fn cc_apps_wsl_lifecycle_target_install_upgrade_and_fail_closed() {
    let _execution = EXECUTION.lock().await;
    use crate::apps::versions::node::NodeRequirement;
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let marker = root.join("argv");
    let manager = root.join("npm");
    executable(&manager, &format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n", marker.display()));
    let wrapper = root.join("wsl.exe");
    executable(
        &wrapper,
        &format!("#!/bin/sh\nexport PATH='{}':\"$PATH\"\nexec /bin/sh -c \"$6\"\n", root.display()),
    );
    let preview = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
    let fresh = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
    wsl::execute_on(
        preview,
        true,
        move || Ok(fresh),
        || Ok(NodeRequirement::NotApplicable),
        || Ok(()),
        wrapper.clone(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "i\n-g\n@openai/codex@latest\n");
    std::fs::remove_file(&marker).unwrap();
    let mut inst = installation();
    inst.path = root.join("codex").to_str().unwrap().into();
    inst.real_path =
        Some(root.join("lib/node_modules/@openai/codex/bin/codex.js").to_str().unwrap().into());
    let managers = vec![manager.to_str().unwrap().into()];
    let preview = wsl::upgrade(target("Ubuntu"), &inst, &managers).unwrap();
    let fresh = wsl::upgrade(target("Ubuntu"), &inst, &managers).unwrap();
    wsl::execute_on(
        preview,
        true,
        move || Ok(fresh),
        || Ok(NodeRequirement::Satisfied { required: "20".into(), current: "22".into() }),
        || Ok(()),
        wrapper.clone(),
    )
    .await
    .unwrap();
    assert!(marker.exists());
    std::fs::remove_file(&marker).unwrap();
    for case in 0..4 {
        let preview = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
        let mut changed = target("Ubuntu");
        if case == 1 {
            changed = parse_target(r"\\wsl$\Ubuntu", Some("bash"), None).unwrap();
        }
        let fresh = wsl::install(changed, AppId::Codex).unwrap();
        let result = wsl::execute_on(
            preview,
            case != 0,
            move || Ok(fresh),
            move || {
                if case == 2 {
                    Ok(NodeRequirement::Gap { required: "24".into(), current: "22".into() })
                } else {
                    Ok(NodeRequirement::NotApplicable)
                }
            },
            move || if case == 3 { Err("postcondition failed".into()) } else { Ok(()) },
            wrapper.clone(),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(marker.exists(), case == 3);
    }
    if !cfg!(windows) {
        let preview = wsl::install(target("Ubuntu"), AppId::Codex).unwrap();
        assert!(
            wsl::execute(
                preview,
                true,
                || panic!("host fallback"),
                || panic!("host gate"),
                || panic!("host probe")
            )
            .await
            .is_err()
        );
    }
}

#[cfg(unix)]
#[test]
fn cc_apps_wsl_lifecycle_target_uninstall_rejects_residual_and_dangling_entries() {
    let temp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(temp.path()).unwrap();
    let real = root.join("hermes-real");
    let entry = root.join("hermes");
    std::os::unix::fs::symlink(&real, &entry).unwrap();
    let wrapper = root.join("wsl.exe");
    executable(&wrapper, "#!/bin/sh\nexec /bin/sh -c \"$6\"\n");
    let mut inst = installation();
    inst.app_id = AppId::Hermes;
    inst.path = entry.to_str().unwrap().into();
    inst.real_path = Some(real.to_str().unwrap().into());
    for script in
        ["#!/bin/sh\nexit 0\n".to_owned(), format!("#!/bin/sh\nrm -- '{}'\n", real.display())]
    {
        executable(&real, &script);
        let plan = wsl::uninstall(target("Ubuntu"), &inst).unwrap();
        assert!(super::process::run_command(plan.command(&wrapper).unwrap()).is_err());
        assert!(std::fs::symlink_metadata(&entry).is_ok());
    }
}
