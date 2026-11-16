use crate::app::{App, Message, message};
use iced::Element;
#[cfg(test)] use iced::Point;
#[cfg(test)]
pub(super) fn tab_drag_start_message(path: &str) -> Message {
    Message::Project(message::ProjectMessage::FileTreeDragStart(path.to_string(), None))
}

#[cfg(test)]
pub(super) fn tab_drag_end_message() -> Message {
    Message::Project(message::ProjectMessage::FileTreeDragEnd)
}

#[cfg(test)]
pub(super) fn tab_close_message(path: &str) -> Message {
    Message::Preview(message::PreviewMessage::Close(path.to_string()))
}

#[cfg(test)]
pub(super) fn tab_select_message(path: &str) -> Message {
    Message::Preview(message::PreviewMessage::Select(path.to_string()))
}

#[cfg(test)]
pub(super) fn open_file_message() -> Message {
    Message::Project(message::ProjectMessage::OpenFilePressed)
}

#[cfg(test)]
pub(super) fn header_tab_item_count(app: &App) -> usize {
    app.navigation_tabs.preview_tabs.len() + 1
}

#[cfg(test)]
pub(super) fn tab_right_click_message(path: &str, pos: Point) -> Message {
    Message::Preview(message::PreviewMessage::TabRightClicked(path.to_string(), pos.x, pos.y))
}

pub fn build_header_tabs(app: &App) -> Element<'_, Message> {
    desktop_editor::components::preview_panel::view::header_tabs::build_header_tabs(
        app.navigation_tabs.preview_tabs.iter().map(|tab| (tab.path.as_str(), tab.title.as_str())),
        app.navigation_tabs.active_preview_path.as_deref(), app.navigation_tabs.preview_tab_menu_path.as_deref(),
        app.navigation_tabs.preview_tab_menu_pos, app.preview_tabs_scroll_id.clone(),
        super::super::styles::should_show_preview_tabs_scrollbar(app), desktop_i18n::I18n::new(app.desktop_locale),
    ).map(map_message)
}
fn map_message(action: desktop_editor::components::preview_panel::view::HeaderMessage) -> Message {
    use desktop_editor::components::preview_panel::view::HeaderMessage as Action;
    match action {
        Action::DragStart(path) => Message::Project(message::ProjectMessage::FileTreeDragStart(path, None)),
        Action::DragEnd => Message::Project(message::ProjectMessage::FileTreeDragEnd),
        Action::OpenFile => Message::Project(message::ProjectMessage::OpenFilePressed),
        Action::Close(path) => Message::Preview(message::PreviewMessage::Close(path)),
        Action::Select(path) => Message::Preview(message::PreviewMessage::Select(path)),
        Action::RightClick(path, x, y) => Message::Preview(message::PreviewMessage::TabRightClicked(path, x, y)),
        Action::MenuClose => Message::Preview(message::PreviewMessage::TabMenuClose),
        Action::TabMenu(action) => super::super::tabs::map_message(action),
    }
}
