//! Git 筛选面板的桌面消息适配。
pub use crate::git_panel::filter_options::{FILTER_HELP_TEXT, FILTER_HELP_TITLE};
use crate::message_git::messages::GitMessage;
use crate::page::Message;
use crate::page::ViewContext;
use iced::Element;
pub fn view(
    app: &ViewContext,
    included: usize,
    excluded: usize,
    new: usize,
    modified: usize,
    deleted: usize,
) -> Element<'static, Message> {
    use crate::git_panel::filter_options::{self, FilterMessage, FilterState};
    filter_options::view(
        &FilterState {
            desktop_locale: app.desktop_locale,
            git_filter_query: &app.git.git_filter_query,
            git_filter_included: app.git.git_filter_included,
            git_filter_excluded: app.git.git_filter_excluded,
            git_filter_new: app.git.git_filter_new,
            git_filter_modified: app.git.git_filter_modified,
            git_filter_deleted: app.git.git_filter_deleted,
        },
        included,
        excluded,
        new,
        modified,
        deleted,
    )
    .map(|event| {
        Message::Git(match event {
            FilterMessage::FilterQueryChanged(value) => GitMessage::FilterQueryChanged(value),
            FilterMessage::FilterToggleIncluded(value) => GitMessage::FilterToggleIncluded(value),
            FilterMessage::FilterToggleExcluded(value) => GitMessage::FilterToggleExcluded(value),
            FilterMessage::FilterToggleNew(value) => GitMessage::FilterToggleNew(value),
            FilterMessage::FilterToggleModified(value) => GitMessage::FilterToggleModified(value),
            FilterMessage::FilterToggleDeleted(value) => GitMessage::FilterToggleDeleted(value),
            FilterMessage::ToggleFilterOptions(value) => GitMessage::ToggleFilterOptions(value),
            FilterMessage::FilterHelpOpen => GitMessage::FilterHelpOpen,
            FilterMessage::ClearFilters => GitMessage::ClearFilters,
        })
    })
}
