use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_divider, settings_error_banner, settings_modal_card, settings_modal_header,
    settings_modal_layout, settings_muted_text_style, settings_select, settings_text_input,
};
use crate::app::i18n::{I18n, tasks::status_label};
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardDetailTab;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::board::{task_label_badge, task_status_marker, task_title};
use desktop_task_board::detail::{ATTACHMENT_INPUT_ID, Event, Field};
use desktop_task_board::helpers::{execution::truncate_for_ui, logs::localized_log_message};
use desktop_task_model::{Task, TaskStatus, TaskWorkflow};
use desktop_ui_controls::system_settings_common::{
    icons::icon_btn_svg_style, settings_divider_style,
};
use iced::widget::{
    Column, Space, Svg, Text, button, column, container, keyed_column, responsive, row, scrollable,
    text,
};
use iced::{Alignment, Border, Element, Font, Length, Theme};
use std::hash::{DefaultHasher, Hash, Hasher};

#[path = "detail/discussion.rs"]
mod discussion;

pub(super) fn view<'a>(app: &'a App, task: &'a Task) -> Element<'a, Message> {
    let mut hasher = DefaultHasher::new();
    task.id.hash(&mut hasher);
    keyed_column([(
        hasher.finish(),
        responsive(move |size| layout(app, task, size.width >= 640.0)).into(),
    )])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn layout<'a>(app: &'a App, task: &'a Task, wide: bool) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let workflow = app.task_board.task_board_settings.workflow();
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let project_name = if project.is_empty() {
        i18n.__("项目").to_owned()
    } else {
        desktop_workspace::basic::display_name_for_path(
            &app.workspace_state.recent_projects_meta,
            project,
        )
    };
    let mut header = row![
        action(Icon::ArrowLeft, i18n.__("返回"), TaskBoardMessage::CloseTaskLogs),
        container(row![icon(Icon::FolderClosed), muted(project_name.clone())]
            .spacing(7)
            .align_y(Alignment::Center))
        .width(Length::Fill)
        .clip(true),
    ];
    if app.task_board.work_viewer_id.is_some() {
        let subscribed = app.task_board.work_task_subscribed(&task.id);
        header = header.push(action(
            Icon::Eye,
            i18n.__(if subscribed { "取消订阅" } else { "订阅任务" }),
            TaskBoardMessage::ToggleWorkSubscription(task.id.clone()),
        ));
    }
    let header = header.push(
        button(row![icon(Icon::ArrowClockwise), text(i18n.__("刷新")).size(13)].spacing(6))
            .padding([6, 10])
            .style(rounded_action_btn_style)
            .on_press_maybe(
                (!app.task_board.detail.write_pending()).then_some(event(Event::Reload))
            ),
    )
    .spacing(10)
    .align_y(Alignment::Center);
    let header = container(header)
        .padding([10, 16])
        .width(Length::Fill)
        .style(super::style::sidebar_surface);

    let identity = row![
        task_status_marker(task.status),
        muted(&task.id).wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut content = column![
        column![identity, heading(task_title(task), if wide { 31 } else { 25 })].spacing(9),
    ]
    .spacing(24)
    .width(Length::Fill);
    if !task.prompt.trim().is_empty() {
        content = content.push(
            column![
                heading(i18n.__("任务需求"), 16),
                text(task.prompt.trim())
                    .size(14)
                    .line_height(1.55)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(9),
        );
    }
    let reason = match task.status {
        TaskStatus::Failed => task.last_error.as_deref(),
        TaskStatus::Paused => task.pause_reason.as_deref(),
        _ => None,
    };
    if let Some(reason) = reason.filter(|reason| !reason.trim().is_empty()) {
        content = content.push(
            container(
                row![
                    task_status_marker(task.status),
                    text(reason).size(13).wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .style(surface),
        );
    }
    if let Some(dependencies) = app.task_board.blocked_task_dependencies.get(&task.id) {
        let label = i18n
            .format("等待任务依赖 · %s", &[dependencies.join("、").into()])
            .expect("valid dependency message");
        content = content.push(text(label).size(13).wrapping(text::Wrapping::WordOrGlyph));
    }
    content = content
        .push(settings_divider())
        .push(activity(app, i18n, task))
        .push(discussion::comments(app, i18n, task));

    let properties_header = heading(i18n.__("属性"), 16);
    let properties = properties(app, task, project, &project_name, workflow.as_ref());
    let composer = discussion::composer(app, i18n, task);
    let body: Element<'_, Message> = if wide {
        row![
            column![
                scroll(
                    container(container(content).width(Length::Fill).max_width(780).padding(24))
                        .center_x(Length::Fill)
                ),
                container(
                    container(
                        container(composer).width(Length::Fill).max_width(780).padding([10, 24])
                    )
                    .center_x(Length::Fill)
                )
                .padding([0, 7])
                .width(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
            container(Space::new()).width(1).height(Length::Fill).style(settings_divider_style),
            container(column![
                container(properties_header).padding([15, 19]),
                settings_divider(),
                scroll(container(properties).padding([12, 12])),
            ])
            .width(256)
            .height(Length::Fill)
            .style(super::style::sidebar_surface),
        ]
        .height(Length::Fill)
        .into()
    } else {
        scroll(
            container(
                content.push(composer).push(settings_divider()).push(properties_header).push(properties)
            )
            .padding([18, 10]),
        )
        .into()
    };

    container(column![header, settings_divider(), body].height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .clip(true)
        .into()
}

fn surface(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: theme.palette().text.scale_alpha(0.12),
            width: 1.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    }
}

fn scroll<'a>(content: impl Into<Element<'a, Message>>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(content).width(Length::Fill).padding([0, 7]))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
}

fn heading<'a>(label: impl Into<String>, size: u16) -> Text<'a> {
    text(label.into())
        .size(f32::from(size))
        .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
}

fn muted<'a>(label: impl Into<String>) -> Text<'a> {
    text(label.into()).size(12).style(settings_muted_text_style)
}

fn icon(icon: Icon) -> Svg<'static> {
    icon_svg(icon, 14.0).style(icon_btn_svg_style)
}

fn action<'a>(
    glyph: Icon,
    label: &'a str,
    message: TaskBoardMessage,
) -> button::Button<'a, Message> {
    button(row![icon(glyph), text(label).size(13)].spacing(6).align_y(Alignment::Center))
        .padding([6, 10])
        .style(rounded_action_btn_style)
        .on_press(message.into())
}

fn time(timestamp: u64) -> String {
    if timestamp == 0 {
        "—".into()
    } else {
        desktop_i18n::chat_time::format_chat_time_label(timestamp)
    }
}

fn event(message: Event) -> Message {
    TaskBoardMessage::Detail(message).into()
}

fn activity<'a>(app: &'a App, i18n: I18n, task: &'a Task) -> Element<'a, Message> {
    let state = &app.task_board.detail;
    let toggle = if state.activity_expanded { Icon::ChevronDown } else { Icon::ChevronRight };
    let mut entries = column![
        row![
            heading(i18n.__("动态"), 17).width(Length::Fill),
            button(row![icon(toggle), text(i18n.__("状态与执行动态")).size(13)].spacing(6))
                .style(rounded_action_btn_style)
                .padding([6, 8])
                .on_press(event(Event::ToggleActivity)),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(10);
    if state.activity_expanded {
        let log_entries = task.logs.iter().rev().take(30).map(|entry| {
            let lead = entry
                .status_to
                .map_or_else(|| Element::from(icon(Icon::Journals)), task_status_marker);
            row![
                lead,
                text(truncate_for_ui(&localized_log_message(i18n, entry), 1000))
                    .size(13)
                    .width(Length::Fill)
                    .wrapping(text::Wrapping::WordOrGlyph),
                muted(time(entry.timestamp_ms)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        });
        let created_at = state.data.as_ref().map_or(task.created_at_ms, |detail| {
            u64::try_from(detail.created_at_ms.0).unwrap_or_default()
        });
        let timeline = column(log_entries).spacing(9).push(
            row![
                icon(Icon::Clock),
                text(i18n.__("任务创建")).size(13).width(Length::Fill),
                muted(time(created_at))
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        entries = entries.push(container(timeline).padding(12).width(Length::Fill).style(surface));
    }
    entries.into()
}

pub(super) fn display_author<'a>(i18n: I18n, id: &'a str) -> &'a str {
    if id == vw_api_transport::id::DEFAULT_USER_ID { i18n.__("本地用户") } else { id }
}

fn field<'a>(
    label: impl Into<String>,
    value: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![muted(label).width(70), value.into()]
        .spacing(8)
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .into()
}

#[derive(Clone)]
struct StatusChoice(TaskStatus, String);

impl PartialEq for StatusChoice {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Display for StatusChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

fn status_picker<'a>(
    i18n: I18n,
    task: &Task,
    workflow: Option<&TaskWorkflow>,
) -> Element<'a, Message> {
    let label = |status| {
        workflow.and_then(|workflow| workflow.status_definition(status)).map_or_else(
            || status_label(i18n, status),
            |definition| {
                if matches!(status, TaskStatus::Custom(_)) {
                    definition.label.clone()
                } else {
                    i18n.__(&definition.label).to_owned()
                }
            },
        )
    };
    let options = workflow
        .into_iter()
        .flat_map(|workflow| &workflow.statuses)
        .filter(|definition| definition.enabled)
        .map(|definition| StatusChoice(definition.status, label(definition.status)))
        .collect::<Vec<_>>();
    let task_id = task.id.clone();
    let control = settings_select(
        options,
        Some(StatusChoice(task.status, label(task.status))),
        move |choice| {
            TaskBoardMessage::TaskStatusChanged { task_id: task_id.clone(), new_status: choice.0 }
                .into()
        },
    )
    .disabled(task.archived || task.deleted)
    .width(Length::Fill);
    row![task_status_marker(task.status), control].spacing(7).align_y(Alignment::Center).into()
}

fn edit_property<'a>(
    value: impl Into<String>,
    target: Field,
    enabled: bool,
) -> Element<'a, Message> {
    button(text(value.into()).size(13).wrapping(text::Wrapping::WordOrGlyph))
        .padding([6, 8])
        .style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(event(Event::EditField(Some(target)))))
        .into()
}

fn properties<'a>(
    app: &'a App,
    task: &'a Task,
    project: &str,
    project_name: &str,
    workflow: Option<&TaskWorkflow>,
) -> Column<'a, Message> {
    use desktop_ui_controls::text_editor_context_menu::TextContextEffect;

    let i18n = I18n::new(app.general_settings.desktop_locale);
    let state = &app.task_board.detail;
    let editable = !task.archived && !task.deleted && !state.write_pending();
    let data = state.data.as_ref();
    let assignee = data.and_then(|detail| detail.assignee.as_deref()).unwrap_or(&task.assignee);
    let assignee = if assignee.trim().is_empty() {
        i18n.__("未分配")
    } else {
        display_author(i18n, assignee)
    };
    let priority = data.and_then(|detail| detail.priority).unwrap_or(task.priority);
    let workflow_name =
        workflow.map_or_else(|| "—".into(), |workflow| i18n.__(workflow.label()).to_owned());
    let workflow_link = button(text(workflow_name).size(13))
        .padding([6, 8])
        .style(rounded_action_btn_style)
        .on_press(Message::View(crate::app::message::ViewMessage::OpenWorkflowManager));
    let mut fields = column![
        field(i18n.__("状态"), status_picker(i18n, task, workflow)),
        field(i18n.__("负责人"), edit_property(assignee, Field::Assignee, editable)),
        field(i18n.__("项目"), text(project_name.to_owned()).size(13)),
        field(i18n.__("工作流"), workflow_link),
        field(i18n.__("优先级"), edit_property(format!("P{priority}"), Field::Priority, editable)),
        field(
            i18n.__("模型"),
            text(desktop_task_board::model_selector::model_display_label(i18n, &task.model))
                .size(13)
        ),
    ]
    .spacing(9)
    .width(Length::Fill);
    if let Some(detail) = data {
        for (label, date, target) in [
            ("开始日期", detail.collaboration.start_date.as_deref(), Field::StartDate),
            ("截止日期", detail.collaboration.due_date.as_deref(), Field::DueDate),
        ] {
            if let Some(date) = date {
                fields = fields.push(field(i18n.__(label), edit_property(date, target, editable)));
            }
        }
    }
    let labels =
        app.task_board.task_labels.iter().filter(|label| task.label_ids.contains(&label.id));
    let mut label_row = row(labels.map(task_label_badge)).spacing(4);
    if !app.task_board.task_labels.is_empty() {
        label_row = label_row.push(edit_property(i18n.__("标签"), Field::Labels, editable));
        fields = fields.push(label_row.wrap());
    }
    for (name, value) in super::properties::task_summary(app, task) {
        fields = fields.push(field(name, text(value).size(13).width(Length::Fill)));
    }
    fields = fields.push(
        button(row![icon(Icon::Plus), text(i18n.__("添加属性")).size(13)].spacing(6))
            .padding([6, 8])
            .style(rounded_action_btn_style)
            .on_press_maybe(editable.then_some(event(Event::EditField(Some(Field::Properties))))),
    );
    let personal =
        app.task_board.personal_tasks.get(project).is_some_and(|ids| ids.contains(&task.id));
    let actions = column![heading(i18n.__("快捷操作"), 15)].spacing(7).push(action(
        Icon::CheckSquare,
        i18n.__(if personal { "移出我的任务" } else { "加入我的任务" }),
        TaskBoardMessage::TogglePersonalTask(task.id.clone()),
    ));
    #[cfg(not(target_arch = "wasm32"))]
    let actions = actions.extend(super::modals::quick_actions::entry(app, &task.id));

    let mut pull_requests = column![
        row![
            heading("Pull Request", 15).width(Length::Fill),
            button(row![icon(Icon::Plus), text(i18n.__("关联 PR")).size(12)].spacing(6))
                .padding([6, 8])
                .style(rounded_action_btn_style)
                .on_press_maybe(
                    (editable && data.is_some())
                        .then_some(event(Event::EditField(Some(Field::PullRequest))))
                ),
        ]
        .align_y(Alignment::Center)
    ]
    .spacing(7);
    if let Some(detail) = data {
        for link in &detail.collaboration.pull_requests {
            let open = crate::app::components::text_editor_context_menu::text_context_message(
                TextContextEffect::OpenUrl(link.clone()),
            );
            pull_requests = pull_requests.push(
                row![
                    button(
                        row![
                            icon(Icon::Link),
                            text(link)
                                .size(12)
                                .width(Length::Fill)
                                .wrapping(text::Wrapping::WordOrGlyph)
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                    )
                    .width(Length::Fill)
                    .padding([6, 8])
                    .style(rounded_action_btn_style)
                    .on_press(open),
                    button(text(i18n.__("解除关联")).size(12))
                        .padding([6, 8])
                        .style(rounded_action_btn_style)
                        .on_press_maybe(
                            editable.then(|| event(Event::UnlinkPullRequest(link.clone())))
                        ),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }
    }

    let logs = action(
        Icon::Journals,
        i18n.__("查看日志"),
        TaskBoardMessage::SelectDetailTab(TaskBoardDetailTab::Logs),
    );
    let mut execution = column![
        heading(i18n.__("执行日志"), 15),
        logs,
        action(
            Icon::ChevronRight,
            i18n.__("查看执行过程"),
            TaskBoardMessage::Detail(Event::ShowExecution)
        ),
    ]
    .spacing(8);
    if let Some(duration_ms) = task.display_execution_duration_ms(runtime_time::now_ms()) {
        let duration = display_format::format_duration((duration_ms / 1000) as i64);
        let duration = if duration.is_empty() { "0s".into() } else { duration };
        execution = execution.push(field(i18n.__("执行时长"), text(duration).size(13)));
    }
    if let Some(branch) =
        task.selected_worktree_branch.as_deref().filter(|branch| !branch.is_empty())
    {
        execution = execution.push(field(i18n.__("工作树"), text(branch).size(13)));
    }
    let mut details = column![heading(i18n.__("详情"), 15)].spacing(9);
    if let Some(detail) = data {
        details = details
            .push(field(
                i18n.__("创建者"),
                text(detail.created_by.as_deref().map_or("—", |id| display_author(i18n, id)))
                    .size(13)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ))
            .push(field(
                i18n.__("创建时间"),
                text(time(u64::try_from(detail.created_at_ms.0).unwrap_or_default())).size(13),
            ))
            .push(field(
                i18n.__("更新时间"),
                text(time(
                    task.updated_at_ms
                        .max(u64::try_from(detail.updated_at_ms.0).unwrap_or_default()),
                ))
                .size(13),
            ));
    }
    column![
        fields,
        settings_divider(),
        actions,
        settings_divider(),
        pull_requests,
        settings_divider(),
        execution,
        settings_divider(),
        details,
    ]
    .spacing(14)
    .width(Length::Fill)
}

pub(super) fn with_field_modal<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    use desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape;
    let state = &app.task_board.detail;
    let Some((field, task)) = state.editor.zip(app.task_board.task_board_viewing_logs.as_ref())
    else {
        return base;
    };
    if state.task_id != task.id
        || app.navigation_tabs.project_path.as_deref() != Some(state.project.as_str())
        || app.navigation_panels.show_system_settings
    {
        return base;
    }
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let close = event(Event::EditField(None));
    let (title, placeholder) = match field {
        Field::Assignee => ("负责人", "输入负责人"),
        Field::Priority => ("优先级", "输入数字"),
        Field::StartDate => ("开始日期", "YYYY-MM-DD"),
        Field::DueDate => ("截止日期", "YYYY-MM-DD"),
        Field::PullRequest => ("关联 PR", "粘贴 PR 链接"),
        Field::Labels => ("标签", ""),
        Field::Properties => ("添加属性", ""),
        Field::Attachment => ("添加附件链接", "粘贴链接"),
    };
    let header =
        settings_modal_header(heading(i18n.__(title), 17), settings_close_button(close.clone()));
    let card = match field {
        Field::Attachment => {
            let valid = crate::app::message::task_board::attachment_link(&state.input).is_some();
            let mut input = settings_text_input(i18n.__(placeholder), &state.input)
                .id(ATTACHMENT_INPUT_ID)
                .width(Length::Fill)
                .on_input(|value| event(Event::FieldInput(value)));
            if valid {
                input = input.on_submit(event(Event::AddAttachment));
            }
            let mut body = column![header, input].spacing(12);
            if !valid && !state.input.trim().is_empty() {
                body =
                    body.push(settings_error_banner(i18n.__("请输入有效的 HTTP 或 HTTPS 链接。")));
            }
            settings_modal_card(body.push(row![
                    Space::new().width(Length::Fill),
                    button(text(i18n.__("添加")).size(13))
                        .padding([7, 12])
                        .style(primary_action_btn_style)
                        .on_press_maybe(valid.then_some(event(Event::AddAttachment)))
                ]))
        }
        Field::Properties => {
            let body = super::properties::task_editor(app, task)
                .unwrap_or_else(|| muted(i18n.__("请在任务设置中创建属性。")).into());
            let choices = row![
                edit_property(i18n.__("开始日期"), Field::StartDate, state.data.is_some()),
                edit_property(i18n.__("截止日期"), Field::DueDate, state.data.is_some()),
                edit_property(
                    i18n.__("标签"),
                    Field::Labels,
                    !app.task_board.task_labels.is_empty()
                ),
            ]
            .spacing(6)
            .wrap();
            let settings = button(text(i18n.__("管理任务属性")).size(13))
                .padding([7, 12])
                .style(rounded_action_btn_style)
                .on_press(Message::View(crate::app::message::ViewMessage::OpenSystemSettingsTab(
                    desktop_settings_shell::SystemTab::TaskProperties,
                )));
            settings_modal_layout(header, column![choices, body].spacing(12), Some(settings.into()))
                .height(350)
        }
        Field::Labels => {
            settings_modal_card(column![header, super::panel::task_labels(app, task)].spacing(14))
        }
        _ => {
            let pending = state.write_pending();
            let valid = match field {
                Field::Priority => state.input.trim().parse::<u32>().is_ok(),
                Field::PullRequest => !state.input.trim().is_empty(),
                _ => true,
            };
            let mut input =
                settings_text_input(i18n.__(placeholder), &state.input).width(Length::Fill);
            if !pending {
                input = input.on_input(|value| event(Event::FieldInput(value)));
                if valid {
                    input = input.on_submit(event(Event::SaveField));
                }
            }
            let mut footer = row![].spacing(8).align_y(Alignment::Center);
            if matches!(field, Field::Assignee | Field::StartDate | Field::DueDate) {
                footer = footer.push(
                    button(text(i18n.__("清空")).size(13))
                        .padding([7, 12])
                        .style(rounded_action_btn_style)
                        .on_press_maybe((!pending).then_some(event(Event::ClearField))),
                );
            }
            let save_label = if field == Field::PullRequest { "关联" } else { "保存" };
            footer = footer.push(Space::new().width(Length::Fill)).push(
                button(text(i18n.__(save_label)).size(13))
                    .padding([7, 12])
                    .style(primary_action_btn_style)
                    .on_press_maybe((!pending && valid).then_some(event(Event::SaveField))),
            );
            let mut body = column![header, input].spacing(14);
            if field == Field::Assignee {
                if let Some(viewer) = app.task_board.work_viewer_id.as_ref() {
                    body = body.push(
                        button(text(i18n.__("分配给我")).size(13))
                            .padding([6, 8])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!pending).then(|| event(Event::FieldInput(viewer.clone()))),
                            ),
                    );
                }
                let query = state.input.trim().to_lowercase();
                let names = app
                    .task_board
                    .task_board_tasks
                    .iter()
                    .map(|task| task.assignee.as_str())
                    .chain(app.agents_settings.entries.iter().map(|entry| entry.label.as_str()))
                    .filter(|name| {
                        !name.trim().is_empty()
                            && (name.to_lowercase().contains(&query)
                                || *name == task.assignee.as_str())
                    })
                    .collect::<std::collections::BTreeSet<_>>();
                body = body.push(
                    row(names.into_iter().take(10).map(|name| {
                        button(text(name).size(13))
                            .padding([6, 8])
                            .style(rounded_action_btn_style)
                            .on_press_maybe(
                                (!pending).then(|| event(Event::FieldInput(name.to_owned()))),
                            )
                            .into()
                    }))
                    .spacing(6)
                    .wrap(),
                );
            }
            settings_modal_card(body.push(footer))
        }
    };
    settings_modal_overlay_with_escape(base, close, card.width(Length::Fill).max_width(460))
}
