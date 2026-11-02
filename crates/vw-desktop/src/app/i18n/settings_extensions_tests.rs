use super::settings_extensions::{operation_error, text};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_i18n_settings_extensions_switches_visible_text() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "加载中…"),
        (DesktopLocale::TraditionalChinese, "載入中…"),
        (DesktopLocale::English, "Loading…"),
    ] {
        assert_eq!(text(locale, "加载中…"), expected);
        let raw = "用户插件/example.org: 外部错误😀";
        assert!(operation_error(locale, raw).ends_with(raw));
    }
}
