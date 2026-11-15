use crate::app::{App, Message};
pub(super) fn view(app: &App) -> iced::Element<'_, Message> {
    let tab = app.preview_tabs.iter().find(|tab| Some(&tab.path) == app.active_preview_path.as_ref());
    desktop_editor::components::preview_panel::view::editor_options::view(tab.map(|tab| &tab.editor.inner), desktop_i18n::I18n::new(app.desktop_locale)).map(super::map_editor_view_message)
}
