use super::delete::ensure_unreferenced;
use vw_api_types::agent_apps::AppId;
fn fixture(app: AppId) -> (tempfile::TempDir, std::path::PathBuf, &'static str) {
    let root = tempfile::tempdir().unwrap();
    let (relative, content) = match app {
        AppId::Kimi => (
            ".kimi-code/config.toml",
            "default_model='alias'\n[providers.used]\napi_key='fixture-secret'\n[models.alias]\nprovider='used'\n",
        ),
        _ => (
            ".config/opencode/opencode.json",
            r#"{"provider":{"used":{"apiKey":"fixture-secret"}}}"#,
        ),
    };
    let path = root.path().join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    (root, path, content)
}
#[test]
fn cc04_delete_rejects_referenced_provider() {
    for app in [AppId::Kimi, AppId::OpenCode] {
        let (root, path, content) = fixture(app);
        assert!(ensure_unreferenced(root.path(), app, "used").is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }
}
#[test]
fn cc04_delete_unused_row_leaves_live_unchanged() {
    for app in [AppId::Kimi, AppId::OpenCode] {
        let (root, path, content) = fixture(app);
        let record = root.path().join("unused.json");
        std::fs::write(&record, b"catalog fixture").unwrap();
        ensure_unreferenced(root.path(), app, "unused").unwrap();
        std::fs::remove_file(&record).unwrap();
        assert!(!record.exists());
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }
}
#[test]
fn cc04_delete_unknown_or_unsafe_configuration_fails_closed() {
    let (root, path, _) = fixture(AppId::OpenCode);
    assert!(ensure_unreferenced(root.path(), AppId::Claude, "unused").is_err());
    std::fs::write(&path, "broken").unwrap();
    assert!(ensure_unreferenced(root.path(), AppId::OpenCode, "unused").is_err());
    std::fs::remove_file(&path).unwrap();
    ensure_unreferenced(root.path(), AppId::OpenCode, "unused").unwrap();
    std::fs::write(path.with_extension("jsonc"), "// preferred").unwrap();
    assert!(ensure_unreferenced(root.path(), AppId::OpenCode, "unused").is_err());
    std::fs::remove_file(path.with_extension("jsonc")).unwrap();
    std::os::unix::fs::symlink(root.path().join("outside"), &path).unwrap();
    assert!(ensure_unreferenced(root.path(), AppId::OpenCode, "unused").is_err());
}
