use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_selection_uses_chinese_keys() {
    for (locale, label, description) in [
        (DesktopLocale::SimplifiedChinese, "语言", "当前桌面端使用的界面语言。"),
        (DesktopLocale::TraditionalChinese, "語言", "目前桌面端使用的介面語言。"),
        (DesktopLocale::English, "Language", "The interface language used by the desktop app."),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(i18n.__("语言"), label);
        assert_eq!(i18n.__("当前桌面端使用的界面语言。"), description);
        let raw = "原始错误 %n %s";
        assert!(crate::__!(i18n, "保存失败：%s", raw).unwrap().ends_with(raw));
    }
}
