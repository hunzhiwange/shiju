//! Desktop message mapping for the shared color controls.
use iced::{Color, Element};
use crate::app::Message;
use desktop_ui_controls::color_picker as controls;
pub use controls::{ColorFormat, Hsv, format_rgba_to_css, format_rgba_to_hex,
    hsla_to_rgba, parse_color, parse_css_color, rgba_to_hsla};

pub fn render_color_picker<'a, F, F2>(
    color: Color, format: ColorFormat, picking: bool,
    on_change: F, on_format_change: F2,
    on_eyedropper: impl Fn() -> Message + Clone + 'static,
) -> Element<'a, Message>
where
    F: Fn(Color) -> Message + Clone + 'static,
    F2: Fn(ColorFormat) -> Message + 'static,
{
    controls::render_color_picker(color, format, picking,
        controls::Message::ColorChanged, controls::Message::FormatChanged,
        || controls::Message::Eyedropper,
    ).map(move |message| match message {
        controls::Message::None => Message::None,
        controls::Message::ColorChanged(color) => on_change(color),
        controls::Message::FormatChanged(format) => on_format_change(format),
        controls::Message::Eyedropper => on_eyedropper(),
    })
}

pub fn render_mini_color_picker<'a, F, F2>(
    color: Color, format: ColorFormat, picking: bool,
    on_change: F, on_format_change: F2,
    on_eyedropper: impl Fn() -> Message + Clone + 'static,
) -> Element<'a, Message>
where
    F: Fn(Color) -> Message + Clone + 'static,
    F2: Fn(ColorFormat) -> Message + 'static,
{
    controls::render_mini_color_picker(color, format, picking,
        controls::Message::ColorChanged, controls::Message::FormatChanged,
        || controls::Message::Eyedropper,
    ).map(move |message| match message {
        controls::Message::None => Message::None,
        controls::Message::ColorChanged(color) => on_change(color),
        controls::Message::FormatChanged(format) => on_format_change(format),
        controls::Message::Eyedropper => on_eyedropper(),
    })
}
