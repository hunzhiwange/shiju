//! 会话原始步骤数据的只读轨迹投影。

#![allow(dead_code)]

mod details;
mod ledger;
mod model;
mod projection;
mod styles;
mod timeline;

#[allow(unused_imports)]
pub(crate) use ledger::{
    MAX_LEDGER_SEARCH_RESULTS, build_ledger_groups, filter_ledger_records, trajectory_ledger,
};

pub(crate) use crate::app::state::TrajectoryDetailsTab;
#[allow(unused_imports)]
pub(crate) use model::{
    MetricAvailability, TrajectoryDetailsAction, TrajectoryDetailsState, TrajectoryMetrics,
    TrajectoryProjection, TrajectoryRecord, TrajectoryRecordDetail, TrajectoryRecordKind,
    TrajectoryStep, TrajectorySummary, TrajectoryTurn, TrajectoryUsage,
};
#[allow(unused_imports)]
pub(crate) use projection::project_trajectory;
#[allow(unused_imports)]
pub(crate) use timeline::{
    MIN_TIMELINE_BLOCK_UNITS, TimelineBlock, TimelineLane, TimelineSummary, TrajectoryTimeline,
    build_timeline, format_timeline_duration, trajectory_timeline,
};

use crate::app::message::chat::ChatMessage;
use crate::app::{App, Message};
use iced::widget::{Space, column, container, mouse_area, row, stack, text, text_input};
use iced::{Element, Length, Theme};

/// 组合概览、搜索账本和详情检查器，数据始终来自当前活跃会话快照。
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let projection = project_trajectory(
        &app.active_session_view_state.steps,
        &app.active_session_view_state.step_llm_raw,
    );
    let selected_record_id = app.trajectory_details.selected_record_id();
    let select_record = |record_id| Message::Chat(ChatMessage::TrajectoryRecordSelected(record_id));

    let timeline = trajectory_timeline(&projection, selected_record_id, select_record);
    let search = text_input("搜索角色、工具、内容或 ID", &app.trajectory_search_text)
        .on_input(|search| Message::Chat(ChatMessage::TrajectorySearchChanged(search)))
        .padding([7, 9])
        .size(12)
        .style(styles::trajectory_search_input_style);
    let ledger = trajectory_ledger(
        &projection,
        &app.trajectory_search_text,
        selected_record_id,
        select_record,
    );
    let session_id = app.active_session_id.as_deref().unwrap_or_default();
    let loading_count = app
        .trajectory_raw_loading
        .iter()
        .filter(|(loading_session_id, _)| loading_session_id == session_id)
        .count();
    let error_count = app
        .trajectory_raw_errors
        .keys()
        .filter(|(error_session_id, _)| error_session_id == session_id)
        .count();
    let status = raw_refresh_status(loading_count, error_count);

    let body =
        container(column![search, ledger].spacing(8)).width(Length::Fill).height(Length::Fill);
    let base: Element<'_, Message> =
        column![status, timeline, body].spacing(10).width(Length::Fill).height(Length::Fill).into();

    let content = if let Some(record) =
        selected_record_id.and_then(|record_id| projection.record(record_id))
    {
        let close = Message::Chat(ChatMessage::TrajectoryDetailsClosed);
        let dismiss_layer = mouse_area(
            container(Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(close.clone());
        let details = details::trajectory_details(
            record,
            app.trajectory_details.active_tab(),
            |tab| Message::Chat(ChatMessage::TrajectoryDetailsTabSelected(tab)),
            close,
        );
        let details_layer = row![
            Space::new().width(Length::FillPortion(3)),
            container(mouse_area(details).on_press(Message::None))
                .width(Length::FillPortion(2))
                .height(Length::Fill),
        ]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill);

        stack![base, dismiss_layer, details_layer].width(Length::Fill).height(Length::Fill).into()
    } else {
        base
    };

    container(content).padding([10, 12]).width(Length::Fill).height(Length::Fill).into()
}

fn raw_refresh_status<'a>(loading_count: usize, error_count: usize) -> Element<'a, Message> {
    if loading_count > 0 {
        return text(format!("正在补载 {loading_count} 个步骤的原始数据…"))
            .size(11)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().primary.base.color),
            })
            .into();
    }
    if error_count > 0 {
        return text(format!("{error_count} 个步骤暂未取得原始数据；切回对话后重新进入轨迹可重试"))
            .size(11)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().warning.base.color),
            })
            .into();
    }
    Space::new().height(Length::Fixed(0.0)).into()
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod projection_tests;

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod ledger_tests;

#[cfg(test)]
#[path = "details_tests.rs"]
mod details_tests;

#[cfg(test)]
#[path = "timeline_tests.rs"]
mod timeline_tests;
