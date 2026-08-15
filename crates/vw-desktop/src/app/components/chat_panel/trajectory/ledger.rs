//! 按轮次和步骤组织的紧凑轨迹账本。

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles;
use super::{
    TrajectoryControlsState, TrajectoryProjection, TrajectoryRecord, TrajectoryRecordKind,
};

const MAX_LEDGER_QUERY_CHARS: usize = 256;
pub(crate) const MAX_LEDGER_SEARCH_RESULTS: usize = 500;

#[derive(Debug)]
pub(crate) struct LedgerRecord<'a> {
    pub(crate) record: &'a TrajectoryRecord,
    pub(crate) selected: bool,
}

#[derive(Debug)]
pub(crate) struct LedgerStep<'a> {
    pub(crate) step_index: u32,
    pub(crate) title: String,
    pub(crate) has_tools: bool,
    pub(crate) tools_collapsed: bool,
    pub(crate) records: Vec<LedgerRecord<'a>>,
}

#[derive(Debug)]
pub(crate) struct LedgerGroup<'a> {
    pub(crate) title: String,
    pub(crate) turn_index: Option<u32>,
    pub(crate) collapsed: bool,
    pub(crate) steps: Vec<LedgerStep<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LedgerFoldAction {
    Turn(u32),
    ToolStep(u32),
}

pub(crate) fn filter_ledger_records<'a>(
    projection: &'a TrajectoryProjection,
    query: &str,
) -> Vec<&'a TrajectoryRecord> {
    let query =
        query.trim().chars().take(MAX_LEDGER_QUERY_CHARS).collect::<String>().to_lowercase();
    let mut records = projection
        .records
        .iter()
        .filter(|record| ledger_record_matches(record, &query))
        .collect::<Vec<_>>();
    records.sort_by_key(|record| {
        (
            record.turn_index.is_some(),
            record.turn_index.unwrap_or_default(),
            record.step_index,
            record.index,
        )
    });
    records.truncate(MAX_LEDGER_SEARCH_RESULTS);
    records
}

pub(crate) fn build_ledger_groups<'a>(
    projection: &'a TrajectoryProjection,
    query: &str,
    selected_record_id: Option<&str>,
    controls: &TrajectoryControlsState,
) -> Vec<LedgerGroup<'a>> {
    let mut groups = Vec::<LedgerGroup<'a>>::new();
    let search_active = !query.trim().is_empty();
    for record in filter_ledger_records(projection, query) {
        let group_title =
            record.turn_index.map_or_else(|| "上下文".to_string(), |index| format!("轮次 {index}"));
        if groups.last().is_none_or(|group| group.title != group_title) {
            groups.push(LedgerGroup {
                title: group_title,
                turn_index: record.turn_index,
                collapsed: record.turn_index.is_some_and(|index| controls.is_turn_collapsed(index))
                    && !search_active,
                steps: Vec::new(),
            });
        }

        let group = groups.last_mut().expect("ledger group was just inserted");
        let selected = selected_record_id.is_some_and(|selected| selected == record.id);
        let revealed = search_active || selected;
        if revealed {
            group.collapsed = false;
        }
        if group.collapsed {
            continue;
        }
        let step_title = format!("步骤 {}", record.step_index);
        if group.steps.last().is_none_or(|step| step.title != step_title) {
            group.steps.push(LedgerStep {
                step_index: record.step_index,
                title: step_title,
                has_tools: false,
                tools_collapsed: controls.is_tool_step_collapsed(record.step_index) && !revealed,
                records: Vec::new(),
            });
        }
        let step = group.steps.last_mut().expect("ledger step was just inserted");
        if record.kind == TrajectoryRecordKind::Tool {
            step.has_tools = true;
            if step.tools_collapsed {
                continue;
            }
        }
        step.records.push(LedgerRecord { record, selected });
    }
    groups
}

pub(crate) fn trajectory_ledger<'a, Message: Clone + 'a>(
    projection: &TrajectoryProjection,
    query: &str,
    selected_record_id: Option<&str>,
    controls: &TrajectoryControlsState,
    on_select: impl Fn(String) -> Message,
    on_fold: impl Fn(LedgerFoldAction) -> Message,
) -> Element<'a, Message> {
    let groups = build_ledger_groups(projection, query, selected_record_id, controls);
    let mut ledger = column![].spacing(8).width(Length::Fill);

    if groups.is_empty() {
        let label =
            if projection.records.is_empty() { "暂无轨迹记录" } else { "没有匹配记录" };
        ledger = ledger.push(
            container(text(label).size(12).style(|theme: &Theme| iced::widget::text::Style {
                color: Some(styles::empty_text_color(theme)),
            }))
            .padding([18, 12])
            .width(Length::Fill)
            .center_x(Length::Fill),
        );
    }

    for group in groups {
        let mut group_content =
            column![ledger_group_header(group.title, group.turn_index, group.collapsed, &on_fold,)]
                .spacing(4)
                .width(Length::Fill);

        for step in group.steps {
            let mut step_header = row![text(step.title).size(11).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(styles::step_header_text_color(theme)) }
            })]
            .spacing(6)
            .align_y(Alignment::Center);
            if step.has_tools {
                step_header = step_header.push(ledger_tool_group_button(
                    step.step_index,
                    step.tools_collapsed,
                    &on_fold,
                ));
            }
            group_content = group_content.push(step_header);
            for record in step.records {
                group_content = group_content.push(record_row(record, &on_select));
            }
        }
        ledger = ledger.push(group_content);
    }

    scrollable(ledger)
        .width(Length::Fill)
        .height(Length::Fill)
        .direction(styles::ledger_scroll_direction())
        .into()
}

pub(crate) fn ledger_group_header<'a, Message: Clone + 'a>(
    title: impl Into<String>,
    turn_index: Option<u32>,
    collapsed: bool,
    on_fold: impl Fn(LedgerFoldAction) -> Message,
) -> Element<'a, Message> {
    let title = title.into();
    if let Some(turn_index) = turn_index {
        return button(
            row![text(fold_arrow(collapsed)).size(11), text(title).size(12)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .width(Length::Fill)
        .style(move |theme, status| styles::ledger_fold_button_style(theme, status, collapsed))
        .on_press(on_fold(LedgerFoldAction::Turn(turn_index)))
        .into();
    }

    container(text(title).size(12))
        .padding([5, 8])
        .width(Length::Fill)
        .style(styles::group_header_style)
        .into()
}

pub(crate) fn ledger_tool_group_button<'a, Message: Clone + 'a>(
    step_index: u32,
    collapsed: bool,
    on_fold: impl Fn(LedgerFoldAction) -> Message,
) -> Element<'a, Message> {
    button(text(format!("{} Tools", fold_arrow(collapsed))).size(10))
        .padding([3, 6])
        .style(move |theme, status| styles::ledger_tool_fold_button_style(theme, status, collapsed))
        .on_press(on_fold(LedgerFoldAction::ToolStep(step_index)))
        .into()
}

const fn fold_arrow(collapsed: bool) -> &'static str {
    if collapsed { "▸" } else { "▾" }
}

fn record_row<'a, Message: Clone + 'a>(
    row_state: LedgerRecord<'_>,
    on_select: &impl Fn(String) -> Message,
) -> Element<'a, Message> {
    let record = row_state.record;
    let record_kind = record.kind;
    let missing = record.preview.trim().is_empty();
    let preview = if missing { "—".to_string() } else { record.preview.clone() };
    let mut leading = row![
        container(text(role_label(record_kind)).size(10))
            .padding([2, 5])
            .style(move |theme: &Theme| styles::role_badge_style(theme, record_kind)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(tool_name) = record.detail.tool_name.as_deref() {
        leading = leading.push(text(tool_name.to_string()).size(11).wrapping(Wrapping::None));
    }

    let selected = row_state.selected;
    let record_id = record.id.clone();
    let record_index = record.index;
    let preview_font = styles::record_preview_font(&preview);
    button(
        row![
            container(leading).width(Length::Fixed(128.0)),
            text(preview).font(preview_font).size(12).wrapping(Wrapping::None).width(Length::Fill),
            text(format!("#{record_index}")).size(10),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(move |theme: &Theme, status| {
        styles::ledger_record_row_style(theme, status, selected, missing)
    })
    .on_press(on_select(record_id))
    .into()
}

fn ledger_record_matches(record: &TrajectoryRecord, query: &str) -> bool {
    query.is_empty()
        || role_label(record.kind).to_lowercase().contains(query)
        || record.preview.to_lowercase().contains(query)
        || record
            .detail
            .tool_name
            .as_deref()
            .is_some_and(|tool_name| tool_name.to_lowercase().contains(query))
}

const fn role_label(kind: TrajectoryRecordKind) -> &'static str {
    match kind {
        TrajectoryRecordKind::System => "SYSTEM",
        TrajectoryRecordKind::User => "USER",
        TrajectoryRecordKind::Context => "CONTEXT",
        TrajectoryRecordKind::Assistant => "ASSISTANT",
        TrajectoryRecordKind::Tool => "TOOL",
    }
}
