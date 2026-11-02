//! Connection settings use Chinese source keys.
use vw_config_types::ui::DesktopLocale;
pub fn text(locale: DesktopLocale, source: &str) -> &str {
    crate::app::i18n::I18n::new(locale).__(source)
}
