use desktop_assets::assets::Icon;
use desktop_terminal::terminal_panel::{group_context_menu, rename_overlay};
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_ui_controls::system_settings_common::icons::{icon_btn_style, icon_btn_svg_style};
use desktop_ui_controls::system_settings_common::{
    icon_svg, settings_divider, settings_divider_style,
};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use iced::widget::{Space, button, column, container, row, scrollable, svg, text, tooltip};
use iced::{Alignment, Border, Element, Length, Theme};

use crate::app::cli_workbench::{self, Event, Surface};
use crate::app::components::workbench_sidebar::{
    matches_project, project_header, project_query, project_row, sidebar_note, sidebar_section,
    with_project_menu,
};
use crate::app::message::TerminalMessage;
use crate::app::project_switcher::Destination;
use crate::app::workbench_projects;
use crate::app::{App, Message, Screen};

pub(super) fn view<'a>(app: &'a App, panes: Element<'a, Message>) -> Element<'a, Message> {
    let mut terminal = column![group_bar(app), settings_divider()].height(Length::Fill);
    if let Some(ssh) = cli_workbench::terminal_ssh_view(app) {
        terminal = terminal.push(ssh);
    } else {
        if let Some(tabs) = cli_workbench::terminal_ssh_tabs(app) {
            terminal = terminal.push(tabs).push(settings_divider());
        }
        terminal = terminal.push(
            container(panes).width(Length::Fill).height(Length::Fill).style(settings_divider_style),
        );
    }
    let content = if app.terminal_projects_collapsed {
        terminal.width(Length::Fill).into()
    } else {
        // The workbench reserves 20% of its width for the project and agent sidebar.
        container(
            row![
                container(projects(app)).width(Length::FillPortion(1)).height(Length::Fill),
                container(terminal).width(Length::FillPortion(9)).height(Length::Fill),
            ]
            .spacing(1),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(settings_divider_style)
        .into()
    };
    if let Some((_, draft)) = &app.terminal.group_rename {
        return rename_overlay(
            content,
            app.general_settings.desktop_locale,
            app.terminal.height < 190.0,
            ("重命名分屏组", draft),
            (
                Message::Terminal(TerminalMessage::RenameGroupCancel),
                Message::Terminal(TerminalMessage::RenameGroupSave),
            ),
            |value| Message::Terminal(TerminalMessage::RenameGroupChanged(value)),
        );
    }
    content
}

fn group_bar(app: &App) -> Element<'_, Message> {
    let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
    let (icon, label) = if app.terminal_projects_collapsed {
        (Icon::LayoutSidebarReverse, i18n.__("展开侧边栏"))
    } else {
        (Icon::LayoutSidebar, i18n.__("收起侧边栏"))
    };
    let open_app_terminal = if app.terminal_project_path().is_some() {
        Message::WorkbenchProjects(workbench_projects::Event::SwitchSurface(
            Screen::Project,
            Destination::AppTerminal,
        ))
    } else {
        Surface::AppTerminal.message(Event::Open { backend: None, native_app: None })
    };
    let mut groups = row![].spacing(4).align_y(Alignment::Center);
    let can_close = app.terminal.groups().nth(1).is_some();
    for (id, selected) in app.terminal.groups() {
        let tab = button(text(group_title(app, id)).size(13))
            .on_press_maybe(
                (!selected).then_some(Message::Terminal(TerminalMessage::SelectGroup(id))),
            )
            .padding([5, 8])
            .style(move |theme, status| group_style(theme, status, selected));
        groups = groups.push(Element::new(
            RightClickArea::new(
                tab.into(),
                Box::new(move |point| Message::Terminal(TerminalMessage::GroupMenuOpen(id, point))),
            )
            .use_global_position(),
        ));
        if selected && can_close {
            groups = groups.push(icon_button(
                Icon::X,
                i18n.__("关闭分屏组"),
                Message::Terminal(TerminalMessage::CloseGroup(id)),
            ));
        }
    }

    let groups = scrollable(groups)
        .direction(scrollable::Direction::Horizontal(scrollbar()))
        .width(Length::Fill);
    let strip = container(
        row![
            icon_button(icon, label, Message::Terminal(TerminalMessage::ToggleProjects)),
            icon_button(
                Icon::Terminal,
                i18n.__("应用终端"),
                open_app_terminal,
            ),
            icon_button(
                Icon::Globe,
                i18n.__("SSH 连接"),
                Surface::Terminal.message(Event::OpenSsh),
            ),
            groups,
            icon_button(
                Icon::Plus,
                i18n.__("新建分屏组"),
                Message::Terminal(TerminalMessage::NewGroup),
            ),
        ]
        .align_y(Alignment::Center)
        .spacing(4),
    )
    .width(Length::Fill)
    .padding([2, 7])
    .style(surface);
    let (menu, point) = match app.terminal.group_menu {
        Some((id, point)) => (
            group_context_menu(
                app.general_settings.desktop_locale,
                Message::Terminal(TerminalMessage::RenameGroupStart(
                    id,
                    group_title(app, id).to_owned(),
                )),
                can_close.then_some(Message::Terminal(TerminalMessage::CloseGroup(id))),
            ),
            point,
        ),
        None => (Space::new().into(), iced::Point::ORIGIN),
    };
    PointBelowOverlay::new(strip, menu)
        .show(app.terminal.group_menu.is_some())
        .global_anchor(point)
        .gap(2.0)
        .close_on_target_click(true)
        .capture_keyboard()
        .on_close(Message::Terminal(TerminalMessage::GroupMenuClose))
        .into()
}

fn group_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let mut style = icon_btn_style(theme, status);
    if selected {
        let palette = theme.extended_palette();
        style.background = Some(palette.background.weak.color.into());
        style.text_color = palette.primary.base.color;
    }
    style
}

fn group_title(app: &App, id: u64) -> &str {
    app.terminal
        .group_name(id)
        .or_else(|| {
            app.terminal_project_path()?
                .trim_end_matches(['/', '\\'])
                .rsplit(['/', '\\'])
                .find(|name| !name.trim().is_empty())
        })
        .unwrap_or_else(|| {
            crate::app::i18n::I18n::new(app.general_settings.desktop_locale).__("终端")
        })
}

fn projects(app: &App) -> Element<'_, Message> {
    let current = app.terminal_project_path();
    let query = project_query(app);
    let mut matches = app.workbench_projects.catalog().into_iter()
        .flat_map(|projects| projects.values())
        .filter(|project| matches_project(&query, &project.name, &project.directory)).peekable();
    let local_gateway = crate::app::config::gateway_client_endpoint().is_loopback();
    let mut items = column![].spacing(3);
    if matches.peek().is_none() {
        let i18n = crate::app::i18n::I18n::new(app.general_settings.desktop_locale);
        let label = if app.workbench_projects.catalog_loading {
            "加载中"
        } else if !query.is_empty() {
            "未找到匹配的项目"
        } else {
            "暂无项目"
        };
        items = items.push(sidebar_note(i18n.__(label)));
    }
    for project in matches {
        let path = project.directory.as_str();
        let selected = current == Some(path);
        items = items.push(project_row(
            project.name.clone(), path, Some(&project.id), selected,
            (local_gateway && !selected)
                .then(|| Message::Terminal(TerminalMessage::SelectProject(path.to_owned()))),
            tooltip::Position::Right,
        ));
    }

    let refresh = Message::WorkbenchProjects(workbench_projects::Event::RefreshTerminal);
    with_project_menu(app, app.navigation_tabs.screen, sidebar_section(project_header(app, Some(refresh)), items))
}

fn scrollbar() -> scrollable::Scrollbar {
    scrollable::Scrollbar::new().width(4).scroller_width(4)
}

fn surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    }
}

fn icon_button(icon: Icon, label: &'static str, message: Message) -> Element<'static, Message> {
    let glyph = icon_svg(icon, 14.0).style(move |theme: &Theme, status| {
        let mut style = icon_btn_svg_style(theme, status);
        if matches!(icon, Icon::X) && matches!(status, svg::Status::Hovered) {
            style.color = Some(theme.extended_palette().danger.base.color);
        }
        style
    });
    let glyph = container(glyph).center_x(27).center_y(24);
    let control = button(glyph).on_press(message).padding(0).style(|theme, status| {
        button::Style { border: Border::default().rounded(5), ..icon_btn_style(theme, status) }
    });
    tooltip(control, text(label).size(12), tooltip::Position::Bottom)
        .gap(5)
        .padding(6)
        .style(tooltip_dark_style)
        .into()
}
