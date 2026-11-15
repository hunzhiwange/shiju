use crate::app::{App, Message};
pub(super) fn tools(app: &App) -> iced::Element<'_, Message> {
    let tab = app.preview_tabs.iter().find(|tab| Some(&tab.path) == app.active_preview_path.as_ref());
    let popover = app.editor_popover.filter(|_| !app.show_preview_context_menu && !app.show_preview_settings && app.preview_nav_popup.is_none());
    desktop_editor::components::preview_panel::view::editor_popovers::tools(
        tab.map(|tab| &tab.editor.inner), tab.is_none_or(|tab| tab.read_only),
        desktop_i18n::I18n::new(app.desktop_locale), popover,
    ).map(super::map_editor_view_message)
}
