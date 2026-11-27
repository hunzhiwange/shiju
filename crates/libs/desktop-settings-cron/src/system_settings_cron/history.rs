//! 当前工作区的有界运行记录列表与单次详情。
use super::*;
use crate::message::HistoryMessage as Event;
use crate::state::{CronHistoryState, CronHistoryStatus};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::select::menu_card_style;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SettingsSelectLabel, icon_svg, settings_pick_list_style,
};
use gateway_cron::CronHistoryRunDto;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{column, pick_list, responsive, scrollable, stack, svg};
use iced::{Border, Padding};

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
fn status_label(i18n: I18n, status: &str) -> String {
    match status {
        "running" => i18n.__("运行中").into(),
        "ok" => i18n.__("成功").into(),
        "error" => i18n.__("失败").into(),
        other => desktop_i18n::__!(i18n, "未知状态（%s）", (other).to_string())
            .expect("valid settings message"),
    }
}
fn duration_label(i18n: I18n, run: &CronHistoryRunDto) -> String {
    if run.status == "running" {
        return "—".into();
    }
    match run.duration_ms {
        Some(ms) if ms >= 0 => {
            desktop_i18n::__!(i18n, "%s 秒", format!("{}.{:03}", ms / 1000, ms % 1000))
                .expect("valid settings message")
        }
        _ => i18n.__("耗时不可用").into(),
    }
}
fn absolute_time(i18n: I18n, value: &str, timezone: NamedTimeZone) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|time| time.with_timezone(&timezone).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|_| {
            if value.is_empty() {
                i18n.__("时间缺失").into()
            } else {
                desktop_i18n::__!(i18n, "时间无效（%s）", (value).to_string())
                    .expect("valid settings message")
            }
        })
}
fn empty_label(i18n: I18n, state: &CronHistoryState) -> Option<&'static str> {
    if state.visible_runs().next().is_some() {
        return None;
    }
    if state.loading && state.finished_limit == 0 {
        return Some(i18n.__("正在加载运行记录…"));
    }
    if state.error.is_some() {
        return None;
    }
    if !state.search.trim().is_empty()
        || state.selected_job_id.is_some()
        || state.status_filter != CronHistoryStatus::All
    {
        Some(i18n.__("此范围内无匹配记录"))
    } else {
        Some(i18n.__("此范围内暂无运行记录"))
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
fn run_status(i18n: I18n, status: &str) -> Element<'static, Message> {
    let color: fn(&Theme) -> iced::Color = match status {
        "running" => |theme: &Theme| theme.palette().primary,
        "ok" => |theme: &Theme| theme.extended_palette().success.base.color,
        "error" => |theme: &Theme| theme.extended_palette().danger.base.color,
        _ => |theme: &Theme| theme.extended_palette().background.weak.text,
    };
    row![
        container(iced::widget::Space::new()).width(6).height(6).style(move |theme| {
            container::Style {
                background: Some(color(theme).into()),
                border: Border { radius: 3.0.into(), ..Default::default() },
                ..Default::default()
            }
        }),
        text(status_label(i18n, status)).size(13),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn record_row(
    i18n: I18n,
    run: &CronHistoryRunDto,
    narrow: bool,
    timezone: NamedTimeZone,
) -> Element<'_, Message> {
    let details =
        small_button(i18n.__("查看详情"), event(Event::DetailsOpened(run_key(run))), false);
    if narrow {
        return settings_group_card(
            column![
                text(if run.name.as_deref().is_none_or(|name| name.trim().is_empty()) {
                    i18n.__("未命名任务")
                } else {
                    CronHistoryState::display_name(run)
                })
                .size(15),
                row![
                    run_status(i18n, &run.status),
                    text(i18n.__("本地")).size(12),
                    text(duration_label(i18n, run)).size(12)
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .wrap(),
                text(
                    desktop_i18n::__!(
                        i18n,
                        "开始：%s",
                        (absolute_time(i18n, &run.started_at, timezone)).to_string()
                    )
                    .expect("valid settings message")
                )
                .size(12),
                details,
            ]
            .spacing(8),
        )
        .into();
    }
    row![
        cell(
            if run.name.as_deref().is_none_or(|name| name.trim().is_empty()) {
                i18n.__("未命名任务")
            } else {
                CronHistoryState::display_name(run)
            },
            3
        ),
        container(run_status(i18n, &run.status)).width(Length::FillPortion(2)),
        cell(absolute_time(i18n, &run.started_at, timezone), 3),
        cell(duration_label(i18n, run), 2),
        cell(i18n.__("本地"), 1),
        container(details).width(Length::FillPortion(2))
    ]
    .spacing(10)
    .padding([12, 0])
    .align_y(Alignment::Center)
    .into()
}
fn task_picker<'a>(app: &ViewData<'a>, i18n: I18n) -> Element<'a, Message> {
    let state = &app.cron_settings.history;
    let all_tasks = i18n.__("全部任务");
    let label = state
        .selected_job_id
        .as_ref()
        .and_then(|id| app.cron_settings.jobs.iter().find(|job| job.id == *id))
        .map(job_title)
        .map(|title| if title == "未命名任务" { i18n.__(title) } else { title })
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

    let mut candidates = column![list::select_option(
        all_tasks.into(),
        state.selected_job_id.is_none(),
        event(Event::TaskSelected(None)),
    )]
    .spacing(2);
    for (id, name) in state.task_candidates(&app.cron_settings.jobs) {
        candidates = candidates.push(list::select_option(
            name,
            state.selected_job_id.as_deref() == Some(id),
            event(Event::TaskSelected(Some(id.into()))),
        ));
    }
    let options = container(
        scrollable(container(candidates).padding([0, 7]).width(Length::Fill))
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
            .height(Length::Shrink),
    )
    .max_height(180);
    let search = search_input(
        "cron-history-task-search",
        i18n.__("搜索任务名称"),
        &state.task_search,
        Event::TaskSearchChanged,
    );
    let menu = container(column![container(search).padding([0, 7]), options].spacing(8))
        .width(248)
        .padding([8, 0])
        .style(menu_card_style);
    BelowOverlay::new(trigger, menu)
        .show(true)
        .align_right(true)
        .gap(8.0)
        .dismiss_on_escape()
        .on_close(event(Event::TaskPickerToggled(false)))
        .into()
}

fn search_input(
    id: &'static str,
    placeholder: &str,
    value: &str,
    on_input: fn(String) -> Event,
) -> Element<'static, Message> {
    stack![
        settings_text_input(placeholder, value)
            .id(id)
            .padding(Padding::from(SETTINGS_CONTROL_PADDING).left(36))
            .on_input(move |value| event(on_input(value))),
        container(
            icon_svg(Icon::Search, 16.0).style(|theme, _| svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.6)),
            })
        )
        .padding([0, 12])
        .center_y(SETTINGS_CONTROL_HEIGHT),
    ]
    .into()
}

pub fn view<'a>(app: &ViewData<'a>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.cron_settings.history;
    let timezone = app.system_timezone;
    let refresh = list::toolbar_icon(
        Icon::ArrowClockwise,
        15.0,
        i18n.__(if state.loading { "刷新中…" } else { "刷新" }),
        (!state.loading).then(|| event(Event::Refresh)),
    );
    let statuses = [
        CronHistoryStatus::All,
        CronHistoryStatus::Running,
        CronHistoryStatus::Success,
        CronHistoryStatus::Failed,
    ]
    .map(|value| HistoryStatusOption { value, label: i18n.__(&value.to_string()).to_owned() });
    let selected_status =
        statuses.iter().find(|option| option.value == state.status_filter).cloned();
    let mut toolbar = row![
        list::toolbar_search(
            i18n,
            state.show_search || !state.search.is_empty(),
            "cron-history-search",
            &state.search,
            event(Event::SearchToggled),
            |query| event(Event::SearchChanged(query)),
        ),
        settings_select(statuses, selected_status, |option| event(Event::StatusChanged(
            option.value
        )))
        .width(120),
        task_picker(app, i18n),
        refresh,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(id) = &state.selected_job_id {
        toolbar = toolbar.push(secondary_button(
            i18n.__("查看完整历史"),
            Message::Settings(message::SettingsMessage::CronJobRunsOpen(id.clone())),
        ));
    }
    let mut content = column![toolbar.wrap()].spacing(12).width(Length::Fill);
    content = content.push(text(i18n.__("最近 200 条已结束记录及当前运行任务，筛选仅作用于此范围。"))
        .size(12).style(settings_muted_text_style));
    if state.truncated {
        content = content.push(text(i18n.__("存在更早的运行记录")).size(12));
    }
    if let Some(error) = &state.error {
        content = content
            .push(settings_error_banner(i18n.__(error).to_owned()))
            .push(text(i18n.__("刷新失败，已有数据可能不是最新状态")))
            .push(secondary_button(i18n.__("重试"), event(Event::Refresh)));
    }
    if let Some(label) = empty_label(i18n, state) {
        content = content.push(
            container(text(label).size(14).style(settings_muted_text_style))
                .padding(16)
                .width(Length::Fill)
                .style(|theme| list::surface(theme, false)),
        );
    }
    if state.visible_runs().next().is_none() {
        return content.height(Length::Fill).into();
    }
    content = content.push(responsive(move |size| {
        let narrow = size.width < 760.0;
        let mut rows = column![].spacing(if narrow { 8 } else { 0 });
        for run in state.visible_runs() {
            rows = rows.push(record_row(i18n, run, narrow, timezone));
            if !narrow {
                rows = rows.push(settings_divider());
            }
        }
        if narrow {
            return scroll(rows);
        }
        let header = container(
            row![
                cell(i18n.__("任务名称"), 3),
                cell(i18n.__("状态"), 2),
                cell(i18n.__("运行时间"), 3),
                cell(i18n.__("耗时"), 2),
                cell(i18n.__("任务类型"), 1),
                cell(i18n.__("操作"), 2)
            ]
            .spacing(10),
        )
        .padding([10, 7])
        .width(Length::Fill)
        .style(|theme| list::surface(theme, false));
        column![header, scroll(rows)].spacing(4).height(Length::Fill).into()
    }));
    content.height(Length::Fill).into()
}
pub fn with_details<'a, M: Clone + 'static>(
    app: &ViewData<'a>,
    dialog: Element<'a, M>,
    map: fn(Message) -> M,
) -> Element<'a, M> {
    let i18n = I18n::new(app.desktop_locale);
    let state = &app.cron_settings.history;
    let timezone = app.system_timezone;
    if state.selected_run.is_none() {
        return dialog;
    }
    let close = event(Event::DetailsClosed);
    let header = settings_modal_header(
        text(i18n.__("运行详情")).size(20),
        settings_close_button(close.clone()),
    );
    let mut actions = row![secondary_button(i18n.__("关闭"), close.clone())].spacing(8);
    let body: Element<'_, Message> = if let Some(run) = selected_run(state) {
        if let Some(output) = &run.output {
            actions = actions.push(secondary_button(
                i18n.__("复制输出"),
                event(Event::CopyOutput(output.clone())),
            ));
        }
        column![
            text(if run.name.as_deref().is_none_or(|name| name.trim().is_empty()) {
                i18n.__("未命名任务")
            } else {
                CronHistoryState::display_name(run)
            })
            .size(18),
            text(
                desktop_i18n::__!(
                    i18n,
                    "状态：%s · 本地",
                    (status_label(i18n, &run.status)).to_string()
                )
                .expect("valid settings message")
            ),
            text(
                desktop_i18n::__!(
                    i18n,
                    "开始：%s",
                    (absolute_time(i18n, &run.started_at, timezone)).to_string()
                )
                .expect("valid settings message")
            ),
            text(
                desktop_i18n::__!(
                    i18n,
                    "结束：%s",
                    (run.finished_at
                        .as_deref()
                        .map(|value| absolute_time(i18n, value, timezone))
                        .unwrap_or_else(|| {
                            if run.status == "running" {
                                i18n.__("尚未结束").into()
                            } else {
                                i18n.__("时间缺失").into()
                            }
                        }))
                    .to_string()
                )
                .expect("valid settings message")
            ),
            text(
                desktop_i18n::__!(i18n, "耗时：%s", (duration_label(i18n, run)).to_string())
                    .expect("valid settings message")
            ),
            settings_divider(),
            text(i18n.__("输出（服务端保留的截断内容）")).size(13),
            text(run.output.as_deref().filter(|v| !v.is_empty()).unwrap_or(
                if run.status == "running" {
                    i18n.__("运行中，尚未产生输出")
                } else {
                    i18n.__("无输出")
                }
            ))
            .size(13),
        ]
        .spacing(12)
        .into()
    } else {
        text(i18n.__("该次运行记录已不可用，可能已删除或超出保留范围。请关闭后刷新。")).into()
    };
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .style(cron_modal_style)
        .width(780)
        .height(580);
    desktop_ui_controls::system_settings_common::settings_modal_overlay_with_escape(
        dialog,
        map(close),
        Element::<Message>::from(container(card).padding([10, 12])).map(map),
    )
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;

#[derive(Clone, PartialEq)]
struct HistoryStatusOption {
    value: CronHistoryStatus,
    label: String,
}
impl std::fmt::Display for HistoryStatusOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
