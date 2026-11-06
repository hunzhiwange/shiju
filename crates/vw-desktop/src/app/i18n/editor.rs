//! Editor UI language follows the explicit desktop locale.
use vw_config_types::ui::DesktopLocale;

pub fn language(locale: DesktopLocale) -> iced_code_editor::Language {
    match locale {
        DesktopLocale::SimplifiedChinese => iced_code_editor::Language::ChineseSimplified,
        // Upstream v0.4.4 has no Traditional Chinese translations.
        DesktopLocale::TraditionalChinese => iced_code_editor::Language::ChineseSimplified,
        DesktopLocale::English => iced_code_editor::Language::English,
    }
}

pub fn refresh(app: &mut crate::app::App) {
    let language = language(app.desktop_locale);
    for tab in &mut app.preview_tabs {
        tab.editor.set_ui_language(language);
        tab.editor.configure_preview_context_menu(app.desktop_locale, true);
    }
    app.git_copy_modal_code_editor.set_language(language);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_lsp_status(
    app: &mut crate::app::App,
    source: &str,
    arguments: Vec<super::format::Argument>,
) {
    let message = super::format::Message::new(source, arguments);
    let raw = message
        .render(super::I18n::new(DesktopLocale::SimplifiedChinese))
        .expect("valid LSP template");
    app.lsp_status = Some(raw.clone());
    app.lsp_status_message = Some((raw, message));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lsp_status(app: &crate::app::App) -> Option<String> {
    let raw = app.lsp_status.as_ref()?;
    if let Some((fallback, message)) = &app.lsp_status_message {
        if fallback == raw {
            return Some(
                message.render(super::I18n::new(app.desktop_locale)).expect("valid LSP template"),
            );
        }
    }
    Some(raw.clone())
}
