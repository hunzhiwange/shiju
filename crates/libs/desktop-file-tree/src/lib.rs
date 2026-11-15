pub mod model;
pub mod icons;
pub mod inline_rename;
pub mod menu;
#[cfg(target_os = "macos")]
pub mod native_drop;
pub mod row_layout;
pub mod selection;
pub mod tree_gutter;
pub mod find_results;
pub mod widgets;
pub mod state;
mod message;
pub use message::{FileTreeAction, Message, ProjectMessage};
#[derive(Clone, Copy)]
pub struct FileTreeView<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub project_path: &'a Option<String>,
    pub find_results_tabs: &'a [state::FindInFolderTab],
    pub active_find_results_tab_id: &'a Option<String>,
    pub find_results_tab_menu_id: &'a Option<String>,
    pub find_results_tab_menu_pos: Option<iced::Point>,
    pub file_tree_rename_value: &'a str,
    pub file_tree_rename_path: &'a Option<String>,
    pub file_tree_menu_path: &'a Option<String>,
    pub file_tree_menu_source: &'a Option<String>,
    pub file_tree_menu_anchor: Option<iced::Point>,
    pub can_paste: bool,
}

use iced::widget::{Scrollable, scrollable};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Element, Length};
use widgets::RightClickArea;
fn scroll_tree(content: Element<'_, Message>, file_tree: bool) -> Element<'_, Message> {
    scroll_surface(
        scrollable(content)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Fill),
        file_tree,
    )
}

fn scroll_surface(scroll: Scrollable<'_, Message>, _drag_targets: bool) -> Element<'_, Message> {
    let surface = RightClickArea::new(scroll.into(), Box::new(|_| Message::None), None, None)
        .track_window_cursor();
    Element::new(surface)
}

#[cfg(test)]
mod model_tests;

#[cfg(test)]
mod icons_tests;

#[cfg(test)]
mod find_results_tests;

#[cfg(test)]
mod widgets_tests;
