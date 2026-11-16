use config_ui::DesktopLocale;
pub use desktop_i18n::editor::*;

pub fn refresh(app: &mut crate::app::App) {
    let language = language(app.general_settings.desktop_locale);
    for tab in &mut app.navigation_tabs.preview_tabs {
        tab.editor.set_ui_language(language);
        tab.editor.configure_preview_context_menu(app.general_settings.desktop_locale, true);
    }
    app.git.git_copy_modal_code_editor.set_language(language);
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
    app.preview_state.lsp_status = Some(raw.clone());
    app.preview_state.lsp_status_message = Some((raw, message));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn lsp_status(app: &crate::app::App) -> Option<String> {
    let raw = app.preview_state.lsp_status.as_ref()?;
    if let Some((fallback, message)) = &app.preview_state.lsp_status_message {
        if fallback == raw {
            return Some(
                message.render(super::I18n::new(app.general_settings.desktop_locale)).expect("valid LSP template"),
            );
        }
    }
    Some(raw.clone())
}
