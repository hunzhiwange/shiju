use super::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_12_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "任务看板"),
        (DesktopLocale::TraditionalChinese, "任務看板"),
        (DesktopLocale::English, "Task board"),
    ] {
        assert_eq!(I18n::new(locale).__("任务看板"), expected);
    }
}
