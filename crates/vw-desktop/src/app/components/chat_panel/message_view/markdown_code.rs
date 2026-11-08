//! Assistant code cards and recognition of standalone package-manager commands.

use iced::widget::{Space, button, column, container, row, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use crate::app::assets::Icon;
use crate::app::i18n::I18n;
use crate::app::{App, Message};

use super::super::tool_text_support::chat_text_font;
use super::super::utils::{icon_svg, is_recent_copy};
use super::markdown::markdown_visual_contract;
use super::parse::hash_chat_content;
use super::styles::{
    is_dark_theme, message_action_button_style, message_action_icon_color, message_meta_text_color,
};

pub(super) fn is_package_command(line: &str) -> bool {
    let mut words = line.trim_start().split_ascii_whitespace();
    let Some("pnpm" | "npm" | "yarn" | "bun") = words.next() else {
        return false;
    };
    let Some(action) = words.next() else {
        return false;
    };
    matches!(
        action,
        "add"
            | "install"
            | "remove"
            | "update"
            | "run"
            | "exec"
            | "dlx"
            | "dev"
            | "start"
            | "build"
            | "test"
            | "lint"
            | "typecheck"
            | "preview"
    ) || (action.contains(':')
        && action.split(':').all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        }))
}

pub(super) fn code_block_view<'a>(
    app: &App,
    language: Option<&'a str>,
    code: &'a str,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let copied =
        app.last_copied_code_hash.is_some() && is_recent_copy(app, hash_chat_content(code));
    let icon = if copied { Icon::Check } else { Icon::Copy };
    let label = if copied { i18n.__("已复制") } else { i18n.__("复制") };
    let copy = button(
        row![
            icon_svg(icon).width(12).height(12).style(move |theme, status| svg::Style {
                color: Some(message_action_icon_color(theme, status, copied, false, false)),
            }),
            text(label).size(12).font(chat_text_font()),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .padding([4, 8])
    .style(|theme, status| message_action_button_style(theme, status, false));
    let copy = if code.is_empty() {
        copy
    } else {
        copy.on_press_with(move || Message::CopyCode(code.to_owned()))
    };

    let header = row![
        text(language.unwrap_or_else(|| i18n.__("代码")))
            .size(12)
            .font(chat_text_font())
            .style(|theme| text::Style { color: Some(message_meta_text_color(theme, false)) }),
        Space::new().width(Length::Fill),
        copy,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(column![header, body].spacing(6))
        .padding(markdown_visual_contract().code_padding)
        .width(Length::Fill)
        .style(code_block_style)
        .into()
}

fn code_block_style(theme: &Theme) -> container::Style {
    let contract = markdown_visual_contract();
    let is_dark = is_dark_theme(theme);
    container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(255, 255, 255, 0.06)
        } else {
            Color::from_rgba8(15, 23, 42, 0.05)
        })),
        border: Border {
            width: contract.code_border_width,
            color: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.10)
            } else {
                Color::from_rgba8(15, 23, 42, 0.10)
            },
            radius: contract.code_corner_radius.into(),
        },
        ..Default::default()
    }
}
