//! Slash 快捷选择面板 overlay。
//!
//! Cursor 风格分组列表：Skills / Tools / MCP / Models / ACP，支持 Show more 与键盘高亮。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::svg;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Background, Border, Color, Element, Length, Padding, Theme};

use crate::app::assets::{self, Icon};
use crate::app::message::chat::input::ranked_slash_palette_sections;
use crate::app::state::SlashPaletteKind;
use crate::app::{App, Message, message};

const SCROLLBAR_WIDTH: f32 = 4.0;
const OVERLAY_WIDTH: f32 = 360.0;
const LIST_HEIGHT: f32 = 320.0;

fn kind_icon(kind: SlashPaletteKind) -> Icon {
    match kind {
        SlashPaletteKind::Skill => Icon::Star,
        SlashPaletteKind::Tool => Icon::Terminal,
        SlashPaletteKind::Mcp => Icon::HddNetwork,
        SlashPaletteKind::Model => Icon::Speedometer2,
        SlashPaletteKind::Acp => Icon::Robot,
    }
}

fn section_header_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.55)) }
}

fn empty_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().secondary.base.text) }
}

fn query_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().primary) }
}

fn description_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.58)) }
}

fn icon_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.70)) }
}

fn entry_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let background = if hovered || selected {
        Some(Background::Color(palette.background.weak.color))
    } else {
        None
    };
    iced::widget::button::Style {
        background,
        border: Border { radius: 6.0.into(), ..Default::default() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

fn show_more_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    iced::widget::button::Style {
        background: hovered
            .then_some(Background::Color(theme.extended_palette().background.weak.color)),
        border: Border { radius: 6.0.into(), ..Default::default() },
        text_color: theme.palette().text.scale_alpha(0.72),
        ..Default::default()
    }
}

fn overlay_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border { width: 1.0, color: palette.background.strong.color, radius: 12.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.12),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

fn highlighted_slash_query(query: &str) -> String {
    format!("/{}", query.trim())
}

/// 构建 Slash 分组 overlay。
pub fn slash_palette_overlay(app: &App) -> Element<'_, Message> {
    let sections = ranked_slash_palette_sections(app);
    let highlighted_query = highlighted_slash_query(&app.slash_palette_query);

    let mut list = column![].spacing(6);
    if sections.is_empty() {
        list = list.push(text("No matching items").size(13).style(empty_text_style));
    } else {
        let mut flat_index = 0usize;
        for section in &sections {
            list = list.push(
                container(text(section.kind.label()).size(11).style(section_header_style))
                    .padding(Padding::from([4, 8])),
            );

            for entry in &section.entries {
                let selected = flat_index == app.slash_palette_selected_index;
                flat_index += 1;

                let icon = svg::Svg::<Theme>::new(assets::get_icon(kind_icon(entry.kind)))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(icon_style);

                let mut label_col = column![text(entry.label.clone()).size(13)].spacing(2);
                if let Some(description) = entry.description.as_deref().filter(|d| !d.is_empty()) {
                    label_col = label_col
                        .push(text(description.to_string()).size(11).style(description_text_style));
                }

                let row_content =
                    row![icon, label_col].spacing(8).align_y(iced::alignment::Vertical::Center);

                let kind = entry.kind;
                let id = entry.id.clone();
                let btn = button(row_content)
                    .width(Length::Fill)
                    .padding([6, 10])
                    .style(move |theme: &Theme, status| entry_style(theme, status, selected))
                    .on_press(Message::Chat(message::ChatMessage::SlashPaletteSelect { kind, id }));
                list = list.push(btn);
            }

            if section.hidden_count > 0 {
                let kind = section.kind;
                let label = format!("Show {} more", section.hidden_count);
                let more = button(text(label).size(12))
                    .width(Length::Fill)
                    .padding([4, 10])
                    .style(show_more_style)
                    .on_press(Message::Chat(message::ChatMessage::SlashPaletteExpandKind(kind)));
                list = list.push(more);
            }

            list = list.push(Space::new().height(Length::Fixed(2.0)));
        }
    }

    container(
        column![
            container(text(highlighted_query).size(12).style(query_text_style)).padding([2, 2]),
            scrollable(list)
                .id(app.slash_palette_scroll_id.clone())
                .height(Length::Fixed(LIST_HEIGHT))
                .direction(Direction::Vertical(
                    Scrollbar::new().width(SCROLLBAR_WIDTH).scroller_width(SCROLLBAR_WIDTH),
                )),
        ]
        .spacing(6),
    )
    .padding([6, 8])
    .width(Length::Fixed(OVERLAY_WIDTH))
    .style(overlay_style)
    .into()
}
