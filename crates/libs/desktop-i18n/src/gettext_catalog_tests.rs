use super::catalog::{I18n, parse_catalog};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_catalog_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "语言"),
        (DesktopLocale::TraditionalChinese, "語言"),
        (DesktopLocale::English, "Language"),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(i18n.__("语言"), expected);
        assert_eq!(i18n.__("尚未收录的中文"), "尚未收录的中文");
    }
}

#[test]
fn desktop_gettext_catalog_rejects_invalid_structure() {
    for source in [
        r#"{"语言":"a","语言":"b"}"#,
        r#"{"语言":{}}"#,
        r#"{"语言":1}"#,
        r#"{"language":"Language"}"#,
        r#"{"settings.语言":"Language"}"#,
        "[]",
        "{} trailing",
    ] {
        assert!(parse_catalog(source).is_err(), "{source}");
    }
    assert!(parse_catalog(r#"{"语言":"Language"}"#).is_ok());
}

#[test]
fn desktop_gettext_catalog_embedded_catalogs_have_matching_coverage() {
    let en = parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    assert!(!en.is_empty());
    assert_eq!(en.keys().collect::<Vec<_>>(), tw.keys().collect::<Vec<_>>());
}
