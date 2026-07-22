//! 人员编排视图的阶段分配面板。

use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, toggler,
};
use iced::{Alignment, Color, Element, Length};
use serde_json::Value;
use std::fmt;

use crate::app::assets::Icon;
use crate::app::message::TaskBoardMessage;
use crate::app::state::{TaskBoardStaffAssignmentTab, TaskBoardStaffOrchestrationTab};
use crate::app::task::staff::models::TaskBoardStageAction;
use crate::app::task::staff::stage_capabilities::{StageCapability, stage_capability_registry};
use crate::app::task::staff::{
    TaskBoardStaffCapability, TaskBoardStaffConfig, TaskBoardStaffLevel, TaskBoardStaffMember,
    TaskBoardStageAssignment,
};
use crate::app::task::workflow::task_workflow_by_id;
use crate::app::task::{TaskStatus, TaskWorkflow};
use crate::app::{App, Message};
use vw_gateway_client::WorkflowRecordSummary;

use super::styles;

#[derive(Clone)]
struct StageGroup {
    label: &'static str,
    statuses: Vec<TaskStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct WorkflowBindingOption {
    uuid: Option<String>,
    label: String,
}

impl fmt::Display for WorkflowBindingOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StageActionCapabilityOption {
    capability_id: Option<String>,
    label: String,
}

impl fmt::Display for StageActionCapabilityOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

const STAFF_LEVELS: [TaskBoardStaffLevel; 4] = [
    TaskBoardStaffLevel::Principal,
    TaskBoardStaffLevel::Manager,
    TaskBoardStaffLevel::Executor,
    TaskBoardStaffLevel::Member,
];

pub(super) fn build_assignment_panel(
    app: &App,
    width: Length,
    height: Length,
) -> Element<'_, Message> {
    build_assignment_panel_content(
        app,
        &app.task_board_staff_config,
        &app.task_board_settings.workflow_id,
        width,
        height,
        true,
        Some(Message::TaskBoard(TaskBoardMessage::SaveStaffStageAssignment)),
    )
}

pub(crate) fn build_embedded_orchestration_panel(
    app: &App,
    config: &TaskBoardStaffConfig,
    workflow_id: &str,
    width: Length,
    height: Length,
    save_message: Option<Message>,
) -> Element<'static, Message> {
    let save_message =
        save_message.filter(|_| stage_assignment_has_unsaved_changes(app, config, workflow_id));
    build_assignment_panel_content(app, config, workflow_id, width, height, false, save_message)
}

fn build_assignment_panel_content(
    app: &App,
    config: &TaskBoardStaffConfig,
    workflow_id: &str,
    width: Length,
    height: Length,
    show_close: bool,
    save_message: Option<Message>,
) -> Element<'static, Message> {
    let assignments =
        app.task_board_staff_assignment_draft.as_deref().unwrap_or(&config.stage_assignments);
    let stage_actions =
        app.task_board_staff_stage_action_draft.as_deref().unwrap_or(&config.stage_actions);
    let assignment_count =
        assignments.iter().filter(|assignment| assignment_has_value(assignment)).count()
            + stage_actions.iter().filter(|action| stage_action_has_value(action)).count();

    let mut actions = row![icon_action_button(Icon::Save, "保存阶段分配", save_message)]
        .spacing(8)
        .align_y(Alignment::Center);
    actions = actions.push(icon_action_button(
        Icon::ArrowCounterClockwise,
        if app.task_board_staff_stage_action_reset_confirm {
            "确认重置默认编排"
        } else {
            "重置默认编排"
        },
        Some(Message::TaskBoard(if app.task_board_staff_stage_action_reset_confirm {
            TaskBoardMessage::ConfirmResetStaffStageActions
        } else {
            TaskBoardMessage::RequestResetStaffStageActions
        })),
    ));
    if show_close || app.task_board_staff_stage_action_reset_confirm {
        actions = actions.push(icon_action_button(
            Icon::X,
            if app.task_board_staff_stage_action_reset_confirm {
                "取消重置"
            } else {
                "取消阶段分配"
            },
            Some(Message::TaskBoard(if app.task_board_staff_stage_action_reset_confirm {
                TaskBoardMessage::CancelResetStaffStageActions
            } else {
                TaskBoardMessage::CloseStaffStageAssignment
            })),
        ));
    }

    let header = row![
        column![
            text("阶段责任编排").size(15).font(styles::bold_font()),
            text(format!(
                "{} 个阶段已分配 · {} 名启用成员",
                assignment_count,
                enabled_member_count(config)
            ))
            .size(11)
            .style(styles::muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        orchestration_tabs(app.task_board_staff_orchestration_tab),
        actions,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut groups = column![].spacing(14).width(Length::Fill);
    for group in stage_groups(config, workflow_id) {
        groups = groups.push(build_stage_group(
            &group,
            workflow_id,
            config,
            assignments,
            stage_actions,
            &app.task_board_staff_workflow_options,
            app.task_board_staff_workflow_options_error.as_deref(),
            &app.task_board_staff_capability_catalog,
            &app.task_board_staff_assignment_tabs,
            app.task_board_staff_orchestration_tab,
        ));
    }

    let mut body = column![header].spacing(12).width(Length::Fill).height(Length::Fill);
    if enabled_member_count(config) == 0 {
        body = body.push(
            container(text("没有启用成员").size(11))
                .padding([8, 10])
                .width(Length::Fill)
                .style(styles::warning_panel_style),
        );
    }
    if app.task_board_staff_stage_action_reset_confirm {
        body = body.push(
            container(text("再次点击重置按钮会覆盖当前阶段能力和自动接续草稿").size(11))
                .padding([8, 10])
                .width(Length::Fill)
                .style(styles::warning_panel_style),
        );
    }
    body = body.push(
        scrollable(groups)
            .direction(styles::vertical_scrollbar())
            .width(Length::Fill)
            .height(Length::Fill),
    );

    container(body).padding(12).width(width).height(height).style(styles::side_panel_style).into()
}

fn stage_groups(config: &TaskBoardStaffConfig, workflow_id: &str) -> Vec<StageGroup> {
    let mut groups: Vec<StageGroup> = Vec::new();
    for stage in &config.workflow {
        let label = stage.role.map(styles::role_label).unwrap_or("展示阶段");
        if let Some(group) = groups.last_mut()
            && group.label == label
        {
            group.statuses.push(stage.status);
            continue;
        }
        groups.push(StageGroup { label, statuses: vec![stage.status] });
    }

    if groups.is_empty() {
        groups.push(StageGroup {
            label: "阶段",
            statuses: TaskStatus::all_for_workflow_id(workflow_id),
        });
    }

    groups
}

fn build_stage_group(
    group: &StageGroup,
    workflow_id: &str,
    config: &TaskBoardStaffConfig,
    assignments: &[TaskBoardStageAssignment],
    stage_actions: &[TaskBoardStageAction],
    workflow_options: &[WorkflowRecordSummary],
    workflow_error: Option<&str>,
    capability_catalog: &[TaskBoardStaffCapability],
    assignment_tabs: &std::collections::HashMap<TaskStatus, TaskBoardStaffAssignmentTab>,
    orchestration_tab: TaskBoardStaffOrchestrationTab,
) -> Element<'static, Message> {
    let mut statuses = column![
        text(group.label).size(12).font(styles::bold_font()).style(styles::muted_text_style)
    ]
    .spacing(8)
    .width(Length::Fill);

    for status in &group.statuses {
        let active_tab = assignment_tabs.get(status).copied().unwrap_or_default();
        statuses = statuses.push(build_status_assignment(
            *status,
            workflow_id,
            config,
            assignments,
            stage_actions,
            workflow_options,
            workflow_error,
            capability_catalog,
            active_tab,
            orchestration_tab,
        ));
    }

    statuses.into()
}

fn build_status_assignment(
    status: TaskStatus,
    workflow_id: &str,
    config: &TaskBoardStaffConfig,
    assignments: &[TaskBoardStageAssignment],
    stage_actions: &[TaskBoardStageAction],
    workflow_options: &[WorkflowRecordSummary],
    workflow_error: Option<&str>,
    capability_catalog: &[TaskBoardStaffCapability],
    active_tab: TaskBoardStaffAssignmentTab,
    orchestration_tab: TaskBoardStaffOrchestrationTab,
) -> Element<'static, Message> {
    let assignment = assignment_for_status(assignments, status);
    let stage_action = stage_action_for_status(stage_actions, status);
    let role_label = config.role_for_status(status).map(styles::role_label).unwrap_or("展示");
    let accent = config
        .role_for_status(status)
        .map(styles::role_color)
        .unwrap_or_else(styles::display_only_color);
    let status_label = status.label_for_workflow_id(workflow_id);

    let chips = row![
        styles::small_chip(status_label.clone(), accent),
        styles::small_chip(role_label.to_string(), accent),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let summary = assignment_summary(config, &assignment);

    let header =
        row![text(status_label).size(13).font(styles::bold_font()).width(Length::Fill), chips]
            .spacing(8)
            .align_y(Alignment::Center);
    let body = match orchestration_tab {
        TaskBoardStaffOrchestrationTab::Dispatch => column![
            autonomous_toggle(status, assignment.autonomous),
            stage_action_control(
                status,
                workflow_id,
                stage_action,
                workflow_options,
                workflow_error,
                capability_catalog,
            ),
        ]
        .spacing(10),
        TaskBoardStaffOrchestrationTab::Staffing => column![
            summary,
            assignment_tabs(status, active_tab, &assignment),
            build_member_picker(active_tab, status, config, &assignment),
        ]
        .spacing(10),
    };
    let content = column![header, body].spacing(10).width(Length::Fill);

    container(content)
        .padding(10)
        .width(Length::Fill)
        .style(move |theme| styles::task_card_style(theme, accent, false))
        .into()
}

fn orchestration_tabs(active_tab: TaskBoardStaffOrchestrationTab) -> Element<'static, Message> {
    row![
        orchestration_tab_button("指挥调度", TaskBoardStaffOrchestrationTab::Dispatch, active_tab,),
        orchestration_tab_button("规划员工", TaskBoardStaffOrchestrationTab::Staffing, active_tab,),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn orchestration_tab_button(
    label: &'static str,
    tab: TaskBoardStaffOrchestrationTab,
    active_tab: TaskBoardStaffOrchestrationTab,
) -> Element<'static, Message> {
    let selected = tab == active_tab;
    button(text(label).size(11))
        .padding([6, 10])
        .style(move |theme, status| styles::tab_button_style(theme, status, selected))
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectStaffOrchestrationTab(tab)))
        .into()
}

fn stage_action_control(
    status: TaskStatus,
    workflow_id: &str,
    stage_action: Option<&TaskBoardStageAction>,
    workflow_options: &[WorkflowRecordSummary],
    workflow_error: Option<&str>,
    capability_catalog: &[TaskBoardStaffCapability],
) -> Element<'static, Message> {
    let capability_options = stage_action_capability_options(capability_catalog);
    let selected_capability =
        stage_action_capability_selected_option(&capability_options, stage_action);
    let capability_picker =
        pick_list(capability_options, selected_capability.clone(), move |option| {
            Message::TaskBoard(TaskBoardMessage::SelectStaffStageActionCapability {
                status,
                capability_id: option.capability_id,
            })
        })
        .width(Length::Fill);

    let mut controls =
        column![text("阶段能力").size(12).font(styles::bold_font()), capability_picker].spacing(6);
    let selected_capability_id =
        selected_capability.and_then(|option| option.capability_id).unwrap_or_default();
    if selected_capability_requires_workflow_uuid(&selected_capability_id, capability_catalog) {
        let workflow_uuid = stage_action.and_then(|action| action.workflow_uuid.as_deref());
        let options = workflow_binding_options(workflow_options, workflow_uuid);
        let selected = workflow_binding_selected_option(&options, workflow_uuid);
        let create_workflow_id = workflow_id.to_string();
        let picker = pick_list(options, selected, move |option| {
            Message::TaskBoard(TaskBoardMessage::SelectStaffStageActionWorkflow {
                status,
                workflow_uuid: option.uuid,
            })
        })
        .width(Length::Fill);
        let workflow_row = row![
            picker,
            button(text("新建 Workflow").size(11))
                .padding([6, 10])
                .style(styles::icon_button_style)
                .on_press(Message::TaskBoard(TaskBoardMessage::CreateStaffStageActionWorkflow {
                    status,
                    workflow_id: create_workflow_id,
                })),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let helper = match workflow_error {
            Some(error) => text(format!("Workflow 列表加载失败：{error}"))
                .size(11)
                .style(styles::muted_text_style),
            None if workflow_options.is_empty() => {
                text("没有已保存 Workflow").size(11).style(styles::muted_text_style)
            }
            None => text("选择已保存 Workflow UUID 作为阶段动作")
                .size(11)
                .style(styles::muted_text_style),
        };
        controls = controls
            .push(text("Workflow UUID").size(12).font(styles::bold_font()))
            .push(workflow_row)
            .push(helper);
    }

    if let Some(inputs) = stage_action_input_controls(
        status,
        stage_action,
        &selected_capability_id,
        capability_catalog,
    ) {
        controls = controls.push(inputs);
    }

    if let Some(summary) =
        selected_stage_capability_summary(&selected_capability_id, capability_catalog)
    {
        controls = controls.push(text(summary).size(11).style(styles::muted_text_style));
    } else if selected_capability_id.is_empty() {
        controls = controls.push(
            text("无能力绑定；自主调度开启时也不会启动阶段能力")
                .size(11)
                .style(styles::muted_text_style),
        );
    }

    container(controls)
        .padding([8, 10])
        .width(Length::Fill)
        .style(|theme| styles::autonomous_toggle_style(theme, false))
        .into()
}

fn stage_action_input_controls(
    status: TaskStatus,
    stage_action: Option<&TaskBoardStageAction>,
    capability_id: &str,
    capability_catalog: &[TaskBoardStaffCapability],
) -> Option<Element<'static, Message>> {
    let fields = selected_capability_input_fields(capability_id, capability_catalog);
    let mut controls =
        column![text("入参").size(12).font(styles::bold_font())].spacing(6).width(Length::Fill);
    let mut pushed = false;
    for field in fields {
        if field == "workflow_uuid" {
            continue;
        }
        pushed = true;
        let value = stage_action_parameter_value(stage_action, &field);
        let input_key = field.clone();
        controls = controls.push(
            row![
                text(field).size(11).width(Length::FillPortion(1)),
                text_input("值", &value)
                    .size(11)
                    .padding(6)
                    .on_input(move |value| {
                        Message::TaskBoard(TaskBoardMessage::UpdateStaffStageActionParameter {
                            status,
                            key: input_key.clone(),
                            value,
                        })
                    })
                    .width(Length::FillPortion(2)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    pushed.then(|| controls.into())
}

fn stage_action_parameter_value(
    stage_action: Option<&TaskBoardStageAction>,
    field: &str,
) -> String {
    stage_action
        .and_then(|action| action.input_parameters.get(field))
        .map(stage_action_parameter_display_value)
        .unwrap_or_default()
}

fn stage_action_parameter_display_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

fn build_member_picker(
    selection: TaskBoardStaffAssignmentTab,
    status: TaskStatus,
    config: &TaskBoardStaffConfig,
    assignment: &TaskBoardStageAssignment,
) -> Element<'static, Message> {
    let mut picker =
        column![text(selection_hint(selection)).size(11).style(styles::muted_text_style)]
            .spacing(6)
            .width(Length::Fill);

    if matches!(selection, TaskBoardStaffAssignmentTab::Primary) {
        picker = picker.push(member_choice_button(
            "未指定".to_string(),
            assignment.primary_member_id.is_none(),
            styles::display_only_color(),
            Message::TaskBoard(TaskBoardMessage::SelectStaffStagePrimaryMember {
                status,
                member_id: None,
            }),
        ));
    }

    let mut pushed_member = false;
    for member in ordered_enabled_members(config) {
        pushed_member = true;
        let selected = match selection {
            TaskBoardStaffAssignmentTab::Primary => {
                assignment.primary_member_id.as_deref() == Some(member.id.as_str())
            }
            TaskBoardStaffAssignmentTab::Manager => {
                assignment.manager_member_ids.iter().any(|member_id| member_id == &member.id)
            }
            TaskBoardStaffAssignmentTab::Backup => {
                assignment.backup_member_ids.iter().any(|member_id| member_id == &member.id)
            }
        };
        picker = picker.push(member_choice_button(
            member_option_label(member),
            selected,
            level_color(member.level),
            member_message(selection, status, member.id.clone()),
        ));
    }

    if !pushed_member {
        picker = picker.push(text("无启用成员").size(11).style(styles::muted_text_style));
    }

    picker.into()
}

fn ordered_enabled_members(config: &TaskBoardStaffConfig) -> Vec<&TaskBoardStaffMember> {
    let mut members = STAFF_LEVELS
        .iter()
        .flat_map(|level| {
            config.members.iter().filter(move |member| member.enabled && member.level == *level)
        })
        .collect::<Vec<_>>();
    members.sort_by(|left, right| {
        level_rank(left.level)
            .cmp(&level_rank(right.level))
            .then_with(|| left.role.key().cmp(right.role.key()))
            .then_with(|| left.name.cmp(&right.name))
    });
    members
}

fn member_option_label(member: &TaskBoardStaffMember) -> String {
    format!("{} · {} · {}", member.name, styles::role_label(member.role), level_label(member.level))
}

fn selection_hint(selection: TaskBoardStaffAssignmentTab) -> &'static str {
    match selection {
        TaskBoardStaffAssignmentTab::Primary => "选择一个主责成员",
        TaskBoardStaffAssignmentTab::Manager => "选择一个或多个管理成员",
        TaskBoardStaffAssignmentTab::Backup => "选择一个或多个候补成员",
    }
}

fn assignment_tabs(
    status: TaskStatus,
    active_tab: TaskBoardStaffAssignmentTab,
    assignment: &TaskBoardStageAssignment,
) -> Element<'static, Message> {
    row![
        assignment_tab_button(
            format!("主责 {}", usize::from(assignment.primary_member_id.is_some())),
            TaskBoardStaffAssignmentTab::Primary,
            status,
            active_tab,
        ),
        assignment_tab_button(
            format!("管理 {}", assignment.manager_member_ids.len()),
            TaskBoardStaffAssignmentTab::Manager,
            status,
            active_tab,
        ),
        assignment_tab_button(
            format!("候补 {}", assignment.backup_member_ids.len()),
            TaskBoardStaffAssignmentTab::Backup,
            status,
            active_tab,
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

fn assignment_tab_button(
    label: String,
    tab: TaskBoardStaffAssignmentTab,
    status: TaskStatus,
    active_tab: TaskBoardStaffAssignmentTab,
) -> Element<'static, Message> {
    let selected = tab == active_tab;
    button(text(label).size(11))
        .padding([6, 10])
        .style(move |theme, button_status| styles::tab_button_style(theme, button_status, selected))
        .on_press(Message::TaskBoard(TaskBoardMessage::SelectStaffStageAssignmentTab {
            status,
            tab,
        }))
        .into()
}

fn assignment_summary(
    config: &TaskBoardStaffConfig,
    assignment: &TaskBoardStageAssignment,
) -> Element<'static, Message> {
    row![
        styles::small_chip(
            format!(
                "主责 {}",
                selected_member_label(config, assignment.primary_member_id.as_deref())
            ),
            styles::priority_color(),
        ),
        styles::small_chip(
            format!("管理 {}", selected_members_label(config, &assignment.manager_member_ids)),
            styles::warning_color(),
        ),
        styles::small_chip(
            format!("候补 {}", selected_members_label(config, &assignment.backup_member_ids)),
            styles::staff_level_color(TaskBoardStaffLevel::Executor),
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .wrap()
    .into()
}

fn level_rank(level: TaskBoardStaffLevel) -> usize {
    STAFF_LEVELS.iter().position(|candidate| *candidate == level).unwrap_or(usize::MAX)
}

fn member_message(
    selection: TaskBoardStaffAssignmentTab,
    status: TaskStatus,
    member_id: String,
) -> Message {
    match selection {
        TaskBoardStaffAssignmentTab::Primary => {
            Message::TaskBoard(TaskBoardMessage::SelectStaffStagePrimaryMember {
                status,
                member_id: Some(member_id),
            })
        }
        TaskBoardStaffAssignmentTab::Manager => {
            Message::TaskBoard(TaskBoardMessage::ToggleStaffStageManagerMember {
                status,
                member_id,
            })
        }
        TaskBoardStaffAssignmentTab::Backup => {
            Message::TaskBoard(TaskBoardMessage::ToggleStaffStageBackupMember { status, member_id })
        }
    }
}

fn member_choice_button(
    label: String,
    selected: bool,
    accent: Color,
    message: Message,
) -> Element<'static, Message> {
    button(
        row![
            styles::icon(if selected { Icon::CheckSquare } else { Icon::Square }, 12.0, true),
            text(label).size(11).width(Length::Fill),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(move |theme, status| {
        styles::assignment_member_button_style(theme, status, selected, accent)
    })
    .on_press(message)
    .into()
}

fn autonomous_toggle(status: TaskStatus, autonomous: bool) -> Element<'static, Message> {
    container(
        row![
            column![
                text("自动接续").size(12).font(styles::bold_font()),
                text(if autonomous {
                    "开启后该阶段可自动接续"
                } else {
                    "关闭后该阶段保持人工确认"
                })
                .size(11)
                .style(styles::muted_text_style),
            ]
            .spacing(3)
            .width(Length::Fill),
            text(if autonomous { "开启" } else { "关闭" }).size(11),
            toggler(autonomous).on_toggle(move |enabled| {
                Message::TaskBoard(TaskBoardMessage::ToggleStaffStageAutonomous { status, enabled })
            }),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(move |theme| styles::autonomous_toggle_style(theme, autonomous))
    .into()
}

fn icon_action_button(
    icon: Icon,
    tooltip_label: &'static str,
    message: Option<Message>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let icon = styles::icon(icon, 13.0, enabled);
    let button = button(icon)
        .padding(6)
        .width(Length::Fixed(28.0))
        .height(Length::Fixed(28.0))
        .style(styles::icon_button_style);
    let button = if let Some(message) = message { button.on_press(message) } else { button };

    styles::tooltip(button.into(), tooltip_label)
}

fn assignment_for_status(
    assignments: &[TaskBoardStageAssignment],
    status: TaskStatus,
) -> TaskBoardStageAssignment {
    assignments.iter().find(|assignment| assignment.status == status).cloned().unwrap_or(
        TaskBoardStageAssignment {
            status,
            primary_member_id: None,
            backup_member_ids: Vec::new(),
            manager_member_ids: Vec::new(),
            autonomous: false,
        },
    )
}

fn stage_action_for_status(
    actions: &[TaskBoardStageAction],
    status: TaskStatus,
) -> Option<&TaskBoardStageAction> {
    actions.iter().find(|action| action.status == status && stage_action_has_value(action))
}

pub(super) fn stage_action_capability_options(
    capability_catalog: &[TaskBoardStaffCapability],
) -> Vec<StageActionCapabilityOption> {
    let mut options = vec![StageActionCapabilityOption {
        capability_id: None,
        label: "无能力绑定".to_string(),
    }];

    if capability_catalog.is_empty() {
        options.extend(stage_capability_registry().all().iter().map(|capability| {
            StageActionCapabilityOption {
                capability_id: Some(capability.stable_id.to_string()),
                label: stage_action_capability_label(capability),
            }
        }));
    } else {
        options.extend(capability_catalog.iter().map(|capability| StageActionCapabilityOption {
            capability_id: Some(capability.id.clone()),
            label: format!("{} ({})", capability.label, capability.id),
        }));
    }
    options
}

fn stage_action_capability_label(capability: &StageCapability) -> String {
    format!("{} ({})", capability.label, capability.stable_id)
}

fn stage_action_capability_selected_option(
    options: &[StageActionCapabilityOption],
    stage_action: Option<&TaskBoardStageAction>,
) -> Option<StageActionCapabilityOption> {
    let capability_id = stage_action
        .map(|action| action.capability_id.trim())
        .filter(|capability_id| !capability_id.is_empty());
    options
        .iter()
        .find(|option| option.capability_id.as_deref() == capability_id)
        .cloned()
        .or_else(|| options.first().cloned())
}

fn selected_stage_capability_summary(
    capability_id: &str,
    capability_catalog: &[TaskBoardStaffCapability],
) -> Option<String> {
    if capability_id.is_empty() {
        return None;
    }
    if let Some(capability) =
        capability_catalog.iter().find(|capability| capability.id == capability_id)
    {
        return Some(format!(
            "{} · {} · {}",
            capability.input_contract, capability.result_semantics, capability.permission_hint
        ));
    }
    selected_builtin_stage_capability(capability_id).map(|capability| {
        format!(
            "{} · {} · {}",
            capability.input_contract, capability.result_semantics, capability.permission_hint
        )
    })
}

fn selected_capability_requires_workflow_uuid(
    capability_id: &str,
    capability_catalog: &[TaskBoardStaffCapability],
) -> bool {
    capability_catalog
        .iter()
        .find(|capability| capability.id == capability_id)
        .map(|capability| capability.requires_workflow_uuid)
        .or_else(|| {
            selected_builtin_stage_capability(capability_id)
                .map(|capability| capability.requires_workflow_uuid)
        })
        .unwrap_or(false)
}

fn selected_capability_input_fields(
    capability_id: &str,
    capability_catalog: &[TaskBoardStaffCapability],
) -> Vec<String> {
    capability_catalog
        .iter()
        .find(|capability| capability.id == capability_id)
        .map(|capability| {
            capability
                .input_summary
                .fields
                .iter()
                .map(|field| field.trim())
                .filter(|field| !field.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn selected_builtin_stage_capability(capability_id: &str) -> Option<&'static StageCapability> {
    stage_capability_registry()
        .all()
        .iter()
        .find(|capability| capability.stable_id == capability_id)
}

pub(super) fn workflow_binding_options(
    workflow_options: &[WorkflowRecordSummary],
    selected_workflow_uuid: Option<&str>,
) -> Vec<WorkflowBindingOption> {
    let mut options =
        vec![WorkflowBindingOption { uuid: None, label: "未选择 Workflow".to_string() }];
    options.extend(workflow_options.iter().map(|record| WorkflowBindingOption {
        uuid: Some(record.uuid.clone()),
        label: workflow_option_label(record),
    }));

    if let Some(selected_workflow_uuid) = selected_workflow_uuid
        && !options.iter().any(|option| option.uuid.as_deref() == Some(selected_workflow_uuid))
    {
        options.push(WorkflowBindingOption {
            uuid: Some(selected_workflow_uuid.to_string()),
            label: format!("已绑定 {selected_workflow_uuid}"),
        });
    }

    options
}

fn workflow_option_label(record: &WorkflowRecordSummary) -> String {
    let name = record.name.trim();
    if name.is_empty() { record.uuid.clone() } else { format!("{name} · {}", record.uuid) }
}

fn workflow_binding_selected_option(
    options: &[WorkflowBindingOption],
    workflow_uuid: Option<&str>,
) -> Option<WorkflowBindingOption> {
    options
        .iter()
        .find(|option| option.uuid.as_deref() == workflow_uuid)
        .cloned()
        .or_else(|| options.first().cloned())
}

fn assignment_has_value(assignment: &TaskBoardStageAssignment) -> bool {
    assignment.primary_member_id.is_some()
        || !assignment.backup_member_ids.is_empty()
        || !assignment.manager_member_ids.is_empty()
        || assignment.autonomous
}

fn stage_action_has_value(action: &TaskBoardStageAction) -> bool {
    !action.capability_id.trim().is_empty()
}

pub(super) fn stage_assignment_has_unsaved_changes(
    app: &App,
    config: &TaskBoardStaffConfig,
    workflow_id: &str,
) -> bool {
    let Some(workflow) = task_workflow_by_id(workflow_id) else {
        return false;
    };
    let assignments =
        app.task_board_staff_assignment_draft.as_deref().unwrap_or(&config.stage_assignments);
    let stage_actions =
        app.task_board_staff_stage_action_draft.as_deref().unwrap_or(&config.stage_actions);

    normalized_stage_assignments(assignments, &workflow)
        != normalized_stage_assignments(&config.stage_assignments, &workflow)
        || normalized_stage_actions(stage_actions, &workflow)
            != normalized_stage_actions(&config.stage_actions, &workflow)
}

fn normalized_stage_assignments(
    assignments: &[TaskBoardStageAssignment],
    workflow: &TaskWorkflow,
) -> Vec<TaskBoardStageAssignment> {
    let mut normalized = assignments
        .iter()
        .filter(|assignment| {
            assignment_has_value(assignment) && workflow_has_status(workflow, assignment.status)
        })
        .cloned()
        .collect::<Vec<_>>();
    normalized.sort_by_key(|assignment| task_status_order(assignment.status, workflow));
    normalized
}

fn normalized_stage_actions(
    actions: &[TaskBoardStageAction],
    workflow: &TaskWorkflow,
) -> Vec<TaskBoardStageAction> {
    let mut normalized = actions
        .iter()
        .filter(|action| {
            stage_action_has_value(action) && workflow_has_status(workflow, action.status)
        })
        .cloned()
        .collect::<Vec<_>>();
    normalized.sort_by_key(|action| task_status_order(action.status, workflow));
    normalized
}

fn workflow_has_status(workflow: &TaskWorkflow, status: TaskStatus) -> bool {
    workflow.statuses().iter().any(|definition| definition.status == status)
}

fn task_status_order(status: TaskStatus, workflow: &TaskWorkflow) -> usize {
    workflow
        .statuses()
        .iter()
        .position(|definition| definition.status == status)
        .unwrap_or(usize::MAX)
}

fn selected_member_label(config: &TaskBoardStaffConfig, member_id: Option<&str>) -> String {
    member_id
        .map(|member_id| member_name(config, member_id))
        .unwrap_or_else(|| "未指定".to_string())
}

fn selected_members_label(config: &TaskBoardStaffConfig, member_ids: &[String]) -> String {
    if member_ids.is_empty() {
        return "未指定".to_string();
    }
    member_ids.iter().map(|member_id| member_name(config, member_id)).collect::<Vec<_>>().join("、")
}

fn member_name(config: &TaskBoardStaffConfig, member_id: &str) -> String {
    config
        .members
        .iter()
        .find(|member| member.id == member_id)
        .map(|member| member.name.clone())
        .unwrap_or_else(|| member_id.to_string())
}

fn enabled_member_count(config: &TaskBoardStaffConfig) -> usize {
    config.members.iter().filter(|member| member.enabled).count()
}

fn level_label(level: TaskBoardStaffLevel) -> &'static str {
    match level {
        TaskBoardStaffLevel::Principal => "主智能体",
        TaskBoardStaffLevel::Manager => "管理智能体",
        TaskBoardStaffLevel::Executor => "执行智能体",
        TaskBoardStaffLevel::Member => "普通智能体",
    }
}

fn level_color(level: TaskBoardStaffLevel) -> Color {
    match level {
        TaskBoardStaffLevel::Principal => Color::from_rgb8(59, 130, 246),
        TaskBoardStaffLevel::Manager => Color::from_rgb8(245, 158, 11),
        TaskBoardStaffLevel::Executor => Color::from_rgb8(16, 185, 129),
        TaskBoardStaffLevel::Member => styles::display_only_color(),
    }
}
