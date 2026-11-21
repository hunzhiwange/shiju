use desktop_assets::assets::{Icon, get_icon};
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_error_banner,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Column, Row, Space, Stack, button, canvas, column, container, image, responsive, row, rule,
    scrollable, svg, text,
};
use iced::{Alignment, Border, Color, Element, Font, Length, Theme, border};

use crate::app::message::view::ViewMessage;
use crate::app::{App, Message};

use super::api::{self, Activity, Employee, Summary, Trend};
use super::portraits;
use super::trend::{Chart, Metric};
use super::{Event, Page, State};

pub(super) type Item<'a> = Element<'a, Message>;

pub(super) struct Palette {
    pub surface: Color,
    page: Color,
    pub header: Color,
    pub border: Color,
    pub ink: Color,
    pub muted: Color,
    pub accent: Color,
    pub blue: Color,
    green: Color,
    red: Color,
    violet: Color,
}

impl Palette {
    pub fn new(theme: &Theme) -> Self {
        if theme.extended_palette().is_dark {
            let palette = theme.extended_palette();
            return Self {
                surface: palette.background.weak.color,
                page: palette.background.base.color,
                header: palette.background.base.color,
                border: palette.background.strong.color,
                ink: theme.palette().text,
                muted: theme.palette().text.scale_alpha(0.72),
                accent: Color::from_rgb8(249, 137, 92),
                blue: Color::from_rgb8(131, 177, 236),
                green: Color::from_rgb8(166, 197, 137),
                red: Color::from_rgb8(246, 145, 138),
                violet: Color::from_rgb8(199, 177, 211),
            };
        }
        Self {
            surface: Color::from_rgb8(254, 254, 254),
            page: Color::from_rgb8(248, 247, 242),
            header: Color::from_rgb8(249, 248, 246),
            border: Color::from_rgb8(230, 225, 216),
            ink: Color::from_rgb8(23, 39, 70),
            muted: Color::from_rgb8(96, 110, 133),
            accent: Color::from_rgb8(210, 96, 56),
            blue: Color::from_rgb8(43, 89, 146),
            green: Color::from_rgb8(78, 111, 57),
            red: Color::from_rgb8(169, 53, 46),
            violet: Color::from_rgb8(116, 99, 132),
        }
    }
}

pub(crate) fn view(app: &App) -> Item<'_> {
    let state = &app.digital_employees;
    responsive(move |size| {
        let base = crate::app::views::project::with_session_sidebar(app, page_content(app));
        let account = app.account.active_account();
        match (state.page, account) {
            (Page::Tasks, Some(_)) => super::tasks_view::overlays(base, &state.tasks, size.width),
            (Page::Shared, Some(_)) => super::shared_view::overlay(base, &state.shared, size.width),
            (Page::Assistants, Some(account)) => super::assistants_view::overlay(
                base,
                &state.assistants,
                account.profile().map(|profile| profile.name()),
                size.height,
            ),
            _ => base,
        }
    })
    .into()
}

fn page_content(app: &App) -> Item<'_> {
    let account = app.account.active_account();
    let profile_name = account.and_then(|account| account.profile()).map(|profile| profile.name());
    let actor = account.map(|_| profile_name.unwrap_or("你好"));
    let state = &app.digital_employees;
    responsive(move |size| {
        let pad = if size.width >= 880.0 { 24.0 } else { 16.0 };
        let width = size.width - 2.0 * pad - 7.0;
        let mut page = column![segmented(
            row![
                page_tab("工作台", Page::Workbench, state.page),
                page_tab("任务看板", Page::Board, state.page),
                page_tab("任务中心", Page::Tasks, state.page),
                page_tab("私有员工", Page::Assistants, state.page),
                page_tab("共享员工", Page::Shared, state.page),
                page_tab("工作流程", Page::Workflows, state.page),
            ]
            .spacing(2)
            .wrap()
            .into()
        )]
        .spacing(20);
        if state.page == Page::Workbench {
            page = page.push(heading(state, actor, size.width < 520.0));
        }
        if state.page == Page::Shared {
            page = page.push(super::shared_view::view(
                &state.shared,
                state.tasks.departments.as_deref(),
                actor.is_some(),
                width,
            ));
        } else if actor.is_none() {
            page = page.push(page_notice(
                column![
                    label("登录识矩后查看数字员工", 14, Tone::Ink),
                    action(
                        "登录识矩",
                        None,
                        Some(Message::Account(
                            crate::app::message::AccountMessage::StartBrowserLogin
                        )),
                        true
                    ),
                ]
                .spacing(12)
                .align_x(Alignment::Center),
            ));
        } else if state.page == Page::Tasks {
            page = page.push(super::tasks_view::view(&state.tasks, width));
        } else if state.page == Page::Assistants {
            page = page.push(super::assistants_view::view(&state.assistants, width, profile_name));
        } else if state.page == Page::Workflows {
            page = page.push(super::workflows_view::view(
                &state.workflows,
                state.tasks.departments.as_deref(),
                width,
            ));
        } else if state.page == Page::Board {
            page = page.push(super::board_view::view(
                &state.board,
                width,
                app.main_window_id.is_some(),
            ));
        } else if let Some(error) = &state.error {
            page = page.push(settings_error_banner(error.as_str()));
        } else if let Some(summary) = &state.summary {
            if summary.partial {
                page = page.push(label("部分工作台数据暂不可用", 13, Tone::Red));
            }
            page = page
                .push(kpis(summary, width))
                .push(panel_pair(decisions(summary), roster(summary), width))
                .push(panel_pair(
                    trends(summary.trend.as_ref(), state.trend_values),
                    activity(summary, width >= 1260.0),
                    width,
                ));
        } else if state.loading {
            page = page.push(page_notice(label("正在加载工作台…", 14, Tone::Muted)));
        }
        container(
            scrollable(
                container(page.width(Length::Fill))
                    .padding(iced::Padding::new(pad).right(pad + 7.0)),
            )
            .id(iced::widget::Id::new(super::board::SCROLL_ID))
            .width(Length::Fill)
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4).margin(3))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme| container::Style::default().background(Palette::new(theme).page))
        .into()
    })
    .into()
}

fn page_tab(title: &'static str, page: Page, active: Page) -> Item<'static> {
    segment(title, page == active, event(Event::SelectPage(page)))
}

fn heading<'a>(state: &'a State, actor: Option<&str>, narrow: bool) -> Item<'a> {
    let subtitle = match (actor, state.summary.as_deref()) {
        (Some(name), Some(data)) => {
            let pending = data.other_todos().count() + data.decision_inbox.len();
            if pending > 0 {
                format!("{name}，{pending} 件事等你处理")
            } else {
                format!("{name}，这是你现在需要关注的事")
            }
        }
        (Some(name), None) => name.to_owned(),
        (None, _) => "识矩数字员工".to_owned(),
    };
    let title = column![strong("工作台", 25, Tone::Ink), label(subtitle, 14, Tone::Muted)]
        .spacing(7)
        .width(Length::Fill);
    let actions = row![
        action(
            if state.loading { "刷新中" } else { "刷新" },
            Some(Icon::ArrowRepeat),
            (actor.is_some() && !state.loading).then(|| event(Event::Refresh)),
            false
        ),
        action(
            "派新任务",
            Some(Icon::ArrowRight),
            actor.map(|_| event(Event::NewTask(String::new()))),
            true
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if narrow {
        column![title, actions].spacing(12).into()
    } else {
        row![title, actions].align_y(Alignment::Center).into()
    }
}

fn kpis(summary: &Summary, width: f32) -> Item<'_> {
    let kpi = &summary.kpi;
    let pending = kpi.pending_human_total.or(kpi.approvals_pending);
    let rate = match (kpi.tasks_total, kpi.tasks_success_rate) {
        (Some(0), _) | (_, None) => "—".to_owned(),
        (_, Some(rate)) => format!("{:.1}%", rate * 100.0),
    };
    let employees = stat(
        "共享员工",
        count(kpi.employees_total),
        kpi.employees_active.map(|n| format!("{n} 名在岗")),
        Icon::Person,
        Tone::Accent,
        event(Event::SelectPage(Page::Shared)),
    );
    let running = stat(
        "执行中任务",
        count(kpi.tasks_running),
        kpi.tasks_total.map(|n| format!("共 {n} 个任务")),
        Icon::ArrowRepeat,
        Tone::Green,
        web("/tasks?phase=executing", None),
    );
    let waiting = stat(
        "等人处理",
        count(pending),
        Some("审批、补材料、转人工".to_owned()),
        Icon::Clock,
        if pending.unwrap_or(0) > 0 { Tone::Accent } else { Tone::Green },
        web("/tasks?phase=waiting_human", None),
    );
    let success = stat(
        "任务成功率",
        rate,
        kpi.tasks_completed_today.map(|n| format!("今日完成 {n}")),
        Icon::Speedometer2,
        Tone::Violet,
        web("/tasks?phase=done", None),
    );
    if width >= 900.0 {
        row![employees, running, waiting, success].spacing(20).into()
    } else if width >= 490.0 {
        column![row![employees, running].spacing(12), row![waiting, success].spacing(12)]
            .spacing(12)
            .into()
    } else {
        column![employees, running, waiting, success].spacing(10).into()
    }
}

fn stat<'a>(
    title: &'static str,
    value: String,
    hint: Option<String>,
    glyph: Icon,
    tone: Tone,
    message: Message,
) -> Item<'a> {
    let badge = container(icon(glyph, 18, tone))
        .width(38)
        .height(38)
        .center(38)
        .style(move |theme| tint_style(theme, tone, 8.0));
    let mut values =
        Row::new().push(strong(value, 23, Tone::Ink)).spacing(6).align_y(Alignment::End);
    if let Some(hint) = hint {
        values = values.push(label(hint, 12, Tone::Muted));
    }
    button(
        row![badge, column![label(title, 13, Tone::Muted), values.wrap()].spacing(8)].spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .on_press(message)
    .style(|theme, status| {
        let mut style = rounded_action_btn_style(theme, status);
        let palette = Palette::new(theme);
        style.border = panel_style(theme).border;
        if status == button::Status::Active {
            style.background = Some(palette.surface.into());
        }
        style
    })
    .into()
}

fn panel_pair<'a>(left: Item<'a>, right: Item<'a>, width: f32) -> Item<'a> {
    if width < 880.0 {
        return column![card(left), card(right)].spacing(16).into();
    }
    let content = row![
        container(left).width(Length::FillPortion(7)),
        container(right).width(Length::FillPortion(5)),
    ]
    .spacing(20);
    let surfaces = row![
        container(Space::new())
            .width(Length::FillPortion(7))
            .height(Length::Fill)
            .style(|theme| panel_style(theme)),
        container(Space::new())
            .width(Length::FillPortion(5))
            .height(Length::Fill)
            .style(|theme| panel_style(theme)),
    ]
    .spacing(20)
    .width(Length::Fill)
    .height(Length::Fill);
    Stack::new().push(content).push_under(surfaces).into()
}

fn panel<'a>(
    title: &'static str,
    glyph: Option<Icon>,
    count: Option<usize>,
    trailing: Item<'a>,
    content: Item<'a>,
) -> Item<'a> {
    let mut header = Row::new().spacing(8).align_y(Alignment::Center);
    if let Some(glyph) = glyph {
        header = header.push(icon(glyph, 16, Tone::Muted));
    }
    let mut header = header.push(strong(title, 15, Tone::Ink));
    if let Some(count) = count {
        header = header.push(label(count.to_string(), 12, Tone::Muted));
    }
    let header = header.push(Space::new().width(Length::Fill)).push(trailing);
    column![header, content,].padding(16).spacing(14).width(Length::Fill).into()
}

fn decisions(summary: &Summary) -> Item<'_> {
    let total = summary.other_todos().count() + summary.decision_inbox.len();
    let admin =
        summary.scope.as_ref().and_then(|scope| scope.role_hint.as_deref()) == Some("tenant_admin");
    let trailing = link(if admin { "各部门待办汇总 →" } else { "全部任务 →" }, web("/tasks", None));
    let mut body = Column::new().spacing(12);
    if total == 0 {
        body = body.push(label(
            if summary.partial {
                "待办数据可能未完整加载"
            } else {
                "眼下没有需要你处理的事"
            },
            13,
            Tone::Muted,
        ));
    }
    for item in &summary.decision_inbox {
        let caption = if item.sla_breached { "已超时" } else { "待处理" };
        let action_label = match item.waiting_type.as_str() {
            "WAITING_APPROVAL" | "approval" => "处理审批",
            "WAITING_INPUT" | "supplemental_input" => "补充材料",
            _ => "查看任务",
        };
        body = body.push(
            container(
                column![
                    row![
                        status_label(
                            caption,
                            if item.sla_breached { Tone::Red } else { Tone::Accent }
                        ),
                        Space::new().width(Length::Fill),
                        label(
                            format!("已等 {} 分钟", item.waiting_duration_ms / 60_000),
                            12,
                            Tone::Muted
                        ),
                    ]
                    .align_y(Alignment::Center),
                    strong(item.task_name.as_str(), 14, Tone::Ink),
                    label(task_ref(&item.task_id), 12, Tone::Muted),
                    label(item.decision_summary.as_str(), 13, Tone::Ink),
                    label(item.impact_hint.as_str(), 12, Tone::Muted),
                    row![
                        label(format!("卡在 {}", item.blocked_step), 12, Tone::Muted)
                            .width(Length::Fill),
                        action(
                            action_label,
                            None,
                            Some(web("/tasks", Some(("task", &item.task_id)))),
                            true
                        ),
                    ]
                    .align_y(Alignment::Center),
                ]
                .spacing(7)
                .padding(12),
            )
            .style(|theme| tint_style(theme, Tone::Muted, 8.0)),
        );
    }
    let kinds = [
        ("approval", "待审批", Tone::Accent),
        ("supplemental_input", "待补充材料", Tone::Accent),
        ("incident", "运行事故", Tone::Red),
        ("failed_operation", "执行失败", Tone::Red),
        ("sla_breached", "SLA 超时", Tone::Red),
        ("human_takeover", "人工接管", Tone::Accent),
    ];
    for (kind, title, tone) in kinds {
        let todos: Vec<_> = summary.other_todos().filter(|todo| todo.kind == kind).collect();
        if todos.is_empty() {
            continue;
        }
        let mut group = column![
            container(
                row![
                    status_label(title, tone),
                    Space::new().width(Length::Fill),
                    label(format!("{} 条", todos.len()), 12, Tone::Muted),
                ]
                .align_y(Alignment::Center)
            )
            .padding([5, 9]),
        ];
        for todo in todos.iter().take(3) {
            let reference = todo.task_id.as_deref().map(task_ref).unwrap_or_default();
            group = group.push(row_button(
                row![
                    label(todo.title.as_str(), 13, Tone::Ink).width(Length::Fill),
                    label(reference, 11, Tone::Muted),
                    icon(Icon::ArrowRight, 12, Tone::Muted),
                ]
                .spacing(7)
                .align_y(Alignment::Center),
                web(&todo.href, todo.task_id.as_deref().map(|id| ("task", id))),
            ));
        }
        if todos.len() > 3 {
            let (destination, place) = if kind == "incident" {
                ("/monitoring", "运行情况")
            } else {
                ("/tasks", "任务中心")
            };
            group = group.push(link(
                format!("还有 {} 条，去{place}处理", todos.len() - 3),
                web(destination, None),
            ));
        }
        body = body.push(group);
    }
    panel("等你拍板", Some(Icon::CheckSquare), (total > 0).then_some(total), trailing, body.into())
}

fn roster(summary: &Summary) -> Item<'_> {
    let (trailing, content) = match &summary.roster {
        Some(roster) if !roster.rows.is_empty() => {
            let items = separated(roster.rows.iter().take(8).map(employee));
            let content =
                column![items, label("执行员工由任务主管在工作流程里安排。", 12, Tone::Muted),]
                    .spacing(10);
            (
                label(
                    format!(
                        "{} 人在干活 · {} 人可接单",
                        roster.working_total, roster.available_total
                    ),
                    12,
                    Tone::Muted,
                )
                .into(),
                content.into(),
            )
        }
        Some(_) => (Space::new().into(), label("还没有在岗的共享员工", 13, Tone::Muted).into()),
        None => {
            (Space::new().into(), label("工位信息暂不可用，请稍后刷新", 13, Tone::Muted).into())
        }
    };
    panel("员工在忙什么", Some(Icon::Person), None, trailing, content)
}

fn employee(person: &Employee) -> Item<'_> {
    let manager = person.agent_kind == "manager";
    let avatar: Item<'_> = match portraits::avatar(&person.name, manager) {
        Some(handle) => image(handle).width(36).height(36).into(),
        None => container(strong("—", 16, Tone::Muted))
            .center(36)
            .style(|theme| tint_style(theme, Tone::Muted, 18.0))
            .into(),
    };
    let who = row![
        avatar,
        column![
            strong(person.name.as_str(), 14, Tone::Ink),
            label(if manager { "任务主管" } else { "执行员工" }, 12, Tone::Muted),
        ]
        .spacing(3),
    ]
    .align_y(Alignment::Center)
    .spacing(8);
    let status: Item<'_> =
        if matches!(person.runtime.status.as_str(), "enabling" | "disabling" | "none") {
            label(person.runtime.label.as_str(), 12, Tone::Muted).into()
        } else if person.state == "abnormal" {
            label(
                format!(
                    "{} · {}",
                    person.runtime.cause.as_deref().unwrap_or("运行异常"),
                    person.runtime.next_action.as_deref().unwrap_or("去查看详情")
                ),
                12,
                Tone::Red,
            )
            .into()
        } else if let Some(task) =
            person.current_work.as_ref().filter(|_| person.state == "working")
        {
            column![
                link(task.task_name.as_str(), web("/tasks", Some(("task", &task.task_id)))),
                label(format!("{}% · {}", task.progress_pct, task.current_step), 12, Tone::Muted),
            ]
            .into()
        } else {
            let status = if manager {
                person.availability.as_deref().unwrap_or("随时可接单")
            } else {
                "等主管安排"
            };
            let mut idle =
                row![label(status, 12, Tone::Muted).width(Length::Fill)].align_y(Alignment::Center);
            if manager && person.state == "available" && person.can_dispatch {
                idle = idle.push(action(
                    "派活",
                    None,
                    Some(web("/tasks", Some(("manager", &person.employee_id)))),
                    false,
                ));
            }
            idle.into()
        };
    container(column![who, status].spacing(7)).padding([9, 0]).width(Length::Fill).into()
}

fn trends(trend: Option<&Trend>, values: [bool; 2]) -> Item<'_> {
    let content = if let Some(trend) = trend.filter(|trend| !trend.days.is_empty()) {
        column![
            series(trend, Metric::Tasks, values[0]),
            series(trend, Metric::SuccessRate, values[1]),
        ]
        .spacing(14)
        .into()
    } else {
        label("趋势数据暂不可用", 13, Tone::Muted).into()
    };
    panel("近 14 天", None, None, Space::new().into(), content)
}

fn series(trend: &Trend, metric: Metric, values: bool) -> Item<'_> {
    let header = row![
        strong(metric.title(), 13, Tone::Ink).width(Length::Fill),
        action(
            if values { "看图表" } else { "看数值" },
            None,
            Some(event(Event::ToggleValues(metric))),
            false
        ),
    ]
    .align_y(Alignment::Center);
    let detail: Item<'_> = if values {
        separated(trend.days.iter().enumerate().map(|(index, day)| {
            container(row![
                label(day.as_str(), 12, Tone::Muted).width(Length::Fill),
                label(metric.format(metric.value(trend, index)), 12, Tone::Ink),
            ])
            .width(Length::Fill)
            .padding([5, 10])
            .into()
        }))
        .into()
    } else {
        canvas(Chart { trend, metric }).width(Length::Fill).height(192).into()
    };
    column![header, detail].spacing(4).into()
}

fn activity(summary: &Summary, wide: bool) -> Item<'_> {
    let mut content = Column::new().spacing(10);
    if summary.recent_activity.is_empty() {
        content = content.push(label(
            if summary.partial { "任务动态暂不可用" } else { "还没有任务动态" },
            13,
            Tone::Muted,
        ));
    } else {
        let header = activity_cells(
            label("任务", 12, Tone::Muted).into(),
            label("当前步骤", 12, Tone::Muted).into(),
            label("状态", 12, Tone::Muted).into(),
            label("更新", 12, Tone::Muted).into(),
            wide,
        );
        let header: Item<'_> = container(header).padding([12, 9]).width(Length::Fill).into();
        content =
            content
                .push(separated(std::iter::once(header).chain(
                    summary.recent_activity.iter().take(8).map(|r| activity_item(r, wide)),
                )));
    }
    if summary
        .scope
        .as_ref()
        .and_then(|scope| scope.role_hint.as_deref())
        .is_some_and(|role| matches!(role, "tenant_admin" | "dept_manager"))
    {
        content = content.push(
            row![
                label("模型费用以模型服务用量记录为准。", 12, Tone::Muted),
                link("查看套餐与用量 →", web("/usage", None)),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .wrap(),
        );
    }
    panel(
        "最近动态",
        Some(Icon::BarChartFill),
        None,
        link("任务中心 →", web("/tasks", None)),
        content.into(),
    )
}

fn activity_item(item: &Activity, wide: bool) -> Item<'_> {
    let title = column![
        label(item.summary.as_deref().unwrap_or("—"), 13, Tone::Ink),
        label(task_ref(&item.id), 11, Tone::Muted),
    ]
    .spacing(3);
    let (status, tone) = match item.kind.as_deref().unwrap_or_default() {
        "SUCCEEDED" => ("已成功", Tone::Green),
        "FAILED" => ("已失败", Tone::Red),
        "RUNNING" => ("执行中", Tone::Blue),
        "QUEUED" => ("排队中", Tone::Muted),
        "SCHEDULED" => ("已计划", Tone::Muted),
        "WAITING_APPROVAL" => ("待审批", Tone::Accent),
        "WAITING_INPUT" => ("待补材料", Tone::Accent),
        "HUMAN_TAKEOVER" => ("人工接管", Tone::Accent),
        "CANCEL_REQUESTED" => ("取消中", Tone::Muted),
        "CANCELLED" => ("已取消", Tone::Muted),
        "REJECTED" => ("已拒绝", Tone::Red),
        _ => ("—", Tone::Muted),
    };
    row_button(
        activity_cells(
            title.into(),
            label(item.current_step.as_deref().unwrap_or("—"), 12, Tone::Muted).into(),
            status_label(status, tone),
            label(format_time(item.at.as_deref()), 12, Tone::Muted).into(),
            wide,
        ),
        web("/tasks", Some(("task", &item.id))),
    )
}

fn activity_cells<'a>(
    title: Item<'a>,
    step: Item<'a>,
    status: Item<'a>,
    time: Item<'a>,
    wide: bool,
) -> Item<'a> {
    if wide {
        return row![
            container(title).width(Length::FillPortion(7)),
            container(step).width(Length::FillPortion(4)),
            container(status).width(Length::FillPortion(3)),
            container(time).width(Length::FillPortion(6)),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .into();
    }
    column![
        row![container(title).width(Length::Fill), status].spacing(7).align_y(Alignment::Center),
        Row::new().push(step).push(time).spacing(7).wrap(),
    ]
    .spacing(4)
    .into()
}

fn event(event: Event) -> Message {
    Message::DigitalEmployees(event)
}

pub(super) fn web(route: &str, query: Option<(&str, &str)>) -> Message {
    if route.split('?').next() == Some("/tasks") {
        if let Some(("manager", employee)) = query {
            return event(Event::NewTask(employee.to_owned()));
        }
        let url = reqwest::Url::parse(&api::console_url(route, query)).expect("fixed console URL");
        let mut phase = super::tasks::Phase::All;
        let mut id = None;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "task" if !value.is_empty() => id = Some(value.into_owned()),
                "phase" => {
                    phase = match value.as_ref() {
                        "queued" => super::tasks::Phase::Queued,
                        "running" | "executing" => super::tasks::Phase::Running,
                        "waiting" | "waiting_human" => super::tasks::Phase::Waiting,
                        "done" | "ended" => super::tasks::Phase::Ended,
                        _ => super::tasks::Phase::All,
                    }
                }
                _ => {}
            }
        }
        return event(Event::TaskRoute(phase, id));
    }
    Message::View(ViewMessage::OpenUrlExternal(api::console_url(route, query)))
}

fn count(value: Option<u64>) -> String {
    value.map(|v| v.to_string()).unwrap_or_else(|| "—".into())
}

pub(super) fn task_ref(id: &str) -> String {
    let short = match (id.get(..8), id.get(id.len().saturating_sub(4)..)) {
        (Some(first), Some(last)) if id.len() > 14 => format!("{first}…{last}"),
        _ => id.to_owned(),
    };
    format!("编号 {short}")
}

pub(super) fn format_time(value: Option<&str>) -> String {
    value
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|date| date.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "—".into())
}

pub(super) fn action<'a>(
    title: &'static str,
    glyph: Option<Icon>,
    message: Option<Message>,
    primary: bool,
) -> Item<'a> {
    let mut body = Row::new().spacing(6).align_y(Alignment::Center);
    if let Some(glyph) = glyph {
        let color = if primary { Tone::White } else { Tone::Ink };
        body = body.push(icon(glyph, 14, color));
    }
    button(body.push(text(title).size(14)))
        .padding([8, 12])
        .on_press_maybe(message)
        .style(move |theme, status| {
            if primary {
                let mut style = primary_action_btn_style(theme, status);
                if !theme.extended_palette().is_dark && status != button::Status::Disabled {
                    let color = match status {
                        button::Status::Pressed => Color::from_rgb8(166, 66, 32),
                        button::Status::Hovered => Color::from_rgb8(189, 76, 41),
                        _ => Palette::new(theme).accent,
                    };
                    style.background = Some(color.into());
                    style.text_color = Color::WHITE;
                }
                style
            } else {
                rounded_action_btn_style(theme, status)
            }
        })
        .into()
}

fn row_button<'a>(body: impl Into<Item<'a>>, message: Message) -> Item<'a> {
    button(body)
        .on_press(message)
        .width(Length::Fill)
        .padding([11, 9])
        .style(|theme, status| {
            let colors = Palette::new(theme);
            button::Style {
                text_color: colors.ink,
                background: (status != button::Status::Active)
                    .then(|| colors.border.scale_alpha(0.32).into()),
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn link<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    message: Message,
) -> Item<'a> {
    button(strong(title, 12, Tone::Ink))
        .on_press(message)
        .padding([5, 3])
        .style(|theme, status| {
            let colors = Palette::new(theme);
            button::Style {
                text_color: colors.ink,
                background: (status != button::Status::Active)
                    .then(|| colors.border.scale_alpha(0.32).into()),
                border: border::rounded(4),
                ..Default::default()
            }
        })
        .into()
}

pub(super) fn page_notice<'a>(content: impl Into<Item<'a>>) -> Item<'a> {
    container(content).center_x(Length::Fill).padding([40, 16]).into()
}

pub(super) fn status_label<'a>(title: &'a str, tone: Tone) -> Item<'a> {
    let dot = container(Space::new()).width(6).height(6).style(move |theme| {
        container::Style::default().background(tone.color(theme)).border(border::rounded(3))
    });
    let text_tone = if matches!(tone, Tone::Red) { Tone::Red } else { Tone::Muted };
    row![dot, label(title, 12, text_tone)].spacing(6).align_y(Alignment::Center).into()
}

pub(super) fn segment<'a>(
    title: impl iced::widget::text::IntoFragment<'a>,
    selected: bool,
    message: Message,
) -> Item<'a> {
    button(text(title).size(13))
        .padding([7, 10])
        .on_press(message)
        .style(move |theme, status| {
            let colors = Palette::new(theme);
            let mut style = rounded_action_btn_style(theme, status);
            style.background = match status {
                button::Status::Pressed => Some(colors.border.into()),
                _ if selected => Some(colors.surface.into()),
                button::Status::Hovered => Some(colors.surface.scale_alpha(0.65).into()),
                _ => None,
            };
            style.text_color = if selected { colors.ink } else { colors.muted };
            style.border = border::rounded(6);
            style
        })
        .into()
}

pub(super) fn segmented(content: Item<'_>) -> Item<'_> {
    container(content)
        .padding(3)
        .style(|theme| {
            container::Style::default()
                .background(Palette::new(theme).border.scale_alpha(0.38))
                .border(border::rounded(8))
        })
        .into()
}

pub(super) fn card<'a>(content: impl Into<Item<'a>>) -> Item<'a> {
    container(content).width(Length::Fill).style(|theme| panel_style(theme)).into()
}

pub(super) fn panel_style(theme: &Theme) -> container::Style {
    let colors = Palette::new(theme);
    container::Style::default().background(colors.surface).border(Border {
        color: colors.border.scale_alpha(0.72),
        width: 1.0,
        radius: 12.0.into(),
    })
}

pub(super) fn table_header_style(theme: &Theme) -> container::Style {
    container::Style::default()
        .background(Palette::new(theme).header)
        .border(border::rounded(border::top(12)))
}

pub(super) fn divider<'a>() -> Item<'a> {
    rule::horizontal(1)
        .style(|theme| rule::Style {
            color: Palette::new(theme).border.scale_alpha(0.62),
            radius: 0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

fn separated<'a>(items: impl IntoIterator<Item = Item<'a>>) -> Column<'a, Message> {
    let mut column = Column::new().width(Length::Fill);
    for (index, item) in items.into_iter().enumerate() {
        if index != 0 {
            column = column.push(divider());
        }
        column = column.push(item);
    }
    column
}

pub(super) fn tint_style(theme: &Theme, tone: Tone, radius: f32) -> container::Style {
    container::Style::default()
        .background(tone.color(theme).scale_alpha(0.12))
        .border(border::rounded(radius))
}

pub(super) fn icon(glyph: Icon, size: u16, tone: Tone) -> svg::Svg<'static> {
    let size = f32::from(size);
    svg(get_icon(glyph))
        .width(size)
        .height(size)
        .style(move |theme, _| svg::Style { color: Some(tone.color(theme)) })
}

pub(super) fn label<'a>(
    value: impl iced::widget::text::IntoFragment<'a>,
    size: u16,
    tone: Tone,
) -> iced::widget::Text<'a> {
    text(value)
        .size(f32::from(size))
        .style(move |theme| iced::widget::text::Style { color: Some(tone.color(theme)) })
}

pub(super) fn strong<'a>(
    value: impl iced::widget::text::IntoFragment<'a>,
    size: u16,
    tone: Tone,
) -> iced::widget::Text<'a> {
    label(value, size, tone)
        .font(Font { weight: iced::font::Weight::Bold, ..Font::with_name("Noto Sans CJK SC") })
}

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Ink,
    Muted,
    Accent,
    Green,
    Red,
    Blue,
    Violet,
    Warning,
    White,
}

impl Tone {
    pub(super) fn color(self, theme: &Theme) -> Color {
        let p = Palette::new(theme);
        match self {
            Self::Ink => p.ink,
            Self::Muted => p.muted,
            Self::Accent => p.accent,
            Self::Green => p.green,
            Self::Red => p.red,
            Self::Blue => p.blue,
            Self::Violet => p.violet,
            Self::Warning => {
                if theme.extended_palette().is_dark {
                    Color::from_rgb8(225, 185, 97)
                } else {
                    Color::from_rgb8(164, 116, 32)
                }
            }
            Self::White if theme.extended_palette().is_dark => {
                theme.extended_palette().primary.base.text
            }
            Self::White => Color::WHITE,
        }
    }
}
