use super::I18n;
use vw_config_types::ui::DesktopLocale;
#[test]
fn desktop_gettext_area_17_uses_chinese_keys() {
    assert_eq!(I18n::new(DesktopLocale::English).__("隐藏识矩"), "Hide ShiJu");
    assert_eq!(I18n::new(DesktopLocale::TraditionalChinese).__("隐藏识矩"), "隱藏識矩");
}

#[test]
fn desktop_gettext_area_17_preserves_user_titles_and_switches_fallback() {
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        let raw = "未命名/%s/%n";
        assert_eq!(super::native::tab_title(i18n, raw.into(), false), raw);
        assert_eq!(super::native::tab_title(i18n, "未命名".into(), true), i18n.__("未命名"));
    }
}

#[test]
fn desktop_gettext_area_17_update_diagnostics_switch_without_interpreting_parameters() {
    let raw = "服务/%s/%n/原始";
    for locale in [
        DesktopLocale::SimplifiedChinese,
        DesktopLocale::TraditionalChinese,
        DesktopLocale::English,
    ] {
        let i18n = I18n::new(locale);
        for restart in [true, false] {
            let rendered = super::native::update_diagnostic(i18n, raw, restart);
            assert!(rendered.starts_with(&format!("{raw}\n")));
            if locale == DesktopLocale::English {
                assert!(rendered.contains("restart") || rendered.contains("Restart"));
            }
        }
    }
}
