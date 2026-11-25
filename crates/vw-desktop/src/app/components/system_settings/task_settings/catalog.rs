use crate::app::Message;
use crate::app::components::system_settings_common::*;
use crate::app::i18n::I18n;
use desktop_assets::assets::Icon;
use desktop_ui_controls::color_picker::{self, parse_color, render_opaque_color_picker};
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use desktop_ui_controls::system_settings_common::panels::settings_modal_overlay_with_escape;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Color, Element, Length, Theme};

pub(super) const DEFAULT_COLOR: &str = "#4D7CFE";
const PRESETS: [&str; 8] =
    [DEFAULT_COLOR, "#14B8A6", "#22C55E", "#EAB308", "#F97316", "#EF4444", "#EC4899", "#64748B"];

pub(super) fn action(label: &str, message: Option<Message>) -> button::Button<'_, Message> {
    button(text(label).size(13))
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe(message)
}

pub(super) fn primary(
    label: &str,
    icon: Option<Icon>,
    message: Option<Message>,
) -> button::Button<'_, Message> {
    let content = row(icon.into_iter().map(|icon| primary_glyph(icon, 14.0).into()))
        .push(text(label).size(13))
        .align_y(Alignment::Center)
        .spacing(6);
    button(content).padding([9, 14]).style(primary_action_btn_style).on_press_maybe(message)
}

pub(in crate::app) fn glyph(icon: Icon, size: f32) -> iced::widget::Svg<'static> {
    icon_svg(icon, size).style(icon_btn_svg_style)
}

pub(super) fn primary_glyph(icon: Icon, size: f32) -> iced::widget::Svg<'static> {
    icon_svg(icon, size).style(|theme, _| iced::widget::svg::Style {
        color: Some(primary_action_btn_style(theme, button::Status::Active).text_color),
    })
}

pub(super) fn menu<'a>(
    actions: Vec<(&'static str, Message)>,
    enabled: bool,
) -> Element<'a, Message> {
    let labels = actions.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    settings_select(0..actions.len(), None, move |index| actions[index].1.clone())
        .format_labels(|index| labels[*index].to_owned())
        .menu_icon(Icon::DotsThreeHorizontal, 140.0)
        .width(36)
        .disabled(!enabled)
        .into()
}

pub(super) fn cell<'a>(text_value: impl Into<String>) -> iced::widget::Text<'a> {
    text(text_value.into()).size(13).style(settings_muted_text_style)
}

pub(super) fn preview(value: &str) -> String {
    let mut chars = value.trim().chars().map(|ch| if ch.is_whitespace() { ' ' } else { ch });
    let mut preview: String = chars.by_ref().take(100).collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

pub(super) fn record<'a>(
    cells: impl IntoIterator<Item = (Element<'a, Message>, u16)>,
    menu: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut line = row(cells
        .into_iter()
        .map(|(cell, portion)| container(cell).width(Length::FillPortion(portion)).into()))
    .spacing(12)
    .align_y(Alignment::Center);
    line = line.push(container(menu.unwrap_or_else(|| Space::new().into())).width(36));
    container(line).width(Length::Fill).padding([10, 0]).into()
}

pub(super) fn toolbar<'a>(
    search: Element<'a, Message>,
    actions: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        container(search).width(Length::Fill).max_width(340),
        Space::new().width(Length::Fill),
        actions.into(),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn modal<'a>(
    i18n: I18n,
    base: Element<'a, Message>,
    title: &'static str,
    body: impl Into<Element<'a, Message>>,
    height: f32,
    close: Message,
    save: Option<Message>,
) -> Element<'a, Message> {
    let header = settings_modal_header(text(title).size(19), settings_close_button(close.clone()));
    let footer = row![
        Space::new().width(Length::Fill),
        action(i18n.__("取消"), Some(close.clone())),
        primary(i18n.__("保存"), None, save),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let footer = column![settings_divider(), footer].spacing(12);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(760)
        .height(Length::Fill)
        .max_height(height);
    settings_modal_overlay_with_escape(base, close, card)
}

pub(in crate::app) fn palette_color(value: &str) -> Option<Color> {
    let value = value.trim();
    (value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
    .then(|| parse_color(value))
    .flatten()
}

pub(in crate::app) fn color_dot<'a>(color: Color, size: f32) -> Element<'a, Message> {
    container(Space::new().width(size).height(size))
        .style(move |theme: &Theme| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: (size / 2.0).into(),
                color: theme.palette().text.scale_alpha(0.16),
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
}

pub(super) fn color_control<'a>(
    i18n: I18n,
    value: &str,
    show_hex: bool,
    open: bool,
    open_message: Option<Message>,
    close: Message,
    change: impl Fn(String) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let color = palette_color(value).unwrap_or_else(|| Color::from_rgb8(77, 124, 254));
    let mut face = row![color_dot(color, 16.0)].spacing(8).align_y(Alignment::Center);
    if show_hex {
        face = face.push(cell(value)).push(glyph(Icon::ChevronDown, 11.0));
    }
    let trigger =
        button(face).style(rounded_action_btn_style).padding([8, 10]).on_press_maybe(open_message);
    if !open {
        return trigger.into();
    }
    let send_color = change.clone();
    let picker =
        render_opaque_color_picker(color, color_picker::Message::ColorChanged).map(move |event| {
            match event {
                color_picker::Message::ColorChanged(color) => {
                    let hex = color_picker::format_rgba_to_hex(color.r, color.g, color.b, 1.0);
                    send_color(hex[..7].to_owned())
                }
                _ => Message::None,
            }
        });
    let swatches = row(PRESETS.into_iter().filter_map(|preset| {
        let color = palette_color(preset)?;
        Some(
            button(color_dot(color, 18.0))
                .style(rounded_action_btn_style)
                .padding(3)
                .on_press(change(preset.to_owned()))
                .into(),
        )
    }))
    .spacing(4)
    .wrap();
    let popup = container(
        column![
            picker,
            settings_text_input("#RRGGBB", value).on_input(change),
            cell(i18n.__("预设颜色")),
            swatches,
        ]
        .spacing(8),
    )
    .width(274)
    .padding(12)
    .style(settings_modal_card_style);

    BelowOverlay::new(trigger, popup)
        .show(true)
        .gap(6.0)
        .within_window()
        .dismiss_on_escape()
        .on_close(close)
        .into()
}

pub(super) fn matches_search(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(query)
}
