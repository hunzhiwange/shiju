//! 内置伙伴目录与无副作用预览，选择复用 Mini 偏好保存消息。
use super::system_settings_common::{rounded_action_btn_style, settings_group_card};
use super::task_pet::avatar_preview;
use crate::app::i18n::I18n;
use crate::app::message::view::ViewMessage;
use crate::app::state::TaskPetAvatarKind;
use crate::app::{App, Message};
use iced::widget::{button, column, container, responsive, row, text};
use iced::{Alignment, Border, Element, Length};

const COMPANIONS: [(TaskPetAvatarKind, &str, &str); 4] = [
    (TaskPetAvatarKind::None, "无", "仅保留迷你工具栏与任务卡片。"),
    (TaskPetAvatarKind::Robot, "识矩小机器人", "陪你专注工作的蓝色伙伴。"),
    (TaskPetAvatarKind::Beauty, "识矩小雅", "温柔陪伴，与你一起整理思绪。"),
    (TaskPetAvatarKind::Handsome, "识矩小朗", "从容同行，与你一起完成任务。"),
];

fn preview_id(avatar: TaskPetAvatarKind) -> &'static str {
    match avatar {
        TaskPetAvatarKind::None => "companion-preview-none",
        TaskPetAvatarKind::Robot => "companion-preview-robot",
        TaskPetAvatarKind::Beauty => "companion-preview-beauty",
        TaskPetAvatarKind::Handsome => "companion-preview-handsome",
    }
}

pub fn content(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let header = row![
        text(i18n.__("迷你界面和智能伙伴")).size(22).width(Length::Fill),
        button(
            text(i18n.__(if app.task_pet_visible { "隐藏 Mini" } else { "显示 Mini" })).size(13)
        )
        .on_press(Message::View(ViewMessage::TaskPetToggleWindow))
        .padding([8, 14])
        .style(rounded_action_btn_style),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let preview = column![
        avatar_preview(app.task_pet_avatar_kind, 96.0 * app.task_pet_preview_scale()),
        // 预览只显示外观，不接受工具栏或操作系统窗口事件。
        container(text(i18n.__("迷你工具栏")).size(13))
            .padding([8, 16])
            .style(container::rounded_box),
    ]
    .spacing(8)
    .align_x(Alignment::Center);
    let preview = settings_group_card(
        container(preview)
            .id(preview_id(app.task_pet_avatar_kind))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(240.0)),
    );
    let cards = responsive(move |size| {
        let columns = if size.width >= 720.0 {
            3
        } else if size.width >= 440.0 {
            2
        } else {
            1
        };
        let width = ((size.width - 12.0 * (columns - 1) as f32) / columns as f32).max(1.0);
        let mut grid = column![].spacing(12).align_x(Alignment::Center);
        for group in COMPANIONS.chunks(columns) {
            let mut line = row![].spacing(12).align_y(Alignment::Center);
            for &(avatar, name, description) in group {
                let selected = avatar == app.task_pet_avatar_kind;
                let card = column![
                    container(avatar_preview(avatar, 72.0)).center_x(Length::Fill).center_y(100),
                    text(i18n.__(name)).size(16),
                    text(i18n.__(description)).size(12).align_x(Alignment::Center),
                    text(i18n.__(if selected { "已选择" } else { "选择伙伴" })).size(12),
                ]
                .spacing(8)
                .align_x(Alignment::Center);
                line = line.push(
                    button(container(card).center_x(Length::Fill).center_y(Length::Fill))
                        .on_press(Message::View(ViewMessage::TaskPetAvatarSelected(avatar)))
                        .width(width)
                        .height(240)
                        .padding(12)
                        .style(move |theme, status| {
                            let mut style = rounded_action_btn_style(theme, status);
                            style.border = Border {
                                radius: 12.0.into(),
                                width: if selected { 2.0 } else { 1.0 },
                                color: if selected {
                                    theme.palette().primary
                                } else {
                                    theme.extended_palette().background.strong.color
                                },
                            };
                            style
                        }),
                );
            }
            grid = grid.push(line);
        }
        grid.into()
    });
    column![
        header,
        text(i18n.__("选择伙伴会立即同步预览与迷你界面。显示与隐藏单独控制。")).size(13),
        preview,
        container(
            button(text(i18n.__("自定义伙伴大小")).size(13))
                .on_press(Message::View(ViewMessage::TaskPetSizeOpen))
                .padding([8, 14])
                .style(rounded_action_btn_style)
        )
        .align_right(Length::Fill),
        text(i18n.__("我的智能伙伴")).size(18),
        cards,
    ]
    .spacing(20)
    .into()
}

#[cfg(test)]
#[path = "system_settings_companion_tests.rs"]
mod tests;

pub(crate) mod size_modal;
