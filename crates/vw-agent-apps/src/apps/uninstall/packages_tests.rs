use super::packages::*;
use vw_api_types::agent_apps::*;

fn installation(path: &str, real: &str) -> AppInstallation {
    AppInstallation {
        id: AppInstallationId("selected".into()),
        app_id: AppId::Gemini,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: path.into(),
        real_path: Some(real.into()),
        source: AppInstallationSource::Nvm,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}

#[test]
fn cc_apps_package_uninstall_preview_verified_npm_prefix() {
    let root = tempfile::tempdir().unwrap();
    let prefix = root.path().to_str().unwrap();
    let package = root.path().join("lib/node_modules/@google/gemini-cli");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("package.json"), r#"{"name":"@google/gemini-cli"}"#).unwrap();
    std::fs::write(package.join("index.js"), "").unwrap();
    let inst =
        installation(&format!("{prefix}/bin/gemini"), package.join("index.js").to_str().unwrap());
    assert!(preview(&inst, &[], None).is_err());
    let command = preview(&inst, &[format!("{prefix}/bin/npm")], None).unwrap();
    assert_eq!(command.program, format!("{prefix}/bin/npm"));
    assert_eq!(command.args, ["uninstall", "-g", "--prefix", prefix, "@google/gemini-cli"]);
    let mut unrelated = inst;
    unrelated.real_path = Some(format!("{prefix}/custom/gemini"));
    assert!(preview(&unrelated, &[command.program], None).is_err());
}

#[test]
fn cc_apps_package_uninstall_preview_brew_tap_and_kind() {
    for (app, marker, token, cask) in
        [(AppId::Gemini, "Cellar", "gemini-cli", false), (AppId::Codex, "Caskroom", "codex", true)]
    {
        let mut inst = installation("/brew/bin/app", &format!("/brew/{marker}/{token}/1/bin/app"));
        inst.app_id = app;
        let mut owner = BrewOwnership {
            prefix: "/brew".into(),
            token: token.into(),
            tap: if cask { "homebrew/cask" } else { "homebrew/core" }.into(),
            cask,
        };
        let managers = ["/brew/bin/brew".into()];
        assert!(preview(&inst, &managers, None).is_err());
        let result = preview(&inst, &managers, Some(&owner)).unwrap();
        assert_eq!(result.program, "/brew/bin/brew");
        assert_eq!(
            result.args,
            [
                "uninstall",
                if cask { "--cask" } else { "--formula" },
                &format!("{}/{token}", owner.tap)
            ]
        );
        owner.tap = "other/tap".into();
        assert!(preview(&inst, &managers, Some(&owner)).is_err());
        owner.tap = if cask { "homebrew/cask" } else { "homebrew/core" }.into();
        owner.cask = !cask;
        assert!(preview(&inst, &managers, Some(&owner)).is_err());
    }
}

#[test]
fn cc_apps_package_uninstall_preview_bun_pnpm_roots() {
    for bun in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().to_str().unwrap();
        let (bin, global, tool) = if bun {
            (format!("{base}/bin"), format!("{base}/install/global"), "bun")
        } else {
            (base.into(), format!("{base}/global/5"), "pnpm")
        };
        let package = format!("{global}/node_modules/@google/gemini-cli");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(format!("{package}/package.json"), r#"{"name":"@google/gemini-cli"}"#)
            .unwrap();
        std::fs::write(format!("{package}/index.js"), "").unwrap();
        let mut inst = installation(&format!("{bin}/gemini"), &format!("{package}/index.js"));
        inst.source = if bun { AppInstallationSource::Bun } else { AppInstallationSource::Pnpm };
        let managers = [format!("{bin}/{tool}")];
        let plan = preview(&inst, &managers, None).unwrap();
        assert_eq!(plan.program, managers[0]);
        assert_eq!(plan.args.last().unwrap(), "@google/gemini-cli");
        if bun {
            assert!(plan.env.contains(&("BUN_INSTALL_GLOBAL_DIR".into(), global)));
        } else {
            assert_eq!(plan.args[2..6], ["--global-dir", &global, "--global-bin-dir", &bin]);
        }
        std::fs::write(format!("{package}/package.json"), r#"{"name":"unrelated"}"#).unwrap();
        assert!(preview(&inst, &managers, None).is_err());
    }
}

#[test]
fn cc_apps_package_uninstall_preview_manual_and_environments() {
    for environment in [
        AppInstallationEnvironment::Macos,
        AppInstallationEnvironment::Windows,
        AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() },
    ] {
        let mut inst = installation("/unknown/bin/gemini", "/unknown/bin/gemini");
        inst.environment = environment;
        inst.source = AppInstallationSource::Unknown;
        assert!(preview(&inst, &["/unknown/bin/npm".into()], None).unwrap_err().contains("manual"));
    }
    let mut hermes = installation("/selected/bin/hermes", "/selected/venv/bin/hermes");
    hermes.app_id = AppId::Hermes;
    let plan = preview(&hermes, &[], None).unwrap();
    assert_eq!(plan.program, "/selected/bin/hermes");
    assert_eq!(plan.args, ["uninstall", "--yes"]);
    hermes.path = "/selected/../other/hermes".into();
    assert!(preview(&hermes, &[], None).is_err());
}

#[cfg(unix)]
#[test]
fn cc_apps_package_uninstall_preview_selected_process_only() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let prefix = root.path().join("selected");
    let other = root.path().join("other");
    for dir in [&prefix, &other] {
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        let npm = dir.join("bin/npm");
        std::fs::write(&npm, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.argv\"\n").unwrap();
        std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let package = prefix.join("lib/node_modules/@google/gemini-cli");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(package.join("package.json"), r#"{"name":"@google/gemini-cli"}"#).unwrap();
    std::fs::write(package.join("index.js"), "").unwrap();
    let inst = installation(
        prefix.join("bin/gemini").to_str().unwrap(),
        package.join("index.js").to_str().unwrap(),
    );
    let npm = prefix.join("bin/npm").to_str().unwrap().to_owned();
    assert!(preview(&inst, &[other.join("bin/npm").to_str().unwrap().into()], None).is_err());
    let plan = preview(&inst, std::slice::from_ref(&npm), None).unwrap();
    assert!(
        std::process::Command::new(&plan.program)
            .args(&plan.args)
            .env("PATH", other.join("bin"))
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        std::fs::read_to_string(format!("{npm}.argv")).unwrap(),
        format!("uninstall\n-g\n--prefix\n{}\n@google/gemini-cli\n", prefix.display())
    );
    assert!(!other.join("bin/npm.argv").exists());
    assert!(package.exists());
}

#[test]
fn cc_apps_package_uninstall_preview_windows_wsl_selected_hermes() {
    for (environment, path, real, expected) in [
        (
            AppInstallationEnvironment::Windows,
            r"C:\Tools\hermes.cmd",
            r"C:\Tools\venv\hermes.exe",
            "C:/Tools/hermes.cmd",
        ),
        (
            AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() },
            "/home/user/bin/hermes",
            "/home/user/venv/bin/hermes",
            "/home/user/bin/hermes",
        ),
    ] {
        let mut inst = installation(path, real);
        inst.environment = environment;
        inst.app_id = AppId::Hermes;
        inst.source = AppInstallationSource::Native;
        let plan = preview(&inst, &[], None).unwrap();
        assert_eq!(plan.program, expected);
        assert_eq!(plan.args, ["uninstall", "--yes"]);
        inst.source = AppInstallationSource::Unknown;
        assert!(preview(&inst, &[], None).is_err());
    }
}
