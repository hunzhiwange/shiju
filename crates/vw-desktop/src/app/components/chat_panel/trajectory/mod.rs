//! 会话原始步骤数据的只读轨迹投影。

#![allow(dead_code)]

mod details;
mod details_content;
mod json_tree;
mod ledger;
mod model;
mod projection;
mod styles;
mod timeline;

pub(crate) use details::trajectory_detail_values;
#[allow(unused_imports)]
pub(crate) use ledger::{
    LedgerFoldAction, MAX_LEDGER_SEARCH_RESULTS, build_ledger_groups, filter_ledger_records,
    trajectory_ledger,
};

pub(crate) use crate::app::state::TrajectoryDetailsTab;
#[allow(unused_imports)]
pub(crate) use model::{
    MetricAvailability, TrajectoryControlsState, TrajectoryDetailsAction, TrajectoryDetailsState,
    TrajectoryDurationMode, TrajectoryMetrics, TrajectoryProjection, TrajectoryRecord,
    TrajectoryRecordDetail, TrajectoryRecordKind, TrajectoryStep, TrajectorySummary,
    TrajectoryToolDefinition, TrajectoryTurn, TrajectoryUsage,
};
#[allow(unused_imports)]
pub(crate) use projection::project_trajectory;
#[allow(unused_imports)]
pub(crate) use timeline::{
    MIN_TIMELINE_BLOCK_UNITS, TimelineBlock, TimelineLane, TimelineSummary, TimelineToolbarAction,
    TrajectoryTimeline, build_timeline, format_timeline_duration, trajectory_timeline,
};

use crate::app::{App, Message};
use crate::app::message::chat::ChatMessage;
pub(crate) fn view(app: &App) -> iced::Element<'_, Message> {
    let session_id = app.active_session_id.as_deref().unwrap_or_default();
    let loading = app.trajectory_raw_loading.iter().filter(|(id, _, _)| id == session_id).count();
    let errors = app.trajectory_raw_errors.keys().filter(|(id, _, _)| id == session_id).count();
    desktop_chat_view::trajectory::view::view(
        crate::app::i18n::I18n::new(app.desktop_locale),
        &app.active_session_view_state.trajectory_projection, &app.trajectory_details,
        &app.trajectory_controls, app.active_session_id.as_deref(), &app.trajectory_search_text,
        loading, errors,
    ).map(|event| {
        use desktop_chat_view::trajectory::view::Message as Event;
        Message::Chat(match event {
            Event::TrajectoryRecordSelected(id) => ChatMessage::TrajectoryRecordSelected(id),
            Event::TrajectoryDurationToggled => ChatMessage::TrajectoryDurationToggled,
            Event::TrajectoryTurnsToggled => ChatMessage::TrajectoryTurnsToggled,
            Event::TrajectoryCallsToggled => ChatMessage::TrajectoryCallsToggled,
            Event::TrajectorySearchChanged(text) => ChatMessage::TrajectorySearchChanged(text),
            Event::TrajectoryTurnToggled(index) => ChatMessage::TrajectoryTurnToggled(index),
            Event::TrajectoryToolStepToggled(index) => ChatMessage::TrajectoryToolStepToggled(index),
            Event::TrajectoryDetailsClosed => ChatMessage::TrajectoryDetailsClosed,
            Event::Details(event) => {
                use desktop_chat_view::trajectory::details::Message as Detail;
                match event {
                    Detail::TrajectoryDetailsClosed => ChatMessage::TrajectoryDetailsClosed,
                    Detail::TrajectoryDetailsTabSelected(tab) => ChatMessage::TrajectoryDetailsTabSelected(tab),
                    Detail::TrajectoryDetailsJsonToggled(path) => ChatMessage::TrajectoryDetailsJsonToggled(path),
                    Detail::TrajectoryDetailsJsonCopied(value) => ChatMessage::TrajectoryDetailsJsonCopied(value),
                    Detail::TrajectoryDetailsEditorAction(idx, action) => ChatMessage::TrajectoryDetailsEditorAction(idx, action),
                    Detail::TrajectoryDetailsContextMenuOpened { index, x, y } => ChatMessage::TrajectoryDetailsContextMenuOpened { index, x, y },
                    Detail::TrajectoryDetailsContextMenuClosed => ChatMessage::TrajectoryDetailsContextMenuClosed,
                    Detail::TrajectoryDetailsContextMenuCopy => ChatMessage::TrajectoryDetailsContextMenuCopy,
                }
            }
        })
    })
}
