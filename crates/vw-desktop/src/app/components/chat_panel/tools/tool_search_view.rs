//! 工具搜索的单行聊天流视图。
//!
//! 聊天流只展示搜索摘要，完整结果统一交给工具详情弹窗。

use iced::widget::{Space, button, container, mouse_area, row, text};
use iced::{Alignment, Element, Length, Padding, Theme};
use serde_json::Value;

use crate::app::assets::Icon;
use crate::app::components::chat_panel::utils::{
    chat_secondary_muted_text_color, eye_icon_button_style, eye_icon_svg_style, icon_svg,
    truncate_chars,
};
use crate::app::{App, Message, message};

use super::tool_meta::tool_header_title;
use super::tool_parse::{tool_name_from_raw, tool_status, tool_summary_text};

pub fn tool_search_view<'a>(
    _app: &'a App,
    msg_idx: usize,
    tool_idx: usize,
    visible: &str,
) -> Option<Element<'a, Message>> {
    let (_, rest) = visible.split_once('\n')?;
    if tool_name_from_raw(visible)?.as_str() != "tool_search" {
        return None;
    }

    let value = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = tool_status(&value);
    let is_error = matches!(status, "error" | "denied");
    let title = match status {
        "running" => "tool_search 运行中",
        "error" | "denied" => "tool_search 失败",
        _ => "tool_search",
    };
    let summary = tool_search_summary(&value);

    let detail_btn = button(
        icon_svg(Icon::ChevronRight)
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(eye_icon_svg_style),
    )
    .padding([2, 4])
    .style(|theme: &Theme, status| eye_icon_button_style(theme, status))
    .on_press(Message::Chat(message::ChatMessage::OpenToolDetail(
        msg_idx,
        tool_idx,
        visible.to_string(),
    )));

    let summary = text(summary).size(13).style(move |theme: &Theme| iced::widget::text::Style {
        color: Some(if is_error {
            theme.extended_palette().danger.base.color
        } else {
            chat_secondary_muted_text_color(theme)
        }),
    });

    Some(
        mouse_area(
            container(
                row![
                    tool_header_title("tool_search", title, is_error),
                    summary,
                    container(Space::new()).width(Length::Fill),
                    detail_btn,
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(Padding { top: 4.0, right: 6.0, bottom: 4.0, left: 0.0 })
            .width(Length::Fill),
        )
        .on_enter(Message::Chat(message::ChatMessage::ToolHover(msg_idx, tool_idx)))
        .on_exit(Message::Chat(message::ChatMessage::ToolHoverLeave))
        .into(),
    )
}

pub(super) fn tool_search_summary(value: &Value) -> String {
    tool_summary_text(value)
        .filter(|summary| !summary.trim().is_empty())
        .map(|summary| truncate_chars(summary.trim(), 80).to_string())
        .unwrap_or_else(|| "查看详情".to_string())
}
