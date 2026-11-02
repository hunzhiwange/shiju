use super::I18n;
use vw_config_types::ui::DesktopLocale;

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

#[test]
fn desktop_gettext_area_7_general_editor_sources_have_catalog_entries() {
    let en = super::catalog::parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = super::catalog::parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    for source in [
        include_str!("../components/system_settings_general.rs"),
        include_str!("../components/system_settings_editor.rs"),
    ] {
        for tail in source.split(".__(\"").skip(1) {
            let key = tail.split('"').next().unwrap();
            assert!(en.contains_key(key), "missing English: {key}");
            assert!(tw.contains_key(key), "missing Traditional Chinese: {key}");
        }
    }
}
