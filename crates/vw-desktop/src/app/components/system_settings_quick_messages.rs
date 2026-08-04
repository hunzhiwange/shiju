//! 系统设置中的聊天快捷消息配置面板。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_muted_text_style, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_text_input_style,
};
use crate::app::message::SettingsMessage;
use crate::app::message::settings::QuickMessagesMessage;
use crate::app::{App, Message};
use iced::widget::{button, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Background, Border, Element, Length, Theme};
use vw_config_types::ui::{QuickMessageGroup, QuickMessageKind};

fn settings_message(message: QuickMessagesMessage) -> Message {
    Message::Settings(SettingsMessage::QuickMessages(message))
}

fn add_message() -> Message {
    settings_message(QuickMessagesMessage::Add)
}

fn save_message() -> Message {
    settings_message(QuickMessagesMessage::Save)
}

fn remove_message(index: usize) -> Message {
    settings_message(QuickMessagesMessage::Remove(index))
}

fn move_up_message(index: usize) -> Message {
    settings_message(QuickMessagesMessage::MoveUp(index))
}

fn move_down_message(index: usize) -> Message {
    settings_message(QuickMessagesMessage::MoveDown(index))
}

fn item_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.weak.color.scale_alpha(0.38))),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.7),
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

fn item_view(app: &App, index: usize) -> Element<'_, Message> {
    let item = &app.quick_messages[index];
    let group = item.group;
    let kind = item.kind;

    let text_control = text_input("输入快捷消息或动作名称", &item.text)
        .on_input(move |value| settings_message(QuickMessagesMessage::TextChanged(index, value)))
        .padding([9, 11])
        .size(13)
        .style(settings_text_input_style)
        .width(Length::Fill);

    let group_control = pick_list(QuickMessageGroup::ALL, Some(group), move |value| {
        settings_message(QuickMessagesMessage::GroupChanged(index, value))
    })
    .padding([9, 11])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(170.0));

    let kind_control = pick_list(QuickMessageKind::ALL, Some(kind), move |value| {
        settings_message(QuickMessagesMessage::KindChanged(index, value))
    })
    .padding([9, 11])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(190.0));

    let actions = row![
        button(text("上移").size(12))
            .on_press(move_up_message(index))
            .padding([7, 9])
            .style(rounded_action_btn_style),
        button(text("下移").size(12))
            .on_press(move_down_message(index))
            .padding([7, 9])
            .style(rounded_action_btn_style),
        button(text("删除").size(12))
            .on_press(remove_message(index))
            .padding([7, 9])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    container(
        column![
            row![
                text(format!("快捷项 {}", index + 1)).size(12).style(settings_muted_text_style),
                group_control,
                kind_control,
                actions,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text_control,
        ]
        .spacing(10)
        .width(Length::Fill),
    )
    .padding(12)
    .width(Length::Fill)
    .style(item_style)
    .into()
}

/// 渲染快捷消息设置面板及其编辑控件。
pub fn view(app: &App) -> Element<'_, Message> {
    let mut items = column![].spacing(10).width(Length::Fill);
    if app.quick_messages.is_empty() {
        items = items.push(
            container(
                text("暂无快捷消息；添加后会显示在聊天输入框上方。")
                    .size(12)
                    .style(settings_muted_text_style),
            )
            .padding([14, 4])
            .center_x(Length::Fill),
        );
    } else {
        for index in 0..app.quick_messages.len() {
            items = items.push(item_view(app, index));
        }
    }

    let footer = row![
        button(text("添加快捷项").size(13))
            .on_press(add_message())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        container(row![
            button(text("保存快捷消息").size(13))
                .on_press(save_message())
                .padding([9, 14])
                .style(primary_action_btn_style),
        ])
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    column![
        settings_section_card("快捷消息", "配置聊天输入框上方的常用消息、摘要接力与 Git 动作。"),
        settings_panel(column![items, footer].spacing(14).width(Length::Fill)),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_quick_messages_tests.rs"]
mod system_settings_quick_messages_tests;
