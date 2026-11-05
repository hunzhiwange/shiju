use super::update;
use vw_api_types::agent_apps::*;

fn installation(
    id: &str,
    path: &str,
    real: &str,
    source: AppInstallationSource,
) -> AppInstallation {
    AppInstallation {
        id: AppInstallationId(id.into()),
        app_id: AppId::Gemini,
        target_host_id: "host".into(),
        environment: AppInstallationEnvironment::Macos,
        path: path.into(),
        real_path: Some(real.into()),
        source,
        is_path_default: true,
        version: Some("1.0.0".into()),
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    }
}

#[test]
fn cc_apps_update_anchors_installation_selected_npm() {
    let inst = installation(
        "one",
        "/node/bin/gemini",
        "/node/lib/node_modules/@google/gemini-cli/index.js",
        AppInstallationSource::Nvm,
    );
    let report = AppInstallationReport { installations: vec![inst.clone()] };
    let plan = update::plan(&report, &inst.id, &["/node/bin/npm".into()]).unwrap();
    assert_eq!(plan.command.program, "/node/bin/npm");
    assert_eq!(plan.command.args, ["i", "-g", "@google/gemini-cli@latest"]);
    assert_eq!(plan.command.path_prefix, "/node/bin");
}

#[test]
fn cc_apps_update_anchors_installation_unknown_rejected() {
    let inst =
        installation("one", "/custom/gemini", "/custom/gemini", AppInstallationSource::Unknown);
    let report = AppInstallationReport { installations: vec![inst.clone()] };
    assert_eq!(update::plan(&report, &inst.id, &[]).unwrap_err(), "unmanaged installation");
}

#[test]
fn cc_apps_update_anchors_installation_path_change_and_confirmation() {
    let first = installation(
        "one",
        "/node/bin/gemini",
        "/node/lib/node_modules/@google/gemini-cli/index.js",
        AppInstallationSource::Nvm,
    );
    let mut second = first.clone();
    second.id.0 = "two".into();
    second.path = "/other/bin/gemini".into();
    let mut report = AppInstallationReport { installations: vec![first.clone(), second] };
    let companions = vec!["/node/bin/npm".into(), "/other/bin/npm".into()];
    let plan = update::plan(&report, &first.id, &companions).unwrap();
    assert!(plan.needs_confirmation);
    assert!(plan.revalidate(&report, &companions, false).is_err());
    report.installations[0].is_path_default = false;
    report.installations[1].is_path_default = true;
    report.installations.reverse();
    assert_eq!(plan.revalidate(&report, &companions, true), Ok(()));
    assert_eq!(plan.command.program, "/node/bin/npm");
    report.installations[1].real_path = Some("/replacement/gemini".into());
    assert!(plan.revalidate(&report, &companions, true).is_err());
    report.installations.pop();
    assert!(plan.revalidate(&report, &companions, true).is_err());
}

#[test]
fn cc_apps_update_anchors_installation_brew_and_npm_channels() {
    for (real, expected) in [
        ("/opt/homebrew/Cellar/gemini-cli/1/bin/gemini", vec!["upgrade", "gemini-cli"]),
        ("/opt/homebrew/Caskroom/gemini-cli/1/gemini", vec!["upgrade", "--cask", "gemini-cli"]),
    ] {
        let inst =
            installation("one", "/opt/homebrew/bin/gemini", real, AppInstallationSource::Homebrew);
        let report = AppInstallationReport { installations: vec![inst.clone()] };
        assert!(update::plan(&report, &inst.id, &["/different/bin/brew".into()]).is_err());
        let plan = update::plan(&report, &inst.id, &["/opt/homebrew/bin/brew".into()]).unwrap();
        assert_eq!(plan.command.program, "/opt/homebrew/bin/brew");
        assert_eq!(plan.command.args, expected);
    }
    let inst = installation(
        "one",
        "/opt/homebrew/bin/gemini",
        "/opt/homebrew/lib/node_modules/@google/gemini-cli/index.js",
        AppInstallationSource::Homebrew,
    );
    let report = AppInstallationReport { installations: vec![inst.clone()] };
    assert_eq!(
        update::plan(&report, &inst.id, &["/opt/homebrew/bin/npm".into()]).unwrap().command.program,
        "/opt/homebrew/bin/npm"
    );
}

#[test]
fn cc_apps_update_anchors_installation_windows_wsl_and_managers() {
    for (source, name, args) in [
        (AppInstallationSource::Volta, "volta", vec!["install", "@google/gemini-cli@latest"]),
        (AppInstallationSource::Bun, "bun", vec!["add", "-g", "@google/gemini-cli@latest"]),
        (AppInstallationSource::Pnpm, "pnpm", vec!["add", "-g", "@google/gemini-cli@latest"]),
    ] {
        let mut inst = installation(
            "one",
            "C:/User Space/bin/gemini.cmd",
            "C:/User Space/node_modules/@google/gemini-cli/index.js",
            source,
        );
        inst.environment = AppInstallationEnvironment::Windows;
        let report = AppInstallationReport { installations: vec![inst.clone()] };
        let program = format!("C:/User Space/bin/{name}.cmd");
        let plan = update::plan(&report, &inst.id, std::slice::from_ref(&program)).unwrap();
        assert_eq!(plan.command.program, program);
        assert_eq!(plan.command.args, args);
        assert!(update::plan(&report, &inst.id, &[]).is_err());
    }
    let mut inst = installation(
        "one",
        "/home/u/node/bin/gemini",
        "/home/u/node/lib/node_modules/@google/gemini-cli/index.js",
        AppInstallationSource::Nvm,
    );
    inst.environment = AppInstallationEnvironment::Wsl { distribution: "Ubuntu".into() };
    let report = AppInstallationReport { installations: vec![inst.clone()] };
    assert_eq!(
        update::plan(&report, &inst.id, &["/home/u/node/bin/npm".into()]).unwrap().command.program,
        "/home/u/node/bin/npm"
    );
}

#[test]
fn cc_apps_update_anchors_installation_native_and_codex_installer() {
    for (app, real, args) in [
        (AppId::Claude, "/home/u/.local/share/claude/versions/1", vec!["update"]),
        (AppId::GrokBuild, "/home/u/.grok/downloads/grok-arm64", vec!["update"]),
        (AppId::OpenCode, "/home/u/.opencode/bin/opencode", vec!["upgrade"]),
        (AppId::Hermes, "/home/u/hermes", vec!["update"]),
        (AppId::Agy, "/opt/homebrew/Caskroom/agy/1/agy", vec!["update"]),
    ] {
        let mut inst = installation("one", "/home/u/bin/tool", real, AppInstallationSource::Native);
        inst.app_id = app;
        let report = AppInstallationReport { installations: vec![inst.clone()] };
        let plan = update::plan(&report, &inst.id, &[]).unwrap();
        assert_eq!(plan.command.program, inst.path);
        assert_eq!(plan.command.args, args);
        if app == AppId::GrokBuild {
            assert!(
                plan.command.env.contains(&("npm_config_ignore_scripts".into(), "false".into()))
            );
        }
    }
    let mut inst = installation(
        "one",
        "/custom bin/codex",
        "/custom home/packages/standalone/releases/1/codex",
        AppInstallationSource::Native,
    );
    inst.app_id = AppId::Codex;
    let report = AppInstallationReport { installations: vec![inst.clone()] };
    let plan = update::plan(&report, &inst.id, &[]).unwrap();
    assert_eq!(plan.command.program, "/bin/bash");
    let script = &plan.command.args[1];
    assert!(script.contains("CODEX_INSTALL_DIR='/custom bin'"));
    assert!(script.contains("CODEX_HOME='/custom home'"));
    assert!(script.contains("&& sh"));
    assert!(!script.contains("npm"));
}

#[test]
fn cc_apps_update_anchors_installation_rejects_stale_scope_and_missing_manager() {
    let inst = installation(
        "one",
        "/node/bin/gemini",
        "/node/lib/node_modules/@google/gemini-cli/index.js",
        AppInstallationSource::Nvm,
    );
    let mut report = AppInstallationReport { installations: vec![inst.clone()] };
    let companions = vec!["/node/bin/npm".into()];
    let plan = update::plan(&report, &inst.id, &companions).unwrap();
    assert!(plan.revalidate(&report, &[], true).is_err());
    report.installations[0].target_host_id = "another-host".into();
    assert!(plan.revalidate(&report, &companions, true).is_err());
    report.installations.push(inst.clone());
    assert!(update::plan(&report, &inst.id, &companions).is_err());
    report.installations = vec![inst];
    report.installations[0].path = "relative/gemini".into();
    assert!(update::plan(&report, &report.installations[0].id, &companions).is_err());
}

#[cfg(unix)]
#[test]
fn cc_apps_update_anchors_installation_executes_only_selected_fixture() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let selected = temp.path().join("selected space");
    let other = temp.path().join("other");
    std::fs::create_dir_all(&selected).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    for dir in [&selected, &other] {
        let manager = dir.join("npm");
        std::fs::write(&manager, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$0.argv\"\n").unwrap();
        std::fs::set_permissions(&manager, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = selected.join("gemini").to_string_lossy().into_owned();
    let mut inst = installation(
        "one",
        &path,
        &format!("{}/node_modules/@google/gemini-cli/index.js", temp.path().display()),
        AppInstallationSource::Nvm,
    );
    let mut second = inst.clone();
    second.id.0 = "two".into();
    second.path = other.join("gemini").to_string_lossy().into_owned();
    inst.is_path_default = false;
    let report = AppInstallationReport { installations: vec![second, inst.clone()] };
    let npm = selected.join("npm").to_string_lossy().into_owned();
    let plan = update::plan(&report, &inst.id, std::slice::from_ref(&npm)).unwrap();
    plan.revalidate(&report, &[npm], true).unwrap();
    let output = std::process::Command::new(&plan.command.program)
        .args(&plan.command.args)
        .env("PATH", &other)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        std::fs::read_to_string(selected.join("npm.argv")).unwrap(),
        "i\n-g\n@google/gemini-cli@latest\n"
    );
    assert!(!other.join("npm.argv").exists());
}

#[test]
fn cc_apps_update_anchors_installation_native_roots_and_rejections() {
    let mut inst = installation(
        "one",
        "/home/u/.kimi/bin/kimi",
        "/home/u/.kimi/bin/kimi",
        AppInstallationSource::Native,
    );
    inst.app_id = AppId::Kimi;
    inst.version = Some("0.42.0".into());
    let mut report = AppInstallationReport { installations: vec![inst.clone()] };
    let legacy = update::plan(&report, &inst.id, &[]).unwrap();
    assert!(legacy.command.args.last().unwrap().contains("KIMI_INSTALL_DIR='/home/u/.kimi'"));
    report.installations[0].version = Some("0.43.0".into());
    assert_eq!(update::plan(&report, &inst.id, &[]).unwrap().command.args, ["upgrade", "--yes"]);
    report.installations[0].app_id = AppId::Omp;
    report.installations[0].real_path = Some("/nix/store/omp/bin/omp".into());
    assert!(update::plan(&report, &inst.id, &[]).is_err());
    report.installations[0].app_id = AppId::Mcode;
    report.installations[0].path = "/home/u/.minimax-code/bin/mcode".into();
    report.installations[0].real_path = Some(report.installations[0].path.clone());
    let mcode = update::plan(&report, &inst.id, &[]).unwrap();
    assert!(
        mcode.command.args.last().unwrap().contains("MCODE_INSTALL_DIR='/home/u/.minimax-code'")
    );
    assert!(!mcode.command.args.last().unwrap().contains("|| npm"));
    report.installations[0].app_id = AppId::Codex;
    report.installations[0].environment = AppInstallationEnvironment::Windows;
    report.installations[0].path = "C:/Custom/bin/codex.exe".into();
    report.installations[0].real_path =
        Some("C:/Custom/home/packages/standalone/releases/1/codex.exe".into());
    let shell = "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe".to_string();
    assert!(update::plan(&report, &inst.id, &[]).is_err());
    let codex = update::plan(&report, &inst.id, std::slice::from_ref(&shell)).unwrap();
    assert_eq!(codex.command.program, shell);
    let script = codex.command.args.last().unwrap();
    assert!(script.contains("CODEX_INSTALL_DIR = 'C:/Custom/bin'"));
    assert!(script.contains("CODEX_HOME = 'C:/Custom/home'"));
}
