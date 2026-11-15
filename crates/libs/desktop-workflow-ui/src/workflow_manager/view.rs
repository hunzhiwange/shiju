use super::*;

#[cfg(test)]
#[path = "form_controls_tests.rs"]
mod form_controls_tests;

/// 构建工作流列表、编辑器、阶段动作页面及确认对话框。
pub fn view<'a>(state: &'a WorkflowManagerState, i18n: desktop_i18n::I18n, theme: &Theme) -> Element<'a, Message> {
    let body: Element<'_, Message> = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition => {
            let list = build_workflow_list(i18n, state);
            let editor = build_editor(i18n, state);
            row![
                container(list).width(Length::Fixed(340.0)).height(Length::Fill),
                container(editor).width(Length::Fill).height(Length::Fill),
            ]
            .spacing(16)
            .align_y(Alignment::Start)
            .into()
        }
    };

    let help_panel = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition if state.show_status_role_help => {
            status_role_help_panel(i18n)
        }
        _ => Space::new().height(Length::Shrink).into(),
    };

    let help_button = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition => button(text(i18n.__("语义角色帮助")).size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ToggleStatusRoleHelp)),
    };

    let header_actions = row![
        button(text(i18n.__("刷新")).size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RefreshLifecycles)),
        help_button,
        button(text(i18n.__("新建")).size(13))
            .padding([9, 14])
            .style(primary_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::StartNew)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let content = column![
        settings_panel(
            column![
                row![
                    settings_hover_help_label(
                        i18n,
                        theme,
                        i18n.__("任务编排"),
                        text(i18n.__("任务编排")).size(20),
                        i18n.__("生命周期定义负责状态与流向。"),
                    )
                    .wrap(),
                    Space::new().width(Length::Fill),
                    header_actions,
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                workflow_manager_tabs(i18n, state.active_tab),
            ]
            .spacing(12)
        ),
        help_panel,
        body,
    ]
    .spacing(16)
    .padding([18, 24])
    .width(Length::Fill)
    .height(Length::Fill);

    let page: Element<'_, Message> = container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(palette.background.base.color.into()),
                ..Default::default()
            }
        })
        .into();

    if let Some(confirmation) = &state.pending_confirmation {
        iced::widget::stack![page, workflow_confirmation_dialog(i18n, confirmation)].into()
    } else {
        page
    }
}

fn workflow_confirmation_dialog(
    i18n: desktop_i18n::I18n,
    confirmation: &WorkflowManagerConfirmation,
) -> Element<'static, Message> {
    let (title, body, confirm_label) = match confirmation {
        WorkflowManagerConfirmation::ResetWorkflow { workflow_label, .. } => (
            i18n.__("确认重置生命周期").to_string(),
            i18n.format(
                "将把内置生命周期「%s」恢复到默认值，当前保存的自定义修改会被丢弃。",
                &[workflow_label.to_string().into()],
            )
            .expect("valid workflow display template"),
            i18n.__("确认重置"),
        ),
        WorkflowManagerConfirmation::DeleteWorkflow { workflow_label, .. } => (
            i18n.__("确认删除生命周期").to_string(),
            i18n.format(
                "将删除生命周期「%s」。此操作不可撤销。",
                &[workflow_label.to_string().into()],
            )
            .expect("valid workflow display template"),
            i18n.__("确认删除"),
        ),
        WorkflowManagerConfirmation::DeleteStatus { key, label, .. } => {
            let display = if label.trim().is_empty() { key.as_str() } else { label.as_str() };
            (
                i18n.__("确认删除阶段").to_string(),
                i18n.format(
                    "将从当前草稿中删除阶段「%s」(%s)。保存前可以取消编辑撤销。",
                    &[display.to_string().into(), key.to_string().into()],
                )
                .expect("valid workflow display template"),
                i18n.__("确认删除"),
            )
        }
    };
    confirm_dialog(
        title,
        body,
        confirm_label,
        i18n.__("取消"),
        Message::WorkflowManager(WorkflowManagerMessage::ConfirmPendingAction),
        Message::WorkflowManager(WorkflowManagerMessage::CancelPendingConfirmation),
    )
}

fn workflow_manager_tabs(
    i18n: desktop_i18n::I18n,
    active_tab: WorkflowManagerTab,
) -> Element<'static, Message> {
    row![workflow_manager_tab_button(
        i18n.__("生命周期定义"),
        WorkflowManagerTab::LifecycleDefinition,
        active_tab
    ),]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn workflow_manager_tab_button(
    label: &'static str,
    tab: WorkflowManagerTab,
    active_tab: WorkflowManagerTab,
) -> Element<'static, Message> {
    let selected = tab == active_tab;
    let style: fn(&Theme, button::Status) -> button::Style =
        if selected { primary_action_btn_style } else { rounded_action_btn_style };
    button(text(label).size(13))
        .padding([8, 12])
        .style(style)
        .on_press(Message::WorkflowManager(WorkflowManagerMessage::SelectTab(tab)))
        .into()
}

fn build_workflow_list(
    i18n: desktop_i18n::I18n,
    state: &WorkflowManagerState,
) -> Element<'_, Message> {
    let mut items = column![].spacing(8);
    for workflow in &state.workflows {
        let selected = state.selected_workflow_id.as_deref() == Some(workflow.id.as_str());
        let badge = if workflow.is_builtin { i18n.__("内置") } else { i18n.__("用户") };
        let label = format!("{} ({})", workflow.label, workflow.id);
        let button_style: fn(&Theme, button::Status) -> button::Style =
            if selected { primary_action_btn_style } else { rounded_action_btn_style };
        items = items.push(
            row![
                button(
                    row![
                        column![
                            text(label).size(13),
                            text(badge).size(11).style(settings_muted_text_style),
                        ]
                        .spacing(3),
                        Space::new().width(Length::Fill),
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .padding([10, 12])
                .style(button_style)
                .on_press(Message::WorkflowManager(
                    WorkflowManagerMessage::SelectWorkflow(workflow.id.clone())
                )),
                button(text(i18n.__("查看")).size(12))
                    .padding([10, 12])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::ViewWorkflow(
                        workflow.id.clone()
                    ))),
                button(text(i18n.__("复制")).size(12))
                    .padding([10, 12])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::CopyWorkflow(
                        workflow.id.clone()
                    ))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    settings_panel(
        column![
            text(i18n.__("预设")).size(15),
            scrollable(
                container(items)
                    .width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(12),
    )
    .height(Length::Fill)
    .into()
}

fn build_editor(
    i18n: desktop_i18n::I18n,
    state: &WorkflowManagerState,
) -> Element<'_, Message> {
    if let Some(draft) = &state.draft {
        return build_draft_editor(i18n, draft);
    }

    let selected = state.selected_item();
    let title = selected.map(|workflow| workflow.label.as_str()).unwrap_or(i18n.__("请选择工作流"));
    let id = selected.map(|workflow| workflow.id.as_str()).unwrap_or("-");
    let is_builtin = selected.is_some_and(|workflow| workflow.is_builtin);
    let selected_workflow = selected
        .and_then(|item| all_task_workflows().into_iter().find(|workflow| workflow.id == item.id));

    let mut actions = row![].spacing(10).align_y(Alignment::Center);
    if let Some(selected) = selected {
        actions = actions.push(
            button(text(i18n.__("复制")).size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::CopyWorkflow(
                    selected.id.clone(),
                ))),
        );
    }
    if selected.is_some() {
        actions = actions.push(
            button(text(i18n.__("编辑")).size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::StartEditSelected)),
        );
        if is_builtin {
            actions = actions.push(
                button(text(i18n.__("重置")).size(13))
                    .padding([9, 14])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::ResetSelected)),
            );
        } else {
            actions = actions.push(
                button(text(i18n.__("删除")).size(13))
                    .padding([9, 14])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::DeleteSelected)),
            );
        }
    }

    let header = settings_panel(
        column![
            row![
                column![text(title).size(18), text(id).size(12).style(settings_muted_text_style)]
                    .spacing(4),
                Space::new().width(Length::Fill),
                actions,
            ]
            .align_y(Alignment::Center)
            .spacing(10),
            selected
                .map(|workflow| {
                    if workflow.is_builtin {
                        settings_value_badge(i18n.__("内置生命周期，可编辑，可重置"))
                    } else {
                        settings_value_badge(i18n.__("用户预设，可编辑"))
                    }
                })
                .unwrap_or_else(|| text(i18n.__("未选择")).style(settings_muted_text_style).into()),
        ]
        .spacing(16),
    );

    let body = selected_workflow
        .map(|workflow| build_workflow_view(i18n, workflow))
        .unwrap_or_else(|| {
            settings_panel(
                text(i18n.__("请选择左侧工作流查看流程")).style(settings_muted_text_style),
            )
            .into()
        });

    column![header, body].spacing(14).height(Length::Fill).into()
}

fn build_workflow_view(
    i18n: desktop_i18n::I18n,
    workflow: TaskWorkflow,
) -> Element<'static, Message> {
    let mut flow = column![].spacing(10);
    for (index, status) in workflow.statuses.iter().enumerate() {
        let routes = workflow
            .status_transitions(status.status)
            .into_iter()
            .filter_map(|transition| {
                let label = transition.display_label();
                workflow.status_definition(transition.target).map(|target| WorkflowRouteView {
                    outcome: transition.outcome,
                    label,
                    target_label: target.label.clone(),
                    target_key: target.key.clone(),
                })
            })
            .collect();
        flow = flow.push(build_workflow_status_view(
            i18n,
            index,
            status.label.clone(),
            status.key.clone(),
            status.role,
            status.enabled,
            routes,
        ));
    }

    settings_panel(
        column![
            row![
                column![
                    text(i18n.__("流程预览")).size(16),
                    text(i18n.__("每个状态可按结果 key 进入不同状态；succeed/failed 为内置结果。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                settings_value_badge(
                    i18n.format("%s 个状态", &[workflow.statuses.len().to_string().into()])
                        .expect("valid workflow display template")
                ),
            ]
            .align_y(Alignment::Center),
            scrollable(
                container(flow)
                    .width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(14),
    )
    .height(Length::Fill)
    .into()
}

fn build_workflow_status_view(
    i18n: desktop_i18n::I18n,
    index: usize,
    label: String,
    key: String,
    role: StatusRole,
    enabled: bool,
    routes: Vec<WorkflowRouteView>,
) -> Element<'static, Message> {
    let is_terminal = routes.is_empty();
    let route_summary: Element<'static, Message> = if is_terminal {
        text(i18n.__("结束点：无后续状态")).size(12).style(settings_muted_text_style).into()
    } else {
        row(routes.into_iter().map(route_badge).collect::<Vec<_>>()).spacing(6).into()
    };
    let role = status_role_display(i18n, role);

    container(
        row![
            container(text(format!("{:02}", index + 1)).size(13))
                .center_x(Length::Fixed(WORKFLOW_STATUS_INDEX_WIDTH)),
            column![
                row![
                    text(label).size(15),
                    settings_value_badge(role),
                    if enabled {
                        Space::new().width(Length::Shrink).into()
                    } else {
                        settings_value_badge(i18n.__("已停用"))
                    },
                    if is_terminal {
                        settings_value_badge(i18n.__("结束"))
                    } else {
                        settings_value_badge(i18n.__("分支"))
                    },
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(key).size(11).style(settings_muted_text_style),
                route_summary,
            ]
            .spacing(5)
            .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12, 14])
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(palette.background.weak.color.scale_alpha(0.62).into()),
            border: iced::Border {
                color: palette.background.strong.color.scale_alpha(0.28),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

#[derive(Debug, Clone)]
struct WorkflowRouteView {
    outcome: StatusOutcome,
    label: String,
    target_label: String,
    target_key: String,
}

fn route_badge(route: WorkflowRouteView) -> Element<'static, Message> {
    settings_value_badge(format!(
        "{} ({}) -> {} ({})",
        route.label,
        route.outcome.key(),
        route.target_label,
        route.target_key
    ))
}

fn role_label(i18n: desktop_i18n::I18n, role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => i18n.__("待办"),
        StatusRole::Queued => i18n.__("排队"),
        StatusRole::Planning => i18n.__("准备"),
        StatusRole::Executing => i18n.__("处理中"),
        StatusRole::Failed => i18n.__("失败分支"),
        StatusRole::Paused => i18n.__("暂停"),
        StatusRole::ReviewReady => i18n.__("待审核"),
        StatusRole::Review => i18n.__("审核"),
        StatusRole::Merging => i18n.__("收尾"),
        StatusRole::Done => i18n.__("成功"),
        StatusRole::Archived => i18n.__("归档"),
        StatusRole::Custom => i18n.__("其他"),
    }
}

fn role_value(role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => "Backlog",
        StatusRole::Queued => "Queued",
        StatusRole::Planning => "Planning",
        StatusRole::Executing => "Executing",
        StatusRole::Failed => "Failed",
        StatusRole::Paused => "Paused",
        StatusRole::ReviewReady => "ReviewReady",
        StatusRole::Review => "Review",
        StatusRole::Merging => "Merging",
        StatusRole::Done => "Done",
        StatusRole::Archived => "Archived",
        StatusRole::Custom => "Custom",
    }
}

fn status_role_display(i18n: desktop_i18n::I18n, role: StatusRole) -> String {
    format!("{} ({})", role_label(i18n, role), role_value(role))
}

fn status_role_help_panel(i18n: desktop_i18n::I18n) -> Element<'static, Message> {
    let mut rows = column![].spacing(8);
    for definition in status_role_help_definitions(i18n) {
        rows = rows.push(
            row![
                container(text(definition.value).size(12)).width(Length::Fixed(110.0)),
                container(text(definition.label).size(12)).width(Length::Fixed(78.0)),
                text(definition.description)
                    .size(12)
                    .style(settings_muted_text_style)
                    .width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }

    settings_panel(
        column![
            row![
                column![
                    text(i18n.__("语义角色如何决定调度")).size(15),
                    text(i18n.__("显示用中文，保存值为英文；调度按英文角色值判断阶段行为。"))
                        .size(12)
                        .style(settings_muted_text_style),
                ]
                .spacing(4),
                Space::new().width(Length::Fill),
                button(text(i18n.__("收起")).size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::ToggleStatusRoleHelp
                    )),
            ]
            .align_y(Alignment::Center),
            rows,
        ]
        .spacing(12),
    )
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusRoleHelpDefinition {
    value: &'static str,
    label: &'static str,
    description: &'static str,
}

fn status_role_help_definitions(i18n: desktop_i18n::I18n) -> Vec<StatusRoleHelpDefinition> {
    StatusRoleOption::all(i18n)
        .into_iter()
        .map(|role| {
            let role = role.0;
            StatusRoleHelpDefinition {
                value: role_value(role),
                label: role_label(i18n, role),
                description: role_scheduler_description(i18n, role),
            }
        })
        .collect()
}

fn role_scheduler_description(i18n: desktop_i18n::I18n, role: StatusRole) -> &'static str {
    match role {
        StatusRole::Backlog => i18n.__("积压池；可被自动提升到下一阶段，不直接占用执行并发。"),
        StatusRole::Queued => i18n.__("待启动；员工开始或调度器可把任务交给执行路径。"),
        StatusRole::Planning => i18n.__("规划中；可启动拆分/准备任务，并计入执行并发。"),
        StatusRole::Executing => i18n.__("执行中；占用执行并发，并参与运行超时监控。"),
        StatusRole::Failed => i18n.__("失败分支；可重试，默认回到排队或规划阶段。"),
        StatusRole::Paused => i18n.__("暂停；不参与自动调度，等待人工处理或恢复。"),
        StatusRole::ReviewReady => {
            i18n.__("待审核；可进入审核，若绑定阶段 Workflow 则优先运行绑定流程。")
        }
        StatusRole::Review => i18n.__("审核中；成功进入下一阶段，失败走 failed 边或暂停。"),
        StatusRole::Merging => i18n.__("收尾/合并；可执行合并或绑定收尾 Workflow。"),
        StatusRole::Done => i18n.__("完成；不再调度，可进入归档。"),
        StatusRole::Archived => i18n.__("归档；终态，不再调度。"),
        StatusRole::Custom => i18n.__("展示或自定义阶段；系统不施加自动调度语义。"),
    }
}

fn build_draft_editor(i18n: desktop_i18n::I18n, draft: &WorkflowDraft) -> Element<'_, Message> {
    let mut status_rows = column![].spacing(10);
    for (index, status) in draft.statuses.iter().enumerate() {
        status_rows = status_rows.push(build_status_row(i18n, index, status, draft));
    }

    let form = column![
        row![
            column![
                text(i18n.__("编辑工作流")).size(18),
                text(i18n.__("为每个状态配置结果 key 与去向；succeed/failed/retry 为内置结果。"))
                    .size(12)
                    .style(settings_muted_text_style),
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            button(text(i18n.__("取消")).size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::CancelDraft)),
            button(text(i18n.__("保存")).size(13))
                .padding([9, 14])
                .style(primary_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::SaveDraft)),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
        input_row(
            column![
                text("ID").size(13),
                text(i18n.__("小写字母、数字、下划线或连字符。"))
                    .size(11)
                    .style(settings_muted_text_style),
            ]
            .spacing(4),
            settings_text_input("handoff_flow", &draft.id)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftIdChanged(value))
                })
                .width(Length::Fill),
        ),
        input_row(
            field_help(i18n, i18n.__("名称"), i18n.__("显示在项目工作流选择和管理列表中。"), 13),
            settings_text_input("Handoff Flow", &draft.label)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftLabelChanged(value))
                })
                .width(Length::Fill),
        ),
        row![
            text(i18n.__("状态")).size(15),
            Space::new().width(Length::Fill),
            button(text(i18n.__("添加状态")).size(13))
                .padding([9, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(WorkflowManagerMessage::AddBlankStatus)),
        ]
        .align_y(Alignment::Center),
        scrollable(
            container(status_rows)
                .width(Length::Fill)
                .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
        )
        .height(Length::Fill)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    ]
    .spacing(14);

    settings_panel(form).height(Length::Fill).into()
}

fn input_row<'a>(
    label: impl Into<Element<'a, Message>>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![container(label).width(Length::Fixed(180.0)), control.into()]
        .spacing(14)
        .align_y(Alignment::Center)
        .into()
}

fn field_help(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    description: &'static str,
    size: u16,
) -> Element<'static, Message> {
    settings_hover_help_label(i18n, None, label, text(label).size(size as f32), description)
        .wrap()
        .into()
}

fn build_status_row<'a>(
    i18n: desktop_i18n::I18n,
    index: usize,
    status: &'a WorkflowDraftStatus,
    draft: &'a WorkflowDraft,
) -> Element<'a, Message> {
    let role_pick = settings_select(
        StatusRoleOption::all(i18n),
        Some(StatusRoleOption::new(status.role, i18n)),
        move |role| {
            Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRoleChanged {
                index,
                role: role.0,
            })
        },
    )
    .width(Length::Fill);

    let route_options = next_options(draft);
    let default_success_key = draft.statuses.get(index + 1).map(|next| next.key.trim().to_string());
    let mut route_rows = column![].spacing(8);
    for (route_index, route) in status.routes.iter().enumerate() {
        route_rows = route_rows.push(build_route_row(
            i18n,
            index,
            route_index,
            route,
            route_options.clone(),
            status.route_target_key(route_index, default_success_key.clone()),
        ));
    }

    settings_panel(
        column![
            row![
                text(format!("#{}", index + 1)).size(13),
                if status.enabled {
                    Space::new().width(Length::Shrink).into()
                } else {
                    settings_value_badge(i18n.__("已停用"))
                },
                Space::new().width(Length::Fill),
                button(
                    text(if status.enabled { i18n.__("停用") } else { i18n.__("启用") }).size(12)
                )
                .padding([7, 10])
                .style(rounded_action_btn_style)
                .on_press(Message::WorkflowManager(
                    WorkflowManagerMessage::ToggleDraftStatusEnabled(index)
                )),
                button(text(i18n.__("上移")).size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((index > 0).then_some(Message::WorkflowManager(
                        WorkflowManagerMessage::MoveDraftStatusUp(index),
                    ))),
                button(text(i18n.__("下移")).size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((index + 1 < draft.statuses.len()).then_some(
                        Message::WorkflowManager(WorkflowManagerMessage::MoveDraftStatusDown(
                            index
                        )),
                    )),
                button(text(i18n.__("删除")).size(12))
                    .padding([7, 10])
                    .style(danger_action_btn_style)
                    .on_press(Message::WorkflowManager(WorkflowManagerMessage::RemoveDraftStatus(
                        index,
                    ))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            input_row(
                field_help(i18n, i18n.__("状态 key"), i18n.__("自定义状态会按 key 持久化。"), 13),
                settings_text_input("ready_for_qa", &status.key)
                    .on_input(move |key| {
                        Message::WorkflowManager(WorkflowManagerMessage::DraftStatusKeyChanged {
                            index,
                            key,
                        })
                    })
                    .width(Length::Fill),
            ),
            input_row(
                field_help(i18n, i18n.__("标签"), i18n.__("用于看板列和状态展示。"), 13),
                settings_text_input("Ready for QA", &status.label)
                    .on_input(move |label| {
                        Message::WorkflowManager(WorkflowManagerMessage::DraftStatusLabelChanged {
                            index,
                            label,
                        })
                    })
                    .width(Length::Fill),
            ),
            input_row(
                field_help(i18n, i18n.__("语义角色"), i18n.__("调度和执行只依赖角色语义。"), 13),
                role_pick,
            ),
            row![
                field_help(
                    i18n,
                    i18n.__("结果去向"),
                    i18n.__("结果 key 可自定义；目标留空表示此结果结束。"),
                    13,
                ),
                Space::new().width(Length::Fill),
                button(text(i18n.__("添加结果")).size(12))
                    .padding([7, 10])
                    .style(rounded_action_btn_style)
                    .on_press(Message::WorkflowManager(
                        WorkflowManagerMessage::AddDraftStatusRoute(index)
                    )),
            ]
            .align_y(Alignment::Center),
            route_rows,
        ]
        .spacing(12),
    )
    .into()
}

fn next_options(draft: &WorkflowDraft) -> Vec<NextStatusOption> {
    let mut options = vec![NextStatusOption::end()];
    options.extend(draft.statuses.iter().filter_map(|status| {
        let key = status.key.trim();
        if key.is_empty() { None } else { Some(NextStatusOption::status(key, status.label.trim())) }
    }));
    options
}

fn build_route_row<'a>(
    i18n: desktop_i18n::I18n,
    status_index: usize,
    route_index: usize,
    route: &'a WorkflowDraftRoute,
    options: Vec<NextStatusOption>,
    selected_key: Option<String>,
) -> Element<'a, Message> {
    let outcome = route.outcome();
    let name_control: Element<'a, Message> = if route.is_builtin() {
        settings_value_badge(i18n.__(&outcome.label()).to_string())
    } else {
        settings_text_input(i18n.__("平台驳回"), &route.label)
            .on_input(move |label| {
                Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteLabelChanged {
                    status_index,
                    route_index,
                    label,
                })
            })
            .width(Length::Fill)
            .into()
    };
    let key_control: Element<'a, Message> = if route.is_builtin() {
        settings_value_badge(outcome.key().to_string())
    } else {
        settings_text_input("manual_review", &route.outcome_key)
            .on_input(move |outcome_key| {
                Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteOutcomeChanged {
                    status_index,
                    route_index,
                    outcome_key,
                })
            })
            .width(Length::Fill)
            .into()
    };
    let remove: Element<'a, Message> = if route.is_builtin() {
        Space::new().width(Length::Fixed(70.0)).into()
    } else {
        button(text(i18n.__("删除")).size(12))
            .padding([7, 10])
            .style(danger_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RemoveDraftStatusRoute {
                status_index,
                route_index,
            }))
            .into()
    };

    row![
        column![field_help(i18n, i18n.__("结果名字"), i18n.__("给人看的名称"), 12), name_control,]
            .spacing(3)
            .width(Length::Fill),
        column![field_help(i18n, i18n.__("结果 key"), i18n.__("程序匹配用"), 12), key_control,]
            .spacing(3)
            .width(Length::Fill),
        route_picker(i18n, status_index, route_index, options, selected_key),
        remove,
    ]
    .spacing(10)
    .align_y(Alignment::End)
    .into()
}

fn route_picker<'a>(
    i18n: desktop_i18n::I18n,
    status_index: usize,
    route_index: usize,
    options: Vec<NextStatusOption>,
    selected_key: Option<String>,
) -> Element<'a, Message> {
    let options = options
        .into_iter()
        .map(|mut option| {
            if option.target_key.is_none() {
                option.label = Some(i18n.__("结束点 (End)").to_string());
            }
            option
        })
        .collect::<Vec<_>>();
    let selected = NextStatusOption::selected(&options, selected_key);
    settings_select(options, Some(selected), move |target| {
        Message::WorkflowManager(WorkflowManagerMessage::SetDraftStatusRoute {
            status_index,
            route_index,
            target_key: target.target_key,
        })
    })
    .width(Length::Fill)
    .into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusRoleOption(StatusRole, String);

impl StatusRoleOption {
    fn new(role: StatusRole, i18n: desktop_i18n::I18n) -> Self {
        Self(role, status_role_display(i18n, role))
    }
    fn all(i18n: desktop_i18n::I18n) -> Vec<Self> {
        [
            StatusRole::Backlog,
            StatusRole::Queued,
            StatusRole::Planning,
            StatusRole::Executing,
            StatusRole::Failed,
            StatusRole::Paused,
            StatusRole::ReviewReady,
            StatusRole::Review,
            StatusRole::Merging,
            StatusRole::Done,
            StatusRole::Archived,
            StatusRole::Custom,
        ]
        .into_iter()
        .map(|role| Self::new(role, i18n))
        .collect()
    }
}
impl fmt::Display for StatusRoleOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.1)
    }
}

#[derive(Debug, Clone, Eq)]
struct NextStatusOption {
    target_key: Option<String>,
    label: Option<String>,
}

impl NextStatusOption {
    fn end() -> Self {
        Self { target_key: None, label: None }
    }

    fn status(key: &str, label: &str) -> Self {
        Self {
            target_key: Some(key.to_string()),
            label: (!label.is_empty()).then(|| label.to_string()),
        }
    }

    fn selected(options: &[Self], target_key: Option<String>) -> Self {
        options
            .iter()
            .find(|option| option.target_key == target_key)
            .cloned()
            .unwrap_or(Self { target_key, label: None })
    }
}

impl PartialEq for NextStatusOption {
    fn eq(&self, other: &Self) -> bool {
        self.target_key == other.target_key
    }
}

impl fmt::Display for NextStatusOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.target_key.as_deref() {
            Some(key) => match self.label.as_deref() {
                Some(label) if label != key => write!(f, "{label} ({key})"),
                _ => f.write_str(key),
            },
            None => f.write_str(self.label.as_deref().unwrap_or("结束点 (End)")),
        }
    }
}
