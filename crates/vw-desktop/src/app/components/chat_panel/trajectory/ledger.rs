//! 按轮次和步骤组织的紧凑轨迹账本。

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};

use super::styles;
use super::{TrajectoryProjection, TrajectoryRecord, TrajectoryRecordKind};

const MAX_LEDGER_QUERY_CHARS: usize = 256;
pub(crate) const MAX_LEDGER_SEARCH_RESULTS: usize = 500;

#[derive(Debug)]
pub(crate) struct LedgerRecord<'a> {
    pub(crate) record: &'a TrajectoryRecord,
    pub(crate) selected: bool,
}

#[derive(Debug)]
pub(crate) struct LedgerStep<'a> {
    pub(crate) title: String,
    pub(crate) records: Vec<LedgerRecord<'a>>,
}

#[derive(Debug)]
pub(crate) struct LedgerGroup<'a> {
    pub(crate) title: String,
    pub(crate) steps: Vec<LedgerStep<'a>>,
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
) -> Vec<LedgerGroup<'a>> {
    let mut groups = Vec::<LedgerGroup<'a>>::new();
    for record in filter_ledger_records(projection, query) {
        let group_title =
            record.turn_index.map_or_else(|| "上下文".to_string(), |index| format!("轮次 {index}"));
        if groups.last().is_none_or(|group| group.title != group_title) {
            groups.push(LedgerGroup { title: group_title, steps: Vec::new() });
        }

        let group = groups.last_mut().expect("ledger group was just inserted");
        let step_title = format!("步骤 {}", record.step_index);
        if group.steps.last().is_none_or(|step| step.title != step_title) {
            group.steps.push(LedgerStep { title: step_title, records: Vec::new() });
        }
        group.steps.last_mut().expect("ledger step was just inserted").records.push(LedgerRecord {
            record,
            selected: selected_record_id.is_some_and(|selected| selected == record.id),
        });
    }
    groups
}

pub(crate) fn trajectory_ledger<'a, Message: Clone + 'a>(
    projection: &TrajectoryProjection,
    query: &str,
    selected_record_id: Option<&str>,
    on_select: impl Fn(String) -> Message,
) -> Element<'a, Message> {
    let groups = build_ledger_groups(projection, query, selected_record_id);
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
        let mut group_content = column![
            container(text(group.title).size(12))
                .padding([5, 8])
                .width(Length::Fill)
                .style(styles::group_header_style)
        ]
        .spacing(4)
        .width(Length::Fill);

        for step in group.steps {
            group_content = group_content.push(text(step.title).size(11).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(styles::step_header_text_color(theme)) }
            }));
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
        TrajectoryRecordKind::Assistant => "ASSISTANT",
        TrajectoryRecordKind::Tool => "TOOL",
    }
}
