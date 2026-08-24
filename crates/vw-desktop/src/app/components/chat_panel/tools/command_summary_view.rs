//! 连续命令的折叠摘要视图。

use iced::widget::{Space, button, column, container, mouse_area};
use iced::{Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{chat_secondary_subtle_text_color, icon_svg};
use crate::app::{App, Message, message};

use super::tool_meta::{ToolVisual, ToolVisualState, tool_activity_header, tool_visual_state};
use super::tool_parse::tool_status;
use super::{CommandItem, tool_bash_view, tool_name_from_raw};

pub(super) const COMMAND_LIST_TOP_SPACING: f32 = 10.0;
const COMMAND_TOGGLE_SLOT_WIDTH: f32 = 12.0;

pub(crate) fn is_command_tool(raw: &str) -> bool {
    tool_name_from_raw(raw).as_deref() == Some("bash")
}

fn command_summary_visual(items: &[CommandItem]) -> ToolVisual {
    let visuals = items.iter().filter_map(|item| {
        let (_, rest) = item.raw.split_once('\n')?;
        let value = serde_json::from_str::<serde_json::Value>(rest.trim()).ok()?;
        Some(tool_visual_state(tool_status(&value)))
    });
    let states = visuals.map(|visual| visual.state).collect::<Vec<_>>();

    if states.contains(&ToolVisualState::Running) {
        tool_visual_state("running")
    } else if states.contains(&ToolVisualState::Waiting) {
        tool_visual_state("waiting")
    } else if states.contains(&ToolVisualState::Failed) {
        tool_visual_state("failed")
    } else if states.contains(&ToolVisualState::Cancelled) {
        tool_visual_state("cancelled")
    } else if !states.is_empty()
        && states.iter().all(|state| *state == ToolVisualState::Completed)
    {
        tool_visual_state("completed")
    } else {
        tool_visual_state("")
    }
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
    let is_hovered = app.chat_tool_hovered_idx == Some(key);

    let toggle = button(
        icon_svg(if expanded { Icon::ChevronDown } else { Icon::ChevronRight })
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(|theme: &Theme, _status| iced::widget::svg::Style {
                color: Some(chat_secondary_subtle_text_color(theme)),
            }),
    )
    .padding([0, 1])
    .style(command_summary_button_style)
    .on_press(Message::Chat(message::ChatMessage::ToggleTool(msg_idx, first_tool_idx)));
    let toggle_slot: Element<'a, Message> = if is_hovered {
        toggle.into()
    } else {
        Space::new().width(Length::Fixed(COMMAND_TOGGLE_SLOT_WIDTH)).into()
    };

    let head = mouse_area(
        container(tool_activity_header(
            "bash",
            "运行了命令",
            format!("{} 条", items.len()),
            command_summary_visual(items),
            toggle_slot,
        ))
        .width(Length::Fill),
    )
    .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, first_tool_idx)))
    .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave));

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
