//! Editor UI language follows the explicit desktop locale.
use config_ui::DesktopLocale;

pub fn language(locale: DesktopLocale) -> iced_code_editor::Language {
    match locale {
        DesktopLocale::SimplifiedChinese => iced_code_editor::Language::ChineseSimplified,
        // Upstream v0.4.4 has no Traditional Chinese translations.
        DesktopLocale::TraditionalChinese => iced_code_editor::Language::ChineseSimplified,
        DesktopLocale::English => iced_code_editor::Language::English,
    }
}
