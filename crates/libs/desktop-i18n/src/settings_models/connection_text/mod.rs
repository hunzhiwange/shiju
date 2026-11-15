//! Connection settings use Chinese source keys.
use config_ui::DesktopLocale;
pub fn text(locale: DesktopLocale, source: &str) -> &str {
    crate::I18n::new(locale).__(source)
}
