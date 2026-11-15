//! Git 筛选面板的桌面消息适配。
use crate::app::{App, Message, message::GitMessage};
use iced::Element;
pub use desktop_git_ui::git_panel::filter_options::{FILTER_HELP_TITLE, FILTER_HELP_TEXT};
pub fn view(app: &App, included: usize, excluded: usize, new: usize, modified: usize, deleted: usize) -> Element<'static, Message> {
    use desktop_git_ui::git_panel::filter_options::{self, FilterMessage, FilterState};
    filter_options::view(&FilterState {
        desktop_locale: app.desktop_locale, git_filter_query: &app.git_filter_query,
        git_filter_included: app.git_filter_included,
        git_filter_excluded: app.git_filter_excluded,
        git_filter_new: app.git_filter_new,
        git_filter_modified: app.git_filter_modified,
        git_filter_deleted: app.git_filter_deleted,
    }, included, excluded, new, modified, deleted).map(|event| Message::Git(match event {
        FilterMessage::FilterQueryChanged(value) => GitMessage::FilterQueryChanged(value),
        FilterMessage::FilterToggleIncluded(value) => GitMessage::FilterToggleIncluded(value),
        FilterMessage::FilterToggleExcluded(value) => GitMessage::FilterToggleExcluded(value),
        FilterMessage::FilterToggleNew(value) => GitMessage::FilterToggleNew(value),
        FilterMessage::FilterToggleModified(value) => GitMessage::FilterToggleModified(value),
        FilterMessage::FilterToggleDeleted(value) => GitMessage::FilterToggleDeleted(value),
        FilterMessage::ToggleFilterOptions(value) => GitMessage::ToggleFilterOptions(value),
        FilterMessage::FilterHelpOpen => GitMessage::FilterHelpOpen,
        FilterMessage::ClearFilters => GitMessage::ClearFilters,
    }))
}
