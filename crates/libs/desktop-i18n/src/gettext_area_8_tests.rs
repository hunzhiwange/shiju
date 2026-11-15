use super::I18n;
use config_ui::DesktopLocale;

#[test]
fn desktop_gettext_area_8_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "请求失败"),
        (DesktopLocale::TraditionalChinese, "請求失敗"),
        (DesktopLocale::English, "Request failed"),
    ] {
        assert_eq!(I18n::new(locale).__("请求失败"), expected);
    }
}

#[test]
fn desktop_gettext_area_8_dynamic_api_url_preserves_raw_argument() {
    let raw = "https://example.invalid/中文/%n/%s";
    let message = super::format::Message::new("接口地址: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "接口地址: "),
        (DesktopLocale::TraditionalChinese, "介面位址: "),
        (DesktopLocale::English, "API URL: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}

#[test]
fn desktop_gettext_area_8_header_templates_preserve_typed_values() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "使用率: 12.5%"),
        (DesktopLocale::TraditionalChinese, "使用率: 12.5%"),
        (DesktopLocale::English, "Usage: 12.5%"),
    ] {
        assert_eq!(I18n::new(locale).format("使用率: %.1f%%", &[12.5.into()]).unwrap(), expected);
    }
}

#[test]
fn desktop_gettext_area_8_trajectory_fields_follow_locale_and_preserve_unknown_ids() {
    assert_eq!(
        super::chat::trajectory_label(I18n::new(DesktopLocale::SimplifiedChinese), "Source"),
        "来源"
    );
    assert_eq!(
        super::chat::trajectory_label(
            I18n::new(DesktopLocale::TraditionalChinese),
            "No payload captured"
        ),
        "未擷取請求承載資料"
    );
    assert_eq!(
        super::chat::trajectory_label(I18n::new(DesktopLocale::English), "Source"),
        "Source"
    );
    let raw = "工具/%n/%s/custom";
    assert_eq!(super::chat::trajectory_label(I18n::new(DesktopLocale::English), raw), raw);
}
