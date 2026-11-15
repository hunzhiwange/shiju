use super::I18n;
use config_ui::DesktopLocale;
#[test]
fn desktop_gettext_area_13_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "草稿待保存"),
        (DesktopLocale::TraditionalChinese, "草稿待儲存"),
        (DesktopLocale::English, "Unsaved draft"),
    ] {
        assert_eq!(I18n::new(locale).__("草稿待保存"), expected);
    }
}
