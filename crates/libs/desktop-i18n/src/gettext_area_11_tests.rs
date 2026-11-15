use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_11_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "暂存完成"),
        (DesktopLocale::TraditionalChinese, "暫存完成"),
        (DesktopLocale::English, "Staging complete"),
    ] {
        assert_eq!(I18n::new(locale).__("暂存完成"), expected);
    }
}

#[test]
fn desktop_gettext_area_11_comment_range_switches_language() {
    assert_eq!(
        super::git::comment_range(I18n::new(DesktopLocale::English), 1, 3, false),
        "Commenting on new lines 2-4"
    );
    assert_eq!(
        super::git::comment_range(I18n::new(DesktopLocale::TraditionalChinese), 1, 1, true),
        "正在評論 舊行 2"
    );
}
