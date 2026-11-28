use std::path::Path;
use std::time::Duration;

use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{icon_svg, rounded_action_btn_style};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use iced::widget::{
    Space, Stack, button, canvas, container, mouse_area, opaque, pin, stack, text, tooltip,
};
use iced::{Element, Length, Theme};

use super::cli_workbench::{self, Surface};
use super::i18n::I18n;
use super::message::{TerminalMessage, ViewMessage, terminal, view};
use super::state::ToastKind;
use super::workbench_projects::Event;
use super::{App, Message, Screen};

mod orb;

const WIDTH: f32 = 286.0;
const HEIGHT: f32 = 144.0;
const ORB_SIZE: f32 = 28.0;
const TARGET_SIZE: f32 = 38.0;

#[derive(Clone, Copy)]
pub(crate) enum Destination {
    Git,
    Workbench,
    AppTerminal,
    ProjectTerminal,
}

impl Destination {
    fn screen(self) -> Screen {
        match self {
            Self::Git => Screen::GitWorkbench,
            Self::Workbench => Screen::CliWorkbench,
            Self::AppTerminal => Screen::AppTerminal,
            Self::ProjectTerminal => Screen::Project,
        }
    }
}

fn source_directory(app: &App) -> Option<&str> {
    if app.navigation_panels.show_system_settings || app.input_feedback.error_message.is_some() {
        return None;
    }
    match app.navigation_tabs.screen {
        Screen::GitWorkbench => app.git_workbench.switcher_project_directory(),
        Screen::CliWorkbench => app.cli_workbench.switcher_project_directory(),
        Screen::AppTerminal => app.app_terminal.switcher_project_directory(),
        Screen::Project
            if cli_workbench::terminal_panel_visible(app)
                && !app.terminal_ssh.has_dialog()
                && app.workspace_state.new_session_picker_project.is_none()
                && !app.current_session_runtime_ref().is_some_and(|runtime| {
                    runtime.acp_options_state.open && runtime.acp_agent.is_some()
                })
                && app.terminal.group_rename.is_none()
                && app.terminal.tabs.iter().all(|tab| tab.edit_title.is_none()) =>
        {
            app.terminal_project_path()
        }
        _ => None,
    }
    .filter(|directory| !directory.is_empty())
}

pub(super) fn close_if_hidden(app: &mut App) {
    if let Some(screen) = app.workbench_projects.switcher_screen
        && (screen != app.navigation_tabs.screen || source_directory(app).is_none())
    {
        app.workbench_projects.switcher_screen = None;
    }
}

pub(super) fn navigate(app: &mut App, source: Screen, target: Destination) -> iced::Task<Message> {
    app.workbench_projects.switcher_screen = None;
    if source != app.navigation_tabs.screen
        || (source == target.screen() && (source != Screen::Project || app.terminal.is_fullscreen))
    {
        return iced::Task::none();
    }
    let Some(directory) = source_directory(app).map(str::to_owned) else {
        return iced::Task::none();
    };
    match target {
        Destination::Git => super::git_workbench::open_project(app, directory),
        Destination::Workbench | Destination::AppTerminal => {
            let surface = Surface::from_screen(target.screen()).expect("CLI workbench destination");
            let message = cli_workbench::Event::OpenProject(directory);
            app.update(surface.message(message))
        }
        Destination::ProjectTerminal => open_project_terminal(app, directory),
    }
}

fn open_project_terminal(app: &mut App, directory: String) -> iced::Task<Message> {
    if !super::config::gateway_client_endpoint().is_loopback() {
        return app.show_localized_toast(ToastKind::Error, "远端终端只能通过网关 PTY 打开", vec![]);
    }
    let path = Path::new(&directory);
    if !path.is_absolute() || !path.is_dir() {
        return app.show_localized_toast(ToastKind::Error, "项目目录不存在或不可访问", vec![]);
    }
    let open = if app.navigation_tabs.screen == Screen::Project
        && app.navigation_tabs.project_path.as_deref() == Some(directory.as_str())
    {
        iced::Task::none()
    } else {
        app.open_project_and_index(directory.clone())
    };
    app.select_terminal_project(directory);
    let show = if app.terminal.is_visible {
        iced::Task::none()
    } else {
        view::update(app, ViewMessage::ToggleTerminalPanel)
    };
    let focus = if app.terminal.is_fullscreen {
        terminal::event::focus_active_terminal(app)
    } else {
        terminal::update(app, TerminalMessage::ToggleFullscreen)
    };
    iced::Task::batch([open, show, focus, terminal::reap_retired_task(app)])
}

pub(super) fn with_switcher<'a>(app: &App, base: Element<'a, Message>) -> Element<'a, Message> {
    let layer: Element<'_, Message> = if source_directory(app).is_some() {
        let bottom = super::impls::STATUS_BAR_HEIGHT - ORB_SIZE / 2.0;
        container(switcher(app))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Bottom)
            .padding(iced::Padding::default().bottom(bottom))
            .into()
    } else {
        Space::new().into()
    };
    stack![base, layer].into()
}

fn switcher(app: &App) -> Element<'static, Message> {
    let screen = app.navigation_tabs.screen;
    let expanded = app.workbench_projects.switcher_screen == Some(screen);
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let open = Message::WorkbenchProjects(Event::SwitcherExpanded(screen, true));
    let close = Message::WorkbenchProjects(Event::SwitcherExpanded(screen, false));
    let center = button(canvas(orb::Orb { expanded }).width(ORB_SIZE).height(ORB_SIZE))
        .padding(0)
        .style(|_, _| button::Style::default())
        .on_press(open.clone());
    let center = tooltip(center, text(i18n.__("切换项目视图")).size(12), tooltip::Position::Top)
        .delay(Duration::from_millis(500))
        .padding(6)
        .style(tooltip_dark_style);
    let mut menu = Stack::with_children([
        Space::new().width(WIDTH).height(HEIGHT).into(),
        pin(opaque(mouse_area(center).on_enter(open)))
            .x((WIDTH - ORB_SIZE) / 2.0)
            .y(HEIGHT - ORB_SIZE)
            .into(),
    ]);
    for (target, icon, label, x, y) in [
        (Destination::Git, Icon::GitBranch, "Git 工作台", 35.0, 86.0),
        (Destination::Workbench, Icon::Grid1x2, "我的工作台", 88.0, 29.0),
        (Destination::AppTerminal, Icon::Terminal, "应用终端", WIDTH - 88.0, 29.0),
        (Destination::ProjectTerminal, Icon::FolderClosed, "项目终端", WIDTH - 35.0, 86.0),
    ]
    .into_iter()
    .filter(|_| expanded)
    {
        let active =
            screen == target.screen() && (screen != Screen::Project || app.terminal.is_fullscreen);
        let icon = icon_svg(icon, 17.0).style(move |theme: &Theme, _| iced::widget::svg::Style {
            color: Some(if active {
                theme.extended_palette().primary.strong.text
            } else {
                theme.palette().text
            }),
        });
        let control = button(container(icon).center_x(TARGET_SIZE).center_y(TARGET_SIZE))
            .padding(0)
            .on_press(Message::WorkbenchProjects(Event::SwitchSurface(screen, target)))
            .style(move |theme, status| target_style(theme, status, active));
        let control = tooltip(control, text(i18n.__(label)).size(12), tooltip::Position::Top)
            .padding(6)
            .gap(5)
            .style(tooltip_dark_style);
        menu = menu.push(pin(control).x(x - TARGET_SIZE / 2.0).y(y - TARGET_SIZE / 2.0));
    }
    let area = mouse_area(menu).on_exit(close);
    if expanded { opaque(area) } else { area.into() }
}

fn target_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let mut style = rounded_action_btn_style(theme, status);
    style.background = Some(
        if active {
            palette.primary.strong.color
        } else if hovered {
            palette.background.strong.color
        } else {
            palette.background.base.color
        }
        .into(),
    );
    style.border = iced::Border {
        color: if active || hovered {
            palette.primary.base.color
        } else {
            palette.secondary.weak.color
        },
        width: 1.0,
        radius: (TARGET_SIZE / 2.0).into(),
    };
    style.shadow = iced::Shadow {
        color: palette.primary.base.color.scale_alpha(if hovered || active { 0.45 } else { 0.2 }),
        offset: iced::Vector::new(0.0, 2.0),
        blur_radius: 10.0,
    };
    style
}
