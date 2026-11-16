//! 底部面板及既有低层组件的兼容导出；生产类型由独立库拥有。

pub(crate) use tui_contracts::presentation::MentionBinding;

pub(crate) use tui_widgets::bottom_pane_view;
pub(crate) use tui_composer::chat_composer_history;
pub(crate) use tui_input::paste_burst;
use tui_composer::pending_input_preview;
pub(crate) use tui_input::textarea;
pub(crate) use bottom_pane_view::BottomPaneView;
#[allow(unused_imports)]
pub(crate) use tui_widgets::action_required_title;
use tui_composer::chat_composer;
pub(crate) use tui_popups::command_popup;
pub(crate) use tui_popups::file_search_popup;
pub(crate) use tui_widgets::list_selection_view;
pub(crate) use tui_widgets::scroll_state;
pub(crate) use tui_widgets::selection_popup_common;
#[allow(unused_imports)]
pub(crate) use tui_widgets::selection_row_layout;
#[allow(unused_imports)]
pub(crate) use tui_widgets::selection_tabs;
pub(crate) use tui_popups::slash_commands;

pub(crate) use chat_composer::ChatComposer;
pub(crate) use pending_input_preview::PendingInputPreview;

pub(crate) use tui_bottom_pane::{BottomPane, BottomPaneParams};
