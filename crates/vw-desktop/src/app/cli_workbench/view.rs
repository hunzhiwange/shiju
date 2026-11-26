use desktop_assets::assets::Icon;
use desktop_terminal::terminal_panel::{TAB_HEIGHT, TAB_TOOLBAR_BUTTON_WIDTH, tab_context_menu};
use desktop_ui_controls::overlays::point_below::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_ui_controls::system_settings_common::*;
use desktop_ui_controls::system_settings_common::icons::{icon_btn_style, icon_btn_svg_style};
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_workspace::views::project::components::session_row::running_session_indicator;
use gateway_interaction::PendingPermissionReplyDto;
use iced::{
    Alignment, Border, Element, Length, Theme,
    widget::{
        Column, Space, button, column, container, pane_grid, responsive, row, scrollable, svg, text,
        text::Wrapping, text_input, tooltip,
    },
    window,
};
use vw_api_acp_model::acp_model_source::AcpModelSource;
use vw_api_types::cli_workbench::{
    CliWorkbenchAgentActivity, CliWorkbenchDraftAction, CliWorkbenchMessageState as Delivery,
    CliWorkbenchProcessState as Process, CliWorkbenchTarget,
};

use super::{
    App, AgentLaunch, Dialog, Message, Project, SessionKey, Surface, Tab, Workbench,
    update::{Action, Event, draft_text},
};
use crate::app::components::workbench_sidebar::{
    matches_project, project_header, project_query, project_row, sidebar_button, sidebar_header,
    sidebar_note, sidebar_section, sidebar_surface, vertical, with_project_menu,
};
use crate::app::i18n::I18n;

pub(super) const DIALOG_INPUT: &str = "cli-workbench-dialog-input";

const SIDEBAR_DIVIDER_WIDTH: f32 = 1.0;
const SIDEBAR_DIVIDER_HIT_WIDTH: f32 = 18.0;
const TOOLBAR_BUTTON_INSET: f32 = 3.0;

#[derive(Clone, Copy)]
pub(super) struct View {
    window: window::Id,
    i18n: I18n,
    locale: config_ui::DesktopLocale,
    surface: Surface,
    status_animation_frame: usize,
}

impl View {
    pub(super) fn msg(self, action: Action) -> Message {
        self.surface.message(Event::Window(self.window, action))
    }

    pub(super) fn label(self, source: &'static str) -> &'static str {
        self.i18n.__(source)
    }
}

impl View {
    fn new(app: &App, window: window::Id, surface: Surface) -> Self {
        let locale = app.general_settings.desktop_locale;
        Self {
            window,
            i18n: I18n::new(locale),
            locale,
            surface,
            status_animation_frame: app.status_animation_frame,
        }
    }
}

pub(crate) fn terminal_ssh_view(app: &App) -> Option<Element<'_, Message>> {
    let state = &app.terminal_ssh;
    if !state.terminal_ssh_visible(app.terminal_project_path(), app.terminal.active_group()) {
        return None;
    }
    let (id, project) = state.selected_project()?;
    let ui = View::new(app, app.main_window_id?, Surface::Terminal);
    Some(workspace(ui, state, id, project, None, app.terminal.is_fullscreen))
}

pub(crate) fn terminal_ssh_tabs(app: &App) -> Option<Element<'_, Message>> {
    use desktop_terminal::terminal_panel::pane_header;

    let state = &app.terminal_ssh;
    let layout = app.terminal.active_group();
    let (project_id, project) = state.terminal_ssh_project(app.terminal_project_path(), layout)?;
    let tabs: Vec<_> = project.panes.grid.iter()
        .flat_map(|(_, pane)| pane.tabs.iter().copied())
        .filter_map(|id| Some((id, tab_label(state, project.tab_for_view(id)?), None)))
        .collect();
    if tabs.is_empty() {
        return None;
    }
    let ui = View::new(app, app.main_window_id?, Surface::Terminal);
    Some(pane_header(
        tabs,
        Some(0), // Workbench session view IDs start at 1. The local terminal is selected.
        true,
        project.tab_menu.map(|(id, _, _)| id),
        project.tab_menu.map(|(_, x, y)| (x, y)),
        ui.locale,
        row![local_terminal_tab(ui, true)].align_y(Alignment::Center),
        move |message| ui.msg(Action::Panel(project_id.into(), layout, message)),
    ))
}

fn local_terminal_tab(ui: View, selected: bool) -> Element<'static, Message> {
    button(text(ui.label("本地终端")).size(13))
        .on_press_maybe((!selected).then(|| ui.msg(Action::ShowLocalTerminal)))
        .padding([5, 8])
        .style(move |theme, status| {
            let mut style = icon_btn_style(theme, status);
            style.text_color = layout_tab_foreground(theme, selected);
            style
        })
        .into()
}

pub(crate) fn terminal_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let state = &app.terminal_ssh;
    if super::terminal_panel_visible(app)
        && state.terminal_directory.as_deref() == app.terminal_project_path()
        && let Some(id) = app.main_window_id
    {
        return overlay(View::new(app, id, Surface::Terminal), state, base);
    }
    base
}

pub(crate) fn view(app: &App, id: window::Id, surface: Surface) -> Element<'_, Message> {
    let ui = View::new(app, id, surface);
    let state = surface.state(app);
    let return_to_project = if surface == Surface::AppTerminal {
        let project_tabs = app.navigation_tabs.open_tabs.iter().rev()
            .filter(|tab| tab.screen == crate::app::Screen::Project);
        let current_project = app.navigation_tabs.project_path.as_ref();
        project_tabs.clone().find(|tab| tab.project_path.as_ref() == current_project)
            .or_else(|| project_tabs.into_iter().next())
            .map(|tab| tab.id.as_str())
    } else {
        None
    };
    responsive(move |size| {
        let selected = state
            .selected
            .as_ref()
            .and_then(|id| state.projects.get(id).map(|project| (id, project)));
        let main = || match selected {
            Some((id, project)) => workspace(ui, state, id, project, return_to_project, false),
            None => {
                let prompt = container(text(ui.label("请选择项目")).size(16)).center(Length::Fill);
                if surface == Surface::AppTerminal {
                    column![
                        container(app_terminal_navigation(ui, state, return_to_project)).padding([5, 7]),
                        prompt,
                    ]
                    .into()
                } else {
                    column![container(ssh_button(ui)).padding([5, 7]), prompt].into()
                }
            }
        };
        let content: Element<'_, Message> = if surface == Surface::Workbench && size.width < 680.0 {
            let sidebar = sidebar(app, ui, state, tooltip::Position::Bottom);
            column![container(sidebar).height(146), main()].spacing(8).into()
        } else {
            let grid = pane_grid::PaneGrid::new(&state.sidebar_layout.workspace, |_, terminal, _| {
                let body = if *terminal {
                    container(main())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(sidebar_surface)
                        .into()
                } else {
                    sidebar(app, ui, state, tooltip::Position::Right)
                };
                pane_grid::Content::new(body)
            })
            .spacing(SIDEBAR_DIVIDER_WIDTH)
            .on_resize(SIDEBAR_DIVIDER_HIT_WIDTH - SIDEBAR_DIVIDER_WIDTH, move |event| {
                ui.msg(Action::ResizeSidebarWidth(event))
            })
            .style(sidebar_divider);
            let grid = if surface == Surface::AppTerminal { grid.min_size(0) } else { grid };
            container(grid).style(settings_divider_style).into()
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
        let base = with_project_menu(app, app.navigation_tabs.screen, base);
        overlay(ui, state, base)
    })
    .into()
}

fn overlay<'a>(ui: View, state: &'a Workbench, base: Element<'a, Message>) -> Element<'a, Message> {
    match &state.dialog {
        Some(Dialog::Ssh(panel)) => super::ssh::view(ui, base, state, panel),
        Some(Dialog::ReadDraft(key, request)) => draft_dialog(ui, base, state, key, request),
        Some(Dialog::LaunchAgent(dialog)) => agent_launch_dialog(ui, base, state, dialog),
        Some(dialog) => dialog_view(
            ui,
            base,
            dialog,
            state.selected_project().map(|(_, project)| project),
            state,
        ),
        None => base,
    }
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

fn sidebar<'a>(
    app: &'a App,
    ui: View,
    state: &'a Workbench,
    project_tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    if state.surface == Surface::AppTerminal {
        return sidebar_projects(app, ui, state, project_tooltip_position);
    }
    let grid = pane_grid::PaneGrid::new(&state.sidebar_layout.sections, |_, agents, _| {
        pane_grid::Content::new(if *agents {
            sidebar_agents(ui, state)
        } else {
            sidebar_projects(app, ui, state, project_tooltip_position)
        })
    })
    .spacing(SIDEBAR_DIVIDER_WIDTH)
    .on_resize(SIDEBAR_DIVIDER_HIT_WIDTH - SIDEBAR_DIVIDER_WIDTH, move |event| {
        ui.msg(Action::ResizeSidebarSections(event))
    })
    .style(sidebar_divider);
    container(grid).style(settings_divider_style).into()
}

fn sidebar_divider(theme: &Theme) -> pane_grid::Style {
    let mut style = pane_grid::default(theme);
    style.picked_split.width = 2.0;
    style.hovered_split.width = 2.0;
    style
}

fn app_terminal_navigation(
    ui: View,
    state: &Workbench,
    return_to_project: Option<&str>,
) -> Element<'static, Message> {
    let back = return_to_project.map(|id| {
        toolbar_icon(
            Icon::ArrowLeft,
            ui.label("返回项目"),
            TOOLBAR_BUTTON_INSET,
            Some(Message::View(crate::app::message::ViewMessage::TabSelected(id.to_owned()))),
        )
    });
    row![]
        .extend(back)
        .push(projects_sidebar_toggle(ui, state))
        .push(ssh_button(ui))
        .spacing(4)
        .into()
}

fn ssh_button(ui: View) -> Element<'static, Message> {
    toolbar_icon(
        Icon::Globe,
        ui.label("SSH 连接"),
        TOOLBAR_BUTTON_INSET,
        Some(ui.msg(Action::Ssh(super::ssh::Action::Open))),
    )
}

fn projects_sidebar_toggle(ui: View, state: &Workbench) -> Element<'static, Message> {
    let (icon, label) = if state.sidebar_layout.collapsed() {
        (Icon::LayoutSidebarReverse, "展开侧边栏")
    } else {
        (Icon::LayoutSidebar, "收起侧边栏")
    };
    toolbar_icon(
        icon,
        ui.label(label),
        TOOLBAR_BUTTON_INSET,
        Some(ui.msg(Action::ToggleProjectsSidebar)),
    )
}

fn sidebar_projects<'a>(
    app: &'a App,
    ui: View,
    state: &'a Workbench,
    tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    let projects_header = project_header(app, (!state.discovering).then_some(ui.msg(Action::Refresh)));
    let query = project_query(app);
    let mut matches = state.projects.iter().filter(|(_, project)| {
        matches_project(&query, &project.name, &project.directory)
    }).peekable();
    let mut projects = column![].spacing(3);
    if matches.peek().is_none() {
        let label = match (state.discovering, state.surface, query.is_empty()) {
            (true, Surface::Workbench, _) => "正在读取工作台项目…",
            (true, _, _) => "正在读取项目…",
            (false, _, false) => "未找到匹配的项目",
            (false, _, true) => "暂无项目",
        };
        projects = projects.push(sidebar_note(ui.label(label)));
    }
    for (id, project) in matches {
        let selected = state.selected.as_ref() == Some(id);
        projects = projects.push(project_row(
            project.name.clone(),
            &project.directory,
            Some(id),
            selected,
            Some(ui.msg(Action::SelectProject(id.clone()))),
            tooltip_position,
        ));
    }
    if !query.is_empty() {
        return sidebar_section(projects_header, projects);
    }
    if let Some(project_id) = &state.selected
        && let Some(project) = state.projects.get(project_id)
        && project.tabs.values().any(|tab| tab.hidden)
    {
        projects = projects.push(sidebar_note(ui.label("已关闭的标签")));
        for (session, tab) in project.tabs.iter().filter(|(_, tab)| tab.hidden) {
            let key = SessionKey { project: project_id.clone(), session: session.clone() };
            projects = projects.push(
                control(tab_label(state, tab), Some(ui.msg(Action::SelectSession(key))), false)
                    .width(Length::Fill),
            );
        }
    }
    let mut drafts = state.projects.iter().flat_map(|(project_id, project)| {
        project.tabs.iter().flat_map(move |(session, tab)| {
            tab.drafts.iter().map(move |draft| (project_id, project, session, tab, draft))
        })
    }).peekable();
    if drafts.peek().is_some() {
        projects = projects.push(sidebar_note(ui.label("待提交草稿")));
        for (project_id, project, session, tab, draft) in drafts {
            let key = SessionKey { project: project_id.clone(), session: session.clone() };
            projects = projects.push(control(
                format!("{} · {} #{}", project.name, tab_label(state, tab), id_suffix(&draft.id.request_id)),
                Some(ui.msg(Action::ShowDraft(key, draft.id.request_id.clone()))),
                false,
            ).width(Length::Fill));
        }
    }

    sidebar_section(projects_header, projects)
}

fn sidebar_agents<'a>(ui: View, state: &'a Workbench) -> Element<'a, Message> {
    let grouped = state.group_agents_by_project;
    let sort = settings_select([false, true], Some(grouped), move |grouped| {
        ui.msg(Action::GroupAgentsByProject(grouped))
    })
    .format_labels(move |grouped| ui.label(if *grouped { "按项目分组" } else { "按优先级排序" }).into())
    .menu_icon(if grouped { Icon::FolderClosed } else { Icon::Clock }, 176.0)
    .width(30)
    .height(26.0);
    let agents_header = sidebar_header(
        ui.label("智能体"),
        toolbar_tooltip(sort.into(), ui.label("智能体排序")),
    );
    sidebar_section(agents_header, agent_sessions(ui, state))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AgentStatus {
    NeedsInput,
    Working,
    Ready,
    Idle,
    Running,
    Stopping,
    Exited,
    Disconnected,
}

impl AgentStatus {
    pub(super) fn for_tab(project: &Project, tab: &Tab) -> Option<Self> {
        use CliWorkbenchAgentActivity as Activity;

        if matches!(tab.session.target, CliWorkbenchTarget::Shell) {
            return None;
        }
        if project.disconnected {
            return Some(Self::Disconnected);
        }
        if tab.stopping {
            return Some(Self::Stopping);
        }
        Some(match tab.session.state {
            Process::WaitingForApproval => Self::NeedsInput,
            Process::Running => match tab.session.agent_activity {
                Some(Activity::NeedsInput) => Self::NeedsInput,
                Some(Activity::Working) => Self::Working,
                Some(Activity::Idle) => Self::Idle,
                None if matches!(tab.session.target, CliWorkbenchTarget::Acp { .. }) => Self::Working,
                None => Self::Running,
            },
            Process::Ready => Self::Ready,
            Process::Stopping => Self::Stopping,
            Process::Exited => Self::Exited,
            Process::Disconnected => Self::Disconnected,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Self::NeedsInput => "需要处理",
            Self::Working => "正在执行",
            Self::Ready => "就绪",
            Self::Idle => "空闲",
            Self::Running => "运行中",
            Self::Stopping => "正在停止",
            Self::Exited => "已退出",
            Self::Disconnected => "连接已断开",
        }
    }

    fn color(self, theme: &Theme) -> iced::Color {
        let palette = theme.extended_palette();
        match self {
            Self::NeedsInput => palette.danger.strong.color,
            Self::Working => palette.success.strong.color,
            _ => theme.palette().text.scale_alpha(0.7),
        }
    }
}

fn agent_attention_dot<M: 'static>() -> Element<'static, M> {
    container(Space::new())
        .width(8)
        .height(8)
        .style(|theme| {
            container::background(AgentStatus::NeedsInput.color(theme))
                .border(Border::default().rounded(4))
        })
        .into()
}

fn agent_sessions<'a>(ui: View, state: &'a Workbench) -> Column<'a, Message> {
    let grouped = state.group_agents_by_project;
    let mut list = column![].spacing(3);
    let mut sessions = Vec::new();
    for (project_id, project) in &state.projects {
        for tab in project.tabs.values() {
            if let Some(status) = AgentStatus::for_tab(project, tab) {
                sessions.push((status, project_id, project, tab));
            }
        }
    }
    if sessions.is_empty() {
        return list.push(sidebar_note(ui.label("暂无智能体")));
    }
    sessions.sort_by(|left, right| {
        if grouped {
            left.2.name.cmp(&right.2.name).then(left.1.cmp(right.1)).then(left.0.cmp(&right.0))
        } else {
            left.0.cmp(&right.0).then(left.2.name.cmp(&right.2.name))
        }
    });
    let mut previous = None;
    for (status, project_id, project, tab) in sessions {
        let section = if grouped { project_id.as_str() } else { status.label() };
        if previous != Some(section) {
            let label = if grouped { &project.name } else { ui.label(status.label()) };
            let mut heading = sidebar_note(label);
            if !grouped && matches!(status, AgentStatus::NeedsInput | AgentStatus::Working) {
                heading = heading.style(move |theme: &Theme| text::Style {
                    color: Some(status.color(theme)),
                });
            }
            list = list.push(container(heading).padding([5, 6]));
            previous = Some(section);
        }
        let selected = state.selected.as_ref() == Some(project_id)
            && project.active_tab().is_some_and(|active| active.view_id == tab.view_id);
        let title = agent_title(state, tab);
        let label = if grouped { title } else { format!("{} · {title}", project.name) };
        let mut summary = column![SingleLineLabel::new(label, 13.0)].width(Length::Fill);
        if grouped {
            summary = summary.push(text(ui.label(status.label())).size(10).style(move |theme: &Theme| text::Style {
                color: Some(status.color(theme)),
            }));
        }
        let mut content = row![agent_icon(tab), summary].spacing(7).align_y(Alignment::Center);
        if status == AgentStatus::Working {
            content = content.push(running_session_indicator(
                &tab.session.id,
                ui.status_animation_frame,
            ));
        }
        list = list.push(sidebar_button(
            content,
            Some(ui.msg(Action::SelectSession(SessionKey {
                project: project_id.clone(),
                session: tab.session.id.clone(),
            }))),
            selected,
        ));
    }
    list
}

fn agent_title(state: &Workbench, tab: &Tab) -> String {
    if let Some(title) = &tab.name {
        return title.clone();
    }
    let name = match &tab.session.target {
        CliWorkbenchTarget::Acp { agent } => state.agents.iter()
            .find(|entry| entry.key == *agent).map(|entry| entry.label()),
        CliWorkbenchTarget::NativeCli { app_id } => state.native_apps.iter()
            .find(|entry| entry.id == *app_id).map(|entry| entry.name.as_str()),
        CliWorkbenchTarget::Shell | CliWorkbenchTarget::Ssh { .. } => None,
    };
    name.map_or_else(|| tab_label(state, tab), str::to_owned)
}

fn agent_icon(tab: &Tab) -> svg::Svg<'static> {
    let key = match &tab.session.target {
        CliWorkbenchTarget::Acp { agent } => agent.strip_prefix("acp-").unwrap_or(agent),
        CliWorkbenchTarget::NativeCli { app_id } => app_id,
        CliWorkbenchTarget::Ssh { .. } => return icon_svg(Icon::Globe, 17.0).style(icon_btn_svg_style),
        CliWorkbenchTarget::Shell => "",
    };
    let icon = match key {
        "claude" | "claude-code" => Icon::AppClaudeCode,
        "codex" => Icon::AppCodex,
        "gemini" | "gemini-cli" => Icon::AppGeminiCli,
        "opencode" => Icon::AppOpenCode,
        "openclaw" => Icon::AppOpenClaw,
        "hermes" => Icon::AppHermes,
        "pi" => Icon::AppPi,
        "kimi" => Icon::AppKimiCode,
        "agy" => Icon::AppAntigravity,
        _ => return icon_svg(Icon::Robot, 17.0).style(icon_btn_svg_style),
    };
    icon_svg(icon, 17.0)
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
    if state.surface != Surface::Terminal && state.terminal_available {
        choices.push(LaunchChoice {
            target: CliWorkbenchTarget::Shell,
            label: i18n.__("新建 Shell").into(),
        });
        let mut apps: Vec<_> = state.native_apps.iter()
            .map(|app| (app.id.as_str(), app.name.as_str()))
            .collect();
        if state.surface == Surface::Workbench && !apps.iter().any(|(id, _)| *id == super::CODEX_APP_ID) {
            apps.push((super::CODEX_APP_ID, "Codex"));
        }
        apps.sort_by_key(|(id, _)| state.native_app.as_deref() != Some(*id));
        choices.extend(apps.into_iter().map(|(id, name)| LaunchChoice {
            target: CliWorkbenchTarget::NativeCli { app_id: id.into() },
            label: if state.surface == Surface::Workbench && id == super::CODEX_APP_ID {
                i18n.__("新建 Codex").into()
            } else {
                format!("{name} · {}", i18n.__("终端"))
            },
        }));
    }
    if let Some(catalog) = &state.ssh.catalog {
        choices.extend(catalog.saved.iter().map(|connection| LaunchChoice {
            target: CliWorkbenchTarget::Ssh { connection_id: connection.id.clone() },
            label: format!("SSH · {}", connection.fields.name),
        }));
    }
    if state.surface == Surface::Workbench {
        let codex = super::update::codex_agent(state);
        if !state.terminal_available && let Some(agent) = codex {
            choices.push(LaunchChoice {
                target: CliWorkbenchTarget::Acp { agent: agent.key.clone() },
                label: i18n.__("新建 Codex").into(),
            });
        }
        choices.extend(state.agents.iter()
            .filter(|agent| codex.is_none_or(|codex| agent.key != codex.key))
            .map(|agent| LaunchChoice {
                target: CliWorkbenchTarget::Acp { agent: agent.key.clone() },
                label: format!("{} · {}", agent.label(), i18n.__("ACP 对话")),
            }));
    }
    choices
}

fn workspace<'a>(
    ui: View,
    state: &'a Workbench,
    project_id: &'a str,
    project: &'a Project,
    return_to_project: Option<&str>,
    terminal_fullscreen: bool,
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
        if state.surface == Surface::AppTerminal {
            ui.label("当前网关未启用终端")
        } else if state.agents_loading {
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
            .width(TAB_TOOLBAR_BUTTON_WIDTH - 2.0 * TOOLBAR_BUTTON_INSET)
            .height(TAB_HEIGHT - 2.0 * TOOLBAR_BUTTON_INSET)
            .disabled(!can_start);
            let launch = container(launch).center_x(TAB_TOOLBAR_BUTTON_WIDTH).center_y(TAB_HEIGHT);
            let mut controls = row![toolbar_tooltip(launch.into(), ui.label("新建会话"))]
                .align_y(Alignment::Center);
            if super::update::split_target(state, project, pane).is_some() {
                let split = terminal_panel::split_control(
                    &project.panes,
                    ui.locale,
                    pane,
                    TOOLBAR_BUTTON_INSET,
                )
                .map(on_panel);
                controls = controls.push(if project.panes.split_menu == Some(pane) {
                    split
                } else {
                    toolbar_tooltip(split, ui.label("分屏"))
                });
            }
            if let Some(tab) =
                current.filter(|tab| state.surface != Surface::Workbench || tab.is_agent())
            {
                let key = SessionKey { project: project_id.into(), session: tab.session.id.clone() };
                let can_stop = !project.disconnected && tab.can_stop()
                    && (!tab.session.capabilities.terminal || state.pty_available(&tab.session.target));
                controls = controls.push(toolbar_icon(
                    Icon::Square,
                    ui.label("停止会话"),
                    TOOLBAR_BUTTON_INSET,
                    can_stop.then_some(ui.msg(Action::Stop(key))),
                ));
            }
            if top_right && state.surface != Surface::Terminal
                && (state.surface != Surface::Workbench || active.is_some_and(Tab::is_agent))
            {
                controls = controls.push(toolbar_icon(
                    Icon::ArrowUp,
                    ui.label("发送消息"),
                    TOOLBAR_BUTTON_INSET,
                    project.tabs.values().any(sendable)
                        .then(|| ui.msg(Action::Compose(active_target.clone()))),
                ));
            }
            if top_right && state.surface == Surface::Terminal {
                let (icon, label) = if terminal_fullscreen {
                    (Icon::FullscreenExit, ui.label("退出全屏"))
                } else {
                    (Icon::Fullscreen, ui.label("全屏"))
                };
                controls = controls
                    .push(local_terminal_tab(ui, false))
                    .push(toolbar_icon(
                        Icon::EyeSlash,
                        ui.label("隐藏终端面板"),
                        TOOLBAR_BUTTON_INSET,
                        Some(Message::View(crate::app::message::ViewMessage::ToggleTerminalPanel)),
                    ))
                    .push(toolbar_icon(
                        icon,
                        label,
                        TOOLBAR_BUTTON_INSET,
                        Some(Message::Terminal(Term::ToggleFullscreen)),
                    ));
            }
            let header = terminal_panel::pane_header(
                pane_state.tabs.iter().filter_map(|&id| {
                    let tab = project.tab_for_view(id)?;
                    let indicator = match AgentStatus::for_tab(project, tab) {
                        Some(AgentStatus::Working) => Some(running_session_indicator(
                            &tab.session.id,
                            ui.status_animation_frame,
                        )),
                        Some(AgentStatus::NeedsInput) => Some(agent_attention_dot()),
                        _ => None,
                    };
                    Some((id, tab_label(state, tab), indicator))
                }),
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
                    project.disconnected || (tab.session.capabilities.terminal && !state.pty_available(&tab.session.target)),
                    project.content_menu,
                    layout,
                )
            } else {
                container(text(ui.label("暂无会话")).size(15)).center(Length::Fill).into()
            };
            (header, terminal_panel::pane_surface(body))
        },
    );
    let navigation = match state.surface {
        Surface::AppTerminal => Some(app_terminal_navigation(ui, state, return_to_project)),
        Surface::Workbench => Some(ssh_button(ui)),
        Surface::Terminal => None,
    };
    let grid = container(grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(settings_divider_style);
    if state.surface == Surface::Terminal {
        return grid.into();
    }
    let mut workspace = column![
        layout_tabs(ui, project_id, project, navigation),
        settings_divider()
    ];
    if let Some(label) = empty_label {
        workspace = workspace.push(text(label).size(12).style(settings_muted_text_style));
    }
    // The shared grid leaves a one-pixel gap between its opaque pane surfaces.
    workspace.push(grid)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn layout_tabs<'a>(
    ui: View,
    project_id: &str,
    project: &Project,
    projects_toggle: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let button_inset = if ui.surface == Surface::AppTerminal { TOOLBAR_BUTTON_INSET } else { 0.0 };
    let mut tabs = row![].spacing(4).align_y(Alignment::Center);
    let can_close = !project.parked_layouts.is_empty();
    for id in project.layout_ids() {
        let selected = id == project.active_layout;
        let label = layout_label(project, id);
        let panes = if selected { Some(&project.panes) } else { project.parked_layouts.get(&id) };
        let mut working_session = None;
        let mut needs_input = false;
        for tab in panes
            .into_iter()
            .flat_map(|panes| panes.grid.iter().flat_map(|(_, pane)| pane.tabs.iter()))
            .filter_map(|&tab_id| project.tab_for_view(tab_id))
        {
            match AgentStatus::for_tab(project, tab) {
                Some(AgentStatus::Working) => {
                    working_session.get_or_insert(tab.session.id.as_str());
                }
                Some(AgentStatus::NeedsInput) => needs_input = true,
                _ => {}
            }
        }
        let content = row![
            icon_svg(Icon::Terminal, 13.0).style(move |theme: &Theme, _| svg::Style {
                color: Some(layout_tab_foreground(theme, selected)),
            }),
            text(label).size(13),
        ]
        .extend(working_session.map(|id| running_session_indicator(id, ui.status_animation_frame)))
        .extend(needs_input.then(agent_attention_dot))
        .spacing(6)
        .align_y(Alignment::Center);
        let tab = button(content)
            .padding([5, 8])
            .style(move |theme, status| {
                let mut style = icon_btn_style(theme, status);
                style.text_color = layout_tab_foreground(theme, selected);
                style.border = Border::default().rounded(5);
                style
            })
            .on_press(ui.msg(Action::SelectLayout(project_id.into(), id)));
        let mut item = row![tab].align_y(Alignment::Center);
        if selected && can_close {
            item = item.push(toolbar_icon(
                Icon::X,
                ui.label("关闭分屏组"),
                button_inset,
                Some(ui.msg(Action::CloseLayout(project_id.into(), id))),
            ));
        }
        let item = container(item).padding(2).style(move |theme| {
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
        });
        let project_key = project_id.to_owned();
        tabs = tabs.push(Element::new(
            RightClickArea::new(
                item.into(),
                Box::new(move |point| ui.msg(Action::OpenLayoutMenu(project_key.clone(), id, point))),
            )
            .use_global_position(),
        ));
    }
    let tabs = scrollable(tabs)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill);
    let controls = row![]
        .extend(projects_toggle)
        .push(tabs)
        .push(toolbar_icon(
            Icon::Plus,
            ui.label("新建分屏组"),
            button_inset,
            Some(ui.msg(Action::NewLayout(project_id.into()))),
        ))
        .spacing(8)
        .align_y(Alignment::Center);
    let strip = container(controls)
    .padding([5, 7])
    .width(Length::Fill)
    .style(|theme| container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    });
    let (menu, point) = match project.layout_menu {
        Some((id, point)) => (
            tab_context_menu(
                ui.locale,
                ui.msg(Action::RenameLayout(project_id.into(), id, layout_label(project, id))),
                can_close.then(|| ui.msg(Action::CloseLayout(project_id.into(), id))),
            ),
            point,
        ),
        None => (Space::new().into(), iced::Point::ORIGIN),
    };
    PointBelowOverlay::new(strip, menu)
        .show(project.layout_menu.is_some())
        .global_anchor(point)
        .gap(2.0)
        .close_on_target_click(true)
        .capture_keyboard()
        .on_close(ui.msg(Action::CloseLayoutMenu(project_id.into())))
        .into()
}

fn layout_label(project: &Project, id: u64) -> String {
    if let Some(name) = project.layout_names.get(&id) {
        return name.clone();
    }
    project.directory.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&project.name)
        .to_owned()
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
        let label = if turn.prompt.is_some() { "你" } else { "协作任务" };
        let mut message = column![text(ui.label(label)).size(12).style(settings_muted_text_style)]
            .spacing(5);
        if let Some(prompt) = &turn.prompt {
            message = message.push(text(prompt).size(14).wrapping(Wrapping::Glyph));
        }
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
            let label = if turn.prompt.is_some() { "正在发送消息…" } else { "正在加载请求…" };
            message = message
                .push(text(ui.label(label)).size(12).style(settings_muted_text_style));
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
    state: &'a Workbench,
) -> Element<'a, Message> {
    let close = ui.msg(Action::Dismiss);
    let confirm = ui.msg(Action::Confirm);
    let (title, draft, enabled, select) = match dialog {
        Dialog::Rename(_, draft) | Dialog::RenameLayout(_, _, draft) => {
            (ui.label("重命名"), draft, !draft.trim().is_empty(), None)
        }
        Dialog::ReadDraft(..) | Dialog::LaunchAgent(..) | Dialog::Ssh(..) => return base,
        Dialog::Compose { target, draft } => {
            let targets: Vec<_> = project
                .into_iter()
                .flat_map(|project| {
                    project.tabs.iter().filter(move |(_, tab)| {
                        !project.disconnected
                            && tab.can_send()
                            && (!tab.session.capabilities.terminal || state.terminal_available)
                    })
                })
                .map(|(id, _)| id.clone())
                .collect();
            let selected = target.as_ref().filter(|id| targets.contains(id)).cloned();
            let enabled = selected.is_some() && !draft.trim().is_empty();
            let select =
                settings_select(targets, selected, move |id| ui.msg(Action::Target(id)))
                    .format_labels(|id| {
                        project
                            .and_then(|project| project.tabs.get(id))
                            .map_or_else(|| id.clone(), |tab| tab_label(state, tab))
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
        fields = fields.push(column![
            settings_field_label(ui.label("目标会话")),
            select,
        ].spacing(6));
    }
    let card = settings_modal_card(
        column![
            row![text(title).size(17).width(Length::Fill), settings_close_button(close.clone())]
                .align_y(Alignment::Center),
            fields.push(input),
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

fn agent_launch_dialog<'a>(
    ui: View,
    base: Element<'a, Message>,
    state: &'a Workbench,
    dialog: &'a AgentLaunch,
) -> Element<'a, Message> {
    let agent = super::update::agent_launch_profile(state, dialog.acp_agent.as_deref());
    let mut fields = column![].spacing(8);
    let title = if dialog.acp_agent.is_none() {
        let native = super::update::launch_target_available(state, &super::update::native_codex_target());
        let modes = [false, true].into_iter().filter(|acp| if *acp { agent.is_some() } else { native });
        let mode = settings_select(modes, Some(dialog.use_acp), move |acp| ui.msg(Action::LaunchMode(acp)))
            .format_labels(move |acp| ui.label(if *acp { "ACP 对话" } else { "原生终端" }).into())
            .width(Length::Fill);
        fields = fields.push(settings_field_label(ui.label("运行方式"))).push(mode);
        ui.label("新建 Codex").to_owned()
    } else {
        agent.map_or_else(
            || ui.label("新建 ACP 会话").to_owned(),
            |profile| format!("{} · {}", profile.label(), ui.label("ACP 对话")),
        )
    };
    if dialog.use_acp {
        let sources = [false, true].into_iter().filter(|gateway| {
            let source = if *gateway { AcpModelSource::SessionModel } else { AcpModelSource::CliLocal };
            agent.is_some_and(|agent| agent.supports(source))
        });
        let gateway = dialog.source == AcpModelSource::SessionModel;
        let source = settings_select(sources, Some(gateway), move |gateway| {
            ui.msg(Action::LaunchModelSource(if gateway { AcpModelSource::SessionModel } else { AcpModelSource::CliLocal }))
        })
        .format_labels(move |gateway| ui.label(if *gateway { "网关模型" } else { "CLI 本地模型" }).into())
        .width(Length::Fill);
        fields = fields.push(settings_field_label(ui.label("模型来源"))).push(source);
        if gateway {
            fields = fields.push(settings_field_label(ui.label("模型")));
            let model: Element<'a, Message> = match &dialog.models {
                Some(Ok(models)) if !models.is_empty() => settings_select(
                    models.iter().cloned(), dialog.model.clone(), move |model| ui.msg(Action::LaunchModel(model)),
                )
                .placeholder(ui.label("选择网关模型"))
                .width(Length::Fill)
                .into(),
                Some(Ok(_)) => text(ui.label("此项目暂无可用的网关模型")).size(13).into(),
                Some(Err(())) => settings_error_banner(ui.label("无法读取网关模型").to_owned()).into(),
                None => text(ui.label("正在读取网关模型…")).size(13).into(),
            };
            fields = fields.push(model);
        }
    }
    let close = ui.msg(Action::Dismiss);
    let enabled = super::update::agent_launch_options(state, dialog).is_some();
    let card = settings_modal_card(
        column![
            row![text(title).size(17).width(Length::Fill), settings_close_button(close.clone())]
                .align_y(Alignment::Center),
            fields,
            row![
                Space::new().width(Length::Fill),
                control(ui.label("取消"), Some(close.clone()), false),
                control(ui.label("启动"), enabled.then(|| ui.msg(Action::Confirm)), true),
            ].spacing(6),
        ].spacing(12),
    ).width(Length::Fill).max_width(480);
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

pub(super) fn tab_label(state: &Workbench, tab: &Tab) -> String {
    if let Some(name) = &tab.name {
        return name.clone();
    }
    let name = match &tab.session.target {
        CliWorkbenchTarget::Acp { agent } => agent,
        CliWorkbenchTarget::NativeCli { app_id } if app_id == super::CODEX_APP_ID => "Codex",
        CliWorkbenchTarget::NativeCli { app_id } => app_id,
        CliWorkbenchTarget::Shell => "Shell",
        CliWorkbenchTarget::Ssh { connection_id } => state.ssh.connection(connection_id)
            .map_or("SSH", |entry| entry.fields.name.as_str()),
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
