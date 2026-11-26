use super::status_catalog::{StatusCatalogRow, status_catalog};
use super::*;
use desktop_assets::assets::Icon;
use desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage;
use desktop_ui_controls::system_settings_common::{
    help::localized_selectable_settings_help_modal,
    icons::icon_btn,
    panels::{settings_inactive_view, with_settings_modal_escape},
};
use iced::widget::column;

#[cfg(test)]
#[path = "form_controls_tests.rs"]
mod form_controls_tests;

/// 构建工作流列表、编辑器、阶段动作页面及确认对话框。
pub fn view<'a>(
    state: &'a WorkflowManagerState,
    i18n: desktop_i18n::I18n,
    theme: &Theme,
    project_path: Option<&'a str>,
    copied_help: Option<u64>,
) -> Element<'a, Message> {
    let page = view_content(state, i18n, theme, project_path);
    with_dialog(state, i18n, copied_help, page, |message| message)
}

pub fn view_content<'a>(
    state: &'a WorkflowManagerState,
    i18n: desktop_i18n::I18n,
    theme: &Theme,
    project_path: Option<&'a str>,
) -> Element<'a, Message> {
    let project = state.project_workflow(project_path);
    let body: Element<'_, Message> = match state.active_tab {
        WorkflowManagerTab::LifecycleDefinition => {
            let editor = build_editor(i18n, state, project_path, project, theme);
            if state.draft.is_some() {
                editor
            } else {
                let list = build_workflow_list(i18n, state, project_path, project);
                row![
                    container(list).width(Length::FillPortion(1)).max_width(290).height(Length::Fill),
                    container(editor).width(Length::FillPortion(2)).height(Length::Fill),
                ]
                .spacing(16)
                .align_y(Alignment::Start)
                .into()
            }
        }
        WorkflowManagerTab::StatusCatalog => {
            build_status_catalog(i18n, state, theme, project_path, project)
        }
    };

    let mut header_actions = row![
        button(text(i18n.__("刷新")).size(13))
            .padding([9, 14])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::RefreshLifecycles)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if state.active_tab == WorkflowManagerTab::LifecycleDefinition && state.draft.is_none() {
        header_actions = header_actions.push(workflow_action(
            i18n.__("新建"),
            WorkflowManagerMessage::StartNew,
            primary_action_btn_style,
        ));
    }

    let content = column![
        settings_panel(
            column![
                row![
                    text(i18n.__("工作流")).size(20),
                    icon_btn(
                        Icon::QuestionCircle,
                        i18n.__("帮助"),
                        Some(Message::WorkflowManager(WorkflowManagerMessage::ToggleStatusRoleHelp)),
                    ),
                    Space::new().width(Length::Fill),
                    header_actions,
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                workflow_manager_tabs(i18n, state.active_tab),
            ]
            .spacing(12)
        ),
        body,
    ]
    .spacing(16)
    .padding([18, 24])
    .width(Length::Fill)
    .height(Length::Fill);

    container(content)
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
        .into()
}

pub fn with_dialog<'a, M: SelectionMessage + 'static>(
    state: &WorkflowManagerState,
    i18n: desktop_i18n::I18n,
    copied_help: Option<u64>,
    page: Element<'a, M>,
    map: impl Fn(Message) -> M + 'static,
) -> Element<'a, M> {
    if let Some(confirmation) = &state.pending_confirmation {
        let close = map(Message::WorkflowManager(WorkflowManagerMessage::CancelPendingConfirmation));
        with_settings_modal_escape(
            iced::widget::stack![
                settings_inactive_view(page),
                workflow_confirmation_dialog(i18n, confirmation).map(map),
            ].into(),
            close,
        )
    } else if state.show_status_role_help {
        let close = map(Message::WorkflowManager(WorkflowManagerMessage::ToggleStatusRoleHelp));
        let help = [
            i18n.__("任务进入：新任务从任务池开始。排队后，符合项目调度条件的任务会被派发。"),
            i18n.__("人工审核：例如执行成功后进入「待审核」，由审核人员确认结果。业务退回是审核决定，可以按配置退回处理；执行失败是运行时发生异常。"),
            i18n.__("自动重试：执行异常后，调度器按项目设置和重试去向再试。模型响应超时按设置等待。达到次数上限后，任务留在失败状态，等待处理。"),
            i18n.__("项目切换：选择预设并确认应用。网关会检查当前任务能否继续使用原有状态。应用成功后，可在「项目使用的工作流」查看实际阶段，并在任务看板确认。"),
        ].join("\n\n");
        let modal = localized_selectable_settings_help_modal(
            i18n,
            copied_help,
            page,
            "工作流帮助",
            help,
            close.clone(),
        );
        with_settings_modal_escape(modal, close)
    } else {
        page
    }
}

fn workflow_confirmation_dialog(
    i18n: desktop_i18n::I18n,
    confirmation: &WorkflowManagerConfirmation,
) -> Element<'static, Message> {
    let mut confirm = WorkflowManagerMessage::ConfirmPendingAction;
    let (title, body, confirm_label) = match confirmation {
        WorkflowManagerConfirmation::ApplyProjectWorkflow { path, workflow_id, workflow_label } => {
            confirm = WorkflowManagerMessage::ApplyProjectWorkflow {
                path: path.clone(),
                workflow_id: workflow_id.clone(),
            };
            (
                i18n.__("应用工作流预设").to_string(),
                i18n.format(
                    "将为项目 %s 应用「%s」。网关会检查进行中的任务和已有状态是否兼容。",
                    &[path.clone().into(), workflow_label.clone().into()],
                ).expect("valid workflow display template"),
                i18n.__("确认应用"),
            )
        }
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
        Message::WorkflowManager(confirm),
        Message::WorkflowManager(WorkflowManagerMessage::CancelPendingConfirmation),
    )
}

fn workflow_manager_tabs(
    i18n: desktop_i18n::I18n,
    active_tab: WorkflowManagerTab,
) -> Element<'static, Message> {
    row![
        workflow_manager_tab_button(
            i18n.__("工作流预览"),
            WorkflowManagerTab::LifecycleDefinition,
            active_tab,
        ),
        workflow_manager_tab_button(
            i18n.__("状态库"),
            WorkflowManagerTab::StatusCatalog,
            active_tab,
        ),
    ]
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

#[derive(Clone, PartialEq)]
struct CatalogSource {
    preset_id: Option<String>,
    label: String,
}

impl fmt::Display for CatalogSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn build_status_catalog<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowManagerState,
    theme: &Theme,
    project_path: Option<&str>,
    project: Option<&'a ProjectWorkflow>,
) -> Element<'a, Message> {
    let mut options = vec![CatalogSource {
        preset_id: None,
        label: i18n.__(if project_path.is_some() { "项目使用的工作流" } else { "当前工作流" }).to_string(),
    }];
    options.extend(state.workflows.iter().filter(|item| item.is_builtin).map(|item| {
        CatalogSource {
            preset_id: Some(item.definition.id.clone()),
            label: item.definition.label.clone(),
        }
    }));
    let selected = options
        .iter()
        .find(|option| option.preset_id == state.catalog_preset_id)
        .cloned();
    let workflow = match state.catalog_preset_id.as_ref() {
        Some(id) => state.workflows.iter().find(|item| item.definition.id == *id)
            .map(|item| &item.definition),
        None if project_path.is_some() => project.and_then(|project| project.definition.as_ref()),
        None => state.selected_item().map(|item| &item.definition),
    };

    let mut groups = column![].spacing(16);
    if let Some(workflow) = workflow {
        for (group, rows) in status_catalog(workflow) {
            let mut section = column![text(group.label(i18n)).size(15)].spacing(8);
            if rows.is_empty() {
                section = section.push(
                    text(i18n.__("此组暂无状态")).size(12).style(settings_muted_text_style),
                );
            }
            for status in rows {
                let can_edit = state.draft.is_none() && state.workflows.iter().any(|item| {
                    item.definition.id == workflow.id
                        && item.definition.statuses.iter().any(|entry| entry.key == status.status.key)
                });
                section = section.push(build_catalog_status_row(
                    i18n,
                    status,
                    state.show_catalog_advanced,
                    can_edit,
                    theme,
                ));
            }
            groups = groups.push(section);
        }
    } else {
        groups = groups.push(
            text(i18n.__("请选择工作流")).size(13).style(settings_muted_text_style),
        );
    }

    let mut header = column![
        row![
            settings_hover_help_label(
                i18n,
                theme,
                i18n.__("状态库"),
                text(i18n.__("状态库")).size(18),
                i18n.__("状态按任务进展分为未开始、已开始、已完成和已关闭。例如暂停属于已开始，恢复前不会自动继续。停用的状态仍可查看和编辑。"),
            ),
            Space::new().width(Length::Fill),
            button(
                text(if state.show_catalog_advanced { i18n.__("收起") } else { i18n.__("高级") })
                    .size(12),
            )
            .padding([7, 10])
            .style(rounded_action_btn_style)
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ToggleCatalogAdvanced)),
        ]
        .align_y(Alignment::Center)
        .spacing(8),
        row![
            settings_select(options, selected, |source| {
                Message::WorkflowManager(WorkflowManagerMessage::SelectCatalogPreset(source.preset_id))
            }),
            text(
                workflow
                    .filter(|_| state.catalog_preset_id.is_none())
                    .map_or("", |workflow| workflow.label.as_str()),
            )
            .size(13)
            .style(settings_muted_text_style),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
    ]
    .spacing(10);
    if state.show_catalog_advanced {
        header = header.push(field_help(
            i18n,
            i18n.__("状态分组规则"),
            i18n.__("状态行为为「其他」时，归入「已开始」。"),
            12,
        ));
    }
    if state.draft.is_some() {
        header = header.push(
            text(i18n.__("保存或取消当前草稿后，可从状态库打开状态编辑。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }

    settings_panel(
        column![
            header,
            scrollable(
                container(groups)
                    .width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER)),
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ]
        .spacing(14),
    )
    .height(Length::Fill)
    .into()
}

fn build_catalog_status_row<'a>(
    i18n: desktop_i18n::I18n,
    entry: StatusCatalogRow<'a>,
    advanced: bool,
    can_edit: bool,
    theme: &Theme,
) -> Element<'a, Message> {
    let status = entry.status;
    let mut name = button(text(status.label.as_str()).size(14))
        .padding([6, 10])
        .style(rounded_action_btn_style);
    if can_edit {
        name = name.on_press(Message::WorkflowManager(WorkflowManagerMessage::EditCatalogStatus {
            workflow_id: entry.workflow_id.to_string(),
            status_key: status.key.clone(),
        }));
    }
    let mut title = row![name].spacing(8).align_y(Alignment::Center);
    if entry.is_builtin {
        title = title.push(settings_value_badge(i18n.__("内置")));
    }
    let description = i18n.__(entry.behavior.summary);
    let mut details = column![
        title,
        settings_hover_help_label(
            i18n,
            theme,
            description,
            text(description).size(12).style(settings_muted_text_style),
            i18n.__(entry.behavior.help),
        ),
    ]
    .spacing(4);
    if advanced {
        details = details.push(
            row![
                text(status.key.as_str()).size(12).style(settings_muted_text_style),
                text(status_role_display(i18n, status.role)).size(12).style(settings_muted_text_style),
            ]
            .spacing(10),
        );
    }

    settings_panel(details).into()
}

fn build_workflow_list<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowManagerState,
    project_path: Option<&'a str>,
    project: Option<&'a ProjectWorkflow>,
) -> Element<'a, Message> {
    let mut items = column![].spacing(8);
    for workflow in &state.workflows {
        let selected = (!state.show_project_workflow || project_path.is_none())
            && state.selected_workflow_id.as_deref() == Some(workflow.definition.id.as_str());
        let badge = if workflow.is_builtin { i18n.__("内置") } else { i18n.__("用户") };
        let button_style: fn(&Theme, button::Status) -> button::Style =
            if selected { primary_action_btn_style } else { rounded_action_btn_style };
        let badge = text(badge).size(11);
        let badge = if selected { badge } else { badge.style(settings_muted_text_style) };
        items = items.push(
            button(column![
                text(workflow.definition.label.as_str()).size(13),
                badge,
            ].spacing(3))
            .width(Length::Fill)
            .padding([10, 12])
            .style(button_style)
            .on_press(Message::WorkflowManager(
                WorkflowManagerMessage::SelectWorkflow(workflow.definition.id.clone()),
            )),
        );
    }

    let mut content = column![].spacing(12);
    if project_path.is_some() {
        let style: fn(&Theme, button::Status) -> button::Style = if state.show_project_workflow {
            primary_action_btn_style
        } else {
            rounded_action_btn_style
        };
        let title = project.and_then(|project| project.definition.as_ref())
            .map_or_else(|| i18n.__("正在读取…"), |workflow| workflow.label.as_str());
        let mut summary = column![text(i18n.__("项目使用的工作流")).size(13)].spacing(4);
        if let Some(error) = project.and_then(|project| project.error.as_deref()) {
            summary = summary.push(text(i18n.__(error)).size(12));
        } else {
            summary = summary.push(text(title).size(14));
        }
        content = content.push(
            button(summary)
                .width(Length::Fill)
                .padding([10, 12])
                .style(style)
                .on_press(Message::WorkflowManager(
                    WorkflowManagerMessage::ViewProjectWorkflow,
                )),
        );
    }
    settings_panel(content
        .push(text(i18n.__("预设")).size(15))
        .push(scrollable(
            container(items)
                .width(Length::Fill)
                .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER)),
        )
        .height(Length::Fill)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))))
    .height(Length::Fill)
    .into()
}

fn build_editor<'a>(
    i18n: desktop_i18n::I18n,
    state: &'a WorkflowManagerState,
    project_path: Option<&'a str>,
    project: Option<&'a ProjectWorkflow>,
    theme: &Theme,
) -> Element<'a, Message> {
    if let Some(draft) = &state.draft {
        return build_draft_editor(i18n, draft);
    }

    let selected = state.selected_item();
    let show_project = state.show_project_workflow && project_path.is_some();
    let workflow = if show_project {
        project.and_then(|project| project.definition.as_ref())
    } else {
        selected.map(|item| &item.definition)
    };
    let Some(workflow) = workflow else {
        let message = if show_project {
            project.and_then(|project| project.error.as_deref()).unwrap_or("正在读取项目工作流…")
        } else {
            "请选择工作流"
        };
        return settings_panel(text(i18n.__(message)).style(settings_muted_text_style)).into();
    };

    let mut controls = column![].spacing(8);
    if show_project {
        if let Some(preset) = state.workflows.iter().find(|item| item.definition.id == workflow.id) {
            controls = controls.push(workflow_action(
                i18n.__("查看对应预设"),
                WorkflowManagerMessage::SelectWorkflow(preset.definition.id.clone()),
                rounded_action_btn_style,
            ));
        }
    } else if let Some(selected) = selected {
        let (label, action) = if selected.is_builtin {
            (i18n.__("重置"), WorkflowManagerMessage::ResetSelected)
        } else {
            (i18n.__("删除"), WorkflowManagerMessage::DeleteSelected)
        };
        controls = controls.push(
            row![
                workflow_action(i18n.__("编辑"), WorkflowManagerMessage::StartEditSelected, rounded_action_btn_style),
                workflow_action(i18n.__("复制"), WorkflowManagerMessage::CopyWorkflow(workflow.id.clone()), rounded_action_btn_style),
                workflow_action(label, action, danger_action_btn_style),
            ].spacing(8),
        );
        if let Some(path) = project_path {
            let can_apply = !state.lifecycles_loading && project.is_some_and(|project| {
                    !project.busy && project.error.is_none() && project.definition.is_some()
            });
            let applying = project.is_some_and(|project| project.busy && project.definition.is_some());
            controls = controls.push(row![
                button(text(i18n.__(if applying { "正在应用…" } else { "应用到项目" })).size(13))
                    .padding([8, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe(can_apply.then(|| Message::WorkflowManager(
                        WorkflowManagerMessage::RequestProjectWorkflow {
                            path: path.to_string(),
                            workflow_id: workflow.id.clone(),
                        },
                    ))),
                settings_hover_help_label(
                    i18n,
                    theme,
                    i18n.__("应用到项目"),
                    text(""),
                    i18n.__("例如，应用新预设后，任务看板会使用它的阶段。网关会检查现有任务，保存或复制预设本身只会修改预设列表。"),
                ),
            ].align_y(Alignment::Center).spacing(4));
        }
    }

    let mut flow = column![].spacing(16);
    for (group, rows) in status_catalog(workflow) {
        let mut section = column![text(group.label(i18n)).size(15)].spacing(8);
        if rows.is_empty() {
            section = section.push(
                text(i18n.__("此组暂无状态")).size(12).style(settings_muted_text_style),
            );
        }
        for entry in rows {
            let editable = state.workflows.iter().any(|item| {
                item.definition.id == workflow.id
                    && item.definition.statuses.iter().any(|status| status.key == entry.status.key)
            });
            section = section.push(build_workflow_status_view(
                i18n,
                workflow,
                entry,
                editable,
                show_project,
            ));
        }
        flow = flow.push(section);
    }

    settings_panel(
        column![
            text(workflow.label.as_str()).size(18),
            controls,
            scrollable(
                container(flow).width(Length::Fill)
                    .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER)),
            )
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
        ].spacing(12),
    ).height(Length::Fill).into()
}

fn workflow_action(
    label: &'static str,
    action: WorkflowManagerMessage,
    style: fn(&Theme, button::Status) -> button::Style,
) -> Element<'static, Message> {
    workflow_action_button(label, style)
        .on_press(Message::WorkflowManager(action))
        .into()
}

fn workflow_action_button(
    label: &'static str,
    style: fn(&Theme, button::Status) -> button::Style,
) -> button::Button<'static, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(style)
}

fn build_workflow_status_view<'a>(
    i18n: desktop_i18n::I18n,
    workflow: &TaskWorkflow,
    entry: StatusCatalogRow<'a>,
    editable: bool,
    show_project: bool,
) -> Element<'a, Message> {
    let status = entry.status;
    let mut header = row![text(status.label.as_str()).size(15), Space::new().width(Length::Fill)]
        .spacing(6)
        .align_y(Alignment::Center);
    if editable {
        header = header.push(workflow_action(
            i18n.__(if show_project { "编辑预设阶段" } else { "编辑阶段" }),
            WorkflowManagerMessage::EditCatalogStatus {
                workflow_id: workflow.id.clone(),
                status_key: status.key.clone(),
            },
            rounded_action_btn_style,
        ));
    }

    let mut details = column![
        header,
        text(i18n.__(entry.behavior.summary)).size(12).style(settings_muted_text_style),
    ].spacing(6);
    if status.enabled {
        let actor = match status.role {
            StatusRole::Queued | StatusRole::Planning | StatusRole::Executing =>
                Some("执行者跟随任务分配与项目执行设置"),
            StatusRole::ReviewReady | StatusRole::Review => Some("由审核人员处理"),
            StatusRole::Paused => Some("由项目成员恢复"),
            _ => None,
        };
        if let Some(actor) = actor {
            details = details.push(text(i18n.__(actor)).size(12).style(settings_muted_text_style));
        }
    }

    for (outcome, label, fallback) in [
        (StatusOutcome::succeed(), "成功", "无后续状态"),
        (StatusOutcome::failed(), "失败", "需人工处理"),
        (StatusOutcome::retry(), "重试", "留在失败状态"),
    ] {
        let configured = status.transition_target(outcome.clone()).is_some();
        let target = workflow.next_status_for_outcome(status.status, outcome.clone())
            .and_then(|target| workflow.status_definition(target));
        let target = if configured {
            target
        } else if outcome.is_failed() && status.role == StatusRole::Executing {
            let mut failures = workflow.statuses.iter()
                .filter(|entry| entry.enabled && entry.role == StatusRole::Failed);
            let first = failures.next();
            first.filter(|_| failures.next().is_none())
        } else if outcome.is_retry() && status.role == StatusRole::Failed && status.next.is_some() {
            workflow.next_status(status.status).and_then(|target| workflow.status_definition(target))
        } else {
            None
        };
        let fallback = if outcome.is_failed() && status.role == StatusRole::Executing {
            "执行失败"
        } else if outcome.is_succeed() && status.role == StatusRole::Executing {
            "按执行失败处理"
        } else if configured && !outcome.is_retry() {
            "需人工处理"
        } else {
            fallback
        };
        if configured
            || status.enabled && match status.role {
                StatusRole::Executing | StatusRole::ReviewReady | StatusRole::Review =>
                    !outcome.is_retry(),
                StatusRole::Failed => outcome.is_retry(),
                _ => false,
            }
        {
            let fallback = if matches!(status.role, StatusRole::Review | StatusRole::ReviewReady)
                && outcome.is_succeed()
            {
                "需人工处理"
            } else {
                fallback
            };
            let destination = target.map_or(i18n.__(fallback), |target| target.label.as_str());
            details = details.push(text(format!("{} → {destination}", i18n.__(label))).size(13));
        }
    }
    settings_panel(details).into()
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

fn build_draft_editor(i18n: desktop_i18n::I18n, draft: &WorkflowDraft) -> Element<'_, Message> {
    let mut status_rows = column![].spacing(10);
    for (index, status) in draft.statuses.iter().enumerate() {
        if draft.focused_status.is_none_or(|focused| focused == index) {
            status_rows = status_rows.push(build_status_row(i18n, index, status, draft));
        }
    }
    let status_action = if draft.focused_status.is_some() {
        button(text(i18n.__("显示全部状态")).size(13))
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::ShowAllDraftStatuses))
    } else {
        button(text(i18n.__("添加状态")).size(13))
            .on_press(Message::WorkflowManager(WorkflowManagerMessage::AddBlankStatus))
    }
    .padding([9, 14])
    .style(rounded_action_btn_style);

    let mut form = column![
        row![
            text(i18n.__("编辑工作流")).size(18),
            Space::new().width(Length::Fill),
            workflow_action(
                i18n.__(if draft.show_advanced { "收起高级设置" } else { "高级设置" }),
                WorkflowManagerMessage::ToggleDraftAdvanced,
                rounded_action_btn_style,
            ),
            workflow_action(i18n.__("取消"), WorkflowManagerMessage::CancelDraft, rounded_action_btn_style),
            workflow_action(i18n.__("保存"), WorkflowManagerMessage::SaveDraft, primary_action_btn_style),
        ]
        .align_y(Alignment::Center)
        .spacing(8),
        input_row(
            field_help(i18n, i18n.__("名称"), i18n.__("显示在项目工作流选择和管理列表中。"), 13),
            settings_text_input("Handoff Flow", &draft.label)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftLabelChanged(value))
                })
                .width(Length::Fill),
        ),
    ].spacing(12);
    if draft.show_advanced {
        form = form.push(input_row(
            field_help(i18n, "ID", i18n.__("小写字母、数字、下划线或连字符。"), 13),
            settings_text_input("handoff_flow", &draft.id)
                .on_input(|value| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftIdChanged(value))
                })
                .width(Length::Fill),
        ));
    }
    form = form.push(
        row![
            text(i18n.__("状态")).size(15),
            Space::new().width(Length::Fill),
            status_action,
        ]
        .align_y(Alignment::Center)
    ).push(scrollable(
            container(status_rows)
                .width(Length::Fill)
                .padding(iced::Padding::default().right(WORKFLOW_SCROLL_RIGHT_GUTTER))
        )
        .height(Length::Fill)
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4))),
    );

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
    let mut fields = column![
        input_row(
            field_help(i18n, i18n.__("阶段名称"), i18n.__("用于看板列和状态展示。"), 13),
            settings_text_input(i18n.__("待审核"), &status.label)
                .on_input(move |label| {
                    Message::WorkflowManager(WorkflowManagerMessage::DraftStatusLabelChanged {
                        index,
                        label,
                    })
                })
                .width(Length::Fill),
        ),
    ].spacing(10);
    let route_options = next_options(draft);
    let default_success_key = draft.statuses.get(index + 1).map(|next| next.key.trim().to_string());
    for (route_index, route) in status.routes.iter().enumerate().filter(|(_, route)| route.is_builtin()) {
        let outcome = route.outcome();
        let (label, empty) = if outcome.is_succeed() {
            ("成功去向", "无后续状态")
        } else if outcome.is_failed() {
            ("失败去向", "使用默认失败处理")
        } else {
            ("重试去向", "无专用重试去向")
        };
        let help = if outcome.is_succeed() {
            "例如，执行成功后可进入待审核。"
        } else if outcome.is_failed() {
            "执行异常后按此去向处理。业务退回可以在高级设置中配置。"
        } else {
            "失败状态达到重试条件时按此去向流转。"
        };
        fields = fields.push(input_row(
            field_help(i18n, i18n.__(label), i18n.__(help), 13),
            route_picker(
                i18n.__(empty),
                index,
                route_index,
                route_options.clone(),
                status.route_target_key(route_index, default_success_key.clone()),
            ),
        ));
    }
    if draft.show_advanced {
        let role_pick = settings_select(
            StatusRoleOption::all(i18n),
            Some(StatusRoleOption::new(status.role, i18n)),
            move |role| Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRoleChanged {
                index,
                role: role.0,
            }),
        ).width(Length::Fill);
        fields = fields
            .push(input_row(
                field_help(i18n, i18n.__("状态 key"), i18n.__("自定义状态会按 key 持久化。"), 13),
                settings_text_input("ready_for_qa", &status.key)
                    .on_input(move |key| {
                        Message::WorkflowManager(WorkflowManagerMessage::DraftStatusKeyChanged { index, key })
                    })
                    .width(Length::Fill),
            ))
            .push(input_row(
                field_help(
                    i18n,
                    i18n.__("状态行为"),
                    i18n.__("决定任务在这个状态下如何调度和执行。"),
                    13,
                ),
                role_pick,
            ))
            .push(row![
                workflow_action(
                    i18n.__(if status.enabled { "停用" } else { "启用" }),
                    WorkflowManagerMessage::ToggleDraftStatusEnabled(index),
                    rounded_action_btn_style,
                ),
                workflow_action_button(i18n.__("上移"), rounded_action_btn_style)
                    .on_press_maybe((index > 0).then_some(Message::WorkflowManager(
                        WorkflowManagerMessage::MoveDraftStatusUp(index),
                    ))),
                workflow_action_button(i18n.__("下移"), rounded_action_btn_style)
                    .on_press_maybe((index + 1 < draft.statuses.len()).then_some(
                        Message::WorkflowManager(WorkflowManagerMessage::MoveDraftStatusDown(index)),
                    )),
                workflow_action(
                    i18n.__("删除"),
                    WorkflowManagerMessage::RemoveDraftStatus(index),
                    danger_action_btn_style,
                ),
            ].spacing(6))
            .push(workflow_action(
                i18n.__("添加自定义结果"),
                WorkflowManagerMessage::AddDraftStatusRoute(index),
                rounded_action_btn_style,
            ));
        for (route_index, route) in status.routes.iter().enumerate().filter(|(_, route)| !route.is_builtin()) {
            fields = fields.push(build_route_row(
                i18n,
                index,
                route_index,
                route,
                route_options.clone(),
                status.route_target_key(route_index, default_success_key.clone()),
            ));
        }
    }
    settings_panel(fields).into()
}

fn next_options(draft: &WorkflowDraft) -> Vec<NextStatusOption> {
    let mut options = vec![NextStatusOption { target_key: None, label: String::new() }];
    options.extend(draft.statuses.iter().filter_map(|status| {
        let key = status.key.trim();
        if key.is_empty() {
            return None;
        }
        let label = status.label.trim();
        Some(NextStatusOption {
            target_key: Some(key.to_string()),
            label: if draft.show_advanced { format!("{label} ({key})") } else { label.to_string() },
        })
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
    let name_control = settings_text_input(i18n.__("平台驳回"), &route.label)
        .on_input(move |label| {
            Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteLabelChanged {
                status_index, route_index, label,
            })
        })
        .width(Length::Fill);
    let key_control = settings_text_input("manual_review", &route.outcome_key)
        .on_input(move |outcome_key| {
            Message::WorkflowManager(WorkflowManagerMessage::DraftStatusRouteOutcomeChanged {
                status_index, route_index, outcome_key,
            })
        })
        .width(Length::Fill);

    row![
        column![field_help(i18n, i18n.__("结果名字"), i18n.__("给人看的名称"), 12), name_control,]
            .spacing(3)
            .width(Length::Fill),
        column![field_help(i18n, i18n.__("结果 key"), i18n.__("程序匹配用"), 12), key_control,]
            .spacing(3)
            .width(Length::Fill),
        route_picker(i18n.__("无后续状态"), status_index, route_index, options, selected_key),
        workflow_action(
            i18n.__("删除"),
            WorkflowManagerMessage::RemoveDraftStatusRoute { status_index, route_index },
            danger_action_btn_style,
        ),
    ]
    .spacing(10)
    .align_y(Alignment::End)
    .into()
}

fn route_picker<'a>(
    empty_label: &str,
    status_index: usize,
    route_index: usize,
    options: Vec<NextStatusOption>,
    selected_key: Option<String>,
) -> Element<'a, Message> {
    let options = options
        .into_iter()
        .map(|mut option| {
            if option.target_key.is_none() {
                option.label = empty_label.to_string();
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
    label: String,
}

impl NextStatusOption {
    fn selected(options: &[Self], target_key: Option<String>) -> Self {
        options
            .iter()
            .find(|option| option.target_key == target_key)
            .cloned()
            .unwrap_or(Self { target_key, label: "目标阶段已移除".to_string() })
    }
}

impl PartialEq for NextStatusOption {
    fn eq(&self, other: &Self) -> bool {
        self.target_key == other.target_key
    }
}

impl fmt::Display for NextStatusOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}
