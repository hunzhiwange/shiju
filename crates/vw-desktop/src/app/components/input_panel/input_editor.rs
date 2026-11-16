fn state(app: &crate::app::App) -> desktop_chat_input::input_panel::input_editor::EditorState {
    desktop_chat_input::input_panel::input_editor::EditorState {
        show_slash_palette: app.input_suggestions.show_slash_palette,
        show_file_search: app.input_suggestions.show_file_search,
        input_editor_id: app.session_state.input_editor_id.clone(),
        i18n: crate::app::i18n::I18n::new(app.general_settings.desktop_locale),
        context_menu_position: app.session_state.input_context_menu.position,
        context_menu_open: app.session_state.input_context_menu.open,
    }
}
pub fn build_input_editor<'a>(
    app: &'a crate::app::App,
    content: &'a iced::widget::text_editor::Content,
    requesting: bool,
    task_mode: bool,
) -> (iced::Element<'a, crate::app::Message>, f32) {
    let chrome = crate::app::theme_catalog::resolve_chrome_style_from_skin(
        app.appearance.active_theme_skin.as_ref(),
        &app.appearance.app_theme,
    );
    let (editor, height) = desktop_chat_input::input_panel::input_editor::build_input_editor(
        state(app),
        content,
        move |theme, status| super::styles::editor_style(theme, status, requesting, Some(&chrome)),
        task_mode,
    );
    (editor.map(super::slice_message), height)
}
