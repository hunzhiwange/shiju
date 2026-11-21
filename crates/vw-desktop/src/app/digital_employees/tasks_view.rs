use std::fmt;

use desktop_assets::assets::Icon;
use desktop_settings_cron::system_settings_cron::date_time::picker_overlay;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SETTINGS_CONTROL_TEXT_SIZE, SettingsSelectLabel, icon_svg,
    primary_action_btn_style, rounded_action_btn_style, select::SettingsSelect,
    settings_checkbox_style, settings_close_button, settings_divider, settings_error_banner,
    settings_group_card, settings_modal_header, settings_modal_layout, settings_modal_overlay_with_escape,
    settings_pick_list_menu_style, settings_pick_list_style, settings_right_drawer_overlay,
    settings_select, settings_text_editor, settings_text_input,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Column, Row, Space, button, checkbox, column, container, image, pick_list, progress_bar,
    row, scrollable, svg, text, tooltip,
};
use iced::{Alignment, Length, Theme};

use super::portraits;
use super::task_api::{self, Directory, Record, Work};
use super::task_schedule::future_once;
use super::task_schedule_view::repeat_view;
use super::tasks::{
    Center, CenterEvent as E, DEPARTMENT_SEARCH_ID, DetailTab, Draft, DraftField, Drawer, Phase,
    ScheduleAction, Sort, Trigger, message,
};
use super::view::{
    Item, Palette, Tone, action, card, divider, format_time, icon, label, page_notice, panel_style,
    segment, segmented, status_label, strong, table_header_style, task_ref, tint_style,
};
use crate::app::Message;

const FAILED: &[&str] = &["FAILED", "REJECTED"];

pub(super) fn view(center: &Center, width: f32) -> Item<'_> {
    let title = strong("任务中心", 24, Tone::Ink).width(Length::Fill);
    let actions = row![
        action("新建定时任务", None, Some(message(E::New(Trigger::Once, String::new()))), false),
        action(
            "新建任务",
            Some(Icon::Plus),
            Some(message(E::New(Trigger::Now, String::new()))),
            true
        ),
    ]
    .spacing(8);
    let heading: Item<'_> = if width >= 650.0 {
        row![title, actions].align_y(Alignment::Center).into()
    } else {
        column![title, actions.wrap()].spacing(12).into()
    };
    let mode = segmented(
        row![
            segment("任务列表", !center.schedules, message(E::ViewSchedules(false))),
            segment("定时任务", center.schedules, message(E::ViewSchedules(true))),
        ]
        .spacing(2)
        .into(),
    );
    let mut body = column![heading, mode].spacing(20).width(Length::Fill);
    if let Some(notice) = center.notice.as_deref() {
        body = body.push(label(notice, 13, Tone::Green));
    }
    if let Some(error) = center.command_error.as_deref() {
        body = body.push(settings_error_banner(error));
    }
    if !center.schedules {
        body = body.push(filters(center, width));
        if let Some(error) = center.directory_error.as_deref() {
            body = body.push(label(format!("员工或流程目录读取失败 · {error}"), 13, Tone::Red));
        }
    }
    if center.loading {
        body = body.push(page_notice(label("正在读取任务…", 14, Tone::Muted)));
    } else if let Some(error) = center.error.as_deref() {
        body = body.push(
            column![
                settings_error_banner(error),
                action("重新加载", None, Some(message(E::Load)), false),
            ]
            .spacing(8),
        );
    } else if center.schedules {
        if let Some(plans) = &center.plans {
            body = body.push(schedules(center, plans));
        }
    } else if let Some(data) = &center.list {
        if let Err(error) = &data.counts {
            body = body.push(label(format!("状态计数暂不可用 · {error}"), 12, Tone::Muted));
        }
        body = body.push(table(center, data, width));
    }
    body.into()
}

fn filters(center: &Center, width: f32) -> Item<'_> {
    let counts = center.list.as_ref().and_then(|data| data.counts.as_ref().ok());
    let mut phases = Row::new().spacing(2);
    for phase in Phase::ALL {
        let count = counts.map_or_else(
            || "—".into(),
            |counts| {
                if phase == Phase::All { counts.total } else { counts.sum(phase.statuses()) }
                    .to_string()
            },
        );
        phases = phases.push(segment(
            format!("{} {count}", phase.label()),
            center.filters.phase == phase,
            message(E::Phase(phase)),
        ));
    }
    let count = |statuses: &[&str]| {
        counts.map_or_else(|| "—".into(), |counts| counts.sum(statuses).to_string())
    };
    let phases = segmented(phases.wrap().into());
    let completed = label(
        format!("成功 {} · 失败或驳回 {}", count(&["SUCCEEDED"]), count(FAILED)),
        12,
        Tone::Muted,
    );
    let overview = row![phases, completed].spacing(10).align_y(Alignment::Center).wrap();
    let keyword = settings_text_input("搜索任务编号、名称或流程", &center.keyword)
        .on_input(|value| message(E::Keyword(value)))
        .on_submit(message(E::Query))
        .width(if width >= 630.0 { Length::Fixed(260.0) } else { Length::Fill });
    let controls = row![
        keyword,
        department_filter(center),
        action("查询", Some(Icon::Search), Some(message(E::Query)), true),
        action(
            if center.expanded { "收起筛选" } else { "更多筛选" },
            None,
            Some(message(E::Expand)),
            false
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let mut body = column![overview, controls].spacing(10);
    if center.expanded {
        let directory = center.directory.as_deref();
        let trigger = center.filters.trigger.map_or("", Trigger::key);
        let mut triggers = vec![Choice::all("全部触发方式")];
        triggers
            .extend(Trigger::ALL.map(|trigger| Choice::new(trigger.key(), trigger.to_string())));
        body = body.push(
            row![
                field(
                    "员工",
                    select(
                        employees(directory, false),
                        &center.filters.employee,
                        "全部员工",
                        E::Employee
                    )
                    .width(168)
                    .into()
                ),
                field(
                    "流程",
                    select(
                        workflows(directory, false),
                        &center.filters.workflow,
                        "全部流程",
                        E::Workflow
                    )
                    .width(168)
                    .into()
                ),
                field(
                    "触发方式",
                    select(triggers, trigger, "全部触发方式", |key| E::TriggerFilter(
                        Trigger::ALL.into_iter().find(|trigger| trigger.key() == key)
                    ))
                    .width(168)
                    .into()
                ),
                field(
                    "排序",
                    select(
                        Sort::ALL.map(|sort| Choice::new(sort.key(), sort.to_string())).to_vec(),
                        center.filters.sort.key(),
                        "最近更新",
                        |key| E::Sort(
                            Sort::ALL
                                .into_iter()
                                .find(|sort| sort.key() == key)
                                .unwrap_or_default()
                        )
                    )
                    .width(168)
                    .into()
                ),
                action("清空筛选", None, Some(message(E::Clear)), false),
            ]
            .spacing(10)
            .align_y(Alignment::End)
            .wrap(),
        );
    }
    body.into()
}

pub(super) fn field<'a>(name: &'static str, component: Item<'a>) -> Item<'a> {
    column![label(name, 12, Tone::Muted), component].spacing(5).into()
}

#[derive(Clone, PartialEq, Eq)]
struct Choice {
    id: String,
    name: String,
}

impl Choice {
    fn new(id: &str, name: impl Into<String>) -> Self {
        Self { id: id.to_owned(), name: name.into() }
    }

    fn all(name: &str) -> Self {
        Self::new("", name)
    }
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

fn select<'a>(
    options: Vec<Choice>,
    id: &str,
    placeholder: &'static str,
    change: impl Fn(String) -> E + 'a,
) -> SettingsSelect<'a, Choice, Message> {
    let active = options.iter().find(|option| option.id == id).cloned();
    settings_select(options, active, move |choice| message(change(choice.id)))
        .placeholder(placeholder)
}

fn department_filter(center: &Center) -> Item<'_> {
    const ALL: &str = "全部可查看部门";
    const WIDTH: f32 = 248.0;
    let Some(departments) = center.departments.as_deref() else {
        return if center.department_error {
            action("部门目录读取失败，重试", None, Some(message(E::LoadDepartments)), false)
        } else {
            action("正在读取部门…", None, None, false)
        };
    };
    let current = center.filters.department.as_str();
    let selected = if current.is_empty() {
        ALL
    } else {
        departments
            .iter()
            .find(|(path, _)| path == current)
            .map_or("当前部门不在目录中", |(_, name)| name.as_str())
    };
    let open = center.department_menu;
    let trigger = button(
        row![
            Item::from(SettingsSelectLabel::new(selected.to_owned())),
            icon_svg(Icon::ChevronDown, 12.0).style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text.scale_alpha(0.7)),
            }),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .height(Length::Fill),
    )
    .width(WIDTH)
    .height(SETTINGS_CONTROL_HEIGHT)
    .padding([9, 12])
    .style(move |theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let state = if open {
            pick_list::Status::Opened { is_hovered: hovered }
        } else if hovered {
            pick_list::Status::Hovered
        } else {
            pick_list::Status::Active
        };
        let style = settings_pick_list_style(theme, state);
        button::Style {
            background: Some(style.background),
            text_color: style.text_color,
            border: iced::Border { radius: 10.0.into(), ..style.border },
            ..Default::default()
        }
    })
    .on_press(message(E::DepartmentMenu(!open)));
    if !open {
        return trigger.into();
    }

    let query = center.department_search.trim().to_lowercase();
    let mut matches = std::iter::once(("", ALL))
        .chain(departments.iter().map(|(path, name)| (path.as_str(), name.as_str())))
        .filter(|(_, name)| name.to_lowercase().contains(&query))
        .peekable();
    let mut options = Column::new().spacing(2);
    if matches.peek().is_none() {
        options = options.push(container(label("未找到匹配的部门", 13, Tone::Muted)).padding(8));
    }
    for (path, name) in matches {
        let is_selected = path == current;
        let mark: Item<'_> = if is_selected {
            icon(Icon::Check, 14, Tone::Ink).into()
        } else {
            Space::new().width(14).into()
        };
        let option =
            button(row![mark, label(name, 14, Tone::Ink)].spacing(8).align_y(Alignment::Center))
                .width(Length::Fill)
                .padding([7, 9])
                .style(move |theme, status| {
                    let mut style = rounded_action_btn_style(theme, status);
                    if is_selected && status == button::Status::Active {
                        style.background =
                            Some(Palette::new(theme).accent.scale_alpha(0.14).into());
                    }
                    style.border.width = 0.0;
                    style
                })
                .on_press(message(E::Department(path.to_owned())));
        options = options.push(option);
    }
    let search = settings_text_input("搜索部门名称", &center.department_search)
        .id(DEPARTMENT_SEARCH_ID)
        .on_input(|query| message(E::DepartmentSearch(query)));
    let list = scrollable(options.padding([0, 7]))
        .direction(Direction::Vertical(scrollbar()))
        .height(Length::Shrink);
    let menu = container(column![search, container(list).max_height(232)].spacing(5))
        .width(WIDTH)
        .padding(7)
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
        .gap(4.0)
        .on_close(message(E::DepartmentMenu(false)))
        .dismiss_on_escape()
        .into()
}

fn employees(directory: Option<&Directory>, dispatch: bool) -> Vec<Choice> {
    let options = directory
        .into_iter()
        .flat_map(|directory| directory.employees.iter())
        .filter(|employee| !dispatch || employee.can_dispatch())
        .map(|employee| Choice::new(&employee.employee_id, &employee.name));
    if dispatch {
        options.collect()
    } else {
        std::iter::once(Choice::all("全部员工")).chain(options).collect()
    }
}

fn workflows(directory: Option<&Directory>, dispatch: bool) -> Vec<Choice> {
    let options = directory
        .into_iter()
        .flat_map(|directory| directory.workflows.iter())
        .filter(|workflow| !dispatch || workflow.version().is_some())
        .map(|workflow| Choice::new(&workflow.workflow_id, &workflow.name));
    if dispatch {
        options.collect()
    } else {
        std::iter::once(Choice::all("全部流程")).chain(options).collect()
    }
}

fn owner<'a>(work: &'a Work, directory: Option<&'a Directory>) -> (&'a str, bool) {
    let employee = directory.and_then(|directory| {
        directory.employees.iter().find(|employee| {
            Some(employee.employee_id.as_str())
                == work.owner_ref.as_deref().or_else(|| work.input_text("employee_id"))
        })
    });
    employee.map_or((work.input_text("employee_name").unwrap_or("—"), true), |employee| {
        (employee.name.as_str(), employee.agent_kind.as_deref() != Some("worker"))
    })
}

fn workflow_name<'a>(work: &'a Work, directory: Option<&'a Directory>) -> &'a str {
    directory
        .and_then(|directory| {
            directory.workflows.iter().find(|workflow| {
                Some(workflow.workflow_id.as_str()) == work.input_text("workflow_id")
            })
        })
        .map(|workflow| workflow.name.as_str())
        .or_else(|| work.input_text("workflow_name"))
        .unwrap_or("—")
}

fn status(work: &Work) -> Item<'_> {
    let (fallback, tone) = match work.status.as_str() {
        "SUCCEEDED" => ("已成功", Tone::Green),
        "FAILED" => ("已失败", Tone::Red),
        "REJECTED" => ("已驳回", Tone::Red),
        "RUNNING" => ("执行中", Tone::Blue),
        "WAITING_APPROVAL" | "WAITING_INPUT" | "HUMAN_TAKEOVER" => ("等人处理", Tone::Warning),
        "CANCELLED" => ("已取消", Tone::Muted),
        "SCHEDULED" => ("已排期", Tone::Muted),
        "CREATED" | "QUEUED" => ("排队中", Tone::Muted),
        _ => (work.status.as_str(), Tone::Muted),
    };
    status_label(work.status_label.as_deref().unwrap_or(fallback), tone)
}

const TABLE_LABELS: [&str; 8] =
    ["任务", "流程", "当前步骤", "状态", "负责人", "进度", "更新时间", "操作"];
const TABLE_PORTIONS: [u16; 9] = [3, 22, 11, 11, 8, 11, 10, 15, 14];

fn table<'a>(center: &'a Center, data: &'a task_api::List, width: f32) -> Item<'a> {
    if data.page.tasks.is_empty() {
        return page_notice(label("没有符合条件的任务", 14, Tone::Muted));
    }
    let all_checked = data.page.tasks.iter().all(|work| center.selected.contains(&work.task_id));
    let head = table_row(
        std::iter::once(selection(all_checked, E::SelectPage, center.busy))
            .chain(TABLE_LABELS.map(|name| strong(name, 12, Tone::Muted).into())),
    );
    let mut rows = column![container(head).style(table_header_style)];
    for work in &data.page.tasks {
        let directory = center.directory.as_deref();
        let (owner, manager) = owner(work, directory);
        let avatar =
            portraits::avatar(owner, manager).map(|handle| image(handle).width(24).height(24));
        let name = button(
            column![
                strong(work.title.as_str(), 13, Tone::Ink),
                label(task_ref(&work.task_id), 11, Tone::Muted),
            ]
            .spacing(3),
        )
        .padding(0)
        .on_press(message(E::Open(work.task_id.clone())))
        .style(|_, _| button::Style::default());
        let progress: Item<'_> = work.progress_pct.map_or_else(
            || label("—", 13, Tone::Muted).into(),
            |pct| {
                row![
                    progress_bar(0.0..=100.0, pct)
                        .girth(4)
                        .length(Length::Fill)
                        .style(progress_style),
                    label(format!("{pct:.0}%"), 12, Tone::Ink),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
            },
        );
        let secondary = action(
            if work.allows("retry") { "详情" } else { "查看进展" },
            None,
            Some(message(E::Open(work.task_id.clone()))),
            false,
        );
        let primary = if work.allows("retry") {
            action(
                "整体重跑",
                None,
                (!center.busy).then(|| message(E::Retry(work.task_id.clone()))),
                true,
            )
        } else {
            secondary
        };
        let more = tooltip(
            action("···", None, Some(cloud_task(work)), false),
            container(label("在云端打开任务", 12, Tone::Ink)).padding(8).style(panel_style),
            tooltip::Position::Top,
        );
        let controls = row![primary, more].spacing(6).wrap().into();
        let cells = table_row([
            selection(
                center.selected.contains(&work.task_id),
                |checked| E::Select(work.task_id.clone(), checked),
                center.busy,
            ),
            name.into(),
            label(workflow_name(work, directory), 13, Tone::Muted).into(),
            label(nonempty(work.current_step.as_deref()), 13, Tone::Muted).into(),
            status(work),
            row![avatar, label(owner, 13, Tone::Ink)].spacing(6).align_y(Alignment::Center).into(),
            progress,
            label(format_time(work.updated_at.as_deref()), 12, Tone::Muted).into(),
            controls,
        ]);
        let selected = center.drawer.as_ref().is_some_and(|drawer| drawer.id == work.task_id);
        let cells = if selected {
            container(cells).style(|theme| tint_style(theme, Tone::Accent, 6.0)).into()
        } else {
            cells
        };
        rows = rows.push(divider()).push(cells);
    }
    let total =
        data.page.total.unwrap_or(center.page * task_api::PAGE_SIZE + data.page.tasks.len());
    let horizontal = if width >= 1000.0 {
        card(rows)
    } else {
        card(
            scrollable(rows.width(1000.0))
                .direction(Direction::Horizontal(scrollbar()))
                .width(Length::Fill),
        )
    };
    let batch = (!center.selected.is_empty()).then(|| {
        let can_retry = data
            .page
            .tasks
            .iter()
            .any(|work| center.selected.contains(&work.task_id) && work.allows("retry"));
        row![
            label(format!("已选 {} 项", center.selected.len()), 13, Tone::Muted),
            action(
                "批量重跑",
                None,
                (can_retry && !center.busy).then(|| message(E::BatchRetry)),
                true
            ),
            action("清空选择", None, (!center.busy).then(|| message(E::SelectPage(false))), false),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    });
    column![batch, horizontal, pagination(center.page, total)].spacing(12).into()
}

fn selection<'a>(checked: bool, on_toggle: impl Fn(bool) -> E + 'a, busy: bool) -> Item<'a> {
    let control = checkbox(checked).size(15).style(settings_checkbox_style);
    if busy {
        control.into()
    } else {
        control.on_toggle(move |checked| message(on_toggle(checked))).into()
    }
}

fn table_row<'a>(cells: impl IntoIterator<Item = Item<'a>>) -> Item<'a> {
    let mut line = Row::new().spacing(10).align_y(Alignment::Center).width(Length::Fill);
    for (item, weight) in cells.into_iter().zip(TABLE_PORTIONS) {
        line = line.push(container(item).width(Length::FillPortion(weight)));
    }
    line.padding([12, 14]).into()
}

fn progress_style(theme: &Theme) -> progress_bar::Style {
    let palette = Palette::new(theme);
    progress_bar::Style {
        background: palette.border.into(),
        bar: palette.accent.into(),
        border: iced::border::rounded(4),
    }
}

fn pagination(page: usize, total: usize) -> Item<'static> {
    let pages = total.div_ceil(task_api::PAGE_SIZE).max(1);
    row![
        label(format!("共 {total} 条 · 第 {} / {pages} 页", page + 1), 13, Tone::Muted),
        Space::new().width(Length::Fill),
        action("上一页", None, (page > 0).then(|| message(E::Page(page - 1))), false),
        action("下一页", None, (page + 1 < pages).then(|| message(E::Page(page + 1))), false),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn schedules<'a>(center: &'a Center, plans: &'a task_api::Schedules) -> Item<'a> {
    if plans.schedules.is_empty() {
        return page_notice(label("暂无定时任务", 14, Tone::Muted));
    }
    let mut body = column![].spacing(10);
    for plan in &plans.schedules {
        let paused = plan.control_state.as_deref() == Some("PAUSE");
        let state = if paused {
            "已暂停"
        } else if plan.enabled {
            "已启用"
        } else {
            "已结束"
        };
        let fire = format_time(plan.next_fire_at.as_deref());
        let time = format!("下次执行 {fire} · {}", nonempty(plan.timezone.as_deref()));
        let toggle = if paused { ScheduleAction::Resume } else { ScheduleAction::Pause };
        let cmd =
            |action| (!center.busy).then(|| message(E::Schedule(plan.schedule_id.clone(), action)));
        body = body.push(card(
            row![
                column![
                    strong(plan.title.as_str(), 14, Tone::Ink),
                    label(task_ref(&plan.schedule_id), 11, Tone::Muted)
                ]
                .spacing(4),
                status_label(
                    state,
                    if paused {
                        Tone::Warning
                    } else if plan.enabled {
                        Tone::Green
                    } else {
                        Tone::Muted
                    }
                ),
                label(nonempty(plan.cron.as_deref()), 13, Tone::Muted),
                label(time, 13, Tone::Muted),
                action(
                    toggle.label(),
                    None,
                    (plan.enabled || paused).then(|| cmd(toggle)).flatten(),
                    false
                ),
                action(
                    ScheduleAction::RunNow.label(),
                    None,
                    plan.enabled.then(|| cmd(ScheduleAction::RunNow)).flatten(),
                    false
                ),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .padding(14)
            .wrap(),
        ));
    }
    let total =
        plans.total.unwrap_or(center.schedule_page * task_api::PAGE_SIZE + plans.schedules.len());
    body.push(pagination(center.schedule_page, total)).into()
}

pub(super) fn overlays<'a>(base: Item<'a>, center: &'a Center, width: f32) -> Item<'a> {
    let base = settings_right_drawer_overlay(
        base,
        center
            .drawer
            .as_ref()
            .map(|drawer| (message(E::Close), drawer_view(center, drawer, width))),
    );
    if let Some(draft) = &center.draft {
        let base = settings_modal_overlay_with_escape(
            base,
            message(E::CloseDraft),
            draft_view(center, draft, width),
        );
        if let Some(picker) = &draft.once_picker {
            picker_overlay(picker, "北京时间（UTC+8）", base, |event| message(E::DatePicker(event)))
        } else {
            base
        }
    } else {
        base
    }
}

fn drawer_view<'a>(center: &'a Center, drawer: &'a Drawer, width: f32) -> Item<'a> {
    let drawer_width = if width < 720.0 { width } else { (width * 0.86).clamp(650.0, 1120.0) };
    let work = drawer.record.as_ref().map(|record| &record.detail.task);
    let title = work.map_or("任务详情", |work| work.title.as_str());
    let heading = row![strong(title, 20, Tone::Ink).width(Length::Fill), work.map(status),]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    let actions = row![
        action(
            if drawer.view_only { "切换到操作模式" } else { "切换到查看模式" },
            None,
            Some(message(E::ViewOnly)),
            false,
        ),
        action(
            "刷新详情",
            Some(Icon::ArrowClockwise),
            (!drawer.loading).then(|| message(E::RefreshDetail)),
            false
        ),
        action("重连实时更新", Some(Icon::ArrowClockwise), Some(message(E::Reconnect)), false),
        action("关闭", None, Some(message(E::Close)), false),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let header: Item<'_> = if drawer_width >= 820.0 {
        row![heading, actions].spacing(12).align_y(Alignment::Center).padding(16).into()
    } else {
        column![heading, actions.wrap()].spacing(10).padding(16).into()
    };
    let tabs = [
        (DetailTab::Overview, "任务概览"),
        (DetailTab::Execution, "执行进度"),
        (DetailTab::Results, "成果与文件"),
        (DetailTab::Collaboration, "协作记录"),
        (DetailTab::Diagnosis, "运行诊断"),
    ];
    let tabs = segmented(
        Row::with_children(
            tabs.map(|(tab, name)| segment(name, drawer.tab == tab, message(E::DetailTab(tab)))),
        )
        .spacing(2)
        .wrap()
        .into(),
    );
    let mut content = Column::new().spacing(14);
    if let Some(error) = drawer.error.as_deref().or(center.command_error.as_deref()) {
        content = content.push(label(error, 13, Tone::Red));
    }
    if work.is_some_and(|task| !Phase::Ended.statuses().contains(&task.status.as_str())) {
        let status = if drawer.fallback {
            "实时连接中断，正在定时刷新"
        } else if drawer.live {
            "实时更新已连接"
        } else {
            "正在连接实时更新…"
        };
        content = content.push(label(status, 12, Tone::Muted));
    }
    if let Some(record) = drawer.record.as_deref() {
        if let Some(blocker) =
            record.detail.task.blocker.as_deref().filter(|value| !value.is_empty())
        {
            content = content.push(label(format!("阻塞 · {blocker}"), 13, Tone::Warning));
        }
        if !drawer.view_only
            && Phase::Waiting.statuses().contains(&record.detail.task.status.as_str())
        {
            content = content.push(action(
                "在云端处理待办",
                None,
                Some(cloud_task(&record.detail.task)),
                false,
            ));
        }
        let tab = match drawer.tab {
            DetailTab::Overview => overview(center, record, drawer.view_only, drawer_width),
            DetailTab::Execution => execution(record),
            DetailTab::Results => results(record),
            DetailTab::Collaboration => collaboration(record),
            DetailTab::Diagnosis => diagnosis(record),
        };
        content = content.push(tab);
    } else if drawer.loading {
        content = content.push(label("加载详情…", 14, Tone::Muted));
    }
    container(column![
        header,
        container(tabs).padding(iced::Padding::new(0.0).left(16.0).right(16.0).bottom(12.0)),
        divider(),
        scrollable(container(content).padding(iced::Padding::new(16.0).right(23.0)))
            .id(iced::widget::Id::new(super::tasks::DETAIL_SCROLL_ID))
            .direction(Direction::Vertical(scrollbar()))
            .height(Length::Fill),
    ])
    .style(|theme| container::Style::default().background(Palette::new(theme).surface))
    .width(drawer_width)
    .height(Length::Fill)
    .into()
}

fn overview<'a>(center: &'a Center, record: &'a Record, view_only: bool, width: f32) -> Item<'a> {
    let directory = center.directory.as_deref();
    let work = &record.detail.task;
    let (owner, _) = owner(work, directory);
    let phase = Phase::ALL
        .into_iter()
        .skip(1)
        .find(|phase| phase.statuses().contains(&work.status.as_str()));
    let progress = work.progress_pct.map_or_else(
        || "未提供".into(),
        |value| format!("{value:.0}%{}", if work.progress_estimated { "（估算）" } else { "" }),
    );
    let next_fire = work
        .next_fire_at
        .as_deref()
        .map(|time| format!(" · {}", format_time(Some(time))))
        .unwrap_or_default();
    let creator = record.creator.as_deref().or_else(|| work.input_text("created_by_name"));
    let department =
        work.department_path.as_deref().filter(|path| !path.is_empty() && *path != "/");
    let department = center
        .departments
        .as_deref()
        .and_then(|departments| {
            departments
                .iter()
                .find(|(path, _)| Some(path.as_str()) == department)
                .map(|(_, name)| name.as_str())
        })
        .or(department)
        .unwrap_or("全公司");
    let left = column![
        meta("发起人", nonempty(creator)),
        meta("流程", workflow_name(work, directory)),
        meta("进度", format!("{} · {progress}", phase.map_or("—", Phase::label))),
        meta("触发方式", format!("{}{next_fire}", nonempty(work.trigger_label.as_deref()))),
        meta("创建时间", format_time(work.created_at.as_deref())),
    ]
    .spacing(14)
    .width(Length::Fill);
    let right = column![
        meta("共享员工", owner),
        meta("当前步骤", nonempty(work.current_step.as_deref())),
        meta("预计费用", "未计入"),
        meta("更新时间", format_time(work.updated_at.as_deref())),
        meta("部门", department),
    ]
    .spacing(14)
    .width(Length::Fill);
    let fields: Item<'_> = if width < 620.0 {
        column![left, right].spacing(14).into()
    } else {
        row![left, right].spacing(20).into()
    };
    let mut body =
        column![label(format!("任务编号 {}", work.task_id), 13, Tone::Muted), fields,].spacing(12);
    if !view_only {
        body = body.push(
            row![
                action("复用配置", None, Some(message(E::Reuse)), false),
                work.allows("retry").then(|| action(
                    "整体重跑",
                    None,
                    Some(message(E::Retry(work.task_id.clone()))),
                    true
                )),
            ]
            .spacing(8),
        );
    }
    body.push(divider()).push(receipt(work, width)).into()
}

fn meta<'a>(name: &'static str, value: impl iced::widget::text::IntoFragment<'a>) -> Item<'a> {
    row![label(name, 13, Tone::Muted).width(84), label(value, 13, Tone::Ink)].spacing(6).into()
}

fn receipt(work: &Work, width: f32) -> Item<'_> {
    let stages = [
        ("派单", "未派单", work.assigned_at.as_deref()),
        ("已读", "等待接收", work.acknowledged_at.as_deref()),
        ("执行中", "未执行", work.started_at.as_deref()),
        ("已交付", "未交付", work.submitted_at.as_deref()),
    ];
    let [first, second, third, fourth] = stages.map(|(name, pending, date)| {
        let tone = if date.is_some() { Tone::Green } else { Tone::Muted };
        column![
            status_label(if date.is_some() { name } else { pending }, tone),
            label(format_time(date), 11, Tone::Muted),
        ]
        .width(Length::Fill)
        .spacing(4)
    });
    let track: Item<'_> = if width >= 620.0 {
        row![first, second, third, fourth].spacing(12).into()
    } else {
        column![row![first, second].spacing(12), row![third, fourth].spacing(12)].spacing(10).into()
    };
    let elapsed = work
        .assigned_at
        .as_deref()
        .zip(work.submitted_at.as_deref())
        .and_then(|(start, end)| {
            Some(
                (chrono::DateTime::parse_from_rfc3339(end).ok()?
                    - chrono::DateTime::parse_from_rfc3339(start).ok()?)
                .num_milliseconds(),
            )
        })
        .and_then(|ms| u64::try_from(ms).ok());
    let hint = row![
        work.acknowledgement_latency_ms.map(|ms| label(
            format!("已读用时 {}", duration(ms)),
            12,
            Tone::Muted
        )),
        elapsed.map(|ms| label(format!("总耗时 {}", duration(ms)), 12, Tone::Muted)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    let mut body = column![strong("执行回执", 15, Tone::Ink), track].spacing(10);
    if work.acknowledgement_latency_ms.is_some() || elapsed.is_some() {
        body = body.push(hint);
    }
    body.into()
}

fn duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{:.1} 秒", ms as f64 / 1000.0)
    } else if ms < 60_000 {
        format!("{} 秒", ms / 1000)
    } else {
        format!("{} 分 {} 秒", ms / 60_000, ms / 1000 % 60)
    }
}

fn execution(record: &Record) -> Item<'_> {
    let mut steps = Column::new().spacing(14);
    for (index, step) in record.detail.steps.iter().enumerate() {
        steps = steps.push(
            column![
                row![
                    strong(
                        format!("{}. {}", index + 1, nonempty(step.name.as_deref())),
                        14,
                        Tone::Ink
                    )
                    .width(Length::Fill),
                    label(step_status(&step.status), 12, Tone::Muted),
                ]
                .spacing(8),
                label(nonempty(step.actor_name.as_deref()), 12, Tone::Muted),
                step.summary.as_deref().map(|summary| label(summary, 13, Tone::Ink)),
            ]
            .spacing(5),
        );
    }
    if record.detail.steps.is_empty() {
        steps = steps.push(label("暂无执行步骤", 13, Tone::Muted));
    }
    column![steps, divider(), strong("事件记录", 15, Tone::Ink), log_rows(record)]
        .spacing(14)
        .into()
}

fn step_status(status: &str) -> &str {
    match status {
        "PENDING" => "待执行",
        "RUNNING" => "执行中",
        "SUCCEEDED" => "已完成",
        "FAILED" => "已失败",
        "SKIPPED" => "已跳过",
        _ => status,
    }
}

fn log_rows(record: &Record) -> Item<'_> {
    match &record.events {
        Ok(page) if page.events.is_empty() => label("暂无事件", 13, Tone::Muted).into(),
        Ok(page) => Column::with_children(page.events.iter().map(|event| {
            row![
                label(event.event_type.as_str(), 13, Tone::Ink).width(Length::Fill),
                label(format_time(event.occurred_at.as_deref()), 12, Tone::Muted),
            ]
            .spacing(8)
            .into()
        }))
        .spacing(6)
        .into(),
        Err(error) => label(error.as_str(), 13, Tone::Red).into(),
    }
}

fn results(record: &Record) -> Item<'_> {
    let mut body = column![strong("成果与文件", 15, Tone::Ink)].spacing(12);
    let artifacts = record
        .detail
        .deliverables
        .as_ref()
        .map_or(record.detail.artifacts.as_slice(), |items| items.items.as_slice());
    for artifact in artifacts {
        body = body.push(
            row![
                strong(artifact.name.as_str(), 13, Tone::Ink),
                label(nonempty(artifact.content_type.as_deref()), 12, Tone::Muted),
                artifact.byte_size().map(|size| label(format!("{size} 字节"), 12, Tone::Muted)),
            ]
            .spacing(10)
            .wrap(),
        );
    }
    match &record.handoff {
        Ok(handoff) => {
            for output in &handoff.outputs {
                body = body.push(meta("交接产出", nonempty(output.name.as_deref())));
            }
            for report in &handoff.reports {
                if let Some(summary) = report.summary.as_deref() {
                    body = body.push(meta("汇报", summary));
                }
            }
            if artifacts.is_empty() && handoff.outputs.is_empty() && handoff.reports.is_empty() {
                body = body.push(label(
                    record
                        .detail
                        .deliverables
                        .as_ref()
                        .and_then(|items| items.empty_state.as_deref())
                        .unwrap_or("暂无成果与文件"),
                    13,
                    Tone::Muted,
                ));
            }
        }
        Err(error) => body = body.push(label(error.as_str(), 13, Tone::Red)),
    }
    if !artifacts.is_empty() {
        body = body.push(action(
            "在云端查看或下载文件",
            None,
            Some(cloud_task(&record.detail.task)),
            false,
        ));
    }
    body.into()
}

fn collaboration(record: &Record) -> Item<'_> {
    let data = match &record.collaboration {
        Ok(data) => data,
        Err(error) => return label(error.as_str(), 13, Tone::Red).into(),
    };
    let mut body = column![].spacing(14);
    if !data.record_complete {
        body = body.push(label("协作记录可能未完全同步", 13, Tone::Warning));
    }
    if data.entries.is_empty() {
        body = body.push(label("暂无协作记录", 13, Tone::Muted));
    }
    for entry in &data.entries {
        let kind = match entry.kind.as_str() {
            "DISPATCH" => "派单",
            "ACKNOWLEDGEMENT" => "已读回执",
            "EMPLOYEE_REPORT" => "员工汇报",
            "HUMAN_MESSAGE" => "人工留言",
            other => other,
        };
        body = body.push(
            column![
                row![
                    strong(kind, 13, Tone::Ink),
                    label(entry.actor.label.as_str(), 13, Tone::Ink),
                    label(format_time(entry.occurred_at.as_deref()), 12, Tone::Muted)
                ]
                .spacing(8)
                .wrap(),
                entry.summary.as_deref().map(|summary| label(summary, 13, Tone::Muted)),
            ]
            .spacing(5),
        );
    }
    body.into()
}

fn diagnosis(record: &Record) -> Item<'_> {
    let mut body = column![].spacing(14);
    if let Some(runtime) = &record.detail.runtime {
        body = body.push(
            column![
                strong("运行环境", 15, Tone::Ink),
                meta("运行后端", nonempty(runtime.backend.as_deref())),
                meta("运行状态", nonempty(runtime.ensure_status.as_deref())),
            ]
            .spacing(8),
        );
    }
    match &record.trace {
        Ok(page) if page.entries.is_empty() => {
            body = body.push(label("暂无运行轨迹", 13, Tone::Muted))
        }
        Ok(page) => {
            for trace in &page.entries {
                body = body.push(
                    column![
                        row![
                            strong(
                                nonempty(trace.title.as_deref().or(trace.kind.as_deref())),
                                13,
                                Tone::Ink
                            ),
                            label(format_time(trace.started_at.as_deref()), 12, Tone::Muted),
                            trace.duration_ms.map(|ms| label(duration(ms), 12, Tone::Muted)),
                        ]
                        .spacing(8)
                        .wrap(),
                        trace.summary.as_deref().map(|summary| label(summary, 13, Tone::Muted)),
                    ]
                    .spacing(5),
                );
            }
        }
        Err(error) => body = body.push(label(error.as_str(), 13, Tone::Red)),
    }
    body.into()
}

fn draft_view<'a>(center: &'a Center, draft: &'a Draft, width: f32) -> Item<'a> {
    let directory = center.directory.as_deref();
    let editable = !center.busy;
    let narrow = width < 740.0;
    let mut prompt = settings_text_editor(&draft.prompt)
        .placeholder("说明任务目标、已有资料和预期交付成果")
        .height(if narrow { 190 } else { 330 });
    if editable {
        prompt = prompt.on_action(|action| message(E::EditPrompt(action)));
    }

    let select_field = |name, options, id: &str, placeholder, draft_field| {
        field(
            name,
            select(options, id, placeholder, move |value| E::Edit(draft_field, value))
                .width(Length::Fill)
                .disabled(!editable)
                .into(),
        )
    };
    let employee = select_field(
        "共享员工",
        employees(directory, true),
        &draft.employee,
        "选择可派工的共享员工",
        DraftField::Employee,
    );
    let workflow = select_field(
        "流程",
        workflows(directory, true),
        &draft.workflow,
        "选择已发布的流程",
        DraftField::Workflow,
    );
    let mut departments = vec![Choice::all("使用默认范围")];
    departments.extend(
        center
            .departments
            .as_deref()
            .into_iter()
            .flatten()
            .map(|(path, name)| Choice::new(path, name.as_str())),
    );
    if !draft.department.is_empty()
        && !departments.iter().any(|choice| choice.id == draft.department)
    {
        departments.push(Choice::new(&draft.department, &draft.department));
    }
    let department = select_field(
        "任务归属部门",
        departments,
        &draft.department,
        "使用默认范围",
        DraftField::Department,
    );
    let trigger = field(
        "什么时候开始",
        settings_select(Trigger::ALL, Some(draft.trigger), |trigger| {
            message(E::DraftTrigger(trigger))
        })
        .width(Length::Fill)
        .disabled(!editable)
        .into(),
    );

    let content = column![
        field(
            "任务名称",
            settings_text_input("输入任务名称", &draft.title)
                .width(Length::Fill)
                .on_input_maybe(
                    editable.then_some(|value| message(E::Edit(DraftField::Title, value)))
                )
                .into(),
        ),
        field("任务说明", prompt.into()),
    ]
    .spacing(16);
    let mut settings = column![employee, workflow, department, trigger].spacing(12);
    settings = match draft.trigger {
        Trigger::Now => settings,
        Trigger::Once => settings.push(once_view(draft, editable)),
        Trigger::Cron => {
            settings.push(settings_divider()).push(repeat_view(&draft.repeat, editable))
        }
    };
    if let Some(error) = center.directory_error.as_deref() {
        settings = settings.push(settings_error_banner(error));
    }
    let settings = settings_group_card(settings).padding([16, 18]);
    let form: Item<'_> = if narrow {
        column![content, settings].spacing(18).into()
    } else {
        row![content.width(Length::FillPortion(3)), settings.width(Length::FillPortion(2))]
            .spacing(24)
            .into()
    };

    let close = message(E::CloseDraft);
    let scheduled = draft.trigger != Trigger::Now;
    let heading = settings_modal_header(
        strong(if scheduled { "新建定时任务" } else { "新建任务" }, 20, Tone::Ink),
        settings_close_button(close.clone()),
    );
    let header = column![heading, settings_divider()].spacing(12).padding([0, 7]);

    let actions = row![
        button(text("取消").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(close)),
        button(
            text(if editable { "创建任务" } else { "正在提交…" }).size(SETTINGS_CONTROL_TEXT_SIZE)
        )
        .padding([9, 14])
        .style(primary_action_btn_style)
        .on_press_maybe((editable && directory.is_some()).then(|| message(E::Submit))),
    ]
    .spacing(8);
    let footer = column![
        center.command_error.as_deref().map(settings_error_banner),
        settings_divider(),
        container(actions).align_right(Length::Fill),
    ]
    .spacing(12)
    .padding([0, 7]);

    let height: u32 = if narrow || draft.trigger == Trigger::Cron { 720 } else { 610 };
    settings_modal_layout(header, form, Some(footer.into()))
        .width(Length::Fill)
        .max_width(960)
        .height(Length::Fill)
        .max_height(height)
        .into()
}

fn once_view(draft: &Draft, editable: bool) -> Item<'static> {
    let value = draft.once_at.map(|time| time.format("%Y-%m-%d %H:%M").to_string());
    let picker = button(
        row![
            icon_svg(Icon::Calendar, 16.0),
            text(value.unwrap_or_else(|| "选择日期和时间".into()))
                .size(SETTINGS_CONTROL_TEXT_SIZE)
                .width(Length::Fill),
            icon_svg(Icon::ChevronDown, 12.0),
        ]
        .spacing(8)
        .height(Length::Fill)
        .align_y(Alignment::Center),
    )
    .height(SETTINGS_CONTROL_HEIGHT)
    .width(Length::Fill)
    .padding([0, 12])
    .style(rounded_action_btn_style)
    .on_press_maybe(editable.then(|| message(E::OpenDatePicker)));
    let clear = draft.once_at.is_some().then(|| {
        button(text("清空").size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 10])
            .height(SETTINGS_CONTROL_HEIGHT)
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then(|| message(E::ClearOnce)))
    });
    let error = draft.once_at.and_then(|time| future_once(time).err());
    column![
        field("执行时间（北京时间）", row![picker, clear].spacing(8).into()),
        error.map(|error| label(error, 12, Tone::Red)),
    ]
    .spacing(6)
    .into()
}

fn cloud_task(work: &Work) -> Message {
    use crate::app::message::view::ViewMessage;
    Message::View(ViewMessage::OpenUrlExternal(super::api::console_url(
        "/tasks",
        Some(("task", &work.task_id)),
    )))
}

fn nonempty(value: Option<&str>) -> &str {
    value.filter(|value| !value.is_empty()).unwrap_or("—")
}

fn scrollbar() -> Scrollbar {
    Scrollbar::new().width(4).scroller_width(4).margin(3)
}
