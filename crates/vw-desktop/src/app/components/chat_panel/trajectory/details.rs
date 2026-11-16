pub use desktop_chat_view::trajectory::details::{trajectory_detail_values, build_trajectory_details, detail_payload_font, detail_payload_wrapping, details_scroll_direction, input_preview_text, DETAILS_SCROLLBAR_WIDTH};
use super::{TrajectoryRecord, TrajectoryDetailsState};
use crate::app::{Message, message::ChatMessage};
pub(crate) fn trajectory_details<'a>(i18n: crate::app::i18n::I18n, record: &TrajectoryRecord, state: &'a TrajectoryDetailsState) -> iced::Element<'a, Message> {
    use desktop_chat_view::trajectory::details::{self as local, Message as Event};
    local::trajectory_details(i18n, record, state).map(|event| Message::Chat(match event {
        Event::TrajectoryDetailsClosed => ChatMessage::TrajectoryDetailsClosed,
        Event::TrajectoryDetailsTabSelected(tab) => ChatMessage::TrajectoryDetailsTabSelected(tab),
        Event::TrajectoryDetailsJsonToggled(path) => ChatMessage::TrajectoryDetailsJsonToggled(path),
        Event::TrajectoryDetailsJsonCopied(value) => ChatMessage::TrajectoryDetailsJsonCopied(value),
        Event::TrajectoryDetailsEditorAction(idx, action) => ChatMessage::TrajectoryDetailsEditorAction(idx, action),
        Event::TrajectoryDetailsContextMenuOpened { index, x, y } => ChatMessage::TrajectoryDetailsContextMenuOpened { index, x, y },
        Event::TrajectoryDetailsContextMenuClosed => ChatMessage::TrajectoryDetailsContextMenuClosed,
        Event::TrajectoryDetailsContextMenuCopy => ChatMessage::TrajectoryDetailsContextMenuCopy,
    }))
}
