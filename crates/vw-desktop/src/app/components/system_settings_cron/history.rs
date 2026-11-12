//! 当前工作区的有界运行记录列表与单次详情。
use super::*;
use crate::app::message::settings::cron_history::CronHistoryMessage as Event;
use crate::app::state::{CronHistoryLocation, CronHistoryState, CronHistoryStatus};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, responsive, scrollable};
use vw_gateway_client::CronHistoryRunDto;

const TYPE_OPTIONS: [CronHistoryLocation; 2] =
    [CronHistoryLocation::All, CronHistoryLocation::Local];
fn event(value: Event) -> Message {
    Message::Settings(message::SettingsMessage::CronHistory(value))
}

pub(crate) fn run_key(run: &CronHistoryRunDto) -> String {
    // Prefer execution identity for active snapshots and database identity for stored rows.
    match (&run.execution_id, run.id) {
        (Some(id), _) => format!("{}:execution:{id}", run.job_id),
        (None, Some(id)) => format!("{}:history:{id}", run.job_id),
        (None, None) => format!("{}:start:{}", run.job_id, run.started_at),
    }
}
fn selected_run(state: &CronHistoryState) -> Option<&CronHistoryRunDto> {
    let key = state.selected_run.as_ref()?;
    state.runs.iter().find(|run| run_key(run) == *key)
}
fn status_label(status: &str) -> String {
    match status {
        "running" => "◌ 运行中".into(),
        "ok" => "成功".into(),
        "error" => "失败".into(),
        other => format!("未知状态（{other}）"),
    }
}
fn duration_label(run: &CronHistoryRunDto) -> String {
    if run.status == "running" {
        return "—".into();
    }
    match run.duration_ms {
        Some(ms) if ms >= 0 => format!("{}.{:03} 秒", ms / 1000, ms % 1000),
        _ => "耗时不可用".into(),
    }
}
fn absolute_time(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %:z").to_string())
        .unwrap_or_else(|_| {
            if value.is_empty() {
                "时间缺失".into()
            } else {
                format!("时间无效（{value}）")
            }
        })
}
pub(super) fn empty_label(state: &CronHistoryState) -> Option<&'static str> {
    if state.visible_runs().next().is_some() {
        return None;
    }
    if state.loading {
        return Some("正在加载运行记录…");
    }
    if state.error.is_some() {
        return None;
    }
    if !state.search.trim().is_empty()
        || state.selected_job_id.is_some()
        || state.status_filter != CronHistoryStatus::All
        || state.location_filter != CronHistoryLocation::All
    {
        Some("此范围内无匹配记录")
    } else {
        Some("此范围内暂无运行记录")
    }
}
fn scroll<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(container(body.into()).padding([0, 7]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into()
}
fn cell<'a>(value: impl Into<String>, portion: u16) -> Element<'a, Message> {
    container(text(value.into()).size(13)).width(Length::FillPortion(portion)).into()
}
fn record_row(run: &CronHistoryRunDto, narrow: bool) -> Element<'_, Message> {
    let details = small_button("查看详情", event(Event::DetailsOpened(run_key(run))), false);
    if narrow {
        return settings_group_card(
            column![
                text(CronHistoryState::display_name(run)).size(15),
                text(format!("{} · 本地 · {}", status_label(&run.status), duration_label(run))),
                text(format!("开始：{}", absolute_time(&run.started_at))).size(12),
                details,
            ]
            .spacing(8),
        )
        .into();
    }
    row![
        cell(CronHistoryState::display_name(run), 3),
        cell(status_label(&run.status), 2),
        cell(absolute_time(&run.started_at), 3),
        cell(duration_label(run), 2),
        cell("本地", 1),
        container(details).width(Length::FillPortion(2))
    ]
    .spacing(10)
    .padding([12, 0])
    .align_y(Alignment::Center)
    .into()
}
pub(super) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.cron_settings.history;
    let task_label = state
        .selected_job_id
        .as_ref()
        .map(|id| {
            app.cron_settings
                .jobs
                .iter()
                .find(|job| job.id == *id)
                .map(job_title)
                .unwrap_or_else(|| id.clone())
        })
        .unwrap_or_else(|| "全部任务".into());
    let mut toolbar = row![
        crate::app::components::system_settings_common::icon_btn(
            crate::app::assets::Icon::Search,
            "搜索任务名称",
            Some(event(Event::SearchToggled))
        ),
        settings_select(
            [
                CronHistoryStatus::All,
                CronHistoryStatus::Running,
                CronHistoryStatus::Success,
                CronHistoryStatus::Failed
            ],
            Some(state.status_filter),
            |v| event(Event::StatusChanged(v))
        )
        .width(130),
        settings_select(TYPE_OPTIONS, Some(state.location_filter), |v| event(
            Event::LocationChanged(v)
        ))
        .width(120),
        button(text(task_label).size(13))
            .width(180)
            .padding(SETTINGS_CONTROL_PADDING)
            .style(rounded_action_btn_style)
            .on_press(event(Event::TaskPickerToggled(!state.task_picker_open))),
    ]
    .spacing(8);
    let mut refresh = button(text(if state.loading { "刷新中…" } else { "刷新" }))
        .padding(SETTINGS_CONTROL_PADDING)
        .style(rounded_action_btn_style);
    if !state.loading {
        refresh = refresh.on_press(event(Event::Refresh));
    }
    toolbar = toolbar.push(refresh);
    let mut content = column![toolbar.wrap()].spacing(12);
    if state.search_open || !state.search.is_empty() {
        content = content.push(
            settings_text_input("搜索任务名称", &state.search)
                .on_input(|v| event(Event::SearchChanged(v))),
        );
    }
    if state.task_picker_open {
        let mut candidates = column![secondary_button(
            if state.selected_job_id.is_none() { "✓ 全部任务" } else { "全部任务" },
            event(Event::TaskSelected(None))
        )]
        .spacing(6);
        for (id, name) in state.task_candidates(&app.cron_settings.jobs) {
            let label = if state.selected_job_id.as_deref() == Some(id) {
                format!("✓ {name}")
            } else {
                name
            };
            candidates = candidates
                .push(secondary_button(label, event(Event::TaskSelected(Some(id.into())))));
        }
        content = content.push(settings_group_card(
            column![
                settings_text_input("搜索任务", &state.task_search)
                    .on_input(|v| event(Event::TaskSearchChanged(v))),
                container(scroll(candidates)).height(160)
            ]
            .spacing(8),
        ));
    }
    content = content.push(text("最近 200 条已结束记录及当前运行任务，筛选仅作用于此范围；时间为本机时区（含 UTC 偏移）").size(12).style(settings_muted_text_style));
    if state.truncated {
        content = content.push(text("存在更早的运行记录").size(12));
    }
    if let Some(error) = &state.error {
        content = content
            .push(settings_error_banner(error.clone()))
            .push(text("刷新失败，已有数据可能不是最新状态"))
            .push(secondary_button("重试", event(Event::Refresh)));
    }
    if let Some(label) = empty_label(state) {
        content = content.push(text(label));
    }
    if state.loading && !state.runs.is_empty() {
        content = content.push(text("正在刷新运行记录…"));
    }
    content = content.push(responsive(move |size| {
        let narrow = size.width < 760.0;
        let mut rows = column![].spacing(6);
        if !narrow {
            rows = rows.push(
                row![
                    cell("任务名称", 3),
                    cell("状态", 2),
                    cell("运行时间", 3),
                    cell("耗时", 2),
                    cell("任务类型", 1),
                    cell("操作", 2)
                ]
                .spacing(10),
            );
        }
        for run in state.visible_runs() {
            rows = rows.push(record_row(run, narrow)).push(settings_divider());
        }
        scroll(rows)
    }));
    content.height(Length::Fill).into()
}
pub(super) fn with_details<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.cron_settings.history;
    if state.selected_run.is_none() {
        return dialog;
    }
    let close = event(Event::DetailsClosed);
    let header =
        settings_modal_header(text("运行详情").size(20), settings_close_button(close.clone()));
    let mut actions = row![secondary_button("关闭", close.clone())].spacing(8);
    let body: Element<'_, Message> = if let Some(run) = selected_run(state) {
        if let Some(output) = &run.output {
            actions = actions
                .push(secondary_button("复制输出", event(Event::CopyOutput(output.clone()))));
        }
        column![
            text(CronHistoryState::display_name(run)).size(18),
            text(format!("状态：{} · 本地", status_label(&run.status))),
            text(format!("开始：{}", absolute_time(&run.started_at))),
            text(format!(
                "结束：{}",
                run.finished_at.as_deref().map(absolute_time).unwrap_or_else(|| {
                    if run.status == "running" {
                        "尚未结束".into()
                    } else {
                        "时间缺失".into()
                    }
                })
            )),
            text(format!("耗时：{}", duration_label(run))),
            settings_divider(),
            text("输出（服务端保留的截断内容）").size(13),
            text(run.output.as_deref().filter(|v| !v.is_empty()).unwrap_or(
                if run.status == "running" {
                    "运行中，尚未产生输出"
                } else {
                    "无输出"
                }
            ))
            .size(13),
        ]
        .spacing(12)
        .into()
    } else {
        text("该次运行记录已不可用，可能已删除或超出保留范围。请关闭后刷新。").into()
    };
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(cron_modal_style)
        .width(780)
        .height(580);
    settings_modal_overlay(Some(dialog), close, container(card).padding([10, 12]))
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
