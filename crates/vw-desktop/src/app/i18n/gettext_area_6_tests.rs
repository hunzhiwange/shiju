use super::settings_extensions::{domain_label, operation_error, text};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_6_uses_chinese_keys() {
    let raw = "用户/%n/原始错误";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "钩子配置"),
        (DesktopLocale::TraditionalChinese, "鉤子設定"),
        (DesktopLocale::English, "Hook configuration"),
    ] {
        assert_eq!(text(locale, "钩子配置"), expected);
        assert!(operation_error(locale, raw).ends_with(raw));
        assert!(domain_label(locale, 42).ends_with("42"));
    }
}

#[test]
fn desktop_gettext_area_6_catalog_covers_hooks_and_http() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        "钩子配置",
        "网络请求配置",
        "操作失败：%s",
        "域名 %u",
        "例如：example.com 或 *.example.org 或 *",
    ] {
        assert!(en.contains_key(source), "{source}");
        assert!(tw.contains_key(source), "{source}");
    }
    let raw = "原始 %s %n";
    let chinese = operation_error(DesktopLocale::SimplifiedChinese, raw);
    let english = operation_error(DesktopLocale::English, raw);
    assert_eq!(chinese, format!("操作失败：{raw}"));
    assert_eq!(english, format!("Operation failed: {raw}"));
}
