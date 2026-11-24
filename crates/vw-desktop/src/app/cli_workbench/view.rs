use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::*;
use gateway_interaction::PendingPermissionReplyDto;
use iced::{
    Alignment, Element, Length,
    widget::{
        Column, Space, button, column, container, pane_grid, responsive, row, scrollable, text,
        text::Wrapping, text_input,
    },
    window,
};
use vw_api_types::cli_workbench::{
    CliWorkbenchMessageState as Delivery, CliWorkbenchProcessState as Process, CliWorkbenchTarget,
};

use super::{
    App, Dialog, Message, Project, SessionKey, Tab, Workbench,
    update::{Action, event},
};
use crate::app::i18n::I18n;

pub(super) const DIALOG_INPUT: &str = "cli-workbench-dialog-input";

#[derive(Clone, Copy)]
struct View {
    window: window::Id,
    i18n: I18n,
}

impl View {
    fn msg(self, action: Action) -> Message {
        event(self.window, action)
    }

    fn label(self, source: &'static str) -> &'static str {
        self.i18n.__(source)
    }
}

pub(crate) fn view(app: &App, id: window::Id) -> Element<'_, Message> {
    let ui = View { window: id, i18n: I18n::new(app.general_settings.desktop_locale) };
    let state = &app.cli_workbench;
    responsive(move |size| {
        let selected = state
            .selected
            .as_ref()
            .and_then(|id| state.projects.get(id).map(|project| (id, project)));
        let sidebar = sidebar(ui, state);
        let main = match selected {
            Some((id, project)) => workspace(ui, state, id, project),
            None => container(text(ui.label("请选择项目")).size(16)).center(Length::Fill).into(),
        };
        let content: Element<'_, Message> = if size.width < 680.0 {
            column![container(sidebar).height(146), main].spacing(8).into()
        } else {
            row![container(sidebar).width(208).height(Length::Fill), main].spacing(8).into()
        };
        let error = state.error.or_else(|| {
            selected.and_then(|(_, project)| {
                project
                    .disconnected
                    .then_some("工作台与网关断开连接")
                    .or(project.approval_error.then_some("无法读取工作台审批"))
            })
        });
        let base = column![
            row![
                text(ui.label("CLI 工作台")).size(17).width(Length::Fill),
                icon_btn(
                    Icon::ArrowClockwise,
                    ui.label("刷新"),
                    (!state.discovering).then_some(ui.msg(Action::Refresh))
                ),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8);
        let base = if let Some(error) = error {
            base.push(settings_error_banner(ui.label(error).to_owned()))
        } else {
            base
        };
        let base = container(base.push(content))
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        match &state.dialog {
            Some(dialog) => dialog_view(ui, base, dialog, selected.map(|(_, project)| project)),
            None => base,
        }
    })
    .into()
}

fn control<'a>(
    label: impl Into<String>,
    action: Option<Message>,
    primary: bool,
) -> button::Button<'a, Message> {
    button(text(label.into()).size(13).wrapping(Wrapping::Glyph))
        .padding([6, 9])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(action)
}

fn vertical<'a>(content: Column<'a, Message>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(content).padding([2, 7]).width(Length::Fill)).direction(
        scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    )
}

fn sidebar<'a>(ui: View, state: &'a Workbench) -> Element<'a, Message> {
    let mut list =
        column![text(ui.label("项目")).size(13).style(settings_muted_text_style)].spacing(5);
    for (id, project) in &state.projects {
        list = list.push(
            control(
                &project.name,
                Some(ui.msg(Action::SelectProject(id.clone()))),
                state.selected.as_ref() == Some(id),
            )
            .width(Length::Fill),
        );
    }
    if state.discovering && state.projects.is_empty() {
        list = list.push(text(ui.label("正在读取工作台项目…")).size(13));
    } else if state.projects.is_empty() {
        list = list.push(text(ui.label("暂无项目")).size(13));
    }
    list = list
        .push(settings_divider())
        .push(text(ui.label("待处理")).size(13).style(settings_muted_text_style));
    let mut pending = false;
    for (project_id, project) in &state.projects {
        for (session, tab) in &project.tabs {
            if tab.session.state != Process::WaitingForApproval {
                continue;
            }
            pending = true;
            let key = SessionKey { project: project_id.clone(), session: session.clone() };
            list = list.push(
                control(
                    format!("{} · {}", project.name, tab_label(tab)),
                    Some(ui.msg(Action::SelectSession(key))),
                    false,
                )
                .width(Length::Fill),
            );
        }
    }
    if !pending {
        list =
            list.push(text(ui.label("暂无待处理事项")).size(12).style(settings_muted_text_style));
    }
    if let Some(project_id) = &state.selected
        && let Some(project) = state.projects.get(project_id)
        && project.tabs.values().any(|tab| tab.hidden)
    {
        list = list
            .push(settings_divider())
            .push(text(ui.label("已关闭的标签")).size(13).style(settings_muted_text_style));
        for (session, tab) in project.tabs.iter().filter(|(_, tab)| tab.hidden) {
            let key = SessionKey { project: project_id.clone(), session: session.clone() };
            list = list.push(
                control(tab_label(tab), Some(ui.msg(Action::SelectSession(key))), false)
                    .width(Length::Fill),
            );
        }
    }
    container(vertical(list).height(Length::Fill))
        .style(settings_panel_style)
        .padding([7, 2])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn workspace<'a>(
    ui: View,
    state: &'a Workbench,
    project_id: &'a str,
    project: &'a Project,
) -> Element<'a, Message> {
    let active = project.panes.get(project.active_pane).and_then(|session| session.as_ref());
    let sendable = |tab: &Tab| tab.can_send() && !project.disconnected;
    let active_target = active.filter(|id| project.tabs.get(*id).is_some_and(sendable)).cloned();
    let mut tools = row![control(
        ui.label("发送消息"),
        project.tabs.values().any(sendable).then_some(ui.msg(Action::Compose(active_target))),
        false,
    ),]
    .spacing(6)
    .align_y(Alignment::Center);
    if state.agents_loading {
        tools = tools
            .push(text(ui.label("正在读取 ACP 智能体…")).size(12).style(settings_muted_text_style));
    } else if state.agents.is_empty() {
        tools = tools.push(
            text(ui.label("此应用暂无可用的 ACP 智能体")).size(12).style(settings_muted_text_style),
        );
    } else {
        let select = settings_select(
            state.agents.iter().map(|agent| agent.key.clone()),
            state.agent.clone(),
            move |key| ui.msg(Action::SelectAgent(key)),
        )
        .format_labels(|key| {
            state
                .agents
                .iter()
                .find(|agent| &agent.key == key)
                .map_or_else(|| key.clone(), |agent| agent.label().to_owned())
        })
        .placeholder(ui.label("选择 ACP 智能体"))
        .width(180)
        .height(32.0);
        tools = tools.push(select).push(control(
            ui.label(if state.starting { "正在创建会话…" } else { "新建 ACP 会话" }),
            (!state.starting && !project.disconnected && state.agent.is_some())
                .then_some(ui.msg(Action::Start)),
            true,
        ));
    }
    if project.panes.len() == 1 && project.split_candidate().is_some() {
        tools = tools
            .push(control(
                ui.label("左右分屏"),
                Some(ui.msg(Action::Split(project_id.into(), pane_grid::Axis::Vertical))),
                false,
            ))
            .push(control(
                ui.label("上下分屏"),
                Some(ui.msg(Action::Split(project_id.into(), pane_grid::Axis::Horizontal))),
                false,
            ));
    }
    let tabs = project.tabs.iter().filter(|(_, tab)| !tab.hidden).fold(
        row![].spacing(5),
        |row, (id, tab)| {
            let key = SessionKey { project: project_id.into(), session: id.clone() };
            let status = process_label(tab, project.disconnected);
            row.push(control(
                format!("{} · {}", tab_label(tab), ui.label(status)),
                Some(ui.msg(Action::SelectSession(key))),
                active == Some(id),
            ))
        },
    );
    let tab_strip = scrollable(container(tabs).padding([3, 0]))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill);
    let grid = pane_grid::PaneGrid::new(&project.panes, move |pane, selected, _| {
        let Some((id, tab)) =
            selected.as_ref().and_then(|id| project.tabs.get(id).map(|tab| (id, tab)))
        else {
            return pane_grid::Content::new(
                container(text(ui.label("暂无会话")).size(15)).center(Length::Fill),
            );
        };
        let key = SessionKey { project: project_id.into(), session: id.clone() };
        let can_stop = !project.disconnected && tab.can_stop();
        let mut controls = row![
            control(ui.label("重命名"), Some(ui.msg(Action::Rename(key.clone()))), false),
            control(ui.label("关闭标签"), Some(ui.msg(Action::Hide(key.clone()))), false),
            control(
                ui.label("停止会话"),
                can_stop.then_some(ui.msg(Action::Stop(key.clone()))),
                false
            ),
        ]
        .spacing(5);
        if project.panes.len() > 1 {
            controls = controls.push(control(
                ui.label("关闭分屏"),
                Some(ui.msg(Action::ClosePane(project_id.into(), pane))),
                false,
            ));
        }
        let heading =
            column![text(tab_label(tab)).size(14).wrapping(Wrapping::Glyph), controls.wrap(),]
                .spacing(3);
        pane_grid::Content::new(session_content(ui, &key, tab, project.disconnected)).title_bar(
            pane_grid::TitleBar::new(heading).padding(6).style(move |theme| {
                let mut style = settings_panel_style(theme);
                if pane == project.active_pane {
                    style.border.color = theme.extended_palette().primary.strong.color;
                    style.border.width = 1.0;
                }
                style
            }),
        )
    })
    .spacing(6)
    .width(Length::Fill)
    .height(Length::Fill)
    .min_size(0)
    .on_click(move |pane| ui.msg(Action::Focus(project_id.into(), pane)))
    .on_drag(move |drag| ui.msg(Action::Drag(project_id.into(), drag)))
    .on_resize(8, move |resize| ui.msg(Action::Resize(project_id.into(), resize)));
    column![tools.wrap(), tab_strip, grid]
        .spacing(5)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn session_content<'a>(
    ui: View,
    key: &SessionKey,
    tab: &'a Tab,
    disconnected: bool,
) -> Element<'a, Message> {
    if tab.session.capabilities.terminal {
        // The native workbench integration mounts its attached PTY here, in the owning pane.
        return container(text(ui.label("终端内容暂不可用")).size(13)).center(Length::Fill).into();
    }
    let mut content = column![
        text(&tab.session.directory)
            .size(12)
            .wrapping(Wrapping::Glyph)
            .style(settings_muted_text_style),
        text(ui.label(process_label(tab, disconnected))).size(13),
    ]
    .spacing(8);
    if let Some(error) = tab.action_error {
        content = content.push(settings_error_banner(ui.label(error).to_owned()));
    }
    if let Some(approval) = &tab.approval {
        let title = approval
            .metadata
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or(&approval.permission);
        let arguments = approval
            .metadata
            .get("arguments")
            .map(|value| value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()));
        let mut body = column![
            text(ui.label("等待审批")).size(13),
            text(title).size(13).wrapping(Wrapping::Glyph),
        ]
        .spacing(6);
        if let Some(arguments) = arguments {
            body = body.push(text(arguments).size(12).wrapping(Wrapping::Glyph));
        }
        let active = !disconnected && tab.approving.is_none() && !tab.stopping;
        let buttons = row![
            control(
                ui.label("允许一次"),
                active.then_some(ui.msg(Action::Approve(
                    key.clone(),
                    approval.id.clone(),
                    PendingPermissionReplyDto::Once
                ))),
                true
            ),
            control(
                ui.label("拒绝"),
                active.then_some(ui.msg(Action::Approve(
                    key.clone(),
                    approval.id.clone(),
                    PendingPermissionReplyDto::Reject
                ))),
                false
            ),
        ]
        .spacing(6)
        .wrap();
        content = content.push(settings_panel(body.push(buttons)));
    }
    for turn in &tab.turns {
        let mut message = column![
            text(ui.label("你")).size(12).style(settings_muted_text_style),
            text(&turn.prompt).size(14).wrapping(Wrapping::Glyph),
        ]
        .spacing(5);
        if let Some(snapshot) = &turn.snapshot {
            message = message.push(
                text(ui.label(delivery_label(snapshot.state)))
                    .size(12)
                    .style(settings_muted_text_style),
            );
            if !snapshot.output.is_empty() {
                message = message.push(text(&snapshot.output).size(14).wrapping(Wrapping::Glyph));
            }
            if snapshot.output_truncated {
                message = message
                    .push(text(ui.label("输出已截断")).size(12).style(settings_muted_text_style));
            }
            if let Some(error) = snapshot.error {
                message = message.push(text(ui.label(super::update::error_label(error))).size(12));
            }
        } else if turn.recover {
            message = message
                .push(text(ui.label("正在发送消息…")).size(12).style(settings_muted_text_style));
        }
        if let Some(error) = turn.error {
            message = message.push(text(ui.label(error)).size(12).wrapping(Wrapping::Glyph));
        }
        content = content.push(settings_panel(message));
    }
    vertical(content).width(Length::Fill).height(Length::Fill).into()
}

fn dialog_view<'a>(
    ui: View,
    base: Element<'a, Message>,
    dialog: &'a Dialog,
    project: Option<&'a Project>,
) -> Element<'a, Message> {
    let close = ui.msg(Action::Dismiss);
    let confirm = ui.msg(Action::Confirm);
    let (title, draft, enabled, select) = match dialog {
        Dialog::Rename(_, draft) => (ui.label("重命名"), draft, !draft.trim().is_empty(), None),
        Dialog::Compose { target, draft } => {
            let targets: Vec<_> = project
                .into_iter()
                .flat_map(|project| {
                    project
                        .tabs
                        .iter()
                        .filter(move |(_, tab)| !project.disconnected && tab.can_send())
                })
                .map(|(id, _)| id.clone())
                .collect();
            let enabled =
                target.as_ref().is_some_and(|id| targets.contains(id)) && !draft.trim().is_empty();
            let select =
                settings_select(targets, target.clone(), move |id| ui.msg(Action::Target(id)))
                    .format_labels(|id| {
                        project
                            .and_then(|project| project.tabs.get(id))
                            .map_or_else(|| id.clone(), tab_label)
                    })
                    .placeholder(ui.label("选择目标会话"))
                    .width(Length::Fill);
            (ui.label("发送消息"), draft, enabled, Some(select))
        }
    };
    let input = text_input(title, draft)
        .id(DIALOG_INPUT)
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .on_input(move |value| ui.msg(Action::Draft(value)))
        .on_submit_maybe(enabled.then_some(confirm.clone()));
    let mut fields = column![].spacing(8);
    if let Some(select) = select {
        fields = fields.push(select);
    }
    let card = settings_modal_card(
        column![
            row![text(title).size(17).width(Length::Fill), settings_close_button(close.clone())]
                .align_y(Alignment::Center),
            vertical(fields.push(input)).height(Length::Shrink),
            row![
                Space::new().width(Length::Fill),
                control(ui.label("取消"), Some(close.clone()), false),
                control(ui.label("确认"), enabled.then_some(confirm), true),
            ]
            .spacing(6),
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .max_width(480);
    settings_modal_overlay_with_escape(base, close, card)
}

pub(super) fn tab_label(tab: &Tab) -> String {
    if let Some(name) = &tab.name {
        return name.clone();
    }
    let name = match &tab.session.target {
        CliWorkbenchTarget::Acp { agent } => agent,
        CliWorkbenchTarget::NativeCli { app_id } => app_id,
        CliWorkbenchTarget::Shell => "Shell",
    };
    let id = &tab.session.id;
    let suffix = id.char_indices().rev().nth(5).map_or(id.as_str(), |(start, _)| &id[start..]);
    format!("{name} #{suffix}")
}

fn process_label(tab: &Tab, disconnected: bool) -> &'static str {
    if disconnected {
        return "连接已断开";
    }
    if tab.stopping {
        return "正在停止";
    }
    match tab.session.state {
        Process::Ready => "就绪",
        Process::Running => "运行中",
        Process::WaitingForApproval => "等待审批",
        Process::Stopping => "正在停止",
        Process::Exited => "已退出",
        Process::Disconnected => "连接已断开",
    }
}

fn delivery_label(state: Delivery) -> &'static str {
    match state {
        Delivery::Accepted => "已接受",
        Delivery::PendingSubmission => "待提交",
        Delivery::Submitted => "已提交",
        Delivery::WaitingForApproval => "等待审批",
        Delivery::Completed => "已完成",
        Delivery::Failed => "失败",
    }
}
