use super::import::*;
use vw_api_types::{agent_apps::AppId, app_providers::import::ImportTrigger::*};
#[test]
fn cc04_import_existing_catalog_is_unchanged() {
    assert_eq!(strategy(AppId::Claude, Startup, true), Ok(ImportStrategy::Skip));
    assert_eq!(strategy(AppId::Claude, Manual, true), Ok(ImportStrategy::Skip));
    assert_eq!(strategy(AppId::Claude, Startup, false), Ok(ImportStrategy::Default));
}
#[test]
fn cc04_import_uses_app_specific_support() {
    for app in [
        AppId::Pi,
        AppId::Omp,
        AppId::OpenCode,
        AppId::OpenClaw,
        AppId::Hermes,
        AppId::Mcode,
        AppId::Kimi,
    ] {
        assert_eq!(strategy(app, Manual, true), Ok(ImportStrategy::Incremental));
        assert_eq!(strategy(app, Startup, false), Ok(ImportStrategy::Skip));
    }
    assert!(strategy(AppId::ClaudeDesktop, Manual, false).is_err());
    assert!(strategy(AppId::Dsh, Manual, false).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn cc04_import_decodes_native_auth_without_copying_login_or_writing_files() {
    use super::{agy, codex_catalog, gemini, grok};
    use vw_api_types::app_providers::{gemini::GeminiConfig, grok::GrokConfig};
    let p = codex_catalog::import(
        Some(b"model='fixture'\n"),
        Some(br#"{"tokens":{"access_token":"fake-login"}}"#),
    )
    .unwrap();
    assert!(p.api_key.is_none());
    assert!(p.preserve_login);
    assert!(!serde_json::to_string(&p).unwrap().contains("fake-login"));
    assert!(codex_catalog::import(Some(b"cli_auth_credentials_store='keyring'"), None).is_err());
    let p =
        gemini::import(b"GEMINI_API_KEY='fake#literal' # comment\nGEMINI_MODEL=fixture\n", None)
            .unwrap();
    assert!(matches!(p.config,GeminiConfig::ThirdParty{api_key,..} if api_key=="fake#literal"));
    assert_eq!(p.model.as_deref(), Some("fixture"));
    assert!(gemini::import(b"GEMINI_API_KEY=PROXY_MANAGED\n", None).is_err());
    assert!(gemini::import(b"GEMINI_API_KEY=$AMBIENT_SECRET\n", None).is_err());
    assert!(grok::import(b"[models]\ndefault='missing'\n", false).is_err());
    assert!(grok::import(b"# native official\n", true).unwrap().is_none());
    assert!(matches!(
        grok::import(b"# native official\n", false).unwrap().unwrap().config,
        GrokConfig::Official {}
    ));
    assert!(agy::import(br#"{"model":null}"#).unwrap().is_none());
    let p =
        agy::import(br#"{"model":"Display Name","apiKey":"ignored-native-key"}"#).unwrap().unwrap();
    assert!(p.api_key.is_none());
    assert!(p.base_url.is_none());
    assert_eq!(p.model.as_deref(), Some("Display Name"));
}
#[cfg(target_os = "macos")]
#[test]
fn cc04_import_native_reads_reject_symlinks_and_proxy_placeholders() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    std::fs::write(external.path().join(".env"), b"GEMINI_API_KEY=fake-key\n").unwrap();
    std::os::unix::fs::symlink(external.path(), root.path().join(".gemini")).unwrap();
    assert!(read(root.path(), AppId::Gemini, Manual).is_err());
    std::fs::remove_file(root.path().join(".gemini")).unwrap();
    std::fs::create_dir(root.path().join(".claude")).unwrap();
    std::fs::write(
        root.path().join(".claude/settings.json"),
        br#"{"env":{"ANTHROPIC_AUTH_TOKEN":"PROXY_MANAGED"}}"#,
    )
    .unwrap();
    assert!(read(root.path(), AppId::Claude, Manual).is_err());
    assert_eq!(std::fs::read(external.path().join(".env")).unwrap(), b"GEMINI_API_KEY=fake-key\n");
}
