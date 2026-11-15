use super::{I18n, catalog::parse_catalog, input::Feedback};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_19_uses_chinese_keys() {
    let raw = "用户/%s/%n/繁體";
    let feedback = Feedback::new("规划 · %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "规划"),
        (DesktopLocale::TraditionalChinese, "規劃"),
        (DesktopLocale::English, "Planning"),
    ] {
        let i18n = I18n::new(locale);
        assert_eq!(feedback.render(i18n, &feedback.fallback), format!("{prefix} · {raw}"));
        assert_eq!(feedback.render(i18n, raw), raw);
    }
    let en = parse_catalog(include_str!("locales/en.json")).unwrap();
    let tw = parse_catalog(include_str!("locales/zh-TW.json")).unwrap();
    assert_eq!(en.keys().collect::<Vec<_>>(), tw.keys().collect::<Vec<_>>());
    assert!(!en.is_empty());
    for source in ["前进", "后退", "知道了", "Web 不支持终端"] {
        assert!(en.contains_key(source), "missing English: {source}");
        assert!(tw.contains_key(source), "missing Traditional Chinese: {source}");
    }
}

#[test]
fn desktop_gettext_area_19_catalog_signatures_match() {
    let en = parse_catalog(include_str!("locales/en.json")).unwrap();
    for source in en.keys().filter(|source| source.contains('%')) {
        for locale in [DesktopLocale::English, DesktopLocale::TraditionalChinese] {
            let result = I18n::new(locale).format(source, &[]);
            if let Err(error) = result {
                assert_eq!(error.0, "argument count mismatch", "{locale:?}: {source}");
            }
        }
    }
}
