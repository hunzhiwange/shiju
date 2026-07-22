//! 聊天输入栏的会话级推理强度选择器。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

use super::dark_tooltip_content;
use super::icons::icon_svg;
use super::styles::{
    BOTTOM_BAR_CHEVRON_ICON_SIZE, BOTTOM_BAR_ICON_SIZE, BOTTOM_BAR_LABEL_SIZE, popover_style,
    selectable_list_button_style, selector_chevron_color, selector_label_font,
    selector_pill_button_style, selector_text_color,
};
use crate::app::assets::Icon;
use crate::app::components::overlays::AboveOverlay;
use crate::app::{App, Message, message};

const SELECTOR_WIDTH: f32 = 180.0;
const ITEM_HEIGHT: f32 = 34.0;
const MAX_LIST_HEIGHT: f32 = 220.0;
const SCROLLBAR_WIDTH: f32 = 4.0;

const CODEX_OPTIONS: [Option<ReasoningEffortDto>; 6] = [
    None,
    Some(ReasoningEffortDto::None),
    Some(ReasoningEffortDto::Low),
    Some(ReasoningEffortDto::Medium),
    Some(ReasoningEffortDto::High),
    Some(ReasoningEffortDto::Xhigh),
];
const CLAUDE_OPTIONS: [Option<ReasoningEffortDto>; 3] =
    [None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max)];
const DEFAULT_OPTIONS: [Option<ReasoningEffortDto>; 1] = [None];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReasoningEffortItem {
    pub(super) value: Option<ReasoningEffortDto>,
    pub(super) label: &'static str,
    pub(super) selected: bool,
}

pub(crate) fn reasoning_effort_label(value: Option<ReasoningEffortDto>) -> &'static str {
    match value {
        None => "默认",
        Some(ReasoningEffortDto::None) => "None",
        Some(ReasoningEffortDto::Low) => "Low",
        Some(ReasoningEffortDto::Medium) => "Medium",
        Some(ReasoningEffortDto::High) => "High",
        Some(ReasoningEffortDto::Xhigh) => "Xhigh",
        Some(ReasoningEffortDto::Max) => "Max",
    }
}

pub(crate) fn reasoning_effort_options(agent: &str) -> &'static [Option<ReasoningEffortDto>] {
    match agent.trim().to_ascii_lowercase().as_str() {
        "codex" | "codex cli" | "codex-cli" => &CODEX_OPTIONS,
        "claude" | "claude code" | "claude-code" | "claudecode" => &CLAUDE_OPTIONS,
        _ => &DEFAULT_OPTIONS,
    }
}

pub(super) fn reasoning_effort_items(
    agent: &str,
    current: Option<ReasoningEffortDto>,
) -> Vec<ReasoningEffortItem> {
    reasoning_effort_options(agent)
        .iter()
        .copied()
        .map(|value| ReasoningEffortItem {
            value,
            label: reasoning_effort_label(value),
            selected: value == current,
        })
        .collect()
}

pub(super) fn reasoning_effort_message(value: Option<ReasoningEffortDto>) -> Message {
    Message::Chat(message::ChatMessage::ReasoningEffortSelected(value))
}

pub(super) fn reasoning_effort_selector<'a>(
    app: &'a App,
    agent: Option<&str>,
    current: Option<ReasoningEffortDto>,
) -> Option<Element<'a, Message>> {
    let agent = agent?;
    let expanded = app.show_reasoning_effort_popover;
    let icon = icon_svg(Icon::Speedometer2, BOTTOM_BAR_ICON_SIZE).style(move |theme: &Theme, _| {
        svg::Style { color: Some(selector_text_color(theme, expanded || current.is_some())) }
    });
    let chevron = icon_svg(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        BOTTOM_BAR_CHEVRON_ICON_SIZE,
    )
    .style(move |theme: &Theme, _| svg::Style {
        color: Some(selector_chevron_color(theme, expanded)),
    });
    let toggle = button(
        row![
            icon,
            text(reasoning_effort_label(current))
                .size(BOTTOM_BAR_LABEL_SIZE)
                .font(selector_label_font()),
            chevron,
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(move |theme: &Theme, status| {
        selector_pill_button_style(theme, status, expanded || current.is_some())
    })
    .on_press(Message::View(message::ViewMessage::ToggleReasoningEffortPopover));
    let toggle: Element<'_, Message> =
        Tooltip::new(toggle, dark_tooltip_content("选择本会话的推理强度"), TooltipPosition::Top)
            .gap(6.0)
            .into();

    let items = reasoning_effort_items(agent, current);
    let list_height = (items.len() as f32 * ITEM_HEIGHT).min(MAX_LIST_HEIGHT);
    let mut list = column![].spacing(4);
    for item in items {
        let check: Element<'_, Message> = if item.selected {
            icon_svg(Icon::Check, 14.0).into()
        } else {
            Space::new().width(Length::Fixed(14.0)).into()
        };
        let value = item.value;
        list = list.push(
            button(
                row![text(item.label).size(13), Space::new().width(Length::Fill), check]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme, status| {
                selectable_list_button_style(theme, status, item.selected)
            })
            .on_press(reasoning_effort_message(value)),
        );
    }

    let popover: Element<'_, Message> = container(
        scrollable(container(list).padding(iced::Padding {
            top: 0.0,
            right: 5.0,
            bottom: 0.0,
            left: 0.0,
        }))
        .id(iced::widget::Id::new("input_panel_reasoning_effort_scroll"))
        .direction(Direction::Vertical(
            Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
        ))
        .height(Length::Fixed(list_height)),
    )
    .style(popover_style)
    .padding(8)
    .width(Length::Fixed(SELECTOR_WIDTH))
    .into();

    Some(
        AboveOverlay::new(toggle, popover)
            .show(expanded)
            .gap(6.0)
            .on_close(Message::View(message::ViewMessage::CloseReasoningEffortPopover))
            .into(),
    )
}
