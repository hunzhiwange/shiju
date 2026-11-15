use super::preferences::{HostPlatform, default_user_preferences, normalize_preferences};
use vw_api_types::agent_apps::{AppHostPreferences, AppId, AppUserPreferences, AppWslPreference};

#[test]
fn cc_apps_preferences_normalization_defaults_and_order() {
    let defaults = default_user_preferences();
    assert_eq!(defaults.visible_apps.len(), 14);
    let user = AppUserPreferences {
        app_order: vec!["codex".into(), "unknown".into(), "claude".into(), "codex".into()],
        visible_apps: vec![AppId::Codex, AppId::Codex],
    };
    let result = normalize_preferences(
        user,
        AppHostPreferences::default(),
        &AppHostPreferences::default(),
        HostPlatform::MacOs,
    )
    .unwrap();
    assert_eq!(result.user.app_order.len(), 14);
    assert_eq!(&result.user.app_order[..4], &["codex", "gemini", "grokbuild", "opencode"]);
    assert_eq!(&result.user.app_order[12..], &["claude", "claude-desktop"]);
    assert_eq!(result.user.visible_apps, vec![AppId::Codex]);
    assert!(!result.restart_required);
}

#[test]
fn cc_apps_preferences_normalization_rejects_empty_and_wrong_host() {
    let empty = AppUserPreferences { app_order: vec![], visible_apps: vec![] };
    assert!(
        normalize_preferences(
            empty,
            AppHostPreferences::default(),
            &AppHostPreferences::default(),
            HostPlatform::Linux
        )
        .is_err()
    );
    for platform in [HostPlatform::MacOs, HostPlatform::Linux] {
        let host =
            AppHostPreferences { preferred_terminal: Some("cmd".into()), ..Default::default() };
        assert!(
            normalize_preferences(
                default_user_preferences(),
                host,
                &AppHostPreferences::default(),
                platform
            )
            .is_err()
        );
    }
}

#[test]
fn cc_apps_preferences_normalization_terminal_and_wsl_rules() {
    for (platform, terminal) in [
        (HostPlatform::MacOs, "iterm2"),
        (HostPlatform::Windows, "wt"),
        (HostPlatform::Linux, "konsole"),
    ] {
        let host =
            AppHostPreferences { preferred_terminal: Some(terminal.into()), ..Default::default() };
        let result = normalize_preferences(
            default_user_preferences(),
            host.clone(),
            &AppHostPreferences::default(),
            platform,
        )
        .unwrap();
        assert!(result.restart_required);
        assert!(
            !normalize_preferences(default_user_preferences(), host.clone(), &host, platform)
                .unwrap()
                .restart_required
        );
    }
    let mut host = AppHostPreferences {
        wsl_shells: vec![AppWslPreference {
            app_id: AppId::Codex,
            shell: Some("/bin/bash".into()),
            flag: Some("auto".into()),
        }],
        ..Default::default()
    };
    assert!(
        normalize_preferences(
            default_user_preferences(),
            host.clone(),
            &AppHostPreferences::default(),
            HostPlatform::MacOs
        )
        .is_err()
    );
    let result = normalize_preferences(
        default_user_preferences(),
        host.clone(),
        &AppHostPreferences::default(),
        HostPlatform::Windows,
    )
    .unwrap();
    assert_eq!(result.host.wsl_shells[0].shell.as_deref(), Some("bash"));
    assert_eq!(result.host.wsl_shells[0].flag.as_deref(), Some("-lic"));
    for (shell, flag) in [("bash;bad", "-c"), ("bash", "--bad"), ("pwsh", "-c")] {
        host.wsl_shells[0].shell = Some(shell.into());
        host.wsl_shells[0].flag = Some(flag.into());
        assert!(
            normalize_preferences(
                default_user_preferences(),
                host.clone(),
                &AppHostPreferences::default(),
                HostPlatform::Windows
            )
            .is_err()
        );
    }
}

#[test]
fn cc_apps_preferences_normalization_wsl_boundaries_and_idempotence() {
    let previous = AppHostPreferences::default();
    for (shell, flag) in [
        ("auto", None),
        ("sh", Some("-c")),
        ("dash", Some("-c")),
        ("fish", Some("-lc")),
        ("zsh", Some("-lic")),
    ] {
        let host = AppHostPreferences {
            wsl_shells: vec![AppWslPreference {
                app_id: AppId::Claude,
                shell: Some(shell.into()),
                flag: None,
            }],
            ..Default::default()
        };
        let result = normalize_preferences(
            default_user_preferences(),
            host,
            &previous,
            HostPlatform::Windows,
        )
        .unwrap();
        assert_eq!(result.host.wsl_shells[0].flag.as_deref(), flag);
        let second = normalize_preferences(
            result.user.clone(),
            result.host.clone(),
            &result.host,
            HostPlatform::Windows,
        )
        .unwrap();
        assert_eq!(second.user, result.user);
        assert_eq!(second.host, result.host);
        assert!(!second.restart_required);
    }
    let entry = AppWslPreference { app_id: AppId::Claude, shell: None, flag: Some("-lc".into()) };
    let duplicate =
        AppHostPreferences { wsl_shells: vec![entry.clone(), entry], ..Default::default() };
    assert!(
        normalize_preferences(
            default_user_preferences(),
            duplicate,
            &previous,
            HostPlatform::Windows
        )
        .is_err()
    );
    let desktop = AppHostPreferences {
        wsl_shells: vec![AppWslPreference {
            app_id: AppId::ClaudeDesktop,
            shell: None,
            flag: None,
        }],
        ..Default::default()
    };
    assert!(
        normalize_preferences(
            default_user_preferences(),
            desktop,
            &previous,
            HostPlatform::Windows
        )
        .is_err()
    );
    let defaults = normalize_preferences(
        default_user_preferences(),
        previous.clone(),
        &previous,
        HostPlatform::MacOs,
    )
    .unwrap();
    assert_eq!(defaults.user.app_order.len(), 14);
    assert_eq!(defaults.user.app_order[0], "claude");
    assert_eq!(defaults.user.app_order[13], "omp");
}

#[test]
fn cc_apps_preferences_normalization_capabilities_follow_execution_platform() {
    for (platform, terminal, wsl) in [
        (HostPlatform::MacOs, "terminal", false),
        (HostPlatform::Windows, "wt", true),
        (HostPlatform::Linux, "konsole", false),
    ] {
        let result = normalize_preferences(
            default_user_preferences(),
            AppHostPreferences::default(),
            &AppHostPreferences::default(),
            platform,
        )
        .unwrap();
        let capabilities = result.capabilities.unwrap();
        assert!(capabilities.terminals.contains(&terminal.to_string()));
        assert_eq!(capabilities.wsl_preferences, wsl);
    }
}
