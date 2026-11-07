//! 系统设置中的聊天快捷消息配置面板。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, primary_action_btn_style,
    rounded_action_btn_style, settings_group_card, settings_muted_text_style,
    settings_page_heading, settings_section_content, settings_select, settings_text_input_style,
};
use crate::app::message::SettingsMessage;
use crate::app::message::settings::QuickMessagesMessage;
use crate::app::{App, Message, i18n::I18n};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
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

#[derive(Clone, PartialEq)]
struct LabeledOption<T>(T, &'static str);

impl<T> std::fmt::Display for LabeledOption<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

fn text_fields<'a>(
    index: usize,
    value: &'a str,
    placeholder: &'static str,
) -> Element<'a, Message> {
    let field = |line: &str| {
        text_input(placeholder, line)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .style(settings_text_input_style)
            .width(Length::Fill)
    };
    if !value.contains(['\r', '\n']) {
        return field(value)
            .on_input(move |edit| settings_message(QuickMessagesMessage::TextChanged(index, edit)))
            .into();
    }

    // Keep the stored separators and accumulate edits from every line in an event batch.
    let mut remaining = value;
    let mut lines = Vec::new();
    while let Some(end) = remaining.find(['\r', '\n']) {
        let line = &remaining[..end];
        let separator =
            if remaining[end..].starts_with("\r\n") { "\r\n" } else { &remaining[end..end + 1] };
        lines.push((Cow::Borrowed(line), separator));
        remaining = &remaining[end + separator.len()..];
    }
    lines.push((Cow::Borrowed(remaining), ""));

    let draft = Rc::new(RefCell::new(lines));
    let mut fields = column![].spacing(4).width(Length::Fill);
    for (line_index, (line, _)) in draft.borrow().iter().enumerate() {
        let draft = Rc::clone(&draft);
        fields = fields.push(field(line).on_input(move |edit| {
            let mut draft = draft.borrow_mut();
            draft[line_index].0 = Cow::Owned(edit);
            let mut value = String::new();
            for (line, separator) in draft.iter() {
                value.push_str(line);
                value.push_str(separator);
            }
            settings_message(QuickMessagesMessage::TextChanged(index, value))
        }));
    }
    fields.into()
}

fn item_view(app: &App, index: usize) -> Element<'_, Message> {
    let item = &app.quick_messages[index];
    let i18n = I18n::new(app.desktop_locale);
    let text_control = text_fields(index, &item.text, i18n.__("输入快捷消息或动作名称"));

    let group_option = |group: QuickMessageGroup| LabeledOption(group, i18n.__(group.label()));
    let group_control = settings_select(
        QuickMessageGroup::ALL.map(group_option),
        Some(group_option(item.group)),
        move |option| settings_message(QuickMessagesMessage::GroupChanged(index, option.0)),
    )
    .width(Length::FillPortion(1));

    let kind_option = |kind: QuickMessageKind| LabeledOption(kind, i18n.__(kind.label()));
    let kind_control = settings_select(
        QuickMessageKind::ALL.map(kind_option),
        Some(kind_option(item.kind)),
        move |option| settings_message(QuickMessagesMessage::KindChanged(index, option.0)),
    )
    .width(Length::FillPortion(1));

    let actions = row![
        button(text(i18n.__("上移")).size(12))
            .on_press_maybe((index > 0).then(|| move_up_message(index)))
            .padding([7, 9])
            .style(rounded_action_btn_style),
        button(text(i18n.__("下移")).size(12))
            .on_press_maybe(
                (index + 1 < app.quick_messages.len()).then(|| move_down_message(index))
            )
            .padding([7, 9])
            .style(rounded_action_btn_style),
        button(text(i18n.__("删除")).size(12))
            .on_press(remove_message(index))
            .padding([7, 9])
            .style(rounded_action_btn_style),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    settings_group_card(
        container(
            column![
                row![
                    text(
                        crate::__!(i18n, "快捷项 %u", index + 1)
                            .expect("valid quick item template")
                    )
                    .size(12)
                    .style(settings_muted_text_style),
                    container(actions)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                row![group_control, kind_control].spacing(10).width(Length::Fill),
                text_control,
            ]
            .spacing(10)
            .width(Length::Fill),
        )
        .padding([14, 0]),
    )
    .into()
}

/// 渲染快捷消息设置面板及其编辑控件。
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let mut items = column![].spacing(10).width(Length::Fill);
    if app.quick_messages.is_empty() {
        items = items.push(
            container(
                text(i18n.__("暂无快捷消息；添加后会显示在聊天输入框上方。"))
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
        button(text(i18n.__("添加快捷项")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .on_press(add_message())
            .padding([9, 14])
            .style(rounded_action_btn_style),
        container(row![
            button(text(i18n.__("保存快捷消息")).size(SETTINGS_CONTROL_TEXT_SIZE))
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
        settings_page_heading(
            i18n.__("快捷消息"),
            i18n.__("配置聊天输入框上方的常用消息、摘要接力与 Git 动作。")
        ),
        settings_section_content(column![items, footer].spacing(14).width(Length::Fill)),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_quick_messages_tests.rs"]
mod system_settings_quick_messages_tests;
