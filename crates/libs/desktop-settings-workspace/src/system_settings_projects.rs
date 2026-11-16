//! Recent project list and current project worktree control.
use desktop_assets::assets::Icon;
use desktop_ui_controls::{overlays::BelowOverlay, system_settings_common::*};
use desktop_i18n::I18n;
use iced::widget::{button, checkbox, column, container, row, svg, text, text_input};
use iced::{Alignment, Element, Length, Theme};
#[derive(Debug, Clone)]
pub enum Event {
    ProjectEnableWorktreeToggled(String, bool),
    OpenRecentPressed(String), ProjectToolsMenuToggled(String), ProjectEditOpened(String),
    ProjectToolsMenuClosed, RecentProjectDeleteRequested(usize), RecentProjectDeleteCanceled,
    RecentProjectDeleteConfirmed(usize), OpenFolderPressed, RecentProjectQueryChanged(String),
}
pub type Avatar<Host> = for<'a> fn(&'a str, &'a str, Option<&'a str>, Option<&'a str>, f32) -> Element<'a, Host>;
pub type Help<Host> = for<'a> fn(I18n, &Theme, &'a str, &'a str, f32, Element<'a, Host>) -> iced::widget::Container<'a, Host>;
pub struct ViewState<'a> {
    pub project_path: &'a Option<String>,
    pub project_worktree_enabled: &'a std::collections::HashMap<String, bool>,
    pub project_tools_menu_path: &'a Option<String>,
    pub recent_projects: &'a [String],
    pub recent_projects_edits: &'a [String],
    pub recent_project_delete_confirm_idx: Option<usize>,
    pub recent_project_query: &'a str,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a Theme,
}
pub fn current_project_worktree<'a, Host: Clone + 'static>(app: &ViewState<'a>, i18n: I18n, map: fn(Event) -> Host, help: Help<Host>) -> Element<'a, Host> {
    let control: Element<'_, Host> = if let Some(path) = app.project_path.as_ref() {
        let checked = app.project_worktree_enabled.get(path).copied().unwrap_or(false);
        checkbox(checked)
            .label(i18n.__("启用"))
            .on_toggle(move |value| {
                map(Event::ProjectEnableWorktreeToggled(
                    path.clone(),
                    value,
                ))
            })
            .style(settings_checkbox_style)
            .into()
    } else {
        text(i18n.__("请先打开项目")).size(12).style(settings_muted_text_style).into()
    };
    help(
        i18n,
        app.app_theme,
        i18n.__("启用 Worktree"),
        i18n.__("新建会话时可选工作区。"),
        110.0,
        control,
    )
    .into()
}

fn quiet_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(palette.background.weak.color.scale_alpha(0.58).into()),
        button::Status::Pressed => Some(palette.background.strong.color.into()),
        _ => None,
    };
    button::Style {
        background,
        text_color: theme.palette().text,
        border: iced::Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

fn project_icon(icon: Icon) -> svg::Svg<'static> {
    icon_svg(icon, 15.0).style(|theme: &Theme, _| svg::Style {
        color: Some(theme.palette().text.scale_alpha(0.78)),
    })
}

fn menu_item<Host: Clone + 'static>(icon: Icon, label: &'static str, message: Host) -> Element<'static, Host> {
    button(row![project_icon(icon), text(label).size(13)].spacing(9).align_y(Alignment::Center))
        .padding([8, 9])
        .width(Length::Fill)
        .on_press(message)
        .style(quiet_button_style)
        .into()
}

fn project_row<'a, Host: Clone + 'static>(
    app: &ViewState<'_>,
    i18n: I18n,
    index: usize,
    path: &'a str,
    name: &'a str,
    map: fn(Event) -> Host,
    avatar: Avatar<Host>,
    appearance: &dyn Fn(&str) -> (Option<&'a str>, Option<&'a str>),
) -> Element<'a, Host> {
    let is_current = app.project_path.as_deref() == Some(path);
    let is_menu_open = app.project_tools_menu_path.as_deref() == Some(path);
    let (icon, color) = appearance(path);
    let mut summary = row![
        avatar(name, path, icon, color, 44.0),
        column![
            text(name).size(16).wrapping(text::Wrapping::None),
            text(path).size(12).wrapping(text::Wrapping::None).style(settings_muted_text_style),
        ]
        .spacing(3)
        .width(Length::Fill)
        .clip(true),
    ]
    .align_y(Alignment::Center)
    .spacing(12);
    if is_current {
        summary = summary.push(settings_value_badge(i18n.__("当前项目")));
    }
    let project = button(summary.push(Element::from(project_icon(Icon::ChevronRight))))
        .clip(true)
        .padding([12, 14])
        .width(Length::Fill)
        .on_press(map(Event::OpenRecentPressed(path.to_owned())))
        .style(|theme, status| {
            let panel = settings_panel_style(theme);
            let mut style = quiet_button_style(theme, status);
            style.border.radius = panel.border.radius;
            style.snap = panel.snap;
            style
        });

    let trigger = icon_btn(
        Icon::DotsThreeVertical,
        i18n.__("项目操作"),
        Some(map(Event::ProjectToolsMenuToggled(path.to_owned()))),
    );
    let actions = if is_menu_open {
        let menu = container(
            column![
                menu_item(
                    Icon::Pencil,
                    i18n.__("编辑项目"),
                    map(Event::ProjectEditOpened(path.to_owned())),
                ),
                settings_divider(),
                menu_item(
                    Icon::Trash,
                    i18n.__("删除"),
                    map(Event::RecentProjectDeleteRequested(index)),
                ),
            ]
            .spacing(3),
        )
        .width(168)
        .padding(5)
        .style(settings_panel_style);
        Element::from(
            BelowOverlay::new(trigger, menu)
                .show(true)
                .gap(4.0)
                .on_close(map(Event::ProjectToolsMenuClosed)),
        )
    } else {
        trigger
    };

    let line = row![project, container(actions).padding(iced::padding::right(8))]
        .align_y(Alignment::Center)
        .spacing(6);
    let mut entry = column![line];

    if app.recent_project_delete_confirm_idx == Some(index) {
        entry = entry.push(settings_divider()).push(
            container(
                row![
                    text(i18n.__("确认删除该项目？")).size(13).width(Length::Fill),
                    button(text(i18n.__("取消")).size(13))
                        .on_press(map(Event::RecentProjectDeleteCanceled))
                        .padding([7, 10])
                        .style(rounded_action_btn_style),
                    button(text(i18n.__("确认删除")).size(13))
                        .on_press(map(Event::RecentProjectDeleteConfirmed(
                            index
                        )))
                        .padding([7, 10])
                        .style(danger_action_btn_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([10, 14]),
        );
    }
    container(entry)
        .width(Length::Fill)
        .style(move |theme| {
            let mut style = settings_panel_style(theme);
            if is_current {
                style.border.color = theme.palette().primary;
            }
            style
        })
        .into()
}

/// 渲染项目管理、搜索和 Worktree 快捷设置。
pub fn view<'a, Host: Clone + 'static>(app: &ViewState<'a>, map: fn(Event) -> Host, avatar: Avatar<Host>, help: Help<Host>, appearance: &dyn Fn(&str) -> (Option<&'a str>, Option<&'a str>)) -> Element<'a, Host> {
    let i18n = I18n::new(app.desktop_locale);
    let open_folder = button(text(i18n.__("打开文件夹")).size(SETTINGS_CONTROL_TEXT_SIZE))
        .on_press(map(Event::OpenFolderPressed))
        .padding([8, 14])
        .style(primary_action_btn_style);

    let search = text_input(i18n.__("搜索项目"), &app.recent_project_query)
        .on_input(move |query| map(Event::RecentProjectQueryChanged(query)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(settings_text_input_style)
        .width(Length::Fill);
    let search = settings_group_card(
        container(row![project_icon(Icon::Search), search].spacing(10).align_y(Alignment::Center))
            .padding([14, 0]),
    );

    let query = app.recent_project_query.trim().to_lowercase();
    let mut projects = column![].spacing(8);
    let mut count = 0;
    for (index, path) in app.recent_projects.iter().enumerate() {
        let name = app
            .recent_projects_edits
            .get(index)
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .or_else(|| std::path::Path::new(path).file_name().and_then(|name| name.to_str()))
            .unwrap_or(path);
        if !query.is_empty()
            && !name.to_lowercase().contains(&query)
            && !path.to_lowercase().contains(&query)
        {
            continue;
        }
        count += 1;
        projects = projects.push(project_row(app, i18n, index, path, name, map, avatar, appearance));
    }
    if count == 0 {
        let (label, glyph) = if app.recent_projects.is_empty() {
            (i18n.__("暂无最近项目"), Icon::FolderOpen)
        } else {
            (i18n.__("没有匹配的项目"), Icon::Search)
        };
        projects = projects.push(
            container(
                column![
                    container(project_icon(glyph))
                        .center_x(44)
                        .center_y(44)
                        .style(settings_icon_tile_style),
                    text(label).size(14).style(settings_muted_text_style),
                ]
                .spacing(10)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .padding([22, 12])
            .style(settings_panel_style),
        );
    }
    let list_heading =
        row![text(i18n.__("历史项目")).size(17).width(Length::Fill), settings_value_badge(count),]
            .align_y(Alignment::Center);
    let heading = settings_page_heading(
        i18n.__("项目配置"),
        i18n.__("选择文件夹开始一个项目，或从下方最近项目继续。"),
    );

    column![
        row![container(heading).width(Length::Fill), open_folder]
            .spacing(12)
            .align_y(Alignment::Center),
        column![search, column![list_heading, projects].spacing(10)].spacing(16),
        text(i18n.__("当前项目")).size(17),
        settings_group_card(current_project_worktree(app, i18n, map, help)),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}
