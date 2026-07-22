use iced::widget::text;
use iced::{Element, Length};

use crate::app::Message;
use crate::app::assets::Icon;

use super::disabled_buttons::{
    disabled_icon_button, disabled_square_content_button, disabled_square_content_button_tiny,
    disabled_square_icon_button, disabled_square_icon_button_small,
    disabled_square_icon_button_tiny,
};

fn assert_tooltip_wrapped_size(element: &Element<'_, Message>, width: Length, height: Length) {
    let size = element.as_widget().size();

    assert_eq!(size.width, width);
    assert_eq!(size.height, height);
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn disabled_icon_button_is_content_sized_and_tooltip_wrapped() {
    let element = disabled_icon_button(Icon::Plus, "add disabled".into());

    assert_tooltip_wrapped_size(&element, Length::Shrink, Length::Shrink);
}

#[test]
fn disabled_square_icon_buttons_keep_standard_dimensions() {
    let regular = disabled_square_icon_button(Icon::Trash, "trash disabled".into());
    let small_icon = disabled_square_icon_button_small(Icon::Image, "image disabled".into());

    for element in [&regular, &small_icon] {
        assert_tooltip_wrapped_size(element, Length::Fixed(32.0), Length::Fixed(32.0));
    }
}

#[test]
fn disabled_square_icon_button_tiny_keeps_compact_dimensions() {
    let element = disabled_square_icon_button_tiny(Icon::FileText, String::new());

    assert_tooltip_wrapped_size(&element, Length::Fixed(28.0), Length::Fixed(28.0));
}

#[test]
fn disabled_square_content_button_keeps_standard_dimensions() {
    let element = disabled_square_content_button(text("A"), "content disabled".into());

    assert_tooltip_wrapped_size(&element, Length::Fixed(32.0), Length::Fixed(32.0));
}

#[test]
fn disabled_square_content_button_tiny_keeps_compact_dimensions() {
    let element = disabled_square_content_button_tiny(text("B"), "暂不可用".into());

    assert_tooltip_wrapped_size(&element, Length::Fixed(28.0), Length::Fixed(28.0));
}

#[test]
fn disabled_content_buttons_accept_borrowed_labels() {
    let label = String::from("borrowed");
    let regular = disabled_square_content_button(text(label.as_str()), String::new());
    let tiny = disabled_square_content_button_tiny(text(label.as_str()), "disabled".into());

    assert_tooltip_wrapped_size(&regular, Length::Fixed(32.0), Length::Fixed(32.0));
    assert_tooltip_wrapped_size(&tiny, Length::Fixed(28.0), Length::Fixed(28.0));
}
