//! 当前工作区的有界运行记录列表与单次详情。
use super::*;
use crate::message::HistoryMessage as Event;
use crate::state::{CronHistoryState, CronHistoryStatus};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SettingsSelectLabel, icon_svg,
    settings_pick_list_menu_style, settings_pick_list_style, theme::is_dark_theme,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, pick_list, responsive, scrollable, stack, svg};
use iced::{Border, Color, Padding};
use gateway_cron::CronHistoryRunDto;

fn event(value: Event) -> Message {
    Message::Settings(message::SettingsMessage::CronHistory(value))
}

pub fn run_key(run: &CronHistoryRunDto) -> String {
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
fn empty_label(state: &CronHistoryState) -> Option<&'static str> {
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
fn refresh_style(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = primary_action_btn_style(theme, status);
    let dark = is_dark_theme(theme);
    let color = match (dark, status) {
        (_, button::Status::Disabled) => return style,
        (true, button::Status::Active) => Color::from_rgb8(255, 143, 70),
        (true, button::Status::Hovered) => Color::from_rgb8(255, 162, 101),
        (true, button::Status::Pressed) => Color::from_rgb8(232, 123, 50),
        (false, button::Status::Active) => Color::from_rgb8(198, 77, 9),
        (false, button::Status::Hovered) => Color::from_rgb8(175, 62, 5),
        (false, button::Status::Pressed) => Color::from_rgb8(153, 52, 3),
    };
    style.background = Some(color.into());
    style.text_color = if dark { Color::from_rgb8(34, 21, 14) } else { Color::WHITE };
    style
}

fn task_option(label: String, id: Option<String>, selected: bool) -> Element<'static, Message> {
    let mut content = row![SettingsSelectLabel::new(label)]
        .spacing(10)
        .height(Length::Fill)
        .align_y(Alignment::Center);
    if selected {
        content = content.push(icon_svg(Icon::Check, 16.0).style(|theme, _| svg::Style {
            color: Some(theme.extended_palette().success.strong.color),
        }));
    }
    button(content)
        .width(Length::Fill)
        .height(36)
        .padding([0, 10])
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            let active =
                selected || matches!(status, button::Status::Hovered | button::Status::Pressed);
            style.background = active.then(|| theme.palette().text.scale_alpha(0.06).into());
            style.border = Border { radius: 9.0.into(), ..Default::default() };
            style
        })
        .on_press(event(Event::TaskSelected(id)))
        .into()
}

fn task_picker<'a>(app: &ViewData<'a>, tr: I18n) -> Element<'a, Message> {
    let state = &app.cron_settings.history;
    let all_tasks = tr.__("全部任务");
    let label = state
        .selected_job_id
        .as_ref()
        .and_then(|id| app.cron_settings.jobs.iter().find(|job| job.id == *id))
        .map(job_title)
        .unwrap_or(all_tasks);
    let open = state.task_picker_open;
    let chevron = if open { Icon::ChevronUp } else { Icon::ChevronDown };
    let trigger = button(
        row![
            SettingsSelectLabel::new(label.into()),
            icon_svg(chevron, 12.0).style(|theme, _| svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.7)),
            })
        ]
        .spacing(12)
        .height(Length::Fill)
        .align_y(Alignment::Center),
    )
    .width(216)
    .height(SETTINGS_CONTROL_HEIGHT)
    .padding([0, 12])
    .style(move |theme, status| {
        let status = if open {
            pick_list::Status::Opened { is_hovered: false }
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            pick_list::Status::Hovered
        } else {
            pick_list::Status::Active
        };
        let select = settings_pick_list_style(theme, status);
        button::Style {
            background: Some(select.background),
            text_color: select.text_color,
            border: Border { radius: 10.0.into(), ..select.border },
            ..Default::default()
        }
    })
    .on_press(event(Event::TaskPickerToggled(!open)));

    if !open {
        return trigger.into();
    }

    let mut candidates =
        column![task_option(all_tasks.into(), None, state.selected_job_id.is_none(),)].spacing(2);
    for (id, name) in state.task_candidates(&app.cron_settings.jobs) {
        candidates = candidates.push(task_option(
            name,
            Some(id.into()),
            state.selected_job_id.as_deref() == Some(id),
        ));
    }
    let options = container(
        scrollable(container(candidates).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Shrink),
    )
    .max_height(180);
    let search = search_input(
        tr.__("搜索任务名称"),
        &state.task_search,
        Event::TaskSearchChanged,
    );
    let menu = container(column![container(search).padding([0, 7]), options].spacing(8))
        .width(248)
        .padding([8, 0])
        .style(|theme| {
            let style = settings_pick_list_menu_style(theme);
            container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                border: style.border,
                shadow: style.shadow,
                ..Default::default()
            }
        });
    BelowOverlay::new(trigger, menu)
        .show(true)
        .align_right(true)
        .gap(8.0)
        .dismiss_on_escape()
        .on_close(event(Event::TaskPickerToggled(false)))
        .into()
}

fn search_input(
    placeholder: &str,
    value: &str,
    on_input: fn(String) -> Event,
) -> Element<'static, Message> {
    stack![
        settings_text_input(placeholder, value)
            .padding(Padding::from(SETTINGS_CONTROL_PADDING).left(36))
            .on_input(move |value| event(on_input(value))),
        container(icon_svg(Icon::Search, 16.0).style(|theme, _| svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.6)),
        }))
        .padding([0, 12])
        .center_y(SETTINGS_CONTROL_HEIGHT),
    ]
    .into()
}

pub fn view<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    let state = &app.cron_settings.history;
    let tr = I18n::new(app.desktop_locale);
    let refresh = button(
        container(
            text(tr.__(if state.loading { "刷新中…" } else { "刷新" }))
                .size(SETTINGS_CONTROL_TEXT_SIZE),
        )
        .center_y(Length::Fill),
    )
    .height(SETTINGS_CONTROL_HEIGHT)
    .padding([0, 14])
    .style(refresh_style)
    .on_press_maybe((!state.loading).then(|| event(Event::Refresh)));
    let toolbar = row![
        container(search_input(
            tr.__("搜索定时任务/记录"),
            &state.search,
            Event::SearchChanged,
        ))
        .width(236),
        settings_select(
            [
                CronHistoryStatus::All,
                CronHistoryStatus::Running,
                CronHistoryStatus::Success,
                CronHistoryStatus::Failed
            ],
            Some(state.status_filter),
            |value| event(Event::StatusChanged(value))
        )
        .width(130),
        task_picker(app, tr),
        refresh,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut content = column![toolbar.wrap()].spacing(12);
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
    if state.visible_runs().next().is_none() {
        return content.height(Length::Fill).into();
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
pub fn with_details<'a, M: Clone + 'static>(app: &ViewData<'a>, dialog: Element<'a, M>, map: fn(Message) -> M) -> Element<'a, M> {
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
    settings_modal_overlay(Some(dialog), map(close), Element::<Message>::from(container(card).padding([10, 12])).map(map))
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
