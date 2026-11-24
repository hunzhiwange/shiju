use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_error_banner,
};
use iced::widget::{
    Column, Row, Space, button, canvas, column, container, image, progress_bar, row, stack,
};
use iced::{Alignment, Color, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

use crate::app::Message;
use crate::app::message::view::ViewMessage;

use super::api::{Employee, employee_config_url};
use super::board::{Board, BoardEvent as Event, PAGE_SIZE, Queue, message};
use super::board_api::{Handoff, Profile, Snapshot, Step, WorkItem};
use super::portraits;
use super::view::{
    Item, Palette, Tone, action, card, divider, format_time, label, link, panel_style, segment,
    segmented, status_label as status, strong, task_ref, tint_style, web,
};

const ORANGE: Color = Color::from_rgb(0.906, 0.475, 0.318);

pub(super) fn view(i18n: desktop_i18n::I18n, board: &Board, width: f32, can_fullscreen: bool) -> Item<'_> {
    let mut page = column![header(i18n, board, width, can_fullscreen)].spacing(20);
    if let Some(error) = &board.error {
        page = page.push(settings_error_banner(error.render(i18n)));
    } else if board.loading {
        page = page.push(label(i18n.__("正在加载办公室…"), 13, Tone::Muted));
    }
    let snapshot = board.snapshot.as_deref();
    page = page.push(summary(i18n, snapshot, width));
    if let Some(snapshot) = snapshot {
        if snapshot.summary.partial || snapshot.summary.roster.is_none() {
            page = page.push(label(i18n.__("部分工作台数据暂不可用"), 13, Tone::Red));
        }
        page = page
            .push(segmented(
                row![
                    segment(i18n.__("数字员工办公室"), !board.task_view, message(Event::TaskView(false))),
                    segment(i18n.__("正在发生的工作"), board.task_view, message(Event::TaskView(true))),
                ]
                .spacing(2)
                .wrap()
                .into(),
            ))
            .push(if board.task_view {
                tasks_panel(i18n, board, snapshot, width)
            } else {
                office(i18n, board, snapshot, width)
            });
    }
    page.width(Length::Fill).into()
}

fn header(i18n: desktop_i18n::I18n, board: &Board, width: f32, can_fullscreen: bool) -> Item<'_> {
    let heading = strong(i18n.__("任务看板"), 24, Tone::Ink).width(Length::Fill);
    let controls = row![
        action(
            i18n.__("刷新"),
            Some(Icon::ArrowRepeat),
            (!board.loading).then(|| message(Event::Refresh)),
            false
        ),
        action(
            if board.still { i18n.__("开启动效") } else { i18n.__("暂停动效") },
            None,
            Some(message(Event::ToggleMotion)),
            false
        ),
        action(
            if board.tour { i18n.__("关闭巡视") } else { i18n.__("自动巡视") },
            None,
            Some(message(Event::ToggleTour)),
            false
        ),
        action(
            if board.fullscreen { i18n.__("退出全屏") } else { i18n.__("全屏展示 ↗") },
            None,
            can_fullscreen.then(|| message(Event::ToggleFullscreen)),
            false
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if width < 880.0 {
        column![heading, controls.wrap()].spacing(12).into()
    } else {
        row![heading, controls].spacing(16).align_y(Alignment::Center).into()
    }
}

fn employees(data: &Snapshot) -> &[Employee] {
    data.summary.roster.as_ref().map(|roster| roster.rows.as_slice()).unwrap_or_default()
}

fn summary(i18n: desktop_i18n::I18n, data: Option<&Snapshot>, width: f32) -> Item<'_> {
    let counts = data
        .map(|data| {
            [
                data.summary.roster.as_ref().map(|roster| roster.rows.len()),
                data.summary.roster.as_ref().map(|roster| {
                    roster
                        .rows
                        .iter()
                        .filter(|person| work_state(i18n, person, &data.tasks).working)
                        .count()
                }),
                Some(data.tasks.iter().filter(|task| task.running()).count()),
                Some(data.tasks.iter().filter(|task| task.waiting()).count()),
            ]
        })
        .unwrap_or_default();
    let notes = data
        .map(|data| {
            let kpi = &data.summary.kpi;
            [
                kpi.employees_total.map(|n| i18n.format("授权范围内共 %s 位", &[format!("{}", n).into()]).expect("valid workbench template")),
                kpi.employees_active.map(|n| i18n.format("在岗 %s 位", &[format!("{}", n).into()]).expect("valid workbench template")),
                kpi.tasks_completed_today.map(|n| i18n.format("今日已完成 %s 项", &[format!("{}", n).into()]).expect("valid workbench template")),
                kpi.approvals_pending.map(|n| i18n.format("其中待审批 %s 项", &[format!("{}", n).into()]).expect("valid workbench template")),
            ]
        })
        .unwrap_or_default();
    let cells: Vec<_> = [i18n.__("画面中的员工"), i18n.__("正在工作的员工"), i18n.__("并行推进的任务"), i18n.__("等你拍板")]
        .into_iter()
        .zip(counts)
        .zip(notes)
        .enumerate()
        .map(|(index, ((title, count), note))| {
            let value = row![
                strong(count.map_or_else(|| "—".into(), |n| n.to_string()), 25, Tone::Ink),
                label(if index < 2 { i18n.__("位") } else { i18n.__("项") }, 12, Tone::Muted),
            ]
            .spacing(5)
            .align_y(Alignment::Center);
            column![
                label(title, 13, Tone::Muted),
                value,
                note.map(|note| label(note, 12, Tone::Muted)),
            ]
            .spacing(5)
            .width(Length::Fill)
            .into()
        })
        .collect();
    let columns = if width >= 720.0 {
        4
    } else if width >= 360.0 {
        2
    } else {
        1
    };
    card(container(grid(cells, columns, 14)).padding(16))
}

struct WorkState<'a> {
    name: &'a str,
    action: &'a str,
    tone: Tone,
    working: bool,
}

fn work_state<'a>(i18n: desktop_i18n::I18n, person: &'a Employee, tasks: &'a [WorkItem]) -> WorkState<'a> {
    let owned = || {
        tasks.iter().filter(|task| task.owner_ref.as_deref() == Some(person.employee_id.as_str()))
    };
    let (name, action, tone, working) = if person.runtime.status == "none" {
        (
            i18n.__("容器未运行"),
            person.runtime.next_action.as_deref().unwrap_or(i18n.__("等待启动")),
            Tone::Warning,
            false,
        )
    } else if matches!(person.runtime.status.as_str(), "enabling" | "disabling") {
        (
            person.runtime.label.as_str(),
            person.runtime.next_action.as_deref().unwrap_or(i18n.__("等待运行状态确认")),
            Tone::Warning,
            false,
        )
    } else if person.state == "abnormal" {
        (i18n.__("需要协助"), person.runtime.cause.as_deref().unwrap_or(i18n.__("运行异常")), Tone::Red, false)
    } else if let Some(task) = owned().find(|task| task.running() && !task.paused()) {
        (i18n.__("工作中"), nonempty(task.current_step.as_deref(), i18n.__("执行任务")), Tone::Accent, true)
    } else if owned().any(WorkItem::paused) {
        (i18n.__("已暂停"), i18n.__("等待继续执行"), Tone::Warning, false)
    } else if let Some(task) = owned().find(|task| task.waiting()) {
        (i18n.__("等你回复"), nonempty(task.blocker.as_deref(), i18n.__("等待人工处理")), Tone::Warning, false)
    } else if person.agent_kind == "manager"
        && tasks.iter().any(|task| {
            task.running() && task.input_text("employee_id") == Some(person.employee_id.as_str())
        })
    {
        (i18n.__("团队执行中"), i18n.__("任务由团队推进"), Tone::Accent, false)
    } else {
        let name = if person.runtime.status == "on_duty" { i18n.__("在岗") } else { i18n.__("状态未确认") };
        (name, nonempty(person.availability.as_deref(), i18n.__("等待工作安排")), Tone::Green, false)
    };
    WorkState { name, action, tone, working }
}

fn office<'a>(i18n: desktop_i18n::I18n, board: &'a Board, data: &'a Snapshot, width: f32) -> Item<'a> {
    let selected = employees(data)
        .iter()
        .find(|employee| Some(employee.employee_id.as_str()) == board.employee.as_deref());
    let wide = width >= 1000.0;
    let inspector_width = if width >= 1250.0 { 330.0 } else { 260.0 };
    let office_width = if wide { width - inspector_width - 52.0 } else { width - 32.0 };
    let focus =
        board.focus.as_ref().and_then(|id| data.tasks.iter().find(|task| &task.task_id == id));
    let title = column![
        strong(i18n.__("数字员工办公室"), 16, Tone::Ink),
        label(
            focus.map_or_else(
                || i18n.__("点击一位员工，看看 TA 正在忙什么").to_owned(),
                |task| i18n.format("正在聚焦：%s", &[format!("{}", task.title).into()]).expect("valid workbench template")
            ),
            12,
            Tone::Muted
        ),
    ]
    .spacing(5)
    .width(Length::Fill);
    let legend = row![
        status(i18n.__("工作中"), Tone::Accent),
        status(i18n.__("在岗"), Tone::Green),
        status(i18n.__("需回应"), Tone::Warning),
        status(i18n.__("需要协助"), Tone::Red),
    ]
    .spacing(12);
    let mut panel =
        column![section_header(title, legend.wrap().into(), office_width < 620.0)].spacing(14);
    if let Some(task) = focus {
        let detail = board
            .details
            .get(&task.task_id)
            .and_then(|entry| entry.as_ref())
            .and_then(|entry| entry.as_ref().ok());
        let people: Vec<_> = employees(data)
            .iter()
            .filter(|person| {
                let id = person.employee_id.as_str();
                task.owner_ref.as_deref() == Some(id)
                    || task.input_text("employee_id") == Some(id)
                    || detail.is_some_and(|record| {
                        record.detail.steps.iter().any(|step| task.step_employee(step) == Some(id))
                    })
            })
            .map(|person| person.name.as_str())
            .collect();
        let dispatch = task
            .input_text("employee_id")
            .zip(task.owner_ref.as_deref())
            .filter(|(from, to)| from != to)
            .and_then(|(from, to)| {
                let name = |id| {
                    employees(data)
                        .iter()
                        .find(|person| person.employee_id == id)
                        .map(|person| person.name.as_str())
                };
                Some(format!("{} → {}", name(from)?, name(to)?))
            });
        let caption = if let Some(dispatch) = dispatch {
            i18n.format("任务协作 · %s", &[format!("{}", dispatch).into()]).expect("valid workbench template")
        } else if people.is_empty() {
            i18n.__("参与员工不在当前工位名单中").to_owned()
        } else {
            i18n.format("参与员工 · %s", &[format!("{}", people.join(" · ")).into()]).expect("valid workbench template")
        };
        panel = panel.push(
            container(
                row![
                    label(caption, 12, Tone::Ink).width(Length::Fill),
                    link(i18n.__("查看整个办公室 ×"), message(Event::Focus(None))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([8, 16])
            .width(Length::Fill)
            .style(|theme| tint_style(theme, Tone::Accent, 8.0)),
        );
    }
    let manager_width = if width >= 1250.0 { 215.0 } else { 170.0 };
    let side_by_side = office_width >= 620.0;
    let worker_width =
        if side_by_side { office_width - manager_width - 12.0 } else { office_width };
    let manager = zone(i18n, i18n.__("任务主管"), true, 1, board, data, focus);
    let worker = zone(i18n,
        i18n.__("员工工作区"),
        false,
        if worker_width >= 700.0 {
            3
        } else if worker_width >= 440.0 {
            2
        } else {
            1
        },
        board,
        data,
        focus,
    );
    let zones: Item<'_> = if side_by_side {
        row![container(manager).width(manager_width), worker].spacing(12).into()
    } else {
        column![manager, worker].spacing(12).into()
    };
    let office = card(
        panel
            .push(zones)
            .push(label(
                i18n.format("展示授权名单中的 %s 位员工", &[format!("{}", employees(data).len()).into()]).expect("valid workbench template"),
                12,
                Tone::Muted,
            ))
            .padding(16),
    );
    let inspect = card(match selected {
        Some(person) => inspector(i18n, board, data, person),
        None => container(label(i18n.__("当前没有可展示的员工"), 13, Tone::Muted)).padding(18).into(),
    });
    if wide {
        row![office, container(inspect).width(inspector_width)].spacing(20).into()
    } else {
        column![office, inspect].spacing(14).into()
    }
}

fn zone<'a>(i18n: desktop_i18n::I18n,
    title: &'static str,
    manager: bool,
    cols: usize,
    board: &'a Board,
    data: &'a Snapshot,
    focus: Option<&'a WorkItem>,
) -> Item<'a> {
    let people: Vec<_> = employees(data)
        .iter()
        .filter(|person| (person.agent_kind == "manager") == manager)
        .map(|person| desk(i18n, board, data, person, manager, focus))
        .collect();
    container(
        column![
            strong(title, 14, Tone::Ink),
            if people.is_empty() {
                label(
                    if manager {
                        i18n.__("当前范围暂无任务主管")
                    } else {
                        i18n.__("当前范围暂无执行员工")
                    },
                    12,
                    Tone::Muted,
                )
                .into()
            } else {
                grid(people, cols, 10)
            },
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .padding([12, 10])
    .style(|theme| tint_style(theme, Tone::Muted, 10.0))
    .into()
}

fn desk<'a>(i18n: desktop_i18n::I18n,
    board: &'a Board,
    data: &'a Snapshot,
    person: &'a Employee,
    manager: bool,
    focus: Option<&WorkItem>,
) -> Item<'a> {
    let state = work_state(i18n, person, &data.tasks);
    let profile = data.profiles.get(&person.employee_id);
    let role = role_title(i18n, profile, manager);
    let selected = board.employee.as_deref() == Some(person.employee_id.as_str());
    let participant = focus.is_some_and(|task| {
        let id = person.employee_id.as_str();
        task.owner_ref.as_deref() == Some(id)
            || task.input_text("employee_id") == Some(id)
            || board
                .details
                .get(&task.task_id)
                .and_then(|entry| entry.as_ref())
                .and_then(|entry| entry.as_ref().ok())
                .is_some_and(|record| {
                    record.detail.steps.iter().any(|step| task.step_employee(step) == Some(id))
                })
    });
    let identity =
        column![strong(person.name.as_str(), 14, Tone::Ink), label(role, 12, Tone::Muted),]
            .spacing(3);
    let height = if manager { 116.0 } else { 156.0 };
    let picture = image(portraits::office_avatar(&person.name, manager))
        .width(height * 0.75)
        .height(height * 0.75);
    let scene = stack![
        container(container(picture).padding(iced::Padding::ZERO.top(6)))
            .center_x(Length::Fill)
            .height(height),
        canvas(Furniture { tone: state.tone, bright: board.pulse && state.working })
            .width(Length::Fill)
            .height(height),
    ];
    let speech = label(
        if state.working && board.pulse {
            format!("{} …", state.action)
        } else {
            state.action.to_owned()
        },
        12,
        Tone::Muted,
    );
    let title = data
        .tasks
        .iter()
        .find(|task| task.owner_ref.as_deref() == Some(person.employee_id.as_str()))
        .map(|task| task.title.as_str())
        .or_else(|| person.current_work.as_ref().map(|work| work.task_name.as_str()))
        .unwrap_or(i18n.__("在岗，可继续安排工作"));
    let body = column![
        identity,
        status(state.name, state.tone),
        scene,
        strong(title, 12, Tone::Ink),
        speech
    ]
    .spacing(6)
    .width(Length::Fill);
    button(body)
        .padding(9)
        .width(Length::Fill)
        .on_press(message(Event::Select(person.employee_id.clone())))
        .style(move |theme, status| {
            let colors = Palette::new(theme);
            let mut style = rounded_action_btn_style(theme, status);
            style.background = Some(
                if selected || participant {
                    colors.accent.scale_alpha(if selected { 0.16 } else { 0.09 })
                } else if status == button::Status::Active {
                    colors.surface
                } else {
                    colors.header
                }
                .into(),
            );
            style.text_color = colors.ink;
            style.border = iced::border::rounded(8);
            style
        })
        .into()
}

fn inspector<'a>(i18n: desktop_i18n::I18n, board: &'a Board, data: &'a Snapshot, person: &'a Employee) -> Item<'a> {
    let state = work_state(i18n, person, &data.tasks);
    let manager = person.agent_kind == "manager";
    let profile = data.profiles.get(&person.employee_id);
    let role = role_title(i18n, profile, manager);
    let portrait =
        portraits::avatar(&person.name, manager).map(|handle| image(handle).width(44).height(44));
    let header = row![
        portrait,
        column![
            strong(person.name.as_str(), 17, Tone::Ink),
            label(role, 12, Tone::Muted),
            status(state.name, state.tone),
        ]
        .spacing(4),
    ]
    .align_y(Alignment::Center)
    .spacing(10);
    let mut info = column![
        label(i18n.__("部门"), 11, Tone::Muted),
        strong(nonempty(person.department_path.as_deref(), i18n.__("部门未登记")), 13, Tone::Ink),
    ]
    .spacing(6);
    if let Some(manager) = profile
        .and_then(|profile| profile.manager_name.as_deref())
        .filter(|name| !name.trim().is_empty())
    {
        info = info.push(label(i18n.__("所属主管"), 11, Tone::Muted)).push(strong(manager, 13, Tone::Ink));
    }
    let mut body = column![info].spacing(11);
    if let Some(profile) = profile {
        let tags = profile
            .tags
            .iter()
            .map(|tag| tag.trim())
            .filter(|tag| !tag.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !tags.is_empty() {
            body = body.push(label(tags, 12, Tone::Muted));
        }
    }
    body = body.push(
        column![strong(i18n.__("当前工作"), 13, Tone::Ink), label(state.action, 12, Tone::Muted)].spacing(5),
    );
    let related: Vec<_> = data
        .tasks
        .iter()
        .filter(|task| {
            task.owner_ref.as_deref() == Some(person.employee_id.as_str())
                || (manager && task.input_text("employee_id") == Some(person.employee_id.as_str()))
        })
        .take(3)
        .collect();
    for task in related.iter().copied().filter(|task| task.running() || task.waiting()) {
        let (caption, tone) = task_status(i18n, task);
        body = body.push(
            column![
                status(caption, tone),
                strong(task.title.as_str(), 13, Tone::Ink),
                label(task_ref(i18n, &task.task_id), 11, Tone::Muted),
                progress(i18n, task),
                link(i18n.__("查看任务 ↗"), web("/tasks", Some(("task", &task.task_id)))),
            ]
            .spacing(7),
        );
    }
    let mut outputs = Vec::new();
    let mut records = Vec::new();
    let mut incomplete = false;
    let mut unavailable = false;
    for task in related {
        match board.details.get(&task.task_id) {
            Some(Some(Ok(record))) => {
                if let Some(handoff) = &record.handoff {
                    incomplete |= !handoff.record_complete;
                    outputs.extend(handoff.outputs.iter().map(|output| (task, output)));
                }
                if let Some(collaboration) = &record.collaboration {
                    incomplete |= !collaboration.record_complete;
                    records.extend(collaboration.entries.iter().filter(|entry| {
                        entry.actor.employee_ref.as_deref() == Some(person.employee_id.as_str())
                            || entry
                                .mentions
                                .iter()
                                .any(|mention| mention.employee_ref == person.employee_id)
                    }));
                }
                if record.handoff.is_none() || record.collaboration.is_none() {
                    unavailable = true;
                    body = body.push(link(i18n.__("重试证据"), message(Event::Retry(task.task_id.clone()))));
                }
            }
            Some(Some(Err(_))) => {
                unavailable = true;
                body = body.push(link(i18n.__("重试任务记录"), message(Event::Retry(task.task_id.clone()))));
            }
            _ => {}
        }
    }
    records.sort_by(|a, b| b.occurred_at.cmp(&a.occurred_at));
    body = body.push(strong(i18n.__("最近协作记录"), 13, Tone::Ink));
    if incomplete || unavailable {
        body = body.push(label(i18n.__("部分协作记录暂不可用"), 11, Tone::Muted));
    }
    for entry in records.into_iter().take(3) {
        let kind = match entry.kind.as_str() {
            "DISPATCH" => i18n.__("派工"),
            "ACKNOWLEDGEMENT" => i18n.__("接单确认"),
            "EMPLOYEE_REPORT" => i18n.__("员工回报"),
            "HUMAN_MESSAGE" => i18n.__("人工消息"),
            _ => i18n.__("协作记录"),
        };
        let mut entry_view = column![
            strong(format!("{kind} · {}", entry.actor.label), 12, Tone::Ink),
            label(format_time(entry.occurred_at.as_deref()), 11, Tone::Muted),
        ]
        .spacing(4);
        if let Some(summary) = &entry.summary {
            entry_view = entry_view.push(label(summary.as_str(), 12, Tone::Muted));
        }
        body = body.push(entry_view);
    }
    let count = outputs.len();
    body = body.push(row![
        strong(i18n.__("参与任务的成果"), 13, Tone::Ink).width(Length::Fill),
        label(i18n.format("%s 份", &[format!("{}", count).into()]).expect("valid workbench template"), 11, Tone::Muted),
    ]);
    for (task, output) in outputs.into_iter().take(3) {
        let title = nonempty(output.name.as_deref(), i18n.__("未命名成果"));
        body = body.push(link(title, web("/tasks", Some(("task", &task.task_id)))));
    }
    body = body.push(action(
        i18n.__("查看员工资料 ↗"),
        None,
        Some(Message::View(ViewMessage::OpenUrlExternal(employee_config_url(&person.employee_id)))),
        false,
    ));
    column![header, body].spacing(16).padding(16).width(Length::Fill).into()
}

fn tasks_panel<'a>(i18n: desktop_i18n::I18n, board: &'a Board, data: &'a Snapshot, width: f32) -> Item<'a> {
    let controls = segmented(
        row![
            segment(i18n.__("执行中"), board.queue == Queue::Running, message(Event::Queue(Queue::Running))),
            segment(
                i18n.__("等你拍板"),
                board.queue == Queue::Waiting,
                message(Event::Queue(Queue::Waiting))
            ),
            segment(i18n.__("全部任务"), board.queue == Queue::All, message(Event::Queue(Queue::All))),
        ]
        .spacing(2)
        .wrap()
        .into(),
    );
    let title = column![
        strong(i18n.__("正在发生的工作"), 16, Tone::Ink),
        label(i18n.__("任务中心的任务 · 查看整体进度、节点状态和每一步的负责人"), 12, Tone::Muted),
    ]
    .spacing(5)
    .width(Length::Fill);
    let tasks: Vec<_> = data.tasks.iter().filter(|task| board.queue.includes(task)).collect();
    let page_count = tasks.len().div_ceil(PAGE_SIZE).max(1);
    let cards: Vec<_> = tasks
        .into_iter()
        .skip(board.page.saturating_mul(PAGE_SIZE))
        .take(PAGE_SIZE)
        .map(|task| task_card(i18n, board, data, task))
        .collect();
    let list = if cards.is_empty() {
        label(i18n.__("当前没有符合条件的任务"), 13, Tone::Muted).into()
    } else {
        grid(
            cards,
            if width >= 1140.0 {
                3
            } else if width >= 760.0 {
                2
            } else {
                1
            },
            12,
        )
    };
    let mut content = column![section_header(title, controls, width < 750.0), list]
        .spacing(14)
        .width(Length::Fill);
    if page_count > 1 {
        content = content.push(
            container(
                row![
                    action(
                        i18n.__("上一页"),
                        None,
                        board.page.checked_sub(1).map(|page| message(Event::Page(page))),
                        false
                    ),
                    label(format!("{} / {page_count}", board.page + 1), 12, Tone::Muted),
                    action(
                        i18n.__("下一页"),
                        None,
                        (board.page + 1 < page_count).then(|| message(Event::Page(board.page + 1))),
                        false
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([4, 12])
            .center_x(Length::Fill),
        );
    }
    content.into()
}

fn task_card<'a>(i18n: desktop_i18n::I18n, board: &'a Board, data: &'a Snapshot, task: &'a WorkItem) -> Item<'a> {
    let workflow = task
        .input_text("workflow_id")
        .and_then(|id| data.workflows.get(id))
        .map(String::as_str)
        .or_else(|| task.input_text("workflow_name"))
        .unwrap_or(i18n.__("协作任务"));
    let (state, tone) = task_status(i18n, task);
    let head = row![label(workflow, 12, Tone::Muted).width(Length::Fill), status(state, tone),]
        .spacing(8)
        .align_y(Alignment::Center);
    let detail = board.details.get(&task.task_id).and_then(|entry| entry.as_ref());
    let current = detail.and_then(|entry| entry.as_ref().ok()).map(|record| &record.detail.steps);
    let active_steps: Vec<_> =
        current.into_iter().flatten().filter(|step| step.status == "RUNNING").collect();
    let mut current_node = column![label(
        if active_steps.len() > 1 { i18n.__("当前并行节点") } else { i18n.__("当前节点") },
        11,
        Tone::Muted
    ),]
    .spacing(4);
    if active_steps.is_empty() {
        current_node = current_node.push(label(
            nonempty(task.current_step.as_deref(), i18n.__("暂无当前节点")),
            13,
            Tone::Ink,
        ));
    } else {
        for step in active_steps {
            let (state, _) = step_status(i18n, task, step);
            let assignee = step_actor(data, task, step).unwrap_or(i18n.__("负责人未分配"));
            current_node = current_node.push(label(
                format!("{assignee} · {state} · {}", nonempty(step.name.as_deref(), i18n.__("未命名节点"))),
                12,
                Tone::Ink,
            ));
        }
    }
    let mut body = column![
        strong(nonempty(Some(&task.title), i18n.__("未命名任务")), 15, Tone::Ink),
        label(task_ref(i18n, &task.task_id), 11, Tone::Muted),
        progress(i18n, task),
        current_node,
    ]
    .spacing(9);
    if let Some(blocker) = task.blocker.as_deref().filter(|value| !value.trim().is_empty()) {
        body = body.push(label(blocker, 12, Tone::Red));
    }
    let mut nodes = column![].spacing(8);
    match detail {
        Some(Ok(record)) => {
            let steps = &record.detail.steps;
            if steps.is_empty() {
                nodes = nodes.push(label(i18n.__("任务中心尚未提供节点记录"), 12, Tone::Muted));
            } else {
                let done = steps.iter().filter(|step| step.status == "SUCCEEDED").count();
                nodes = nodes.push(row![
                    strong(i18n.__("任务节点"), 13, Tone::Ink).width(Length::Fill),
                    label(i18n.format("已完成 %s / %s", &[format!("{}", done).into(), format!("{}", steps.len()).into()]).expect("valid workbench template"), 11, Tone::Muted),
                ]);
                let entries = column(
                    steps.iter().enumerate().map(|(index, step)| node(i18n, data, task, step, index)),
                )
                .spacing(8)
                .width(Length::Fill);
                nodes = nodes.push(
                    container(
                        iced::widget::scrollable(entries).height(Length::Shrink).direction(
                            iced::widget::scrollable::Direction::Vertical(
                                iced::widget::scrollable::Scrollbar::new()
                                    .width(4)
                                    .scroller_width(4)
                                    .margin(3),
                            ),
                        ),
                    )
                    .max_height(116),
                );
            }
        }
        Some(Err(error)) => {
            nodes = nodes
                .push(label(error.render(i18n), 12, Tone::Red))
                .push(link(i18n.__("重试节点"), message(Event::Retry(task.task_id.clone()))));
        }
        None => nodes = nodes.push(label(i18n.__("正在读取节点…"), 12, Tone::Muted)),
    }
    let expanded = board.expanded.as_deref() == Some(task.task_id.as_str());
    body = body.push(nodes).push(link(
        if expanded { i18n.__("收起工作交接") } else { i18n.__("工作交接") },
        message(Event::Expand(task.task_id.clone())),
    ));
    if expanded {
        let handoff =
            detail.and_then(|entry| entry.as_ref().ok()).and_then(|record| record.handoff.as_ref());
        body = body.push(match handoff {
            Some(handoff) => handoff_view(i18n, handoff),
            None if detail.is_none() => label(i18n.__("正在读取工作交接…"), 12, Tone::Muted).into(),
            None => link(i18n.__("重试工作交接"), message(Event::Retry(task.task_id.clone()))),
        });
    }
    body = body.push(label(
        i18n.format("更新于 %s", &[format!("{}", format_time(task.updated_at.as_deref())).into()]).expect("valid workbench template"),
        11,
        Tone::Muted,
    ));
    let footer = row![
        action(
            i18n.__("在办公室看协作"),
            None,
            Some(message(Event::Focus(Some(task.task_id.clone())))),
            false
        ),
        link(i18n.__("任务详情 ↗"), web("/tasks", Some(("task", &task.task_id)))),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .wrap();
    container(column![head, body, divider(), footer].spacing(12))
        .padding(16)
        .width(Length::Fill)
        .style(move |theme| {
            let mut style = panel_style(theme);
            if board.focus.as_deref() == Some(task.task_id.as_str()) {
                style.border.color = Tone::Accent.color(theme);
            }
            style
        })
        .into()
}

fn step_actor<'a>(data: &'a Snapshot, task: &'a WorkItem, step: &'a Step) -> Option<&'a str> {
    step.actor_name.as_deref().filter(|name| !name.trim().is_empty()).or_else(|| {
        let id = task.step_employee(step)?;
        employees(data)
            .iter()
            .find(|person| person.employee_id == id)
            .map(|person| person.name.as_str())
    })
}

fn node<'a>(i18n: desktop_i18n::I18n, data: &'a Snapshot, task: &'a WorkItem, step: &'a Step, index: usize) -> Item<'a> {
    let (state, tone) = step_status(i18n, task, step);
    let marker =
        if step.status == "SUCCEEDED" { "✓".to_owned() } else { format!("{:02}", index + 1) };
    let name = step_actor(data, task, step).unwrap_or(i18n.__("尚未分配"));
    let actor = task
        .step_employee(step)
        .filter(|id| employees(data).iter().any(|person| person.employee_id == *id));
    let actor: Item<'_> = match actor {
        Some(id) => link(name, message(Event::Select(id.to_owned()))),
        None => label(name, 12, Tone::Ink).into(),
    };
    let human = matches!(step.kind.as_deref(), Some("approval" | "input" | "takeover" | "human"));
    let mut info = column![
        row![
            label(nonempty(step.name.as_deref(), i18n.__("未命名节点")), 13, Tone::Ink).width(Length::Fill),
            label(state, 11, tone)
        ]
        .spacing(6),
        row![label(if human { i18n.__("处理人") } else { i18n.__("执行员工") }, 11, Tone::Muted), actor]
            .spacing(7)
            .align_y(Alignment::Center),
    ]
    .spacing(3)
    .width(Length::Fill);
    if step.status == "FAILED"
        && let Some(reason) = &step.summary
    {
        info = info.push(label(reason.as_str(), 12, Tone::Red));
    }
    row![
        container(label(marker, 12, tone))
            .center(24)
            .style(move |theme| tint_style(theme, tone, 6.0)),
        info,
    ]
    .spacing(8)
    .into()
}

fn task_status(i18n: desktop_i18n::I18n, task: &WorkItem) -> (&str, Tone) {
    if task.paused() {
        return (i18n.__("已暂停"), Tone::Warning);
    }
    match task.status.as_str() {
        "CREATED" | "PENDING" => (i18n.__("未开始"), Tone::Muted),
        "RUNNING" => (i18n.__("执行中"), Tone::Accent),
        "QUEUED" => (i18n.__("排队中"), Tone::Muted),
        "SCHEDULED" => (i18n.__("定时待触发"), Tone::Muted),
        "SUCCEEDED" => (i18n.__("已完成"), Tone::Green),
        "FAILED" => (i18n.__("失败"), Tone::Red),
        "CANCELLED" => (i18n.__("已取消"), Tone::Muted),
        "CANCEL_REQUESTED" => (i18n.__("取消中"), Tone::Muted),
        "REJECTED" => (i18n.__("被驳回"), Tone::Red),
        "WAITING_APPROVAL" => (i18n.__("待人工审批"), Tone::Warning),
        "WAITING_INPUT" => (i18n.__("待补充材料"), Tone::Warning),
        "HUMAN_TAKEOVER" => (i18n.__("人工接管中"), Tone::Warning),
        _ => (nonempty(task.status_label.as_deref(), i18n.__("状态未提供")), Tone::Muted),
    }
}

fn step_status(i18n: desktop_i18n::I18n, task: &WorkItem, step: &Step) -> (&'static str, Tone) {
    match step.status.as_str() {
        "SUCCEEDED" => (i18n.__("已完成"), Tone::Green),
        "FAILED" => (i18n.__("失败"), Tone::Red),
        "PENDING" => (i18n.__("未开始"), Tone::Muted),
        "WAITING" => (i18n.__("等待中"), Tone::Warning),
        "SKIPPED" => (i18n.__("已跳过"), Tone::Muted),
        "CANCELLED" => (i18n.__("已取消"), Tone::Muted),
        "RUNNING" if task.paused() => (i18n.__("已暂停"), Tone::Warning),
        "RUNNING" => match step.kind.as_deref() {
            Some("approval") => (i18n.__("待人工审批"), Tone::Warning),
            Some("input") => (i18n.__("待补充材料"), Tone::Warning),
            Some("takeover") => (i18n.__("人工接管中"), Tone::Warning),
            _ => (i18n.__("执行中"), Tone::Accent),
        },
        _ => (i18n.__("状态未记录"), Tone::Muted),
    }
}

fn progress(i18n: desktop_i18n::I18n, task: &WorkItem) -> Item<'_> {
    let value = task.progress();
    let mut content = column![
        row![
            label(
                if task.progress_estimated { i18n.__("预估任务进度") } else { i18n.__("任务进度") },
                12,
                Tone::Muted
            )
            .width(Length::Fill),
            strong(
                value.map_or_else(|| i18n.__("未提供").to_owned(), |pct| format!("{pct:.0}%")),
                14,
                Tone::Ink
            ),
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(5);
    if let Some(value) = value {
        content = content.push(progress_bar(0.0..=100.0, value).girth(3).style(|theme| {
            let mut style = progress_bar::primary(theme);
            style.background = Palette::new(theme).border.into();
            style.bar = ORANGE.into();
            style
        }));
    }
    content.into()
}

fn role_title(i18n: desktop_i18n::I18n, profile: Option<&Profile>, manager: bool) -> &str {
    nonempty(
        profile.and_then(|profile| profile.role_title.as_deref()),
        if manager { i18n.__("任务主管") } else { i18n.__("执行员工") },
    )
}

fn nonempty<'a>(value: Option<&'a str>, fallback: &'a str) -> &'a str {
    value.filter(|value| !value.trim().is_empty()).unwrap_or(fallback)
}

fn section_header<'a>(title: Column<'a, Message>, controls: Item<'a>, narrow: bool) -> Item<'a> {
    if narrow {
        column![title, controls].spacing(10).into()
    } else {
        row![title, controls].spacing(12).align_y(Alignment::Center).into()
    }
}

fn grid<'a>(cells: Vec<Item<'a>>, columns: usize, gap: u32) -> Item<'a> {
    let mut cells = cells.into_iter();
    let mut rows = Column::new().spacing(gap).width(Length::Fill);
    loop {
        let Some(first) = cells.next() else { break };
        let mut row = Row::new().spacing(gap).width(Length::Fill).push(first);
        for _ in 1..columns {
            row = row.push(cells.next().unwrap_or_else(|| Space::new().width(Length::Fill).into()));
        }
        rows = rows.push(row);
    }
    rows.into()
}

struct Furniture {
    tone: Tone,
    bright: bool,
}

impl canvas::Program<Message> for Furniture {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let colors = Palette::new(theme);
        let scale = (bounds.width / 220.0).min(bounds.height / 180.0);
        if scale <= 0.0 {
            return Vec::new();
        }
        frame.translate(iced::Vector::new(
            (bounds.width - 220.0 * scale) / 2.0,
            bounds.height - 180.0 * scale,
        ));
        frame.scale(scale);
        frame.fill(&canvas::Path::circle(Point::new(110.0, 164.0), 1.0), colors.muted);
        frame.fill_rectangle(
            Point::new(40.0, 146.0),
            Size::new(4.0, 22.0),
            colors.muted.scale_alpha(0.58),
        );
        frame.fill_rectangle(
            Point::new(176.0, 146.0),
            Size::new(4.0, 22.0),
            colors.muted.scale_alpha(0.58),
        );
        let chair = canvas::Path::new(|path| {
            path.move_to(Point::new(110.0, 146.0));
            path.line_to(Point::new(110.0, 164.0));
            path.line_to(Point::new(89.0, 169.0));
            path.move_to(Point::new(110.0, 164.0));
            path.line_to(Point::new(131.0, 169.0));
        });
        frame.stroke(&chair, canvas::Stroke::default().with_width(4.0).with_color(colors.muted));
        frame.fill_rectangle(Point::new(25.0, 141.0), Size::new(170.0, 6.0), colors.surface);
        let top = canvas::Path::new(|path| {
            path.move_to(Point::new(25.0, 141.0));
            path.line_to(Point::new(43.0, 126.0));
            path.line_to(Point::new(177.0, 126.0));
            path.line_to(Point::new(195.0, 141.0));
            path.close();
        });
        frame.fill(&top, colors.surface);
        frame.stroke(&top, canvas::Stroke::default().with_color(colors.border));
        frame.fill_rectangle(
            Point::new(27.0, 143.0),
            Size::new(166.0, 2.0),
            self.tone.color(theme).scale_alpha(if self.bright { 0.95 } else { 0.54 }),
        );
        frame.fill_rectangle(Point::new(58.0, 124.0), Size::new(12.0, 14.0), ORANGE);
        let laptop = canvas::Path::new(|path| {
            path.move_to(Point::new(89.0, 111.0));
            path.line_to(Point::new(145.0, 111.0));
            path.line_to(Point::new(142.0, 138.0));
            path.line_to(Point::new(93.0, 138.0));
            path.close();
        });
        frame.fill(&laptop, Color::from_rgb8(212, 218, 228));
        frame.stroke(&laptop, canvas::Stroke::default().with_color(colors.muted));
        frame.fill_rectangle(Point::new(86.0, 138.0), Size::new(62.0, 3.0), colors.muted);
        frame.fill_rectangle(Point::new(98.0, 120.0), Size::new(30.0, 2.0), colors.muted);
        frame.fill_rectangle(
            Point::new(98.0, 126.0),
            Size::new(if self.bright { 25.0 } else { 18.0 }, 2.0),
            colors.muted,
        );
        vec![frame.into_geometry()]
    }
}

fn handoff_view(i18n: desktop_i18n::I18n, handoff: &Handoff) -> Item<'_> {
    let mut content = column![].spacing(6);
    if !handoff.record_complete {
        content = content.push(label(i18n.__("部分交接记录暂不可用"), 11, Tone::Muted));
    }
    for (title, notes) in [(i18n.__("接到的要求"), &handoff.inputs), (i18n.__("最近工作回报"), &handoff.reports)]
    {
        if !notes.is_empty() {
            content = content.push(strong(title, 12, Tone::Ink));
            for note in notes.iter().rev().take(3) {
                content = content.push(label(
                    nonempty(note.summary.as_deref(), i18n.__("正文未提供或不可见")),
                    12,
                    Tone::Muted,
                ));
            }
        }
    }
    content =
        content.push(strong(i18n.format("交付成果 · %s 份", &[format!("{}", handoff.outputs.len()).into()]).expect("valid workbench template"), 12, Tone::Ink));
    for output in handoff.outputs.iter().take(3) {
        let name = nonempty(output.name.as_deref(), i18n.__("未命名成果"));
        let producer = nonempty(output.producer.as_deref(), i18n.__("提交人未记录"));
        content = content.push(label(format!("{name} · {producer}"), 12, Tone::Muted));
    }
    if !handoff.reviews.is_empty() {
        content = content.push(strong(i18n.__("验收与修改意见"), 12, Tone::Ink));
        for review in handoff.reviews.iter().rev().take(3) {
            let status = match review.outcome.as_str() {
                "ACCEPTED" => i18n.__("验收通过"),
                "REVISION_REQUESTED" => i18n.__("需要修改"),
                _ => i18n.__("结论未确认"),
            };
            content = content.push(label(
                format!("{status} · {}", nonempty(review.summary.as_deref(), i18n.__("未登记意见"))),
                12,
                Tone::Muted,
            ));
        }
    }
    content.into()
}
