use desktop_i18n::I18n;
use desktop_ui_controls::overlays::AboveOverlay;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};

use super::quick_message::Message;
use super::quick_messages::{dropdown_icon, shortcut_trigger};
use super::quick_styles;

const MODEL_COMMANDS: &[(&str, &str, bool)] = &[
    ("/model", "查看当前模型", false),
    ("/model list", "查看可用模型", false),
    ("/model shiju-direct:", "补充模型名以切换模型", true),
    ("/model reset", "恢复全局默认模型", false),
    ("/model info shiju-direct:", "补充模型名以查看详情", true),
    ("/model help", "查看模型命令帮助", false),
];

const CONVERSATION_COMMANDS: &[(&str, &str, bool)] = &[
    ("/new", "开启新对话并保存长期记忆", false),
    ("/compact", "压缩当前对话", false),
    ("/clear", "清空对话上下文", false),
    ("/stop", "停止当前任务", false),
    ("/history", "查看对话消息和上下文用量", false),
    ("/message ", "补充序号以查看消息详情", true),
    ("/compact_str", "查看当前压缩摘要", false),
    ("/summarize_status", "查看后台摘要状态", false),
    ("/dump_history", "导出对话历史", false),
    ("/load_history", "从文件载入历史并替换当前对话", false),
];

pub(super) fn menu(i18n: I18n, open: bool) -> Element<'static, Message> {
    let trigger = shortcut_trigger(
        row![text(i18n.__("数字员工指令")).size(12), dropdown_icon()]
            .spacing(5)
            .align_y(Alignment::Center),
        Message::EmployeeCommandMenuChanged(!open),
        open,
    );

    let mut items = column![].spacing(3).width(Length::Fill);
    for (title, commands) in [("模型命令", MODEL_COMMANDS), ("对话命令", CONVERSATION_COMMANDS)]
    {
        items = items.push(container(text(i18n.__(title)).size(12)).padding([5, 8]));
        for &(input, description, needs_argument) in commands {
            items = items.push(
                button(
                    column![text(input.trim_end()).size(13), text(i18n.__(description)).size(11)]
                        .spacing(2),
                )
                .on_press(Message::EmployeeCommandSelected { input, needs_argument })
                .padding([6, 8])
                .width(Length::Fill)
                .style(|theme, status| {
                    quick_styles::shiju_skill_button_style(theme, status, false)
                }),
            );
        }
    }

    let menu =
        container(scrollable(items).height(Length::Fixed(280.0)).direction(Direction::Vertical(
            Scrollbar::new().width(4).scroller_width(4).spacing(7),
        )))
        .padding(6)
        .width(Length::Fixed(260.0))
        .style(quick_styles::quick_message_menu_style);

    AboveOverlay::new(trigger, menu)
        .show(open)
        .gap(2.0)
        .block_background_events(true)
        .on_close(Message::EmployeeCommandMenuChanged(false))
        .into()
}
