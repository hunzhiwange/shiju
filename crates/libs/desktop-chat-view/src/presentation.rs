use iced::{Color, Theme, Length};
use iced::widget::Svg;
use desktop_assets::assets::{self, Icon};
pub fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}
pub fn bold_font() -> iced::Font { iced::Font { weight: iced::font::Weight::Bold, ..Default::default() } }
pub fn icon_svg(icon: Icon) -> Svg<'static> { Svg::new(assets::get_icon(icon)).width(Length::Fixed(14.0)).height(Length::Fixed(14.0)) }
pub fn chat_secondary_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.92)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.90)
    }
}
pub fn chat_secondary_subtle_text_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) {
        theme.palette().text.scale_alpha(0.56)
    } else {
        theme.extended_palette().secondary.base.text.scale_alpha(0.58)
    }
}
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
