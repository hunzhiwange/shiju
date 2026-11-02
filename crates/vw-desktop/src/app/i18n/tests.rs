use super::{SettingsText, save_error, settings_text};
use vw_config_types::ui::DesktopLocale;

#[test]
fn desktop_i18n_catalogs_cover_three_locales() {
    for (locale, language, error) in [
        (DesktopLocale::SimplifiedChinese, "语言", "保存失败："),
        (DesktopLocale::TraditionalChinese, "語言", "儲存失敗："),
        (DesktopLocale::English, "Language", "Failed to save: "),
    ] {
        assert_eq!(settings_text(locale, SettingsText::Language), language);
        let raw = "原始內容 / raw {value}";
        assert_eq!(save_error(locale, raw), format!("{error}{raw}"));
    }
}
