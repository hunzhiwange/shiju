//! Skills controls use a warm accent and neutral, theme-aware surfaces.
use desktop_assets::assets::Icon;
use desktop_ui_controls::system_settings_common::{
    icon_svg, icons::icon_btn_svg_style, primary_action_btn_style, settings_checkbox_style,
    settings_modal_card_style, settings_text_input_style, theme::is_dark_theme,
};
use iced::{
    Background, Border, Color, Element, Theme, Vector,
    widget::{
        button, checkbox as check_widget, container, radio as radio_widget, stack, text_input,
    },
};

fn accent(theme: &Theme) -> Color {
    if is_dark_theme(theme) { Color::from_rgb8(255, 143, 70) } else { Color::from_rgb8(198, 77, 9) }
}

pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = primary_action_btn_style(theme, status);
    let color = match status {
        button::Status::Active => accent(theme),
        button::Status::Hovered if is_dark_theme(theme) => Color::from_rgb8(255, 162, 101),
        button::Status::Hovered => Color::from_rgb8(175, 62, 5),
        button::Status::Pressed if is_dark_theme(theme) => Color::from_rgb8(232, 123, 50),
        button::Status::Pressed => Color::from_rgb8(153, 52, 3),
        button::Status::Disabled => return style,
    };
    style.background = Some(color.into());
    style.text_color =
        if is_dark_theme(theme) { Color::from_rgb8(34, 21, 14) } else { Color::WHITE };
    style.border.radius = 7.0.into();
    style
}

pub fn checkbox(theme: &Theme, status: check_widget::Status) -> check_widget::Style {
    let mut style = settings_checkbox_style(theme, status);
    style.border.radius = 4.0.into();
    if matches!(
        status,
        check_widget::Status::Active { is_checked: true }
            | check_widget::Status::Hovered { is_checked: true }
    ) {
        let color =
            if is_dark_theme(theme) { accent(theme) } else { Color::from_rgb8(235, 102, 17) };
        style.background = color.into();
        style.border.color = color;
        if is_dark_theme(theme) {
            style.icon_color = Color::from_rgb8(34, 21, 14);
        }
    }
    style
}

pub fn radio(theme: &Theme, status: radio_widget::Status) -> radio_widget::Style {
    let mut style = radio_widget::default(theme, status);
    let selected = matches!(
        status,
        radio_widget::Status::Active { is_selected: true }
            | radio_widget::Status::Hovered { is_selected: true }
    );
    style.border_width = if selected { 0.0 } else { 1.5 };
    style.border_color = divider(theme);
    let dark = is_dark_theme(theme);
    let fill = if dark { accent(theme) } else { Color::from_rgb8(235, 102, 17) };
    style.dot_color = if dark { theme.palette().background } else { Color::WHITE };
    style.background = if selected { fill } else { Color::TRANSPARENT }.into();
    style
}

pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = settings_text_input_style(theme, status);
    style.border.radius = 7.0.into();
    if matches!(status, text_input::Status::Focused { .. }) {
        style.border.color = accent(theme);
    } else {
        style.border.color = divider(theme);
    }
    style
}

fn divider(theme: &Theme) -> Color {
    theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.28 } else { 0.19 })
}

pub fn table(theme: &Theme) -> container::Style {
    container::Style {
        border: Border { color: divider(theme), width: 1.0, radius: 10.0.into() },
        ..Default::default()
    }
}

pub fn toolbar(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(if is_dark_theme(theme) {
            theme.extended_palette().background.weak.color
        } else {
            Color::from_rgb8(244, 244, 246)
        })),
        ..Default::default()
    }
}

pub fn dialog(theme: &Theme) -> container::Style {
    let mut style = settings_modal_card_style(theme);
    style.background =
        Some(if is_dark_theme(theme) { theme.palette().background } else { Color::WHITE }.into());
    style.border = Border { color: divider(theme), width: 1.0, radius: 15.0.into() };
    style.shadow.offset = Vector::new(0.0, 14.0);
    style.shadow.blur_radius = 35.0;
    style
}

pub fn popover(theme: &Theme) -> container::Style {
    let mut style = dialog(theme);
    style.border.radius = 9.0.into();
    style.shadow.offset = Vector::new(0.0, 4.0);
    style.shadow.blur_radius = 12.0;
    style
}

pub fn search<'a, Message: Clone + 'static>(
    field: text_input::TextInput<'a, Message>,
) -> Element<'a, Message> {
    stack![
        field.padding(iced::Padding::from([7, 9]).left(30)).style(input),
        container(icon_svg(Icon::Search, 14.0).style(icon_btn_svg_style))
            .padding(iced::Padding::ZERO.left(9))
            .center_y(32),
    ]
    .into()
}
