use super::catalog::read;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, catalog::*},
};
fn row(app: AppId, id: &str) -> AppProviderSummary {
    AppProviderSummary { id: id.into(), app_id: app, name: id.into(), configured: true }
}
#[test]
fn cc04_catalog_visible_rows_are_scoped() {
    let home = tempfile::tempdir().unwrap();
    let result = read(
        home.path(),
        AppId::Claude,
        vec![row(AppId::Codex, "other"), row(AppId::Claude, "ours")],
    );
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].provider.id, "ours");
}
#[test]
fn cc04_catalog_current_distinguishes_direct_and_in_use() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".kimi-code")).unwrap();
    std::fs::write(home.path().join(".kimi-code/config.toml"), "default_model = 'alias'\n[providers.native]\napi_key = 'fake-secret'\n[models.alias]\nprovider = 'native'\n").unwrap();
    let result = read(home.path(), AppId::Kimi, vec![]);
    assert_eq!(result.in_use, ProviderSelection::Selected { id: "native".into() });
    assert_eq!(
        result.direct,
        ProviderSelection::Unavailable { reason: ProviderReadUnavailable::NotImplemented }
    );
    assert_eq!(result.rows[0].native_member, ProviderMembership::Present);
}
#[test]
fn cc04_catalog_reads_hide_nested_secrets() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
    std::fs::write(
        home.path().join(".config/opencode/opencode.json"),
        r#"{"provider":{"native":{"name":"fake-secret","options":{"apiKey":"fake-secret"}}}}"#,
    )
    .unwrap();
    let result = read(home.path(), AppId::OpenCode, vec![]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.in_use, ProviderSelection::NotApplicable);
    let output = format!("{result:?} {}", serde_json::to_string(&result).unwrap());
    assert!(!output.contains("fake-secret"));
}
#[test]
fn cc04_catalog_missing_invalid_and_unsupported_are_distinct() {
    let home = tempfile::tempdir().unwrap();
    let result = read(home.path(), AppId::Kimi, vec![]);
    assert_eq!(result.in_use, ProviderSelection::Unconfigured);
    std::fs::create_dir(home.path().join(".kimi-code")).unwrap();
    let config = home.path().join(".kimi-code/config.toml");
    for invalid in ["not toml", "providers = 'secret'", "default_model = 'missing'"] {
        std::fs::write(&config, invalid).unwrap();
        let result = read(home.path(), AppId::Kimi, vec![]);
        assert_eq!(result.native_read, Some(ProviderReadUnavailable::InvalidConfiguration));
        assert_ne!(result.in_use, ProviderSelection::Unconfigured);
    }
    let result = read(home.path(), AppId::Codex, vec![row(AppId::Codex, "saved")]);
    assert_eq!(result.native_read, Some(ProviderReadUnavailable::NotImplemented));
    assert_eq!(result.rows[0].native_member, ProviderMembership::Unknown);
}
#[test]
fn cc04_catalog_jsonc_precedence_and_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let directory = home.path().join(".config/opencode");
    std::fs::create_dir_all(&directory).unwrap();
    let json = directory.join("opencode.json");
    std::fs::write(&json, r#"{"provider":{"native":{}}}"#).unwrap();
    std::fs::write(directory.join("opencode.jsonc"), "// secret").unwrap();
    let result = read(home.path(), AppId::OpenCode, vec![]);
    assert_eq!(result.native_read, Some(ProviderReadUnavailable::InvalidConfiguration));
    assert!(result.rows.is_empty());
    std::fs::remove_file(directory.join("opencode.jsonc")).unwrap();
    std::fs::remove_file(&json).unwrap();
    symlink("/etc/passwd", &json).unwrap();
    assert_eq!(
        read(home.path(), AppId::OpenCode, vec![]).native_read,
        Some(ProviderReadUnavailable::Unreadable)
    );
    std::fs::remove_file(&json).unwrap();
    std::fs::remove_dir(&directory).unwrap();
    symlink(home.path(), &directory).unwrap();
    assert_eq!(
        read(home.path(), AppId::OpenCode, vec![]).native_read,
        Some(ProviderReadUnavailable::Unreadable)
    );
}

#[test]
fn cc04_catalog_additive_apps_never_invent_a_unique_current() {
    let home = tempfile::tempdir().unwrap();
    for app in [AppId::Hermes, AppId::Pi, AppId::Mcode, AppId::Dsh, AppId::Omp] {
        let result = read(home.path(), app, vec![row(app, "saved")]);
        assert_eq!(result.direct, ProviderSelection::NotApplicable);
        assert_eq!(result.in_use, ProviderSelection::NotApplicable);
        if matches!(app, AppId::Hermes | AppId::Pi) {
            assert_eq!(result.rows[0].native_member, ProviderMembership::Absent);
            assert_eq!(result.native_read, None);
        } else {
            assert_eq!(result.rows[0].native_member, ProviderMembership::Unknown);
            assert_eq!(result.native_read, Some(ProviderReadUnavailable::NotImplemented));
        }
    }
}

#[test]
fn cc04_catalog_custom_home_is_unavailable_without_fallback() {
    if let Some(home) = std::env::var_os("SHIJU_CC04_OVERRIDE_CHILD") {
        let result = read(std::path::Path::new(&home), AppId::Kimi, vec![]);
        assert_eq!(result.native_read, Some(ProviderReadUnavailable::CustomPathNotSupported));
        assert!(result.rows.is_empty());
        return;
    }
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir(home.path().join(".kimi-code")).unwrap();
    let config = home.path().join(".kimi-code/config.toml");
    std::fs::write(&config, "[providers.should_not_read]\napi_key = 'fake-secret'\n").unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "providers::catalog_tests::cc04_catalog_custom_home_is_unavailable_without_fallback",
            "--nocapture",
        ])
        .env("SHIJU_CC04_OVERRIDE_CHILD", home.path())
        .env("KIMI_CODE_HOME", home.path().join("override"))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(std::fs::read_to_string(config).unwrap().contains("should_not_read"));
}
