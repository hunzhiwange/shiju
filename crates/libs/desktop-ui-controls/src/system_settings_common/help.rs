use desktop_assets::assets::Icon;
use super::icons::{icon_svg, icon_btn, icon_btn_tooltip_style};
use super::theme::is_dark_theme;
use iced::widget::{Row, Text, button, container, mouse_area, row, svg};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector, mouse};
use std::hash::{Hash, Hasher};
pub fn settings_close_button<Message: Clone + 'static>(on_press: Message) -> Element<'static, Message> {
    button(
        container(icon_svg(Icon::X, 12.0).style(settings_close_icon_style))
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .padding(0)
    .style(settings_close_button_style)
    .on_press(on_press)
    .into()
}
pub fn settings_close_icon_style(theme: &iced::Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(if is_dark_theme(theme) {
            theme.palette().text.scale_alpha(0.72)
        } else {
            theme.palette().text.scale_alpha(0.82)
        }),
    }
}
pub fn settings_close_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => {
            Some(palette.background.weak.color.scale_alpha(0.72).into())
        }
        iced::widget::button::Status::Pressed => {
            Some(palette.background.strong.color.scale_alpha(0.82).into())
        }
        _ => None,
    };

    iced::widget::button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 999.0.into() },
        ..Default::default()
    }
}
pub fn settings_help_button<Message: Clone + 'static>(on_press: Message) -> Element<'static, Message> {
    settings_help_button_with_locale(
        config_ui::DesktopLocale::SimplifiedChinese,
        on_press,
    )
}
pub fn settings_help_button_with_locale<Message: Clone + 'static>(
    locale: config_ui::DesktopLocale,
    on_press: Message,
) -> Element<'static, Message> {
    icon_btn(Icon::QuestionCircle, desktop_i18n::I18n::new(locale).__("帮助"), Some(on_press))
}
pub fn settings_help_copy_state(
    last_copied_code_hash: Option<u64>,
    help_text: &str,
) -> (Icon, &'static str) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    help_text.hash(&mut hasher);
    let help_text_hash = hasher.finish();

    if last_copied_code_hash == Some(help_text_hash) {
        (Icon::Check, "已复制")
    } else {
        (Icon::Copy, "复制帮助")
    }
}
pub fn settings_hover_help_label<'a, Message: Clone + 'a>(label: Text<'a>, content: Element<'a, Message>) -> Row<'a, Message> {
    let icon = mouse_area(
        container(icon_svg(Icon::QuestionCircle, 14.0).style(settings_close_icon_style))
            .center_x(22)
            .center_y(22),
    )
    .interaction(mouse::Interaction::Pointer);
    let card = container(content)
        .width(300)
        .padding([10, 12])
        .style(|theme| container::Style {
            shadow: Shadow {
                color: Color::BLACK.scale_alpha(0.22),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..icon_btn_tooltip_style(theme)
        });
    row![label, SideOverlay::new(icon, card).hover_and_click().gap(4.0).min_y(8.0)]
        .spacing(4)
        .align_y(Alignment::Center)
}
pub fn with_settings_help_modal<'a, Message: Clone + 'static>(base: Element<'a, Message>, title: String, help_text: String, copy_icon: Icon, copy_tip: &'static str, copy_message: Message, close_message: Message) -> Element<'a, Message> {
    let close_btn = settings_close_button(close_message.clone());
    let copy_btn = icon_btn(copy_icon, copy_tip, Some(copy_message));
    let header = super::panels::settings_modal_header(
        iced::widget::text(title).size(16),
        row![copy_btn, close_btn].spacing(10).align_y(Alignment::Center),
    );
    let body = iced::widget::text(help_text).size(13).style(super::styles::settings_muted_text_style);
    let card = super::panels::settings_modal_layout(header, body, None).width(720).height(560);

    super::panels::settings_modal_overlay(Some(base), close_message, card)
}