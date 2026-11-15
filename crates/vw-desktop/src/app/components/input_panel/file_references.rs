pub use desktop_chat_input::input_panel::file_references::extract_file_mentions;
pub fn render_file_references<'a>(
    i18n: crate::app::i18n::I18n,
    files: &[String],
    hovered: Option<usize>,
) -> iced::Element<'a, crate::app::Message> {
    desktop_chat_input::input_panel::file_references::render_file_references(i18n, files, hovered)
        .map(super::slice_message)
}
