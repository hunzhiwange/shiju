//! 主题编辑器实时预览面板。

use crate::app::Message;
use crate::app::state::ThemeEditorDraft;
use crate::app::theme_catalog::{
    ThemeAssets, ThemeChrome, ThemeDefinition, ThemePaletteHex, to_iced_theme,
};
use iced::widget::{button, column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Theme};

fn parse_hex_or(value: &str, fallback: Color) -> Color {
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes[0] == b'#' {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&value[1..3], 16),
            u8::from_str_radix(&value[3..5], 16),
            u8::from_str_radix(&value[5..7], 16),
        ) {
            return Color::from_rgb8(r, g, b);
        }
    }
    fallback
}

fn definition_from_draft(draft: &ThemeEditorDraft) -> ThemeDefinition {
    ThemeDefinition {
        id: if draft.id.is_empty() { "preview".to_string() } else { draft.id.clone() },
        name: if draft.name.trim().is_empty() { "预览".to_string() } else { draft.name.clone() },
        tagline: None,
        palette: ThemePaletteHex {
            background: draft.background.clone(),
            text: draft.text.clone(),
            primary: draft.primary.clone(),
            success: draft.success.clone(),
            warning: draft.warning.clone(),
            danger: draft.danger.clone(),
        },
        assets: ThemeAssets {
            cover: draft.cover_relative.clone(),
            background: draft.background_relative.clone(),
            chat_header: draft.chat_header_relative.clone(),
            chat_body: draft.chat_body_relative.clone(),
            chat_input: draft.chat_input_relative.clone(),
        },
        chrome: ThemeChrome {
            input_bg: non_empty_opt(&draft.chrome_input_bg),
            input_border: non_empty_opt(&draft.chrome_input_border),
            input_text: non_empty_opt(&draft.chrome_input_text),
            panel_bg: non_empty_opt(&draft.chrome_panel_bg),
            panel_border: non_empty_opt(&draft.chrome_panel_border),
            button_bg: non_empty_opt(&draft.chrome_button_bg),
            button_text: non_empty_opt(&draft.chrome_button_text),
            radius_px: draft.chrome_radius_px.trim().parse().ok(),
            panel_alpha: draft.chrome_panel_alpha.trim().parse().ok(),
            chat_header_alpha: draft.chrome_chat_header_alpha.trim().parse().ok(),
            chat_body_alpha: draft.chrome_chat_body_alpha.trim().parse().ok(),
            chat_input_alpha: draft.chrome_chat_input_alpha.trim().parse().ok(),
            chat_header_image_fit: non_empty_opt(&draft.chrome_chat_header_image_fit),
            chat_body_image_fit: non_empty_opt(&draft.chrome_chat_body_image_fit),
            chat_input_image_fit: non_empty_opt(&draft.chrome_chat_input_image_fit),
        },
    }
}

fn non_empty_opt(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn status_chip(label: &'static str, color: Color, text_color: Color) -> Element<'static, Message> {
    container(text(label).size(11).color(text_color))
        .padding([4, 10])
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(color)),
            border: Border { radius: 6.0.into(), width: 0.0, color: Color::TRANSPARENT },
            ..Default::default()
        })
        .into()
}

/// 用当前草稿渲染小型配色预览。
pub(crate) fn theme_preview_panel(draft: &ThemeEditorDraft) -> Element<'static, Message> {
    let definition = definition_from_draft(draft);
    let preview_theme = to_iced_theme(&definition).ok();
    let palette = preview_theme.as_ref().map(|theme| theme.palette()).unwrap_or_else(|| {
        iced::theme::Palette {
            background: parse_hex_or(&draft.background, Color::from_rgb8(15, 23, 42)),
            text: parse_hex_or(&draft.text, Color::from_rgb8(226, 232, 240)),
            primary: parse_hex_or(&draft.primary, Color::from_rgb8(56, 189, 248)),
            success: parse_hex_or(&draft.success, Color::from_rgb8(34, 197, 94)),
            warning: parse_hex_or(&draft.warning, Color::from_rgb8(245, 158, 11)),
            danger: parse_hex_or(&draft.danger, Color::from_rgb8(239, 68, 68)),
        }
    });

    let bg = palette.background;
    let fg = palette.text;
    let primary = palette.primary;
    let success = palette.success;
    let warning = palette.warning;
    let danger = palette.danger;

    let title = if draft.name.trim().is_empty() {
        "未命名主题".to_string()
    } else {
        draft.name.clone()
    };

    let sample = column![
        text(title).size(15).color(fg),
        text("这是一段预览文本，用于检查背景与文字对比。").size(12).color(fg),
        button(text("主色按钮").size(12).color(bg)).padding([8, 14]).style(
            move |_theme: &Theme, status: iced::widget::button::Status| {
                let alpha = match status {
                    iced::widget::button::Status::Hovered => 0.92,
                    iced::widget::button::Status::Pressed => 0.84,
                    _ => 1.0,
                };
                iced::widget::button::Style {
                    background: Some(Background::Color(Color { a: alpha, ..primary })),
                    text_color: bg,
                    border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
                    ..Default::default()
                }
            }
        ),
        row![
            status_chip("成功", success, bg),
            status_chip("警告", warning, bg),
            status_chip("危险", danger, bg),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .width(Length::Fill);

    container(sample)
        .padding(16)
        .width(Length::Fill)
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: Color::from_rgba8(148, 163, 184, 0.35),
            },
            ..Default::default()
        })
        .into()
}
