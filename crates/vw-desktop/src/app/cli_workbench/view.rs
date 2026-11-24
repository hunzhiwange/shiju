use desktop_assets::assets::Icon;
use desktop_terminal::terminal_panel::{TAB_HEIGHT, TAB_TOOLBAR_BUTTON_WIDTH};
use desktop_ui_controls::system_settings_common::*;
use desktop_ui_controls::system_settings_common::icons::{icon_btn_style, icon_btn_svg_style};
use desktop_ui_controls::system_settings_common::tabs::settings_compact_tab_button;
use gateway_interaction::PendingPermissionReplyDto;
use iced::{
    Alignment, Border, Element, Length, Theme,
    widget::{
        Column, Space, button, column, container, responsive, row, scrollable, svg, text,
        text::Wrapping, text_input, tooltip,
    },
    window,
};
use vw_api_types::cli_workbench::{
    CliWorkbenchDraftAction, CliWorkbenchMessageState as Delivery,
    CliWorkbenchProcessState as Process, CliWorkbenchTarget,
};

use super::{
    App, Dialog, Message, Project, SessionKey, Tab, Workbench,
    update::{Action, draft_text, event},
};
use crate::app::i18n::I18n;

pub(super) const DIALOG_INPUT: &str = "cli-workbench-dialog-input";

#[derive(Clone, Copy)]
struct View {
    window: window::Id,
    i18n: I18n,
    locale: config_ui::DesktopLocale,
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
    let locale = app.general_settings.desktop_locale;
    let ui = View { window: id, i18n: I18n::new(locale), locale };
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
        let base = column![].spacing(6);
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
            Some(Dialog::ReadDraft(key, request)) => draft_dialog(ui, base, state, key, request),
            Some(dialog) => dialog_view(
                ui,
                base,
                dialog,
                selected.map(|(_, project)| project),
                state.terminal_available,
            ),
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
    let mut list = column![row![
        text(ui.label("项目")).size(13).style(settings_muted_text_style).width(Length::Fill),
        icon_btn(
            Icon::ArrowClockwise,
            ui.label("刷新"),
            (!state.discovering).then_some(ui.msg(Action::Refresh)),
        ),
    ].align_y(Alignment::Center)].spacing(5);
    for (id, project) in &state.projects {
        list = list.push(
            settings_compact_tab_button(
                project.name.as_str(),
                state.selected.as_ref() == Some(id),
                ui.msg(Action::SelectProject(id.clone())),
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
            let key = SessionKey { project: project_id.clone(), session: session.clone() };
            if tab.session.state == Process::WaitingForApproval {
                pending = true;
                list = list.push(
                    control(
                        format!("{} · {}", project.name, tab_label(tab)),
                        Some(ui.msg(Action::SelectSession(key.clone()))),
                        false,
                    )
                    .width(Length::Fill),
                );
            }
            for draft in &tab.drafts {
                pending = true;
                list = list.push(
                    control(
                        format!(
                            "{} · {} · {} #{}",
                            project.name,
                            tab_label(tab),
                            ui.label("待提交"),
                            id_suffix(&draft.id.request_id),
                        ),
                        Some(ui.msg(Action::ShowDraft(key.clone(), draft.id.request_id.clone()))),
                        false,
                    )
                    .width(Length::Fill),
                );
            }
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

#[derive(Clone, PartialEq)]
pub(super) struct LaunchChoice {
    pub(super) target: CliWorkbenchTarget,
    label: String,
}

impl std::fmt::Display for LaunchChoice {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub(super) fn launch_choices(state: &Workbench, i18n: I18n) -> Vec<LaunchChoice> {
    let mut choices = Vec::new();
    if state.terminal_available {
        choices.push(LaunchChoice {
            target: CliWorkbenchTarget::Shell,
            label: i18n.__("新建 Shell").into(),
        });
        let mut apps: Vec<_> = state.native_apps.iter()
            .map(|app| (app.id.as_str(), app.name.as_str()))
            .collect();
        if !apps.iter().any(|(id, _)| *id == super::CODEX_APP_ID) {
            apps.push((super::CODEX_APP_ID, "Codex"));
        }
        apps.sort_by_key(|(id, _)| state.native_app.as_deref() != Some(*id));
        choices.extend(apps.into_iter().map(|(id, name)| LaunchChoice {
            target: CliWorkbenchTarget::NativeCli { app_id: id.into() },
            label: format!("{name} · {}", i18n.__("终端")),
        }));
    }
    choices.extend(state.agents.iter()
        .filter(|agent| !state.terminal_available || agent.matching_backend(Some(super::CODEX_APP_ID)).is_none())
        .map(|agent| LaunchChoice {
            target: CliWorkbenchTarget::Acp { agent: agent.key.clone() },
            label: format!("{} · {}", agent.label(), i18n.__("ACP 对话")),
        }));
    choices
}

fn workspace<'a>(
    ui: View,
    state: &'a Workbench,
    project_id: &'a str,
    project: &'a Project,
) -> Element<'a, Message> {
    let active = project.active_tab();
    let sendable = |tab: &Tab| {
        tab.can_send()
            && !project.disconnected
            && (!tab.session.capabilities.terminal || state.terminal_available)
    };
    let active_target = active.filter(|tab| sendable(tab)).map(|tab| tab.session.id.clone());
    let can_start = !state.starting && !project.disconnected;
    let choices = launch_choices(state, ui.i18n);
    let empty_label = choices.is_empty().then(|| {
        if state.agents_loading {
            ui.label("正在读取 ACP 智能体…")
        } else {
            ui.label("此项目暂无可用的 ACP 智能体")
        }
    });
    use desktop_terminal::{message::{Message as Panel, TerminalMessage as Term}, terminal_panel};
    let layout = project.active_layout;
    let on_panel = move |message| ui.msg(Action::Panel(project_id.into(), layout, message));
    let focus = active.and_then(|tab| tab.terminal.view.as_ref()).map(|view| view.terminal().widget_id());
    let grid = terminal_panel::shared_grid(
        &project.panes,
        state.dialog.is_none().then_some(focus).flatten(),
        move |event| on_panel(Panel::Terminal(event)),
        move |pane, pane_state, top_right| {
            let current = pane_state.active_id.and_then(|id| project.tab_for_view(id));
            let launch = settings_select(choices.iter().cloned(), None, move |choice| {
                ui.msg(Action::Start(project_id.into(), layout, choice.target, pane))
            })
            .menu_icon(Icon::Plus, 220.0)
            .width(TAB_TOOLBAR_BUTTON_WIDTH)
            .height(TAB_HEIGHT)
            .disabled(!can_start);
            let mut controls = row![toolbar_tooltip(launch.into(), ui.label("新建会话"))]
                .align_y(Alignment::Center);
            if super::update::split_target(state, project, pane).is_some() {
                let split = terminal_panel::split_control(&project.panes, ui.locale, pane).map(on_panel);
                controls = controls.push(if project.panes.split_menu == Some(pane) {
                    split
                } else {
                    toolbar_tooltip(split, ui.label("分屏"))
                });
            }
            if let Some(tab) = current {
                let key = SessionKey { project: project_id.into(), session: tab.session.id.clone() };
                let can_stop = !project.disconnected && tab.can_stop()
                    && (!tab.session.capabilities.terminal || state.terminal_available);
                controls = controls.push(toolbar_icon(
                    Icon::Square,
                    ui.label("停止会话"),
                    0.0,
                    can_stop.then_some(ui.msg(Action::Stop(key))),
                ));
            }
            if top_right {
                controls = controls.push(toolbar_icon(
                    Icon::ArrowUp,
                    ui.label("发送消息"),
                    3.0,
                    project.tabs.values().any(sendable)
                        .then(|| ui.msg(Action::Compose(active_target.clone()))),
                ));
            }
            let header = terminal_panel::pane_header(
                pane_state.tabs.iter().filter_map(|id| project.tab_for_view(*id).map(|tab| (*id, tab_label(tab)))),
                pane_state.active_id,
                true,
                project.tab_menu.map(|(id, _, _)| id),
                project.tab_menu.map(|(_, x, y)| (x, y)),
                ui.locale,
                controls,
                on_panel,
            );
            let body = if let Some(tab) = current {
                let key = SessionKey { project: project_id.into(), session: tab.session.id.clone() };
                session_content(
                    ui, &key, tab,
                    project.disconnected || (tab.session.capabilities.terminal && !state.terminal_available),
                    project.content_menu,
                    layout,
                )
            } else {
                container(text(ui.label("暂无会话")).size(15)).center(Length::Fill).into()
            };
            (header, terminal_panel::pane_surface(body))
        },
    );
    let mut workspace = column![layout_tabs(ui, project_id, project), settings_divider()];
    if let Some(label) = empty_label {
        workspace = workspace.push(text(label).size(12).style(settings_muted_text_style));
    }
    // The shared grid leaves a one-pixel gap between its opaque pane surfaces.
    let grid = container(grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(settings_divider_style);
    workspace.push(grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn layout_tabs<'a>(ui: View, project_id: &str, project: &Project) -> Element<'a, Message> {
    let mut tabs = row![].spacing(4).align_y(Alignment::Center);
    let can_close = !project.parked_layouts.is_empty();
    for id in project.layout_ids() {
        let selected = id == project.active_layout;
        let label = ui.i18n.sprintf("分屏组 %u", &[id.into()]);
        let tab = button(
            row![
                icon_svg(Icon::Terminal, 13.0).style(move |theme: &Theme, _| svg::Style {
                    color: Some(layout_tab_foreground(theme, selected)),
                }),
                text(label).size(13),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .style(move |theme, status| {
            let mut style = icon_btn_style(theme, status);
            style.text_color = layout_tab_foreground(theme, selected);
            style.border = Border::default().rounded(5);
            style
        })
        .on_press(ui.msg(Action::SelectLayout(project_id.into(), id)));
        let mut item = row![tab].align_y(Alignment::Center);
        if can_close {
            item = item.push(toolbar_icon(
                Icon::X,
                ui.label("关闭分屏组"),
                0.0,
                Some(ui.msg(Action::CloseLayout(project_id.into(), id))),
            ));
        }
        tabs = tabs.push(container(item).padding(2).style(move |theme| {
            let background = theme.extended_palette().background;
            container::Style {
                background: selected.then(|| background.weak.color.into()),
                border: if selected {
                    Border::default().rounded(7).width(1).color(background.strong.color)
                } else {
                    Border::default()
                },
                ..Default::default()
            }
        }));
    }
    container(
        row![
            scrollable(tabs)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                ))
                .width(Length::Fill),
            toolbar_icon(
                Icon::Plus,
                ui.label("新建分屏组"),
                0.0,
                Some(ui.msg(Action::NewLayout(project_id.into()))),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([5, 7])
    .width(Length::Fill)
    .style(|theme| container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    })
    .into()
}

fn layout_tab_foreground(theme: &Theme, selected: bool) -> iced::Color {
    theme.palette().text.scale_alpha(if selected { 1.0 } else { 0.72 })
}

fn session_content<'a>(
    ui: View,
    key: &SessionKey,
    tab: &'a Tab,
    disconnected: bool,
    content_menu: Option<(u64, iced::Point)>,
    layout: u64,
) -> Element<'a, Message> {
    if tab.session.capabilities.terminal {
        let terminal = if let Some(view) = &tab.terminal.view {
            use desktop_terminal::message::{Message as Panel, TerminalMessage as Term};
            let key = key.clone();
            desktop_terminal::terminal_panel::terminal_content(
                view.terminal(), tab.view_id, ui.locale, content_menu, false,
            ).map(move |message| match message {
                Panel::Terminal(Term::Event(event)) => ui.msg(Action::TerminalEvent(key.clone(), event)),
                message => ui.msg(Action::Panel(key.project.clone(), layout, message)),
            })
        } else {
            let label = if !disconnected && tab.session.state == Process::Running {
                "正在连接终端…"
            } else {
                "终端内容暂不可用"
            };
            container(text(ui.label(label)).size(13)).center(Length::Fill).into()
        };
        let mut content = column![].spacing(4);
        if disconnected || tab.session.state == Process::Disconnected {
            content = content.push(settings_error_banner(ui.label("终端连接已断开").to_owned()));
        }
        if let Some(error) = tab.action_error.or(tab.terminal.error) {
            if !disconnected || error != "终端连接已断开" {
                content = content.push(settings_error_banner(ui.label(error).to_owned()));
            }
        }
        if tab.terminal.truncated {
            content = content
                .push(text(ui.label("终端回放已截断")).size(12).style(settings_muted_text_style));
        }
        if !tab.drafts.is_empty() {
            content = content.push(control(
                ui.label("查看待提交草稿"),
                Some(ui.msg(Action::ShowDraft(key.clone(), tab.drafts[0].id.request_id.clone()))),
                false,
            ));
        }
        if let Some(turn) = tab.turns.last() {
            let status = turn
                .error
                .or_else(|| {
                    turn.snapshot.as_ref().and_then(|snapshot| {
                        snapshot
                            .error
                            .map(super::update::error_label)
                            .or(Some(delivery_label(snapshot.state)))
                    })
                })
                .unwrap_or("正在发送消息…");
            content =
                content.push(text(ui.label(status)).size(12).style(settings_muted_text_style));
        }
        return content
            .push(container(terminal).width(Length::Fill).height(Length::Fill))
            .height(Length::Fill)
            .into();
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

fn toolbar_icon(
    icon: Icon,
    label: &'static str,
    background_inset: f32,
    action: Option<Message>,
) -> Element<'static, Message> {
    let icon = container(icon_svg(icon, 14.0).style(icon_btn_svg_style))
        .center_x(TAB_TOOLBAR_BUTTON_WIDTH - 2.0 * background_inset)
        .center_y(TAB_HEIGHT - 2.0 * background_inset);
    let control = button(icon).padding(0).style(icon_btn_style).on_press_maybe(action);
    let slot = container(control).center_x(TAB_TOOLBAR_BUTTON_WIDTH).center_y(TAB_HEIGHT);
    toolbar_tooltip(slot.into(), label)
}

fn toolbar_tooltip<'a>(control: Element<'a, Message>, label: &'static str) -> Element<'a, Message> {
    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .gap(6)
        .padding(7)
        .style(desktop_ui_controls::tooltip::tooltip_dark_style)
        .into()
}

fn dialog_view<'a>(
    ui: View,
    base: Element<'a, Message>,
    dialog: &'a Dialog,
    project: Option<&'a Project>,
    terminal_available: bool,
) -> Element<'a, Message> {
    let close = ui.msg(Action::Dismiss);
    let confirm = ui.msg(Action::Confirm);
    let (title, draft, enabled, select) = match dialog {
        Dialog::Rename(_, draft) => (ui.label("重命名"), draft, !draft.trim().is_empty(), None),
        Dialog::ReadDraft(..) => return base,
        Dialog::Compose { target, draft } => {
            let targets: Vec<_> = project
                .into_iter()
                .flat_map(|project| {
                    project.tabs.iter().filter(move |(_, tab)| {
                        !project.disconnected
                            && tab.can_send()
                            && (!tab.session.capabilities.terminal || terminal_available)
                    })
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

fn draft_dialog<'a>(
    ui: View,
    base: Element<'a, Message>,
    state: &'a Workbench,
    key: &SessionKey,
    request: &str,
) -> Element<'a, Message> {
    let close = ui.msg(Action::Dismiss);
    let Some(draft) = draft_text(state, key, request) else { return base };
    let enabled = state.terminal_available
        && state.projects.get(&key.project).is_some_and(|project| {
            !project.disconnected
                && project.tabs.get(&key.session).is_some_and(|tab| tab.resolving_draft.is_none())
        });
    let resolve =
        |action| enabled.then(|| ui.msg(Action::ResolveDraft(key.clone(), request.into(), action)));
    let card = settings_modal_card(
        column![
            row![
                text(ui.label("待提交草稿")).size(17).width(Length::Fill),
                settings_close_button(close.clone()),
            ]
            .align_y(Alignment::Center),
            text(ui.label("将草稿粘贴到目标终端，按 Enter 提交。"))
                .size(13)
                .wrapping(Wrapping::Glyph),
            container(
                vertical(column![text(draft).size(14).wrapping(Wrapping::Glyph)])
                    .height(Length::Shrink)
            )
            .max_height(280),
            row![
                control(
                    ui.label("复制草稿"),
                    Some(ui.msg(Action::CopyDraft(key.clone(), request.into()))),
                    false
                ),
                control(ui.label("撤销草稿"), resolve(CliWorkbenchDraftAction::Cancel), false),
                control(
                    ui.label("标记已手动提交"),
                    resolve(CliWorkbenchDraftAction::MarkSubmitted),
                    false
                ),
                control(
                    ui.label("粘贴到终端"),
                    enabled.then(|| ui.msg(Action::PasteDraft(key.clone(), request.into()))),
                    true
                ),
            ]
            .spacing(6)
            .wrap(),
        ]
        .spacing(12),
    )
    .width(Length::Fill)
    .max_width(560);
    settings_modal_overlay_with_escape(base, close, card)
}

pub(super) fn tab_label(tab: &Tab) -> String {
    if let Some(name) = &tab.name {
        return name.clone();
    }
    let name = match &tab.session.target {
        CliWorkbenchTarget::Acp { agent } => agent,
        CliWorkbenchTarget::NativeCli { app_id } if app_id == super::CODEX_APP_ID => "Codex",
        CliWorkbenchTarget::NativeCli { app_id } => app_id,
        CliWorkbenchTarget::Shell => "Shell",
    };
    name.to_owned()
}

fn id_suffix(id: &str) -> &str {
    id.char_indices().rev().nth(5).map_or(id, |(start, _)| &id[start..])
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
