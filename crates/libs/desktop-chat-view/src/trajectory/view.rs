use super::{details, styles};
use super::model::{TrajectoryProjection, TrajectoryControlsState, TrajectoryDetailsState, TrajectoryRecordKind};
use super::timeline::{trajectory_timeline, TimelineToolbarAction};
use super::ledger::{trajectory_ledger, LedgerFoldAction};
#[derive(Debug, Clone)]
pub enum Message {
    TrajectoryRecordSelected(String), TrajectoryDurationToggled, TrajectoryTurnsToggled,
    TrajectoryCallsToggled, TrajectorySearchChanged(String), TrajectoryTurnToggled(u32),
    TrajectoryToolStepToggled(u32), TrajectoryDetailsClosed, Details(details::Message),
}
use iced::widget::{Space, column, container, mouse_area, opaque, row, stack, text, text_input};
use iced::{Element, Length, Theme};

/// 组合概览、搜索账本和详情检查器，数据始终来自当前活跃会话快照。
pub fn view<'a>(i18n: desktop_i18n::I18n, projection: &'a TrajectoryProjection, details_state: &'a TrajectoryDetailsState, controls: &TrajectoryControlsState, session_id: Option<&str>, search_text: &'a str, loading_count: usize, error_count: usize) -> Element<'a, Message> {
    let selected_record_id = details_state.selected_record_id();
    let select_record = |record_id| Message::TrajectoryRecordSelected(record_id);
    let trajectory_controls = controls.for_session(session_id);
    let turns_collapsed = !projection.turns.is_empty()
        && projection.turns.iter().all(|turn| trajectory_controls.is_turn_collapsed(turn.index));
    let tool_steps = projection
        .records
        .iter()
        .filter(|record| record.kind == TrajectoryRecordKind::Tool)
        .map(|record| record.step_index)
        .collect::<std::collections::HashSet<_>>();
    let calls_collapsed = !tool_steps.is_empty()
        && tool_steps.iter().all(|step| trajectory_controls.is_tool_step_collapsed(*step));

    let timeline = trajectory_timeline(
        i18n,
        &projection,
        trajectory_controls.duration_mode(),
        turns_collapsed,
        calls_collapsed,
        selected_record_id,
        |action| {
            match action {
                TimelineToolbarAction::ToggleDuration => Message::TrajectoryDurationToggled,
                TimelineToolbarAction::ToggleTurns => Message::TrajectoryTurnsToggled,
                TimelineToolbarAction::ToggleCalls => Message::TrajectoryCallsToggled,
            }
        },
        select_record,
    );
    let search = text_input(
        i18n.__("搜索角色、工具、内容或 ID"),
        &search_text,
    )
    .on_input(|search| Message::TrajectorySearchChanged(search))
    .padding([7, 9])
    .size(12)
    .style(styles::trajectory_search_input_style);
    let ledger = trajectory_ledger(
        i18n,
        &projection,
        &search_text,
        selected_record_id,
        &trajectory_controls,
        select_record,
        |action| {
            match action {
                LedgerFoldAction::Turn(turn_index) => {
                    Message::TrajectoryTurnToggled(turn_index)
                }
                LedgerFoldAction::ToolStep(step_index) => {
                    Message::TrajectoryToolStepToggled(step_index)
                }
            }
        },
    );
    let status = raw_refresh_status(
        i18n,
        loading_count,
        error_count,
    );

    let body =
        container(column![search, ledger].spacing(8)).width(Length::Fill).height(Length::Fill);
    let base: Element<'_, Message> =
        column![status, timeline, body].spacing(10).width(Length::Fill).height(Length::Fill).into();

    let content = if let Some(record) =
        selected_record_id.and_then(|record_id| projection.record(record_id))
    {
        let close = Message::TrajectoryDetailsClosed;
        let dismiss_layer = opaque(
            mouse_area(
                container(Space::new().width(Length::Fill).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(close),
        );
        let details = details::trajectory_details(
            i18n,
            record,
            details_state,
        ).map(Message::Details);
        let details_layer = row![
            Space::new().width(Length::FillPortion(3)),
            container(opaque(details)).width(Length::FillPortion(2)).height(Length::Fill),
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

fn raw_refresh_status<'a>(
    i18n: desktop_i18n::I18n,
    loading_count: usize,
    error_count: usize,
) -> Element<'a, Message> {
    if loading_count > 0 {
        return text(
            i18n.format("正在自动补载 %u 个步骤的原始数据（最多 3 次）…", &[loading_count.into()])
                .expect("valid raw refresh template"),
        )
        .size(11)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().primary.base.color),
        })
        .into();
    }
    if error_count > 0 {
        return text(
            i18n.format("%u 个步骤自动重试后仍未取得原始数据，可稍后重试", &[error_count.into()])
                .expect("valid raw error template"),
        )
        .size(11)
        .style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.extended_palette().warning.base.color),
        })
        .into();
    }
    Space::new().height(Length::Fixed(0.0)).into()
}
