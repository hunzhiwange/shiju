//! 项目设置中的最近项目列表与当前项目的 Worktree 选项。

use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style, icon_btn,
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_divider, settings_flexible_row_with_help, settings_group_card,
    settings_icon_tile_style, settings_muted_text_style, settings_page_heading,
    settings_panel_style, settings_text_input_style, settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::views::home::{parse_hex_color, project_avatar};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, svg, text, text_input};
use iced::{Alignment, Element, Length, Theme};

fn current_project_worktree(app: &App, i18n: I18n) -> Element<'_, Message> {
    let control: Element<'_, Message> = if let Some(path) = &app.project_path {
        let checked = app.project_worktree_enabled.get(path).copied().unwrap_or(false);
        checkbox(checked)
            .label(i18n.__("启用"))
            .on_toggle(move |value| {
                Message::Settings(SettingsMessage::ProjectEnableWorktreeToggled(
                    path.clone(),
                    value,
                ))
            })
            .style(settings_checkbox_style)
            .into()
    } else {
        text(i18n.__("请先打开项目")).size(12).style(settings_muted_text_style).into()
    };
    settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
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

fn menu_item(icon: Icon, label: &'static str, message: Message) -> Element<'static, Message> {
    button(row![project_icon(icon), text(label).size(13)].spacing(9).align_y(Alignment::Center))
        .padding([8, 9])
        .width(Length::Fill)
        .on_press(message)
        .style(quiet_button_style)
        .into()
}

fn project_row<'a>(
    app: &App,
    i18n: I18n,
    index: usize,
    path: &'a str,
    name: &'a str,
) -> Element<'a, Message> {
    let is_current = app.project_path.as_deref() == Some(path);
    let is_menu_open = app.project_tools_menu_path.as_deref() == Some(path);
    let meta = app.recent_projects_meta.iter().find(|meta| meta.path == path);
    let icon = meta.and_then(|meta| meta.icon.as_deref());
    let color = meta.and_then(|meta| meta.icon_color.as_deref()).and_then(parse_hex_color);
    let mut summary = row![
        project_avatar(name, path, icon, color, 44.0),
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
        .on_press(Message::Project(ProjectMessage::OpenRecentPressed(path.to_owned())))
        .style(quiet_button_style);

    let trigger = icon_btn(
        Icon::DotsThreeVertical,
        i18n.__("项目操作"),
        Some(Message::Project(ProjectMessage::ProjectToolsMenuToggled(path.to_owned()))),
    );
    let actions = if is_menu_open {
        let menu = container(
            column![
                menu_item(
                    Icon::Pencil,
                    i18n.__("编辑项目"),
                    Message::Project(ProjectMessage::ProjectEditOpened(path.to_owned())),
                ),
                settings_divider(),
                menu_item(
                    Icon::Trash,
                    i18n.__("删除"),
                    Message::Settings(SettingsMessage::RecentProjectDeleteRequested(index)),
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
                .on_close(Message::Project(ProjectMessage::ProjectToolsMenuClosed)),
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
                        .on_press(Message::Settings(SettingsMessage::RecentProjectDeleteCanceled))
                        .padding([7, 10])
                        .style(rounded_action_btn_style),
                    button(text(i18n.__("确认删除")).size(13))
                        .on_press(Message::Settings(SettingsMessage::RecentProjectDeleteConfirmed(
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
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let open_folder = button(text(i18n.__("打开文件夹")).size(SETTINGS_CONTROL_TEXT_SIZE))
        .on_press(Message::Project(ProjectMessage::OpenFolderPressed))
        .padding([8, 14])
        .style(primary_action_btn_style);

    let search = text_input(i18n.__("搜索项目"), &app.recent_project_query)
        .on_input(|query| Message::Settings(SettingsMessage::RecentProjectQueryChanged(query)))
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
        projects = projects.push(project_row(app, i18n, index, path, name));
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
        settings_group_card(current_project_worktree(app, i18n)),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_projects_tests.rs"]
mod system_settings_projects_tests;

#[cfg(test)]
#[path = "system_settings_projects_ui_refresh_30_tests.rs"]
mod system_settings_projects_ui_refresh_30_tests;
