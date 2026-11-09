//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

mod common;
mod general_tab;
mod launch_tab;
mod refresh_tab;
mod scheduling_tab;

use crate::app::components::color_picker::{parse_color, render_color_picker};
use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_modal_header, settings_modal_layout, settings_modal_overlay_with_backdrop_dismiss,
    settings_tab_row,
};
use crate::app::i18n::I18n;
use crate::app::message::ProjectMessage;
use crate::app::state::ProjectEditTab;
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Color, Element, Length};

use super::{App, Message};
use common::{format_hex_color, parse_hex_color, tab_button};
use general_tab::general_tab;
use launch_tab::launch_tab;
use refresh_tab::refresh_tab;
use scheduling_tab::scheduling_tab;

pub(crate) fn with_project_edit<'a>(
    app: &'a App,
    root_content: Element<'a, Message>,
) -> Element<'a, Message> {
    let Some(path) = &app.project_edit_path else {
        return root_content;
    };
    let i18n = I18n::new(app.desktop_locale);
    let cancel = Message::Project(ProjectMessage::ProjectEditCanceled);
    let tabs = settings_tab_row([
        tab_button(
            i18n.__("基础信息"),
            ProjectEditTab::General,
            app.project_edit_tab == ProjectEditTab::General,
        ),
        tab_button(
            i18n.__("启动与工作区"),
            ProjectEditTab::Launch,
            app.project_edit_tab == ProjectEditTab::Launch,
        ),
        tab_button(
            i18n.__("刷新策略"),
            ProjectEditTab::Refresh,
            app.project_edit_tab == ProjectEditTab::Refresh,
        ),
        tab_button(
            i18n.__("调度与保护"),
            ProjectEditTab::Scheduling,
            app.project_edit_tab == ProjectEditTab::Scheduling,
        ),
    ])
    .wrap()
    .vertical_spacing(8);
    let header = column![
        settings_modal_header(
            text(i18n.__("编辑项目")).size(18),
            settings_close_button(cancel.clone()),
        ),
        tabs,
    ]
    .spacing(10);

    let body = match app.project_edit_tab {
        ProjectEditTab::General => general_tab(app, path),
        ProjectEditTab::Launch => launch_tab(app),
        ProjectEditTab::Refresh => refresh_tab(app),
        ProjectEditTab::Scheduling => scheduling_tab(app),
    };
    let actions = row![
        Space::new().width(Length::Fill),
        button(text(i18n.__("取消")).size(13))
            .on_press(cancel.clone())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        button(text(i18n.__("保存")).size(13))
            .on_press(Message::Project(ProjectMessage::ProjectEditSaved))
            .padding([9, 14])
            .style(primary_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let card = settings_modal_layout(header, body, Some(actions.into()))
        .width(Length::Fixed(760.0))
        .height(Length::Fixed(590.0));
    let project_modal = settings_modal_overlay_with_backdrop_dismiss(root_content, cancel, card);
    if !app.project_edit_icon_color_picker_open {
        return project_modal;
    }

    let color = parse_color(&app.project_edit_icon_color)
        .or_else(|| parse_hex_color(&app.project_edit_icon_color))
        .unwrap_or(Color::from_rgb8(96, 165, 250));
    let close = Message::Project(ProjectMessage::ProjectEditIconColorPickerClosed);
    let card = settings_modal_card(
        column![
            settings_modal_header(
                text(i18n.__("选择图标颜色")).size(16),
                settings_close_button(close.clone()),
            ),
            render_color_picker(
                color,
                app.project_edit_icon_color_format,
                false,
                |color| Message::Project(ProjectMessage::ProjectEditIconColorChanged(
                    format_hex_color(color),
                )),
                |format| Message::Project(ProjectMessage::ProjectEditIconColorFormatChanged(
                    format
                )),
                || Message::None,
            ),
            row![
                Space::new().width(Length::Fill),
                button(text(i18n.__("确定")).size(13))
                    .on_press(close.clone())
                    .padding([9, 14])
                    .style(primary_action_btn_style),
            ],
        ]
        .spacing(12),
    )
    .width(Length::Fixed(300.0));
    settings_modal_overlay_with_backdrop_dismiss(project_modal, close, card)
}

#[cfg(test)]
mod tests;
