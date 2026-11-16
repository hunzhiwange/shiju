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
pub fn lsp_status(app: &crate::app::App) -> Option<String> {
    desktop_i18n::editor::lsp_status(
        app.general_settings.desktop_locale,
        app.preview_state.lsp_status.as_deref(),
        app.preview_state.lsp_status_message.as_ref(),
    )
}
