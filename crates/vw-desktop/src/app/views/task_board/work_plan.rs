use crate::app::components::system_settings_common::*;
use crate::app::i18n::{I18n, tasks::status_label};
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_task_board::board::{task_status_marker, task_title};
use desktop_task_board::work_views::{WorkPlanGroupKey as Key, WorkPlanProgress, WorkPlanView};
use desktop_task_model::{Task, TaskWorkflow};
use iced::widget::{Column, button, column, container, row, scrollable, text};
use iced::{Element, Length};
use vw_api_agent_scheduler::{
    AgentSchedulerDispatchBlockerDto as Blocker, AgentSchedulerTaskPlanPlacementDto as Placement,
};

const CHILD_PROMPT_INPUT: &str = "work-plan-child-prompt";
pub(super) const OUTER_INPUT: &str = "work-plan-outer-batch";

fn message(value: TaskBoardMessage) -> Message {
    Message::TaskBoard(value)
}

pub(super) fn open_detail(project: &str, id: &str) -> Message {
    message(TaskBoardMessage::OpenTaskDetail { project_path: project.into(), task_id: id.into() })
}

pub(super) fn current(app: &App) -> Option<(&str, &WorkPlanView)> {
    let project = app.navigation_tabs.project_path.as_deref()?;
    let plan = &app.task_board.work_plan;
    (plan.project.as_deref() == Some(project)).then_some((project, plan))
}

fn workflow(app: &App) -> TaskWorkflow {
    app.task_board
        .task_board_settings
        .workflow()
        .unwrap_or_else(|| TaskWorkflow::from(desktop_task_model::default_task_workflow()))
}

fn progress(i18n: desktop_i18n::I18n, value: &WorkPlanProgress) -> String {
    i18n.format("完成 %s/%s · 进行中 %s · 等待 %s", &[format!("{}", value.completed).into(), format!("{}", value.total).into(), format!("{}", value.executing).into(), format!("{}", value.waiting).into()]).expect("valid workbench template")
}

fn sum<'a>(items: impl Iterator<Item = &'a WorkPlanProgress>) -> WorkPlanProgress {
    items.fold(WorkPlanProgress::default(), |mut total, item| {
        total.total += item.total;
        total.completed += item.completed;
        total.executing += item.executing;
        total.waiting += item.waiting;
        total
    })
}

fn action<'a>(label: impl Into<String>) -> button::Button<'a, Message> {
    button(text(label.into()).size(13)).padding([6, 10]).style(rounded_action_btn_style)
}

fn group_button<'a>(plan: &WorkPlanView, key: Key, label: String) -> button::Button<'a, Message> {
    let arrow = if plan.collapsed.contains(&key) { "▸" } else { "▾" };
    action(format!("{arrow} {label}"))
        .width(Length::Fill)
        .on_press(message(TaskBoardMessage::ToggleWorkPlanGroup(key)))
}

fn scroll<'a>(body: Column<'a, Message>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(body).width(Length::Fill).padding([0, 7])).direction(
        scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    )
}

fn blocker_label(i18n: desktop_i18n::I18n, blocker: &Blocker) -> String {
    match blocker {
        Blocker::WaitingForEarlierBatch { task_ids } => {
            i18n.format("等待前序批次 · %s", &[format!("{}", task_ids.join("、")).into()]).expect("valid workbench template")
        }
        Blocker::EarlierBatchFailed { task_ids } => {
            i18n.format("前序批次失败 · %s", &[format!("{}", task_ids.join("、")).into()]).expect("valid workbench template")
        }
        Blocker::ParentNotReleased { parent_task_id } => {
            i18n.format("等待父任务放行 · %s", &[format!("{}", parent_task_id).into()]).expect("valid workbench template")
        }
        Blocker::LegacyWorkPlanRequiresResplit => vw_api_agent_scheduler::LEGACY_WORK_PLAN_RESPLIT_REASON.to_owned(),
        Blocker::WorkPlanNotPublished => i18n.__("等待启动工作计划").to_owned(),
        Blocker::ExplicitDependency { task_ids } => {
            i18n.format("等待任务依赖 · %s", &[format!("{}", task_ids.join("、")).into()]).expect("valid workbench template")
        }
        Blocker::WorkPlanChildrenIncomplete { children } => {
            let reasons = children.iter().map(|child| {
                let status = match child.status.as_str() {
                    "submitted" => i18n.__("待审核或合并"),
                    "failed" => i18n.__("失败"),
                    "cancelled" => i18n.__("已取消"),
                    "paused" => i18n.__("已暂停"),
                    "archived" => i18n.__("已归档"),
                    _ => &child.status_key,
                };
                format!("{}（{status}）", child.task_id)
            }).collect::<Vec<_>>();
            i18n.format("等待子任务完成 · %s", &[format!("{}", reasons.join("、")).into()]).expect("valid workbench template")
        }
        Blocker::WorkPlanSummaryUnavailable => i18n.__("汇总暂不可派发").into(),
    }
}

fn task_row<'a>(
    app: &App,
    project: &str,
    plan: &WorkPlanView,
    task: &'a Task,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let snapshot = plan.tasks.get(&task.id);
    let status = snapshot
        .and_then(|snapshot| snapshot.status_label.as_deref())
        .map_or_else(|| status_label(i18n, task.status), |label| i18n.__(label).to_owned());
    let selected = app.task_board.task_board_editing_task_id.as_ref() == Some(&task.id);
    let agent = task
        .acp_agent
        .as_deref()
        .or(task.agent.as_deref())
        .unwrap_or(desktop_task_model::TASK_AGENT_MAIN);
    let mut body = column![row![
        task_status_marker(task.status),
        text(task_title(task)).size(13).width(Length::Fill),
        text(&task.id).size(11).style(settings_muted_text_style),
    ].spacing(8).align_y(iced::Alignment::Center),
    text(format!("{status} · {agent}")).size(11).style(settings_muted_text_style),
    ].spacing(4);
    if let Some(blocker) = snapshot.and_then(|snapshot| snapshot.blocker.as_ref()) {
        body = body.push(text(blocker_label(i18n, blocker)).size(12).style(settings_muted_text_style));
    }
    if let Some(error) = task.last_error.as_deref().filter(|error| !error.is_empty()) {
        body = body.push(text(i18n.format("执行失败 · %s", &[format!("{}", error).into()]).expect("valid workbench template")).size(12));
    }
    button(body)
        .padding([7, 10])
        .width(Length::Fill)
        .style(move |theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(open_detail(project, &task.id))
        .into()
}

pub(super) fn project_view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let Some((project, plan)) = current(app) else {
        return text(i18n.__("正在加载工作计划…")).into();
    };
    let board = &app.task_board;
    let search = plan.search.trim().to_lowercase();
    let groups = plan.roots(&board.task_board_tasks, &workflow(app), |task| {
        board.work_task_visible(project, task)
            && (task.id.to_lowercase().contains(&search)
                || task.description.to_lowercase().contains(&search)
                || task.prompt.to_lowercase().contains(&search))
    });
    let mut body = column![].spacing(8);
    if groups.values().all(|group| group.rows.is_empty()) {
        body = body.push(text(if board.task_board_loading {
            i18n.__("正在加载工作计划…")
        } else if groups.is_empty() {
            i18n.__("当前项目暂无主任务")
        } else {
            i18n.__("没有匹配的主任务")
        }));
    }
    if let Some(group) = groups.get(&None).filter(|group| !group.rows.is_empty()) {
        body = body.push(group_button(
            plan,
            Key::Ordinary,
            i18n.format("普通主任务 · %s", &[format!("{}", progress(i18n, &group.progress)).into()]).expect("valid workbench template"),
        ));
        if !plan.collapsed.contains(&Key::Ordinary) {
            for task in &group.rows {
                body = body.push(task_row(app, project, plan, task));
            }
        }
    }
    let mut outer_groups = std::collections::BTreeMap::<u32, Vec<_>>::new();
    for (key, group) in &groups {
        if let Some((outer, inner)) = *key {
            outer_groups.entry(outer).or_default().push((inner, group));
        }
    }
    for (outer, batches) in outer_groups {
        if batches.iter().all(|(_, group)| group.rows.is_empty()) {
            continue;
        }
        let key = Key::Outer(outer);
        let total = sum(batches.iter().map(|(_, group)| &group.progress));
        body = body.push(group_button(
            plan,
            key.clone(),
            i18n.format("外层批次 %s · %s", &[format!("{}", outer).into(), format!("{}", progress(i18n, &total)).into()]).expect("valid workbench template"),
        ));
        if plan.collapsed.contains(&key) {
            continue;
        }
        let mut contents = column![].spacing(6);
        for (inner, group) in batches.into_iter().filter(|(_, group)| !group.rows.is_empty()) {
            let key = Key::Root(outer, inner);
            contents = contents.push(group_button(
                plan,
                key.clone(),
                format!("{outer}.{inner} · {}", progress(i18n, &group.progress)),
            ));
            if !plan.collapsed.contains(&key) {
                for task in &group.rows {
                    contents = contents.push(task_row(app, project, plan, task));
                }
            }
        }
        body = body.push(container(contents).padding([0, 12]));
    }
    column![
        text(i18n.__("工作计划")).size(20),
        settings_text_input(i18n.__("搜索任务标题、编号或提示词"), &plan.search)
            .on_input(|value| message(TaskBoardMessage::WorkPlanSearchChanged(value))),
        scroll(body).height(Length::Fill),
    ]
    .spacing(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

pub(super) fn detail<'a>(app: &'a App, task: &Task) -> Option<Element<'a, Message>> {
    if !task.subtasks.is_empty() {
        let project = app.navigation_tabs.project_path.as_deref()?;
        if !desktop_task_board::work_views::can_resplit_legacy_work_plan(task, &workflow(app)) {
            return None;
        }
        let i18n = I18n::new(app.general_settings.desktop_locale);
        return Some(column![
            text(i18n.__(vw_api_agent_scheduler::LEGACY_WORK_PLAN_RESPLIT_REASON)).size(13),
            action(i18n.__("重新拆分")).style(primary_action_btn_style).on_press_maybe(
                (!app.task_board.work_plan.publishing.contains(&task.id)).then(|| {
                    message(TaskBoardMessage::ResplitLegacyWorkPlan {
                        project_path: project.to_owned(),
                        task_id: task.id.clone(),
                    })
                })
            ),
        ].spacing(8).into());
    }
    let (project, plan) = current(app)?;
    let snapshot = plan.tasks.get(&task.id)?;
    let mut body = column![].spacing(8);
    if let Some(blocker) = &snapshot.blocker {
        body = body.push(text(blocker_label(i18n, blocker)).size(13));
    }
    if let Some(Placement::Child { parent_task_id, batch }) = &snapshot.placement {
        let parent = app.task_board.task_board_tasks.iter().find(|task| &task.id == parent_task_id);
        return Some(
            body.push(
                row![
                    text(i18n.format("子任务批次 %s", &[format!("{}", batch).into()]).expect("valid workbench template")).size(13),
                    action(i18n.format("返回父任务 %s", &[format!("{}", parent_task_id).into()]).expect("valid workbench template"))
                        .on_press_maybe(parent.map(|_| open_detail(project, parent_task_id))),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .into(),
        );
    }

    let groups = plan.children(&app.task_board.task_board_tasks, &workflow(app), &task.id);
    let publishing = plan.publishing.contains(&task.id);
    let can_create = !task.archived && !task.deleted && !publishing;
    let can_start = can_create
        && !groups.is_empty()
        && matches!(&snapshot.blocker, Some(Blocker::WorkPlanNotPublished));
    let total = sum(groups.values().map(|group| &group.progress));
    body = body
        .push(settings_field_label(i18n.__("子任务")))
        .push(
            row![
                action(i18n.__("新增子任务")).on_press_maybe(
                    can_create.then_some(message(TaskBoardMessage::OpenWorkPlanChild))
                ),
                action(if publishing { i18n.__("正在启动…") } else { i18n.__("开始执行计划") })
                    .style(primary_action_btn_style)
                    .on_press_maybe(can_start.then(|| message(TaskBoardMessage::StartWorkPlan {
                        project_path: project.into(),
                        task_id: task.id.clone(),
                    }))),
            ]
            .spacing(8)
            .wrap(),
        )
        .push(
            text(if groups.is_empty() { i18n.__("暂无子任务").into() } else { progress(i18n, &total) }).size(13),
        );
    for (batch, group) in &groups {
        let key = Key::Child(task.id.clone(), *batch);
        body = body.push(group_button(
            plan,
            key.clone(),
            i18n.format("批次 %s · %s", &[format!("{}", batch).into(), format!("{}", progress(i18n, &group.progress)).into()]).expect("valid workbench template"),
        ));
        if !plan.collapsed.contains(&key) {
            for child in &group.rows {
                body = body.push(task_row(app, project, plan, child));
            }
        }
    }
    Some(body.into())
}

pub(super) fn history<'a>(app: &'a App, task: &Task) -> Option<Element<'a, Message>> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let (_, plan) = current(app)?;
    let parent = plan.tasks.values().any(|snapshot| {
        matches!(&snapshot.placement, Some(Placement::Child { parent_task_id, .. }) if parent_task_id == &task.id)
    });
    let mut buttons = row![
        action(i18n.__("当前任务执行记录"))
            .on_press(message(TaskBoardMessage::LoadWorkPlanHistory { include_children: false })),
    ]
    .spacing(8);
    if parent {
        buttons =
            buttons.push(action(i18n.__("包含子任务")).on_press(message(
                TaskBoardMessage::LoadWorkPlanHistory { include_children: true },
            )));
    }
    let mut body = column![buttons.wrap()].spacing(6);
    if let Some(history) = plan.history.as_ref().filter(|history| history.task_id == task.id) {
        let scope =
            if history.include_children { i18n.__("当前任务及子任务") } else { i18n.__("当前任务") };
        body = body.push(text(i18n.format("执行记录 · %s", &[format!("{}", scope).into()]).expect("valid workbench template")).size(13));
        match history.result.as_ref() {
            None => body = body.push(text(i18n.__("正在加载执行记录…")).size(12)),
            Some(Err(error)) => {
                body = body.push(text(i18n.format("加载执行记录失败 · %s", &[format!("{}", error).into()]).expect("valid workbench template")).size(12))
            }
            Some(Ok(records)) if records.is_empty() => {
                body = body.push(text(i18n.__("暂无执行记录")).size(12))
            }
            Some(Ok(records)) => {
                let rows = records.iter().fold(column![].spacing(6), |rows, event| {
                    let source = if event.task_id.as_ref() == task.id.as_str() {
                        i18n.__("当前任务")
                    } else {
                        i18n.__("子任务")
                    };
                    rows.push(
                        column![
                            text(i18n.format("%s %s · 运行 %s", &[format!("{}", source).into(), format!("{}", event.task_id.as_ref()).into(), format!("{}", event.run_id.as_deref().unwrap_or_default()).into()]).expect("valid workbench template"))
                            .size(12),
                            text(format!(
                                "{} · {}",
                                history_time(event.created_at_ms.0),
                                event.message
                            ))
                            .size(12),
                        ]
                        .spacing(3),
                    )
                });
                body = body.push(container(scroll(rows).height(Length::Shrink)).max_height(155));
            }
        }
    }
    Some(body.into())
}

fn history_time(timestamp_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(timestamp_ms)
        .map(|date| date.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| timestamp_ms.to_string())
}

pub(super) fn with_child_modal<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let Some(draft) = current(app).and_then(|(_, plan)| plan.child_draft.as_ref()) else {
        return base;
    };
    let close = message(TaskBoardMessage::CloseWorkPlanChild);
    let submit = message(TaskBoardMessage::CreateWorkPlanChild);
    let mut prompt = settings_text_editor(&draft.prompt).id(CHILD_PROMPT_INPUT).height(112);
    let mut batch = settings_text_input(i18n.__("正整数"), &draft.batch);
    if !draft.pending {
        prompt =
            prompt.on_action(|action| message(TaskBoardMessage::WorkPlanChildPromptEdited(action)));
        batch = batch
            .on_input(|value| message(TaskBoardMessage::WorkPlanChildBatchChanged(value)))
            .on_submit(submit.clone());
    }
    let mut fields = column![
        settings_field_label(i18n.__("任务提示词")),
        prompt,
        settings_field_label(i18n.__("子任务批次")),
        batch,
    ]
    .spacing(8);
    if let Some(error) = &draft.error {
        fields = fields.push(text(error.render(i18n)).size(13));
    }
    let header = row![
        text(i18n.__("新增子任务")).size(18),
        iced::widget::Space::new().width(Length::Fill),
        settings_close_button(close.clone()),
    ]
    .align_y(iced::Alignment::Center);
    let card = settings_modal_card(
        column![
            header,
            container(scroll(fields).height(Length::Shrink)).max_height(260),
            action(if draft.pending { i18n.__("正在创建…") } else { i18n.__("创建子任务") })
                .style(primary_action_btn_style)
                .on_press_maybe((!draft.pending).then_some(submit)),
        ]
        .spacing(12),
    )
    .width(460);
    desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape(
        base, close, card,
    )
}
