use desktop_i18n::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_18_builtin_tabs_translate_without_changing_user_titles() {
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        assert_eq!(
            crate::state::localized_tool_name(I18n::new(locale), "自定义/%s/%n"),
            "自定义/%s/%n"
        );
    }
}
