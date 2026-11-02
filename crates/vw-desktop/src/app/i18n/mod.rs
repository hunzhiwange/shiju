//! Explicit locale catalogs. Raw user and service content is never translated.
#[cfg(test)]
mod tests;

use vw_config_types::ui::DesktopLocale;

/// Text owned by the settings domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsText {
    Language,
}

pub fn settings_text(locale: DesktopLocale, key: SettingsText) -> &'static str {
    match (locale, key) {
        (DesktopLocale::SimplifiedChinese, SettingsText::Language) => "语言",
        (DesktopLocale::TraditionalChinese, SettingsText::Language) => "語言",
        (DesktopLocale::English, SettingsText::Language) => "Language",
    }
}

/// Localize the application prefix while retaining the service error verbatim.
pub fn save_error(locale: DesktopLocale, raw_error: &str) -> String {
    let prefix = match locale {
        DesktopLocale::SimplifiedChinese => "保存失败：",
        DesktopLocale::TraditionalChinese => "儲存失敗：",
        DesktopLocale::English => "Failed to save: ",
    };
    format!("{prefix}{raw_error}")
}
