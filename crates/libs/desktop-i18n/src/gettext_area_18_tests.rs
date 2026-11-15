use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_18_uses_chinese_keys() {
    let raw = "server/%s/%n/原始";
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "即将完成..."),
        (DesktopLocale::TraditionalChinese, "即將完成..."),
        (DesktopLocale::English, "Finishing..."),
    ] {
        let rendered = super::other::lsp_finishing(I18n::new(locale), raw);
        assert_eq!(rendered, format!("LSP: {raw} ({expected})"));
    }
}
