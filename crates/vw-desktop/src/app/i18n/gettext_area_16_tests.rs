use super::I18n;
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_gettext_area_16_uses_chinese_keys() {
    for (locale, expected) in [
        (DesktopLocale::SimplifiedChinese, "未选择文件"),
        (DesktopLocale::TraditionalChinese, "未選擇檔案"),
        (DesktopLocale::English, "No file selected"),
    ] {
        assert_eq!(I18n::new(locale).__("未选择文件"), expected);
    }
}

#[test]
fn desktop_gettext_area_16_editor_traditional_is_isolated() {
    use iced_code_editor::{Language, Translations};
    let traditional = Translations::new(Language::ChineseTraditional);
    let english = Translations::new(Language::English);
    assert_eq!(traditional.search_placeholder(), "搜尋...");
    assert_eq!(english.search_placeholder(), "Search...");
    assert_eq!(traditional.context_menu_copy(), "複製");
}

#[test]
fn desktop_gettext_area_16_dynamic_parameters_remain_raw() {
    let raw = "服務/%s/%n/原始";
    let message = super::format::Message::new("保存失败: %s", vec![raw.into()]);
    for (locale, prefix) in [
        (DesktopLocale::SimplifiedChinese, "保存失败: "),
        (DesktopLocale::TraditionalChinese, "儲存失敗："),
        (DesktopLocale::English, "Failed to save: "),
    ] {
        assert_eq!(message.render(I18n::new(locale)).unwrap(), format!("{prefix}{raw}"));
    }
}
