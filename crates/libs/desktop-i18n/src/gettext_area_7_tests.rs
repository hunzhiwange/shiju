use super::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_7_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "应用外观与预览行为。"),
        (DesktopLocale::TraditionalChinese, "應用程式外觀與預覽行為。"),
        (DesktopLocale::English, "Application appearance and preview behavior."),
    ] {
        assert_eq!(I18n::new(locale).__("应用外观与预览行为。"), expected);
    }
}
