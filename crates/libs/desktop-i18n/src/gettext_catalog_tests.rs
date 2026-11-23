use super::catalog::{I18n, RuntimeCatalog, parse_catalog};
use config_ui::DesktopLocale;

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

#[test]
fn desktop_gettext_catalog_runtime_falls_back_for_blank_or_invalid_catalog() {
    use std::sync::LazyLock;

    assert_eq!(
        I18n::new(DesktopLocale::SimplifiedChinese).translate_with_catalogs(
            "语言",
            || panic!("Simplified Chinese must not load English"),
            || panic!("Simplified Chinese must not load Traditional Chinese"),
        ),
        "语言",
    );

    for broken_locale in [DesktopLocale::English, DesktopLocale::TraditionalChinese] {
        for (json, expected) in [
            (r#"{"语言":"Translated","空串":"","空白":" \t\n　"}"#, "Translated"),
            ("{}", "语言"),
            ("{", "语言"),
            (r#"{"语言":"a","语言":"b"}"#, "语言"),
            (r#"{"语言":{}}"#, "语言"),
            ("[]", "语言"),
        ] {
            let en = LazyLock::new(|| RuntimeCatalog::load(json));
            let zh_tw = LazyLock::new(|| RuntimeCatalog::load(json));
            let healthy_en = LazyLock::new(|| RuntimeCatalog::load(r#"{"语言":"Language"}"#));
            let healthy_tw = LazyLock::new(|| RuntimeCatalog::load(r#"{"语言":"語言"}"#));
            // Inject each damaged locale without touching global catalogs or language state.
            // Separate LazyLocks exercise the same cached loading path as production.
            let check = |en: &RuntimeCatalog, zh_tw: &RuntimeCatalog| {
                for _ in 0..2 {
                    let broken = I18n::new(broken_locale);
                    for source in ["缺项", "空串", "空白"] {
                        assert_eq!(broken.translate_with_catalogs(source, || en, || zh_tw), source);
                    }
                    assert_eq!(broken.translate_with_catalogs("语言", || en, || zh_tw), expected);
                    let (healthy_locale, expected) = match broken_locale {
                        DesktopLocale::English => (DesktopLocale::TraditionalChinese, "語言"),
                        _ => (DesktopLocale::English, "Language"),
                    };
                    assert_eq!(
                        I18n::new(healthy_locale).translate_with_catalogs("语言", || en, || zh_tw),
                        expected
                    );
                    assert_eq!(
                        I18n::new(DesktopLocale::SimplifiedChinese).translate_with_catalogs(
                            "语言",
                            || en,
                            || zh_tw
                        ),
                        "语言"
                    );
                }
            };
            match broken_locale {
                DesktopLocale::English => check(&en, &healthy_tw),
                _ => check(&healthy_en, &zh_tw),
            }
        }
    }
}
