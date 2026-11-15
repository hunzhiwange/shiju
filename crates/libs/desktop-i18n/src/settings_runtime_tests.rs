use super::settings_runtime::text;
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_i18n_settings_runtime_switches_visible_text() {
    assert_eq!(text(DesktopLocale::SimplifiedChinese, "运行时类型"), "运行时类型");
    assert_eq!(text(DesktopLocale::TraditionalChinese, "运行时类型"), "執行環境類型");
    assert_eq!(text(DesktopLocale::English, "运行时类型"), "Runtime type");
}

#[test]
fn desktop_i18n_settings_runtime_memory_storage_and_observability() {
    for (key, zh, tw, en) in [
        ("记忆系统配置", "记忆系统配置", "記憶系統設定", "Memory system configuration"),
        ("存储类型", "存储类型", "儲存類型", "Storage type"),
        ("可观测性配置", "可观测性配置", "可觀測性設定", "Observability configuration"),
    ] {
        assert_eq!(text(DesktopLocale::SimplifiedChinese, key), zh);
        assert_eq!(text(DesktopLocale::TraditionalChinese, key), tw);
        assert_eq!(text(DesktopLocale::English, key), en);
    }
}

#[test]
fn desktop_i18n_settings_runtime_dynamic_errors_preserve_raw_content() {
    use super::settings_runtime::validation_error;
    use super::settings_security::text as security_text;
    let raw = "用户/admin.example.com:外部错误😀";
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        assert_eq!(validation_error(locale, raw), raw);
        let duplicate = validation_error(locale, &format!("列表项已存在：{raw}"));
        assert!(duplicate.ends_with(raw));
        assert!(!security_text(locale, "Bash 安全检查").is_empty());
    }
    assert_eq!(
        validation_error(DesktopLocale::English, "Docker CPU 限制 必须大于 0"),
        "Docker CPU limit must be greater than 0"
    );
    assert_eq!(
        validation_error(DesktopLocale::TraditionalChinese, "列表项不能为空"),
        "清單項目不得為空"
    );
}
