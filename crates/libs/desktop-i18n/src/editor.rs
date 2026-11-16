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

/// Store the stable source alongside its rendered fallback.
pub fn set_lsp_status(
    status: &mut Option<String>,
    localized: &mut Option<(String, super::format::Message)>,
    source: &str,
    arguments: Vec<super::format::Argument>,
) {
    let message = super::format::Message::new(source, arguments);
    let raw = message
        .render(super::I18n::new(DesktopLocale::SimplifiedChinese))
        .expect("valid LSP template");
    *status = Some(raw.clone());
    *localized = Some((raw, message));
}

/// Translate only the fallback we generated; retain raw server messages.
pub fn lsp_status(
    locale: DesktopLocale,
    raw: Option<&str>,
    localized: Option<&(String, super::format::Message)>,
) -> Option<String> {
    let raw = raw?;
    if let Some((fallback, message)) = localized {
        if fallback == raw {
            return Some(message.render(super::I18n::new(locale)).expect("valid LSP template"));
        }
    }
    Some(raw.to_owned())
}
