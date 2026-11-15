use iced::{Element, Length, Theme};
use iced::widget::{container, stack};
pub fn settings_panel_width(width: f32) -> f32 {
 if width.is_finite() { width.clamp(200.0, 800.0) } else { 370.0 }
}
pub fn view<'a, M: 'a>(base: Element<'a, M>, drag: Element<'a, M>, picker: Element<'a, M>, style: fn(&Theme) -> container::Style) -> Element<'a, M> {
 container(stack![base, drag, picker]).width(Length::Fill).height(Length::Fill).style(style).into()
}
