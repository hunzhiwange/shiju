//! 工作台和终端共享的项目侧栏控件。
use crate::app::workbench_projects::{Event, SEARCH_INPUT};
use crate::app::{App, Message, Screen};
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::{BelowOverlay, PointBelowOverlay};
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::{
    icon_btn, icon_svg, icons::icon_btn_style, settings_divider, settings_muted_text_style,
    settings_panel_style, settings_text_input,
};
use desktop_ui_controls::tooltip::project_tooltip;
use iced::widget::{Column, button, column, container, row, scrollable, svg, text, tooltip};
use iced::{Alignment, Border, Element, Length, Theme};

pub(crate) fn project_row<'a>(
    name: String,
    directory: &'a str,
    id: Option<&'a str>,
    selected: bool,
    action: Option<Message>,
    tooltip_position: tooltip::Position,
) -> Element<'a, Message> {
    let icon = icon_svg(if selected { Icon::FolderOpen } else { Icon::FolderClosed }, 15.0).style(
        |theme: &Theme, _| svg::Style {
            color: Some(theme.extended_palette().primary.strong.color),
        },
    );
    let badge = container(icon).center_x(23).center_y(23).style(|theme: &Theme| container::Style {
        background: Some(theme.extended_palette().primary.base.color.scale_alpha(0.12).into()),
        border: Border::default().rounded(6),
        ..Default::default()
    });
    let content = row![badge, SingleLineLabel::new(name.clone(), 13.0)]
        .spacing(7)
        .align_y(Alignment::Center);
    let row = project_tooltip(sidebar_button(content, action, selected), name, directory, tooltip_position);
    Element::new(RightClickArea::new(row, Box::new(move |point| {
        Message::WorkbenchProjects(Event::OpenMenu(id.map(str::to_owned), directory.to_owned(), point))
    })).preserve_on_right_click().use_global_position())
}

pub(crate) fn project_header<'a>(
    app: &'a App,
    refresh: Option<Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let add = (refresh.is_some() && !app.workbench_projects.adding)
        .then_some(Message::WorkbenchProjects(Event::Add));
    let screen = app.navigation_tabs.screen;
    let search = app.workbench_projects.search(screen);
    let label = i18n.__("搜索项目");
    let close = Message::WorkbenchProjects(Event::SearchClosed(screen));
    let trigger = icon_btn(Icon::Search, label, Some(Message::WorkbenchProjects(Event::SearchToggled(screen))));
    let trigger = container(trigger).style(move |theme: &Theme| container::Style {
        background: search.filter(|search| !search.query.is_empty())
            .map(|_| theme.extended_palette().primary.base.color.scale_alpha(0.12).into()),
        border: Border::default().rounded(6),
        ..Default::default()
    });
    let search_button: Element<'a, Message> = if let Some(search) = search.filter(|search| search.open) {
        let input = settings_text_input(label, &search.query)
            .id(SEARCH_INPUT)
            .on_input(move |query| Message::WorkbenchProjects(Event::SearchChanged(screen, query)))
            .on_submit(close.clone());
        let clear = icon_btn(Icon::X, i18n.__("清空"), Some(Message::WorkbenchProjects(Event::SearchChanged(screen, String::new()))));
        let mut controls = row![input].spacing(3).align_y(Alignment::Center);
        if !search.query.is_empty() {
            controls = controls.push(clear);
        }
        BelowOverlay::new(trigger, container(controls).width(248).padding(6).style(settings_panel_style))
            .show(true).gap(5.0).within_window().align_right(true).dismiss_on_escape().on_close(close)
            .into()
    } else {
        trigger.into()
    };
    let actions = row![
        search_button,
        icon_btn(Icon::ArrowClockwise, i18n.__("刷新"), refresh),
        icon_btn(Icon::Plus, i18n.__("新增项目"), add),
    ]
    .spacing(3)
    .align_y(Alignment::Center);
    sidebar_header(i18n.__("项目"), actions.into())
}

pub(crate) fn project_query(app: &App) -> String {
    app.workbench_projects.search(app.navigation_tabs.screen)
        .map_or_else(String::new, |search| search.query.trim().to_lowercase())
}

pub(crate) fn matches_project(query: &str, name: &str, directory: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(query) || directory.to_lowercase().contains(query)
}

pub(crate) fn with_project_menu<'a>(
    app: &App,
    screen: Screen,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(menu) = app.workbench_projects.menu(screen) else { return base };
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let item = |icon, label, event| {
        sidebar_button(
            row![icon_svg(icon, 15.0), text(label).size(13)].spacing(9).align_y(Alignment::Center),
            Some(Message::WorkbenchProjects(event)), false,
        )
    };
    let mut items = column![item(Icon::Pencil, i18n.__("编辑项目"), Event::Edit)].spacing(3);
    if !desktop_workspace::projects::is_builtin_workspace_project(&menu.directory) {
        items = items.push(settings_divider()).push(item(Icon::Trash, i18n.__("删除"), Event::Remove));
    }
    items = items.push(item(Icon::Copy, i18n.__("复制路径"), Event::CopyPath));
    let popup = container(items).width(168).padding(5).style(settings_panel_style);
    PointBelowOverlay::new(base, popup)
        .show(true)
        .global_anchor(menu.point)
        .gap(3.0)
        .close_on_target_click(true)
        .capture_keyboard()
        .on_close(Message::WorkbenchProjects(Event::CloseMenu))
        .into()
}

pub(crate) fn sidebar_section<'a>(
    header: Element<'a, Message>,
    rows: Column<'a, Message>,
) -> Element<'a, Message> {
    container(column![header, vertical(rows).height(Length::Fill)])
        .style(sidebar_surface)
        .padding([2, 2])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(crate) fn sidebar_header<'a>(
    title: &'static str,
    action: Element<'a, Message>,
) -> Element<'a, Message> {
    container(row![sidebar_note(title).width(Length::Fill), action].align_y(Alignment::Center))
        .padding([3, 7])
        .width(Length::Fill)
        .into()
}

pub(crate) fn sidebar_note(label: &str) -> iced::widget::Text<'_> {
    text(label).size(12).style(settings_muted_text_style)
}

pub(crate) fn sidebar_button<'a>(
    content: iced::widget::Row<'a, Message>,
    action: Option<Message>,
    selected: bool,
) -> button::Button<'a, Message> {
    button(content).padding([4, 6]).width(Length::Fill).on_press_maybe(action).style(
        move |theme, status| {
            let mut style = icon_btn_style(theme, status);
            if selected && matches!(status, button::Status::Active | button::Status::Disabled) {
                style.background =
                    Some(theme.extended_palette().primary.base.color.scale_alpha(0.12).into());
            }
            style
        },
    )
}

pub(crate) fn sidebar_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        ..Default::default()
    }
}

pub(crate) fn vertical<'a>(content: Column<'a, Message>) -> scrollable::Scrollable<'a, Message> {
    scrollable(container(content).padding([2, 7]).width(Length::Fill)).direction(
        scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(4).scroller_width(4)),
    )
}
