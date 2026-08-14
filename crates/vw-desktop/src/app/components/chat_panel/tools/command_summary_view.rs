//! 连续命令的折叠摘要视图。

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::tool_text_support::chat_text_font;
use crate::app::components::chat_panel::utils::{chat_secondary_subtle_text_color, icon_svg};
use crate::app::{App, Message, message};

use super::{CommandItem, tool_bash_view, tool_name_from_raw};

pub(super) const COMMAND_LIST_TOP_SPACING: f32 = 10.0;

pub(crate) fn is_command_tool(raw: &str) -> bool {
    tool_name_from_raw(raw).as_deref() == Some("bash")
}

fn command_summary_button_style(
    theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        border: iced::Border::default(),
        text_color: chat_secondary_subtle_text_color(theme),
        ..Default::default()
    }
}

pub(crate) fn tool_command_summary_view<'a>(
    app: &'a App,
    msg_idx: usize,
    items: &[CommandItem],
) -> Option<Element<'a, Message>> {
    let first_tool_idx = items.first()?.tool_idx;
    let key = ((msg_idx as u64) << 32) | first_tool_idx as u64;
    let expanded = app.chat_tool_expanded.contains(&key);

    let toggle = button(
        icon_svg(if expanded { Icon::ChevronUp } else { Icon::ChevronDown })
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(|theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            }),
    )
    .padding([0, 1])
    .style(command_summary_button_style)
    .on_press(Message::Chat(message::ChatMessage::ToggleTool(msg_idx, first_tool_idx)));

    let head = container(
        row![
            text("运行了命令")
                .size(13)
                .font(chat_text_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chat_secondary_subtle_text_color(theme)),
                }),
            text(format!("{} 条", items.len())).size(13).style(|theme: &Theme| {
                iced::widget::text::Style { color: Some(chat_secondary_subtle_text_color(theme)) }
            }),
            toggle,
            container(Space::new()).width(Length::Fill),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill);

    let mut content = column![head].spacing(COMMAND_LIST_TOP_SPACING);
    if expanded {
        let mut commands = column![].spacing(3);
        for item in items {
            if let Some(view) = tool_bash_view(app, msg_idx, item.tool_idx, &item.raw) {
                commands = commands.push(view);
            }
        }
        content = content.push(container(commands).width(Length::Fill));
    }

    Some(container(content).width(Length::Fill).into())
}
