use std::{collections::BTreeMap, fmt};

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style,
    settings_modal_card, settings_modal_overlay_with_escape, settings_pick_list_menu_style,
    settings_pick_list_style, settings_right_drawer_overlay, settings_text_input_style,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Column, Row, Space, Stack, button, checkbox, column, container, image, pick_list, progress_bar,
    responsive, row, scrollable, text_input, tooltip,
};
use iced::{Alignment, Length, Theme};

use super::portraits;
use super::task_api::{self, Directory, Record, Work};
use super::tasks::{
    Center, CenterEvent as E, DetailTab, Draft, DraftField, Drawer, Phase, ScheduleAction, Sort,
    Trigger, message,
};
use super::view::{
    Item, Palette, Tone, action, card, chip, divider, format_time, icon, label, panel_style,
    strong, task_ref, tint_style,
};
use crate::app::Message;

const FAILED: &[&str] = &["FAILED", "REJECTED"];

pub(super) fn view(center: &Center, width: f32) -> Item<'_> {
    let title = column![
        strong("任务中心", 25, Tone::Ink),
        label("看任务跑到哪了，哪些在等你处理", 14, Tone::Muted),
    ]
    .spacing(8)
    .width(Length::Fill);
    let actions = row![
        action("新建定时任务", None, Some(message(E::New(Trigger::Once, String::new()))), false),
        action("+ 新建任务", None, Some(message(E::New(Trigger::Now, String::new()))), true),
    ]
    .spacing(8);
    let heading: Item<'_> = if width >= 650.0 {
        row![title, actions].align_y(Alignment::Center).into()
    } else {
        column![title, actions].spacing(12).into()
    };
    let mode = segmented(
        row![
            segment("任务列表", !center.schedules, E::ViewSchedules(false)),
            segment("定时任务", center.schedules, E::ViewSchedules(true)),
        ]
        .spacing(4)
        .into(),
    );
    let mut body = column![heading, mode].spacing(16).width(Length::Fill);
    if let Some(notice) = center.notice.as_deref() {
        body = body.push(label(notice, 13, Tone::Green));
    }
    if let Some(error) = center.command_error.as_deref() {
        body = body.push(label(error, 13, Tone::Red));
    }
    if !center.schedules {
        body = body.push(stats(center, width)).push(filters(center));
        if let Some(error) = center.directory_error.as_deref() {
            body = body.push(label(format!("员工或流程目录读取失败 · {error}"), 13, Tone::Red));
        }
    }
    if center.loading {
        body = body.push(card(container(label("正在读取任务…", 14, Tone::Muted)).padding(16)));
    } else if let Some(error) = center.error.as_deref() {
        body = body.push(card(
            column![
                label(error, 14, Tone::Red),
                action("重新加载", None, Some(message(E::Load)), false),
            ]
            .spacing(8)
            .padding(16),
        ));
    } else if center.schedules {
        if let Some(plans) = &center.plans {
            body = body.push(schedules(center, plans));
        }
    } else if let Some(data) = &center.list {
        if let Err(error) = &data.counts {
            body = body.push(label(format!("状态计数暂不可用 · {error}"), 12, Tone::Muted));
        }
        body = body.push(Space::new().height(12)).push(table(center, data, width));
    }
    body.into()
}

fn stats(center: &Center, width: f32) -> Item<'_> {
    let counts = center.list.as_ref().and_then(|data| data.counts.as_ref().ok());
    let cards = [
        ("执行中", Icon::ArrowRight, Tone::Blue, Phase::Running.statuses()),
        ("等人处理", Icon::Clock, Tone::Green, Phase::Waiting.statuses()),
        ("已完成", Icon::Check, Tone::Green, &["SUCCEEDED"][..]),
        ("失败或被驳回", Icon::X, Tone::Red, FAILED),
    ];
    let mut cells = cards.into_iter().map(|(name, glyph, tone, statuses)| {
        let count = counts.map_or_else(|| "—".into(), |counts| counts.sum(statuses).to_string());
        card(
            row![
                container(icon(glyph, 18, tone))
                    .padding(10)
                    .style(move |theme| tint_style(theme, tone, 8.0)),
                column![label(name, 13, Tone::Muted), strong(count, 24, Tone::Ink)].spacing(7),
            ]
            .spacing(12)
            .padding(14),
        )
    });
    let top = row![cells.next(), cells.next()].spacing(12);
    if width >= 760.0 {
        top.push(cells.next()).push(cells.next()).into()
    } else {
        column![top, row![cells.next(), cells.next()].spacing(12)].spacing(12).into()
    }
}

fn filters(center: &Center) -> Item<'_> {
    let counts = center.list.as_ref().and_then(|data| data.counts.as_ref().ok());
    let mut phases = Row::new().spacing(4);
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
            E::Phase(phase),
        ));
    }
    let mut departments = vec![Choice::all("全部可查看部门")];
    departments.extend(department_options(center.directory.as_deref()));
    let scope = field(
        "筛选部门",
        select(departments, &center.filters.department, "全部可查看部门", E::Department),
    );
    let keyword = field(
        "关键字",
        text_input("搜索任务编号、名称或流程", &center.keyword)
            .on_input(|value| message(E::Keyword(value)))
            .on_submit(message(E::Query))
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .width(240.0)
            .style(settings_text_input_style)
            .into(),
    );
    let controls = row![
        segmented(phases.wrap().into()),
        scope,
        keyword,
        action("查询", Some(Icon::Search), Some(message(E::Query)), true),
        action(
            if center.expanded { "收起" } else { "展开" },
            None,
            Some(message(E::Expand)),
            false
        ),
    ]
    .spacing(10)
    .align_y(Alignment::End)
    .wrap();
    let mut body = column![controls].spacing(10).padding(15);
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
                ),
                field(
                    "流程",
                    select(
                        workflows(directory, false),
                        &center.filters.workflow,
                        "全部流程",
                        E::Workflow
                    )
                ),
                field(
                    "触发方式",
                    select(triggers, trigger, "全部触发方式", |key| E::TriggerFilter(
                        Trigger::ALL.into_iter().find(|trigger| trigger.key() == key)
                    ))
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
                ),
                action("清空筛选", None, Some(message(E::Clear)), false),
            ]
            .spacing(10)
            .align_y(Alignment::End)
            .wrap(),
        );
    }
    card(body)
}

fn segment<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    selected: bool,
    event: E,
) -> Item<'a> {
    button(strong(title, 13, if selected { Tone::Accent } else { Tone::Muted }))
        .padding([8, 11])
        .on_press(message(event))
        .style(move |theme, status| {
            let palette = Palette::new(theme);
            button::Style {
                background: if selected {
                    Some(palette.accent.scale_alpha(0.12).into())
                } else if status != button::Status::Active {
                    Some(palette.border.scale_alpha(0.5).into())
                } else {
                    None
                },
                border: iced::border::rounded(7),
                text_color: palette.ink,
                ..Default::default()
            }
        })
        .into()
}

fn segmented(content: Item<'_>) -> Item<'_> {
    container(content)
        .padding(4)
        .style(|theme| panel_style(theme).background(Palette::new(theme).header))
        .into()
}

fn field<'a>(name: &'static str, component: Item<'a>) -> Item<'a> {
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
) -> Item<'a> {
    let active = options.iter().find(|option| option.id == id).cloned();
    pick_list(options, active, move |choice| message(change(choice.id)))
        .placeholder(placeholder)
        .padding(SETTINGS_CONTROL_PADDING)
        .text_size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .into()
}

fn department_options(directory: Option<&Directory>) -> Vec<Choice> {
    let Some(directory) = directory else { return Vec::new() };
    let mut paths: BTreeMap<_, _> = directory
        .departments
        .iter()
        .filter(|(path, _)| !path.is_empty() && path != "/")
        .map(|(path, name)| (path.as_str(), name.as_str()))
        .collect();
    for path in
        directory.employees.iter().filter_map(|employee| employee.department_path.as_deref())
    {
        if !path.is_empty() && path != "/" {
            paths.entry(path).or_insert(path);
        }
    }
    paths.into_iter().map(|(path, name)| Choice::new(path, name)).collect()
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
    chip(strong(work.status_label.as_deref().unwrap_or(fallback), 12, tone), tone)
}

const TABLE_LABELS: [&str; 8] =
    ["任务", "流程", "当前步骤", "状态", "负责人", "进度", "更新时间", "操作"];
const TABLE_PORTIONS: [u16; 9] = [3, 22, 11, 11, 8, 11, 10, 15, 14];

fn table<'a>(center: &'a Center, data: &'a task_api::List, width: f32) -> Item<'a> {
    if data.page.tasks.is_empty() {
        return card(container(label("没有符合条件的任务", 14, Tone::Muted)).padding(20));
    }
    let all_checked = data.page.tasks.iter().all(|work| center.selected.contains(&work.task_id));
    let head = table_row(
        std::iter::once(selection(all_checked, E::SelectPage, center.busy))
            .chain(TABLE_LABELS.map(|name| strong(name, 12, Tone::Muted).into())),
    );
    let mut rows = column![
        container(head)
            .style(|theme| { container::Style::default().background(Palette::new(theme).header) })
    ];
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
        rows = rows.push(divider()).push(container(cells).style(move |theme| {
            let palette = Palette::new(theme);
            container::Style::default().background(if selected {
                palette.accent.scale_alpha(0.12)
            } else {
                palette.surface
            })
        }));
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
        return card(container(label("暂无定时任务", 14, Tone::Muted)).padding(20));
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
                label(state, 13, if paused { Tone::Warning } else { Tone::Muted }),
                label(nonempty(plan.cron.as_deref()), 13, Tone::Muted),
                label(time, 13, Tone::Muted),
                Space::new().width(Length::Fill),
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
            .wrap()
            .padding(14),
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
        settings_modal_overlay_with_escape(base, message(E::CloseDraft), draft_view(center, draft))
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
    let mut nav = Row::new().spacing(5);
    for (tab, name) in tabs {
        let control = segment(name, drawer.tab == tab, E::DetailTab(tab));
        if drawer_width >= 490.0 {
            nav = nav.push(container(control).center_x(Length::Fill));
        } else {
            nav = nav.push(control);
        }
    }
    let tabs =
        if drawer_width >= 490.0 { nav.width(Length::Fill).into() } else { nav.wrap().into() };
    let mut content = column![segmented(tabs)].spacing(12);
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
            DetailTab::Overview => {
                overview(center.directory.as_deref(), record, drawer.view_only, drawer_width)
            }
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

fn overview<'a>(
    directory: Option<&'a Directory>,
    record: &'a Record,
    view_only: bool,
    width: f32,
) -> Item<'a> {
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
    let department = directory
        .and_then(|directory| {
            directory
                .departments
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
    let mut body = column![
        container(label(format!("任务编号 {}", work.task_id), 13, Tone::Muted))
            .padding(12)
            .width(Length::Fill)
            .style(|theme| container::Style::default()
                .background(Palette::new(theme).header)
                .border(iced::border::rounded(8))),
        card(container(fields).padding(16)),
    ]
    .spacing(12);
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
    body.push(receipt(work)).into()
}

fn meta<'a>(name: &'static str, value: impl iced::widget::text::IntoFragment<'a>) -> Item<'a> {
    row![label(name, 13, Tone::Muted).width(84), label(value, 13, Tone::Ink)].spacing(6).into()
}

fn receipt(work: &Work) -> Item<'_> {
    let stages = [
        ("派单", "未派单", work.assigned_at.as_deref()),
        ("已读", "等待接收", work.acknowledged_at.as_deref()),
        ("执行中", "未执行", work.started_at.as_deref()),
        ("已交付", "未交付", work.submitted_at.as_deref()),
    ];
    let mut track = Row::new().spacing(8);
    for (name, pending, date) in stages {
        let done = date.is_some();
        track = track.push(
            container(
                column![
                    strong(
                        if done { "●" } else { "○" },
                        15,
                        if done { Tone::Green } else { Tone::Muted }
                    ),
                    strong(
                        if done { name } else { pending },
                        12,
                        if done { Tone::Ink } else { Tone::Muted }
                    ),
                    label(format_time(date), 11, Tone::Muted).align_x(Alignment::Center),
                ]
                .align_x(Alignment::Center)
                .spacing(4),
            )
            .center_x(Length::Fill),
        );
    }
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
        work.acknowledgement_latency_ms.map(|ms| chip(
            strong(format!("已读用时 {}", duration(ms)), 12, Tone::Accent),
            Tone::Accent
        )),
        elapsed.map(|ms| label(format!("总耗时 {}", duration(ms)), 12, Tone::Muted)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let line = row![
        Space::new().width(Length::FillPortion(1)),
        container(divider())
            .width(Length::FillPortion(6))
            .padding(iced::Padding::new(0.0).top(10.0)),
        Space::new().width(Length::FillPortion(1)),
    ];
    let track = Stack::new().push(track).push_under(line);
    column![card(container(track).padding(12)), hint].spacing(8).into()
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
    let mut steps = Column::new().spacing(8);
    for (index, step) in record.detail.steps.iter().enumerate() {
        steps = steps.push(card(
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
            .spacing(6)
            .padding(12),
        ));
    }
    if record.detail.steps.is_empty() {
        steps = steps.push(label("暂无执行步骤", 13, Tone::Muted));
    }
    column![steps, strong("事件记录", 15, Tone::Ink), log_rows(record)].spacing(12).into()
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
    let mut body = column![strong("成果与文件", 15, Tone::Ink)].spacing(8);
    let artifacts = record
        .detail
        .deliverables
        .as_ref()
        .map_or(record.detail.artifacts.as_slice(), |items| items.items.as_slice());
    for artifact in artifacts {
        body = body.push(card(
            row![
                strong(artifact.name.as_str(), 13, Tone::Ink).width(Length::Fill),
                label(nonempty(artifact.content_type.as_deref()), 12, Tone::Muted),
                artifact.byte_size.map(|size| label(format!("{size} 字节"), 12, Tone::Muted)),
            ]
            .spacing(10)
            .wrap()
            .padding(12),
        ));
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
    let mut body = column![].spacing(8);
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
        body = body.push(card(
            column![
                row![
                    strong(kind, 13, Tone::Ink),
                    label(entry.actor.label.as_str(), 13, Tone::Ink).width(Length::Fill),
                    label(format_time(entry.occurred_at.as_deref()), 12, Tone::Muted)
                ]
                .spacing(8)
                .wrap(),
                entry.summary.as_deref().map(|summary| label(summary, 13, Tone::Muted)),
            ]
            .spacing(6)
            .padding(12),
        ));
    }
    body.into()
}

fn diagnosis(record: &Record) -> Item<'_> {
    let mut body = column![].spacing(8);
    if let Some(runtime) = &record.detail.runtime {
        body = body.push(card(
            column![
                meta("运行后端", nonempty(runtime.backend.as_deref())),
                meta("运行状态", nonempty(runtime.ensure_status.as_deref())),
            ]
            .spacing(8)
            .padding(12),
        ));
    }
    match &record.trace {
        Ok(page) if page.entries.is_empty() => {
            body = body.push(label("暂无运行轨迹", 13, Tone::Muted))
        }
        Ok(page) => {
            for trace in &page.entries {
                body = body.push(card(
                    column![
                        row![
                            strong(
                                nonempty(trace.title.as_deref().or(trace.kind.as_deref())),
                                13,
                                Tone::Ink
                            )
                            .width(Length::Fill),
                            label(format_time(trace.started_at.as_deref()), 12, Tone::Muted),
                            trace.duration_ms.map(|ms| label(duration(ms), 12, Tone::Muted)),
                        ]
                        .spacing(8)
                        .wrap(),
                        trace.summary.as_deref().map(|summary| label(summary, 13, Tone::Muted)),
                    ]
                    .spacing(6)
                    .padding(12),
                ));
            }
        }
        Err(error) => body = body.push(label(error.as_str(), 13, Tone::Red)),
    }
    body.into()
}

fn draft_view<'a>(center: &'a Center, draft: &'a Draft) -> Item<'a> {
    responsive(move |size| {
        let directory = center.directory.as_deref();
        let edit = |field, title, placeholder, value: &'a str| {
            field_input(title, placeholder, value, field, center.busy)
        };
        let mut form = column![
            edit(DraftField::Title, "任务名称", "输入任务名称", &draft.title),
            field(
                "共享员工",
                select(
                    employees(directory, true),
                    &draft.employee,
                    "选择可派工的共享员工",
                    |value| E::Edit(DraftField::Employee, value)
                )
            ),
            field(
                "流程",
                select(
                    workflows(directory, true),
                    &draft.workflow,
                    "选择已发布的流程",
                    |value| E::Edit(DraftField::Workflow, value)
                )
            ),
            edit(DraftField::Prompt, "任务要求", "输入任务要求", &draft.prompt),
            field("部门", {
                let mut departments = vec![Choice::all("使用默认范围")];
                departments.extend(department_options(directory));
                if !draft.department.is_empty()
                    && !departments.iter().any(|item| item.id == draft.department)
                {
                    departments.push(Choice::new(&draft.department, &draft.department));
                }
                select(departments, &draft.department, "使用默认范围", |value| {
                    E::Edit(DraftField::Department, value)
                })
            }),
            field(
                "触发方式",
                select(
                    Trigger::ALL
                        .map(|trigger| Choice::new(trigger.key(), trigger.to_string()))
                        .to_vec(),
                    draft.trigger.key(),
                    "触发方式",
                    |key| E::DraftTrigger(
                        Trigger::ALL
                            .into_iter()
                            .find(|trigger| trigger.key() == key)
                            .unwrap_or_default()
                    )
                )
            ),
        ]
        .spacing(10)
        .width(Length::Fill);
        match draft.trigger {
            Trigger::Once => {
                form = form.push(edit(
                    DraftField::Time,
                    "执行时间（北京时间）",
                    "YYYY-MM-DD HH:MM",
                    &draft.time,
                ))
            }
            Trigger::Cron => {
                form =
                    form.push(edit(DraftField::Time, "Cron（北京时间）", "0 12 * * *", &draft.time))
            }
            Trigger::Now => {}
        }
        if let Some(error) = center.command_error.as_deref().or(center.directory_error.as_deref()) {
            form = form.push(label(error, 13, Tone::Red));
        }
        let width = (size.width - 32.0).clamp(260.0, 540.0);
        container(
            settings_modal_card(
                column![
                    strong(
                        if draft.trigger == Trigger::Now {
                            "新建任务"
                        } else {
                            "新建定时任务"
                        },
                        20,
                        Tone::Ink
                    ),
                    scrollable(form.padding(iced::Padding::new(0.0).right(7.0)))
                        .direction(Direction::Vertical(scrollbar()))
                        .height(Length::Fill),
                    row![
                        Space::new().width(Length::Fill),
                        action("取消", None, (!center.busy).then(|| message(E::CloseDraft)), false),
                        action(
                            if center.busy { "正在提交…" } else { "创建任务" },
                            None,
                            (!center.busy && directory.is_some()).then(|| message(E::Submit)),
                            true
                        ),
                    ]
                    .spacing(8),
                ]
                .spacing(14),
            )
            .width(width),
        )
        .height((size.height - 32.0).clamp(230.0, 620.0))
        .center_x(Length::Fill)
        .into()
    })
    .into()
}

fn field_input<'a>(
    title: &'static str,
    placeholder: &'static str,
    value: &'a str,
    draft_field: DraftField,
    disabled: bool,
) -> Item<'a> {
    let input = text_input(placeholder, value)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style);
    field(
        title,
        if disabled {
            input.into()
        } else {
            input.on_input(move |value| message(E::Edit(draft_field, value))).into()
        },
    )
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
