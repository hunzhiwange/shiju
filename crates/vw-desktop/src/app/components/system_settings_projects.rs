//! 项目设置中的最近项目列表与当前项目的 Worktree 选项。

use crate::app::assets::Icon;
use crate::app::components::overlays::BelowOverlay;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, danger_action_btn_style, icon_btn,
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_divider, settings_group_card, settings_muted_text_style, settings_panel_style,
    settings_row_with_flexible_control, settings_text_input_style,
};
use crate::app::i18n::I18n;
use crate::app::message::{ProjectMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, svg, text, text_input};
use iced::{Alignment, Element, Length, Theme};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 110.0, control)
}

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
    field_row(i18n.__("启用 Worktree"), i18n.__("新建会话时可选工作区。"), control)
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
    let project = button(
        row![
            project_icon(Icon::FolderOpen),
            column![
                text(name).size(14).wrapping(text::Wrapping::None),
                text(path).size(12).wrapping(text::Wrapping::None).style(settings_muted_text_style),
            ]
            .spacing(3)
            .width(Length::Fill),
            project_icon(Icon::ChevronRight),
        ]
        .align_y(Alignment::Center)
        .spacing(10)
        .clip(true),
    )
    .clip(true)
    .padding([9, 10])
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

    let badge = text(i18n.__("当前项目")).size(11).style(settings_muted_text_style);
    let mut line = row![project].align_y(Alignment::Center).spacing(6);
    if is_current {
        line = line.push(badge);
    }
    let line = line.push(container(actions).padding(iced::padding::right(8)));
    let mut entry = column![container(line).width(Length::Fill).style(move |theme: &Theme| {
        container::Style {
            background: is_current
                .then(|| theme.extended_palette().background.weak.color.scale_alpha(0.52).into()),
            border: iced::Border { radius: 8.0.into(), ..Default::default() },
            ..Default::default()
        }
    })];

    if app.recent_project_delete_confirm_idx == Some(index) {
        entry = entry.push(
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
            .padding([6, 8]),
        );
    }
    entry.push(settings_divider()).into()
}

/// 渲染项目管理、搜索和 Worktree 快捷设置。
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let open_folder = button(text(i18n.__("打开文件夹")).size(13))
        .on_press(Message::Project(ProjectMessage::OpenFolderPressed))
        .padding([8, 14])
        .style(|theme, status| {
            let mut style = primary_action_btn_style(theme, status);
            style.border.radius = 18.0.into();
            style
        });

    let search = text_input(i18n.__("搜索项目"), &app.recent_project_query)
        .on_input(|query| Message::Settings(SettingsMessage::RecentProjectQueryChanged(query)))
        .padding(SETTINGS_CONTROL_PADDING)
        .size(SETTINGS_CONTROL_TEXT_SIZE)
        .style(|theme, status| {
            let mut style = settings_text_input_style(theme, status);
            style.border.radius = 18.0.into();
            style
        });
    let search = row![project_icon(Icon::Search), search].spacing(7).align_y(Alignment::Center);
    let toolbar = row![
        text(i18n.__("历史项目")).size(17),
        container(container(search).max_width(248)).align_right(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let query = app.recent_project_query.trim().to_lowercase();
    let mut projects = column![
        container(text(i18n.__("名称")).size(12).style(settings_muted_text_style)).padding([5, 10]),
        settings_divider(),
    ];
    let mut has_results = false;
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
        has_results = true;
        projects = projects.push(project_row(app, i18n, index, path, name));
    }
    if !has_results {
        let label = if app.recent_projects.is_empty() {
            i18n.__("暂无最近项目")
        } else {
            i18n.__("没有匹配的项目")
        };
        projects = projects.push(
            container(text(label).size(13).style(settings_muted_text_style))
                .center_x(Length::Fill)
                .padding([20, 8]),
        );
    }

    column![
        row![text(i18n.__("项目配置")).size(28).width(Length::Fill), open_folder,]
            .spacing(12)
            .align_y(Alignment::Center),
        column![toolbar, projects].spacing(8),
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
