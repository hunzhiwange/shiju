pub fn file_search_overlay(app: &crate::app::App) -> iced::Element<'_, crate::app::Message> {
    let files = app.cached_file_search_entries();
    let entries = files
        .iter()
        .map(|e| desktop_chat_input::input_panel::file_search::FileSearchEntry {
            path: &e.path,
            is_dir: e.is_dir,
        })
        .collect::<Vec<_>>();
    desktop_chat_input::input_panel::file_search::file_search_overlay(
        crate::app::i18n::I18n::new(app.desktop_locale),
        &entries,
        &app.file_search_query,
        app.project_path.as_deref(),
        app.file_search_selected_index,
        app.file_search_scroll_id.clone(),
    )
    .map(super::slice_message)
}
